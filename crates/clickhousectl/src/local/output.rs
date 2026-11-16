//! Structured output types for local commands.
//!
//! Successful output types support both JSON serialization and human-readable
//! display. Runtime failures use the stable envelope below, which keeps
//! foreign text out of `message` and in `details`.

use crate::error::{
    Error, ManagedClientError, ManagedClientErrorKind, ManagedClientSelection, NO_PROJECT_FOUND,
    NetworkStage, PortKind, ProjectServerCommand, ProjectServerNotFound, ProjectServerStateMissing,
};
use crate::init::{ProjectRoot, ProjectRootKind};
use serde::Serialize;
use std::fmt;
use std::path::Path;
use tabled::{Table, Tabled, settings::Style};

const ABSENT: &str = "-";

fn or_absent<T: fmt::Display>(value: Option<T>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| ABSENT.to_string())
}

/// Stable codes for local runtime failures. New codes may be added, but
/// existing spellings and meanings are part of the machine-output contract.
///
/// [`Self::as_str`] is the only spelling: the JSON envelope serializes through
/// it and telemetry records it as `failure_kind`, so the two cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalErrorCode {
    ManagedClientServerNotFound,
    ManagedClientServerNotRunning,
    ManagedClientBinaryNotFound,
    ManagedClientProjectStateUnavailable,
    ServerNotFound,
    ServerSelectionRequired,
    ServerNotRunning,
    ServerRunning,
    InvalidServerName,
    ConfigNotFound,
    InvalidConfigName,
    UdfDefinitionInvalid,
    UdfSourceInvalid,
    UdfNotFound,
    UdfInterpreterNotFound,
    /// The local server answered a UDF statement with an error. The message
    /// is a fixed summary; the server's text is in `details`.
    UdfQueryFailed,
    /// Native UDFs are Linux amd64/arm64 binaries; this host cannot run them.
    UdfRuntimeUnsupported,
    /// A running server accepted a deployment but never loaded the function;
    /// the message names the server log.
    UdfNotLoaded,
    /// The running server rejected a deployed definition on reload; the
    /// files stay deployed and block every reload until fixed or removed.
    UdfRejected,
    /// The local server's HTTP port did not answer a UDF statement. The HTTP
    /// client's text is in `details`.
    UdfServerUnreachable,
    /// A function reload by `remove` (after deleting the files) or `reload`
    /// was rejected because a deployed function is broken; it blocks every
    /// reload on the server until fixed or removed.
    UdfReloadBlocked,
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
    /// `local install --binary` was given a file that does not run as
    /// ClickHouse or report a ClickHouse version.
    NotAClickHouseBinary,
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
    SqlInputOpenFailed,
    SqlInputReadFailed,
    /// A managed server metadata file contains invalid JSON. The structured
    /// body names the file and gives conservative recovery guidance without
    /// exposing serde's source text.
    ServerMetadataInvalid,
    /// `CLICKHOUSECTL_PROJECT_DIR` names no usable project root.
    InvalidProjectDir,
    IoError,
    LocalError,
}

impl LocalErrorCode {
    /// The wire value. Literal strings only, so neither the JSON envelope nor
    /// telemetry can carry user data through this field.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ManagedClientServerNotFound => "managed_client_server_not_found",
            Self::ManagedClientServerNotRunning => "managed_client_server_not_running",
            Self::ManagedClientBinaryNotFound => "managed_client_binary_not_found",
            Self::ManagedClientProjectStateUnavailable => {
                "managed_client_project_state_unavailable"
            }
            Self::ServerNotFound => "server_not_found",
            Self::ServerSelectionRequired => "server_selection_required",
            Self::ServerNotRunning => "server_not_running",
            Self::ServerRunning => "server_running",
            Self::InvalidServerName => "invalid_server_name",
            Self::ConfigNotFound => "config_not_found",
            Self::InvalidConfigName => "invalid_config_name",
            Self::UdfDefinitionInvalid => "udf_definition_invalid",
            Self::UdfSourceInvalid => "udf_source_invalid",
            Self::UdfNotFound => "udf_not_found",
            Self::UdfInterpreterNotFound => "udf_interpreter_not_found",
            Self::UdfQueryFailed => "udf_query_failed",
            Self::UdfRuntimeUnsupported => "udf_runtime_unsupported",
            Self::UdfNotLoaded => "udf_not_loaded",
            Self::UdfRejected => "udf_rejected",
            Self::UdfServerUnreachable => "udf_server_unreachable",
            Self::UdfReloadBlocked => "udf_reload_blocked",
            Self::InvalidVersion => "invalid_version",
            Self::VersionNotInstalled => "version_not_installed",
            Self::BinaryNotLaunchable => "binary_not_launchable",
            Self::VersionSelectionRequired => "version_selection_required",
            Self::VersionAlreadyInstalled => "version_already_installed",
            Self::NotAClickHouseBinary => "not_a_clickhouse_binary",
            Self::VersionUnavailable => "version_unavailable",
            Self::VersionIsDefault => "version_is_default",
            Self::UnsupportedClientVersion => "unsupported_client_version",
            Self::UnsupportedPlatform => "unsupported_platform",
            Self::PortInUse => "port_in_use",
            Self::StartupExit => "startup_exit",
            Self::StartupTimeout => "startup_timeout",
            Self::DownloadFailed => "download_failed",
            Self::NetworkError => "network_error",
            Self::DockerUnavailable => "docker_unavailable",
            Self::DockerError => "docker_error",
            Self::ContainerNameConflict => "container_name_conflict",
            Self::PostgresError => "postgres_error",
            Self::SqlInputOpenFailed => "sql_input_open_failed",
            Self::SqlInputReadFailed => "sql_input_read_failed",
            Self::ServerMetadataInvalid => "server_metadata_invalid",
            Self::InvalidProjectDir => "invalid_project_dir",
            Self::IoError => "io_error",
            Self::LocalError => "local_error",
        }
    }
}

impl Serialize for LocalErrorCode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalErrorDetail {
    code: LocalErrorCode,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    /// The foreign text (ClickHouse's response, a managed server's stderr or
    /// log tail, the Docker daemon's, the HTTP client's, the OS's or serde's
    /// error) behind a failure whose `message` is this crate's own summary.
    /// Only the variants that opt in via [`Mapping::details`] carry it.
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

/// Removing the broken function unblocks a server's reloads; with no culprit
/// identified, `list` shows which deployed functions are not loaded.
fn udf_culprit_command(culprit: Option<&String>, server: &str) -> String {
    match culprit {
        Some(culprit) => format!("clickhousectl local udf remove {culprit} --server {server}"),
        None => format!("clickhousectl local udf list --server {server}"),
    }
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
    details: Option<String>,
}

impl Mapping {
    /// The JSON message is the error's `Display` text, so machine output
    /// carries exactly the detail and remediation human output prints.
    fn parity(code: LocalErrorCode) -> Self {
        Self {
            code,
            command: None,
            redacted: None,
            details: None,
        }
    }

    /// The JSON message is `message`, not the error's `Display` text. For
    /// errors whose text interpolates foreign output.
    fn redacted(code: LocalErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            command: None,
            redacted: Some(message.into()),
            details: None,
        }
    }

    /// A safe, runnable recovery command for this failure.
    fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// Carry the foreign text the summary left out in a separate `details`
    /// field: it is the actionable part (what ClickHouse, Docker, the OS or
    /// serde objected to). Never reaches telemetry: only this DTO holds it.
    fn details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    fn into_detail(self, error: &Error) -> LocalErrorDetail {
        LocalErrorDetail {
            code: self.code,
            message: self.redacted.unwrap_or_else(|| error.to_string()),
            command: self.command,
            details: self.details,
        }
    }
}

/// The I/O group: one fixed summary, with the OS's or serde's text in
/// `details`.
fn io_mapping(source: &dyn std::error::Error) -> Mapping {
    Mapping::redacted(LocalErrorCode::IoError, "Local I/O operation failed")
        .details(source.to_string())
}

/// How the project in `project_scope.path` was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LocalProjectScopeKind {
    /// A `.clickhouse/` directory marks `path` as the project root: the
    /// current directory's own, or the nearest one in a parent directory.
    ProjectRoot,
    /// No project was found, so the current directory was used.
    CurrentDirectory,
    /// `CLICKHOUSECTL_PROJECT_DIR` named `path`; discovery did not run.
    EnvOverride,
}

impl From<ProjectRootKind> for LocalProjectScopeKind {
    fn from(kind: ProjectRootKind) -> Self {
        match kind {
            ProjectRootKind::Marked => Self::ProjectRoot,
            ProjectRootKind::Unmarked => Self::CurrentDirectory,
            ProjectRootKind::Override => Self::EnvOverride,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ServerProjectScope {
    kind: LocalProjectScopeKind,
    path: String,
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
struct ServerMetadataParseErrorDetail {
    code: LocalErrorCode,
    message: &'static str,
    path: String,
    guidance: Vec<LocalGuidance>,
}

/// What one `guidance[]` step does: a stable, closed vocabulary that lets an
/// agent act on a step without reading its `message`. New actions may be
/// added; existing spellings are part of the machine-output contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocalGuidanceAction {
    ListProjectServers,
    ListGlobalServers,
    ReturnToProjectRoot,
    StopGlobalProjectServer,
    StartServer,
    InstallVersion,
    ConnectDirectly,
    RepairProjectState,
    RepairMetadata,
    VerifyPostgresContainer,
}

/// One recovery step. Every local error and list output that carries
/// `guidance` uses this one shape, so `action` is always present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct LocalGuidance {
    action: LocalGuidanceAction,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<&'static str>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ManagedClientErrorDetail {
    code: LocalErrorCode,
    message: &'static str,
    project_scope: ServerProjectScope,
    server: LocalManagedServer,
    guidance: Vec<LocalGuidance>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ProjectServerErrorDetail {
    code: LocalErrorCode,
    message: String,
    project_scope: ServerProjectScope,
    server: LocalProjectServer,
    guidance: Vec<LocalGuidance>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ProjectServerStateMissingDetail {
    code: LocalErrorCode,
    message: &'static str,
    project_scope: ServerProjectScope,
    guidance: Vec<LocalGuidance>,
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
    ServerMetadataParse(ServerMetadataParseErrorDetail),
}

impl LocalErrorBody {
    fn code(&self) -> LocalErrorCode {
        match self {
            Self::General(detail) => detail.code,
            Self::ManagedClient(detail) => detail.code,
            Self::ProjectServer(detail) => detail.code,
            Self::ProjectServerStateMissing(detail) => detail.code,
            Self::ServerMetadataParse(detail) => detail.code,
        }
    }

    /// Classify one local runtime failure.
    ///
    /// Two rules govern the `message` field, and every arm below picks one
    /// deliberately:
    ///
    /// * **Parity** ([`Mapping::parity`]) — the error's own human text is
    ///   rendered verbatim, so a JSON consumer gets exactly the detail and
    ///   remediation human mode prints. This is the default: these messages are
    ///   composed by this crate from its own fields.
    /// * **Summary** ([`Mapping::redacted`]) — a curated summary replaces
    ///   text that interpolates foreign output (subprocess stderr, Docker
    ///   daemon or OS/serde source strings, download bodies), so `message`
    ///   stays this crate's own text. The foreign text is what a consumer
    ///   acts on, and human mode prints it to the same stderr, so these arms
    ///   also opt into [`Mapping::details`], which carries it in the
    ///   envelope's `details` field (#1072). Telemetry never sees it: it
    ///   records only the code. A variant that must withhold its foreign
    ///   text says why at its arm.
    ///
    /// The match is exhaustive on purpose: a new [`Error`] variant must be
    /// classified here rather than silently collapsing to
    /// `local_error`/"Local command failed".
    fn from_error(error: &Error) -> Self {
        let mapping = match error {
            // ── structured bodies (their own DTOs, not code/message/command) ─
            Error::ManagedClient(managed) => {
                return Self::ManagedClient(ManagedClientErrorDetail::from_error(managed));
            }
            Error::ProjectServerNotFound(not_found) => {
                return Self::ProjectServer(ProjectServerErrorDetail::from_error(not_found));
            }
            Error::ProjectServerStateMissing(missing) => {
                return Self::ProjectServerStateMissing(
                    ProjectServerStateMissingDetail::from_error(missing),
                );
            }
            Error::ServerMetadataParse { path, .. } => {
                return Self::ServerMetadataParse(ServerMetadataParseErrorDetail::from_path(path));
            }
            // The rollback note is a cleanup detail on top of the failure that
            // actually stopped the command; classify by that primary failure.
            Error::PostgresStartupRollback { primary, .. } => {
                return Self::from_error(primary);
            }

            // ── servers ─────────────────────────────────────────────────────
            // Name-only variants are raised for Postgres instances too, and an
            // engine's own list cannot show the other engine: `local status`
            // covers both.
            Error::ServerNotFound(_) => Mapping::parity(LocalErrorCode::ServerNotFound)
                .command("clickhousectl local status"),
            Error::ServerSelectionRequired { engine, .. } => {
                Mapping::parity(LocalErrorCode::ServerSelectionRequired).command(match engine {
                    crate::local::server::Engine::Clickhouse => {
                        "clickhousectl local clickhouse list"
                    }
                    crate::local::server::Engine::Postgres => "clickhousectl local postgres list",
                })
            }
            Error::ServerRemoveSelectionRequired { .. } => {
                Mapping::parity(LocalErrorCode::ServerSelectionRequired)
                    .command("clickhousectl local clickhouse list")
            }
            // The same ambiguity across projects: the global list shows which
            // project root to pass to `--project`.
            Error::ServerInMultipleProjects { .. } => {
                Mapping::parity(LocalErrorCode::ServerSelectionRequired)
                    .command("clickhousectl local clickhouse list --global")
            }
            // Deliberately the status list, not a `start` command: this variant
            // is also raised for Postgres servers and for global PID lookups,
            // where the name is not a `clickhouse start` argument.
            Error::ServerNotRunning(_) => Mapping::parity(LocalErrorCode::ServerNotRunning)
                .command("clickhousectl local status"),
            Error::ServerAlreadyRunning(_) => {
                Mapping::parity(LocalErrorCode::ServerRunning).command("clickhousectl local status")
            }
            // Stopping *this* server is the recovery; `clickhouse list` only
            // restates what the error already says.
            Error::ServerRunningCannotRemove { command, .. } => {
                Mapping::parity(LocalErrorCode::ServerRunning).command(command.clone())
            }
            Error::InvalidServerName(_) => Mapping::parity(LocalErrorCode::InvalidServerName)
                .command("clickhousectl local status"),

            // ── server configs ──────────────────────────────────────────────
            Error::ConfigNotFound(_) => Mapping::parity(LocalErrorCode::ConfigNotFound)
                .command("clickhousectl local clickhouse configs"),
            Error::InvalidConfigName(_) => Mapping::parity(LocalErrorCode::InvalidConfigName)
                .command("clickhousectl local clickhouse configs"),

            // ── local UDFs ──────────────────────────────────────────────────
            Error::UdfDefinitionParse { path, source } => Mapping::redacted(
                LocalErrorCode::UdfDefinitionInvalid,
                crate::error::udf_definition_parse_message(path, source),
            )
            // No help screen documents the `udf.json` schema, and `init` keeps
            // an existing `udf.json`, so neither is a runnable remedy: the
            // message and `details` say what to fix.
            .details(source.to_string()),
            Error::UdfDefinitionInvalid { .. } => {
                Mapping::parity(LocalErrorCode::UdfDefinitionInvalid)
            }
            // Missing entrypoints, binaries, and symbolic links are fixed in
            // the source directory itself; the message names what is wrong.
            Error::UdfSourceInvalid { .. } => Mapping::parity(LocalErrorCode::UdfSourceInvalid),
            Error::UdfSourceMissing {
                name,
                scaffold_here,
                ..
            } => {
                let mapping = Mapping::parity(LocalErrorCode::UdfSourceInvalid);
                if *scaffold_here {
                    mapping.command(format!("clickhousectl local udf init {name}"))
                } else {
                    mapping
                }
            }
            Error::UdfPathUnquotable { role, .. } => match role {
                crate::error::UdfCommandPath::Interpreter => {
                    Mapping::parity(LocalErrorCode::UdfInterpreterNotFound)
                        .command("clickhousectl local udf deploy --help")
                }
                crate::error::UdfCommandPath::StagedScript => {
                    Mapping::parity(LocalErrorCode::UdfSourceInvalid)
                }
            },
            Error::UdfNotFound { server, .. } => Mapping::parity(LocalErrorCode::UdfNotFound)
                .command(format!("clickhousectl local udf list --server {server}")),
            // `deploy --help` documents `--python`, the remedy.
            Error::UdfInterpreterNotFound(_) => {
                Mapping::parity(LocalErrorCode::UdfInterpreterNotFound)
                    .command("clickhousectl local udf deploy --help")
            }
            // The server is up and answering; ClickHouse's text in `details`
            // is the only lead, and no command would add to it.
            Error::UdfQueryFailed { server, details } => Mapping::redacted(
                LocalErrorCode::UdfQueryFailed,
                format!("ClickHouse server '{server}' rejected the query"),
            )
            .details(details.clone()),
            Error::UdfServerUnreachable {
                server,
                port,
                details,
            } => Mapping::redacted(
                LocalErrorCode::UdfServerUnreachable,
                format!("Could not reach server '{server}' on port {port}"),
            )
            .details(details.clone())
            .command("clickhousectl local clickhouse list"),
            // Same code as a missing server elsewhere; the remedy differs
            // because `local udf` never creates one. Outside any project,
            // `clickhouse start` would begin a new one here, so the global list
            // (which finds running servers in other projects) is the lead.
            Error::UdfServerNotFound {
                name,
                project_has_state,
                ..
            } => Mapping::parity(LocalErrorCode::ServerNotFound).command(if *project_has_state {
                format!("clickhousectl local clickhouse start {name}")
            } else {
                "clickhousectl local clickhouse list --global".to_owned()
            }),
            Error::UdfServerNotRunning(name) => Mapping::parity(LocalErrorCode::ServerNotRunning)
                .command(format!("clickhousectl local clickhouse start {name}")),
            // `deploy --help` states the Linux amd64/arm64 limit.
            Error::UdfRuntimeUnsupported(_) => {
                Mapping::parity(LocalErrorCode::UdfRuntimeUnsupported)
                    .command("clickhousectl local udf deploy --help")
            }
            Error::UdfNotLoaded { deploy_command, .. } => {
                Mapping::parity(LocalErrorCode::UdfNotLoaded).command(deploy_command.clone())
            }
            Error::UdfRejected(rejection) => Mapping::redacted(
                LocalErrorCode::UdfRejected,
                crate::error::udf_rejected_message(rejection),
            )
            .details(rejection.details.clone())
            .command(udf_culprit_command(
                rejection
                    .blocking
                    .iter()
                    .find(|culprit| **culprit == rejection.name)
                    .or(rejection.blocking.first()),
                &rejection.server,
            )),
            Error::UdfReloadBlocked {
                removed,
                server,
                blocking,
                details,
            } => Mapping::redacted(
                LocalErrorCode::UdfReloadBlocked,
                crate::error::udf_reload_blocked_message(removed.as_deref(), server, blocking),
            )
            .details(details.clone())
            .command(udf_culprit_command(blocking.first(), server)),

            // ── versions ────────────────────────────────────────────────────
            Error::InvalidVersion(_) => Mapping::parity(LocalErrorCode::InvalidVersion)
                .command("clickhousectl local install --help"),
            // Not installed locally: the installed list, not the remote one, is
            // what resolves these.
            Error::VersionNotFound(_) | Error::StaleDefaultVersion(_) => {
                Mapping::parity(LocalErrorCode::VersionNotInstalled)
                    .command("clickhousectl local list")
            }
            Error::NoClientVersionInstalled => Mapping::parity(LocalErrorCode::VersionNotInstalled)
                .command("clickhousectl local install latest"),
            Error::ClientVersionNotInstalled(version)
            | Error::ServerVersionNotInstalled { version, .. } => {
                Mapping::parity(LocalErrorCode::VersionNotInstalled)
                    .command(format!("clickhousectl local install {version}"))
            }
            // Installed but unusable: the message is entirely self-composed
            // (path plus a closed-vocabulary problem), so it renders at parity
            // and names the repair (#471), which depends on the problem: see
            // `BinaryLaunchProblem::repair_command`.
            Error::BinaryNotLaunchable {
                version, problem, ..
            } => Mapping::parity(LocalErrorCode::BinaryNotLaunchable)
                .command(problem.repair_command(version)),
            Error::NoDefaultVersion | Error::AmbiguousClientVersion => {
                Mapping::parity(LocalErrorCode::VersionSelectionRequired)
                    .command("clickhousectl local list")
            }
            Error::VersionAlreadyInstalled(_) => {
                Mapping::parity(LocalErrorCode::VersionAlreadyInstalled)
                    .command("clickhousectl local list")
            }
            // The reason can quote the file's own output, so it goes in
            // `details` beside a fixed summary.
            Error::NotAClickHouseBinary { reason, .. } => Mapping::redacted(
                LocalErrorCode::NotAClickHouseBinary,
                "The --binary file is not a usable ClickHouse binary; nothing was installed",
            )
            .details(reason.clone()),
            Error::RepeatedClientQueryUnsupported { .. } => {
                Mapping::parity(LocalErrorCode::UnsupportedClientVersion)
            }
            Error::ClientLoginUnsupported { .. } => {
                Mapping::parity(LocalErrorCode::UnsupportedClientVersion)
                    .command("clickhousectl local install latest")
            }
            // The blocking servers are named — including their project root —
            // because they may live outside the current project, where neither
            // `clickhouse list` nor the caller's own state can find them.
            Error::VersionInUse { .. } => Mapping::parity(LocalErrorCode::ServerRunning)
                .command("clickhousectl local clickhouse list --global"),
            Error::VersionIsDefault {
                recovery_command, ..
            } => {
                Mapping::parity(LocalErrorCode::VersionIsDefault).command(recovery_command.clone())
            }
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
                    PortKind::Http | PortKind::Tcp => "clickhousectl local clickhouse start --help",
                })
            }
            // No CLI command frees a port another process holds; the message
            // names the port to free.
            Error::PostgresResumePortInUse { .. } | Error::PostgresPortNotPublished { .. } => {
                Mapping::parity(LocalErrorCode::PortInUse)
            }
            // `details` is the human text, including the managed server's own
            // stderr or log tail; the message is a one-line summary.
            Error::StartupExit {
                kind,
                name,
                details,
            } => Mapping::redacted(
                LocalErrorCode::StartupExit,
                format!("{kind} server '{name}' exited before becoming ready"),
            )
            .details(details.clone())
            .command(startup_list_command(*kind)),
            Error::StartupTimeout {
                kind,
                name,
                seconds,
                details,
            } => Mapping::redacted(
                LocalErrorCode::StartupTimeout,
                format!("{kind} server '{name}' did not become ready within {seconds} seconds"),
            )
            .details(details.clone())
            .command(startup_list_command(*kind)),

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
            // The HTTP client's, downloader's or extractor's text goes in
            // `details`.
            Error::Http(source) => {
                Mapping::redacted(LocalErrorCode::NetworkError, "HTTP request failed")
                    .details(source.to_string())
            }
            Error::Download(details) => {
                Mapping::redacted(LocalErrorCode::DownloadFailed, "Download failed")
                    .details(details.clone())
            }
            Error::Extract(details) => {
                Mapping::redacted(LocalErrorCode::DownloadFailed, "Extraction failed")
                    .details(details.clone())
            }
            Error::ExtractArchive { source, .. } => {
                Mapping::redacted(LocalErrorCode::DownloadFailed, "Extraction failed")
                    .details(source.to_string())
            }

            // ── Docker ──────────────────────────────────────────────────────
            // The unavailability text is built from a classified failure kind
            // and platform guidance; the daemon's own message is used for
            // classification only and never rendered (see `local::docker`).
            Error::DockerNotAvailable(_) => Mapping::parity(LocalErrorCode::DockerUnavailable),
            // The daemon's text for any other failed operation goes in
            // `details`.
            Error::DockerError(details) => {
                Mapping::redacted(LocalErrorCode::DockerError, "Docker operation failed")
                    .details(details.clone())
            }
            // Self-composed name-conflict guidance, unlike the daemon text
            // above.
            Error::ContainerNameConflict(_) => {
                Mapping::parity(LocalErrorCode::ContainerNameConflict)
            }

            // ── filesystem and metadata ─────────────────────────────────────
            // The OS's or serde's text goes in `details`; the paths stay out.
            Error::Io(source) | Error::ProcessScan(source) => io_mapping(source),
            Error::InvalidProjectDir(_) => Mapping::parity(LocalErrorCode::InvalidProjectDir),
            Error::Json(source) => io_mapping(source),
            Error::CreateDir { source, .. }
            | Error::ServerMetadataPermission { source, .. }
            | Error::ServerMetadataRead { source, .. }
            | Error::ServerMetadataWrite { source, .. }
            | Error::ServerLock { source, .. } => io_mapping(source),
            Error::ServerMetadataUtf8 { source, .. } => io_mapping(source),

            // ── postgres ────────────────────────────────────────────────────
            // Self-composed validation and state guidance; the foreign-text
            // sibling `Error::Postgres` stays in the fallback below.
            Error::PostgresUsage(_) => Mapping::parity(LocalErrorCode::PostgresError),
            // The OS cause goes in `details`; the path stays out.
            Error::SqlInputOpen { source, .. } => Mapping::redacted(
                LocalErrorCode::SqlInputOpenFailed,
                "Could not open SQL input file; check that --queries-file exists and is readable",
            )
            .details(source.to_string()),
            Error::SqlInputRead(source) => Mapping::redacted(
                LocalErrorCode::SqlInputReadFailed,
                "Could not read SQL input; check the file or stdin source is readable",
            )
            .details(source.to_string()),

            // ── bounded fallback ────────────────────────────────────────────
            // Subprocess text and `Postgres` (OS text from a failed psql
            // exec) are foreign output, carried in `details`.
            Error::Exec(details) | Error::Postgres(details) => {
                Mapping::redacted(LocalErrorCode::LocalError, "Local command failed")
                    .details(details.clone())
            }
            // These belong to other command surfaces and are never printed
            // through this envelope; `ChildExit` passes the child's status
            // through without an error object at all.
            Error::CloudDetailed(_)
            | Error::AuthRequired(_)
            | Error::Usage(_)
            | Error::Skills(_)
            | Error::UpdateCheck { .. }
            | Error::PackageManagedInstall { .. }
            | Error::ArchiveVerification(_)
            | Error::ChildExit(_) => {
                Mapping::redacted(LocalErrorCode::LocalError, "Local command failed")
            }
            #[cfg(feature = "feedback")]
            Error::Feedback(_) => {
                Mapping::redacted(LocalErrorCode::LocalError, "Local command failed")
            }
            Error::Cancelled => Mapping::parity(LocalErrorCode::LocalError),
        };
        Self::General(mapping.into_detail(error))
    }
}

impl ServerMetadataParseErrorDetail {
    fn from_path(path: &Path) -> Self {
        Self {
            code: LocalErrorCode::ServerMetadataInvalid,
            message: "Server metadata is not valid JSON",
            path: path.display().to_string(),
            guidance: vec![
                LocalGuidance {
                    action: LocalGuidanceAction::RepairMetadata,
                    message: "Repair the metadata file, then retry",
                    command: None,
                },
                LocalGuidance {
                    action: LocalGuidanceAction::ListGlobalServers,
                    message: "For ClickHouse, if repair is not possible, confirm that the running server is discoverable before moving the metadata file aside",
                    command: Some("clickhousectl local clickhouse list --global"),
                },
                LocalGuidance {
                    action: LocalGuidanceAction::VerifyPostgresContainer,
                    message: "For Postgres, verify the container state separately before moving the metadata file aside",
                    command: None,
                },
                LocalGuidance {
                    action: LocalGuidanceAction::ListProjectServers,
                    message: "Retry from the owning project; ClickHouse recovery requires the server to remain running and discoverable",
                    command: Some("clickhousectl local clickhouse list"),
                },
            ],
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
            action: LocalGuidanceAction::ListProjectServers,
            message: "List managed servers in this project",
            command: Some("clickhousectl local clickhouse list"),
        }];
        match &error.kind {
            ManagedClientErrorKind::ServerNotFound => {
                if !error.project.is_marked() {
                    guidance.push(LocalGuidance {
                        action: LocalGuidanceAction::ReturnToProjectRoot,
                        message: "No project was found; change to a directory inside the project that owns the managed server",
                        command: None,
                    });
                }
                guidance.push(start_guidance(error.selection));
            }
            ManagedClientErrorKind::ServerNotRunning => {
                guidance.push(start_guidance(error.selection));
            }
            ManagedClientErrorKind::BinaryNotFound => {
                guidance.push(LocalGuidance {
                    action: LocalGuidanceAction::InstallVersion,
                    message: "Install the version selected by the managed server metadata",
                    command: Some("clickhousectl local install <version>"),
                });
            }
            ManagedClientErrorKind::ProjectStateUnavailable(_) => {
                guidance.insert(
                    0,
                    LocalGuidance {
                        action: LocalGuidanceAction::RepairProjectState,
                        message: "Repair the reported project state error before retrying",
                        command: None,
                    },
                );
            }
        }
        guidance.push(LocalGuidance {
            action: LocalGuidanceAction::ConnectDirectly,
            message: "Bypass managed project lookup and connect directly",
            command: Some("clickhousectl local client --host <host> --port <port>"),
        });

        Self {
            code,
            message,
            project_scope: server_project_scope(&error.project),
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
            project_scope: server_project_scope(&error.project),
            server: LocalProjectServer {
                name: error.server_name.clone(),
            },
            guidance: project_scope_guidance(error.project.kind, Some(error.command)),
        }
    }
}

impl ProjectServerStateMissingDetail {
    fn from_error(error: &ProjectServerStateMissing) -> Self {
        Self {
            code: LocalErrorCode::ServerSelectionRequired,
            message: "No project was found from the current directory; no server was removed",
            project_scope: ServerProjectScope {
                kind: LocalProjectScopeKind::CurrentDirectory,
                path: error.project_dir.display().to_string(),
            },
            guidance: project_scope_guidance(ProjectRootKind::Unmarked, Some(error.command)),
        }
    }
}

pub(crate) fn server_project_scope(project: &ProjectRoot) -> ServerProjectScope {
    ServerProjectScope {
        kind: project.kind.into(),
        path: project.dir.display().to_string(),
    }
}

/// Changing directory helps only when no project was found: inside one, the
/// project's own list is the next step.
pub(crate) fn project_list_guidance(
    kind: ProjectRootKind,
    list: &'static str,
) -> Vec<LocalGuidance> {
    match kind {
        ProjectRootKind::Marked | ProjectRootKind::Override => vec![LocalGuidance {
            action: LocalGuidanceAction::ListProjectServers,
            message: "List the servers in this project",
            command: Some(list),
        }],
        ProjectRootKind::Unmarked => vec![
            LocalGuidance {
                action: LocalGuidanceAction::ReturnToProjectRoot,
                message: "Change to a directory inside the project where the server was started",
                command: Some("cd <project-root>"),
            },
            LocalGuidance {
                action: LocalGuidanceAction::ListProjectServers,
                message: "List servers after changing to that project",
                command: Some(list),
            },
        ],
    }
}

pub(crate) fn project_scope_guidance(
    kind: ProjectRootKind,
    command: Option<ProjectServerCommand>,
) -> Vec<LocalGuidance> {
    let mut guidance = project_list_guidance(kind, "clickhousectl local clickhouse list");
    guidance.push(LocalGuidance {
        action: LocalGuidanceAction::ListGlobalServers,
        message: "Locate running ClickHouse servers across projects",
        command: Some("clickhousectl local clickhouse list --global"),
    });
    if command == Some(ProjectServerCommand::Stop) {
        guidance.push(LocalGuidance {
            action: LocalGuidanceAction::StopGlobalProjectServer,
            message: "After confirming the project, stop the server with explicit global project selection",
            command: Some(
                "clickhousectl local clickhouse stop <name> --global --project <project-root>",
            ),
        });
    }
    guidance
}

fn start_guidance(selection: ManagedClientSelection) -> LocalGuidance {
    match selection {
        ManagedClientSelection::Default => LocalGuidance {
            action: LocalGuidanceAction::StartServer,
            message: "Start the default managed server in this project",
            command: Some("clickhousectl local clickhouse start"),
        },
        ManagedClientSelection::Named => LocalGuidance {
            action: LocalGuidanceAction::StartServer,
            message: "Start the selected named managed server in this project",
            command: Some("clickhousectl local clickhouse start <name>"),
        },
    }
}

/// The code [`print_error`] would emit for this failure, which telemetry
/// records as `failure_kind` (#1063). Derived from the same classification, so
/// an event and the command's `--json` error always agree.
pub(crate) fn error_code(error: &Error) -> LocalErrorCode {
    LocalErrorBody::from_error(error).code()
}

/// The code and recovery command this module reports for a failure to select
/// or launch an installed ClickHouse client, or `None` for any other error.
/// `cloud service client` runs the same pre-flight as `local client`, and
/// reuses this classification so the two report a broken local install
/// alike (#1443). Every variant here maps at parity: the message is the
/// error's own text.
pub(crate) fn installed_client_error(error: &Error) -> Option<(LocalErrorCode, Option<String>)> {
    if !matches!(
        error,
        Error::InvalidVersion(_)
            | Error::VersionNotFound(_)
            | Error::StaleDefaultVersion(_)
            | Error::NoClientVersionInstalled
            | Error::ClientVersionNotInstalled(_)
            | Error::NoDefaultVersion
            | Error::AmbiguousClientVersion
            | Error::BinaryNotLaunchable { .. }
            | Error::ClientLoginUnsupported { .. }
    ) {
        return None;
    }
    match LocalErrorBody::from_error(error) {
        LocalErrorBody::General(detail) => Some((detail.code, detail.command)),
        _ => None,
    }
}

/// Write exactly one local runtime error object to stderr, and return its
/// code so telemetry reuses this classification instead of building it again.
/// The serialized DTO is allowlisted above; foreign text appears only in
/// `details`, for the variants that opt in.
pub(crate) fn print_error(error: &Error) -> LocalErrorCode {
    let body = LocalErrorBody::from_error(error);
    crate::stdout::eprint_error_envelope(&body);
    body.code()
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
            write!(f, "Run: clickhousectl local install latest")?;
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
    /// Every project-local path this invocation created, e.g. `.clickhouse/`,
    /// `.clickhouse/.gitignore`, `clickhouse/`, or `postgres/`.
    pub paths: Vec<String>,
    /// Human-output detail only: the project dir already existed before this
    /// run. This affects human wording only, so it is not serialized.
    #[serde(skip)]
    pub already_initialized: bool,
    /// The project this directory was nested inside: it had no `.clickhouse/`
    /// and an ancestor project was found, so commands run here acted on that
    /// one before this run and act on this new project after it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enclosing_project: Option<String>,
    /// The directory initialized, which `paths` are relative to, when
    /// `CLICKHOUSECTL_PROJECT_DIR` named it instead of the current directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_dir: Option<String>,
}

impl fmt::Display for InitOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.already_initialized {
            write!(f, "Initialized ClickHouse project in .clickhouse/")?;
        } else if self
            .paths
            .iter()
            .any(|path| path == ".clickhouse/.gitignore")
        {
            write!(f, "Restored runtime ignore at .clickhouse/.gitignore")?;
        } else {
            write!(f, "Already initialized at .clickhouse/")?;
        }
        for path in self
            .paths
            .iter()
            .filter(|path| !path.starts_with(".clickhouse/"))
        {
            write!(f, "\nCreated project scaffold in {path}")?;
        }
        if let Some(dir) = &self.project_dir {
            write!(
                f,
                "\nProject root: '{dir}' (from {})",
                crate::init::PROJECT_DIR_ENV
            )?;
        }
        if let Some(enclosing) = &self.enclosing_project {
            write!(
                f,
                "\nNote: this directory is inside the project at '{enclosing}'; commands run here and below now use this new nested project."
            )?;
        }
        Ok(())
    }
}

// ── udf ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct UdfInitOutput {
    pub name: String,
    /// The UDF directory, e.g. `clickhouse/udfs/my_fn`.
    pub dir: String,
    /// Files and directories (with a trailing `/`) relative to `dir` this
    /// invocation created; empty when everything existed.
    pub created: Vec<String>,
    /// What the user must still do before the UDF can run, if anything.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_step: Option<String>,
    /// Limits of the scaffolded UDF on this host, e.g. a `native` UDF that
    /// only a Linux host can deploy locally. Human mode prints them to stderr.
    pub warnings: Vec<String>,
}

impl fmt::Display for UdfInitOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.created.is_empty() {
            write!(f, "UDF {} already exists in {}", self.name, self.dir)?;
        } else {
            write!(f, "Scaffolded UDF {} in {}", self.name, self.dir)?;
            for file in &self.created {
                write!(f, "\nCreated {}/{file}", self.dir)?;
            }
        }
        if let Some(next_step) = &self.next_step {
            write!(f, "\n{next_step}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UdfDeployOutput {
    pub name: String,
    pub server: String,
    pub r#type: String,
    pub runtime: String,
    /// Whether the server was running, so deploy reloaded its functions; a
    /// stopped server loads them on its next start.
    pub reloaded: bool,
    /// `Some(true)` once a running server reports the function; `None` when
    /// the server is not running. A running server that does not load it is
    /// an error, never `Some(false)`.
    pub loaded: Option<bool>,
    /// Absolute interpreter baked into the command for runtime python3.11.
    pub interpreter: Option<String>,
    /// Cloud-only definition fields that have no local equivalent.
    pub ignored_fields: Vec<String>,
    /// Source files the local server does nothing with: `requirements.txt`
    /// (copied, not installed) or, for `native`, the other architecture's
    /// directory, such as `arm64/` (not copied).
    pub ignored_files: Vec<String>,
    /// Differences from Cloud worth knowing, e.g. an interpreter that is not
    /// Python 3.11. Human mode prints them to stderr.
    pub warnings: Vec<String>,
    pub function_config: String,
    pub scripts_dir: String,
}

impl fmt::Display for UdfDeployOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Deployed UDF {} to server '{}' ({}, {})",
            self.name, self.server, self.runtime, self.r#type
        )?;
        let mut detail = |label: &str, value: &str| {
            // Width of the longest label, "ignored fields:", plus one space.
            writeln!(f, "  {:<16}{value}", format!("{label}:"))
        };
        detail("definition", &self.function_config)?;
        detail("scripts", &self.scripts_dir)?;
        if let Some(interpreter) = &self.interpreter {
            detail("interpreter", interpreter)?;
        }
        if !self.ignored_fields.is_empty() {
            detail("ignored fields", &self.ignored_fields.join(", "))?;
        }
        if !self.ignored_files.is_empty() {
            detail("ignored files", &self.ignored_files.join(", "))?;
        }
        if self.reloaded {
            write!(f, "Reloaded functions; {} is loaded.", self.name)
        } else {
            write!(
                f,
                "Server '{}' is not running; the function loads on its next start.",
                self.server
            )
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UdfListEntry {
    pub name: String,
    pub r#type: Option<String>,
    pub runtime: Option<String>,
    /// `None` when the server is not running.
    pub loaded: Option<bool>,
    /// The running server rejected the last deploy of these files, so a
    /// loaded function still runs an earlier definition.
    pub last_deploy_rejected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UdfListOutput {
    pub server: String,
    pub server_running: bool,
    pub udfs: Vec<UdfListEntry>,
}

#[derive(Tabled)]
struct UdfListRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Type")]
    kind: String,
    #[tabled(rename = "Runtime")]
    runtime: String,
    #[tabled(rename = "Loaded")]
    loaded: String,
}

impl fmt::Display for UdfListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.udfs.is_empty() {
            return write!(f, "No UDFs deployed to server '{}'", self.server);
        }
        let rows: Vec<UdfListRow> = self
            .udfs
            .iter()
            .map(|udf| UdfListRow {
                name: udf.name.clone(),
                kind: udf.r#type.clone().unwrap_or_else(|| "-".into()),
                runtime: udf.runtime.clone().unwrap_or_else(|| "-".into()),
                loaded: match (udf.loaded, udf.last_deploy_rejected) {
                    (Some(true), false) => "yes".into(),
                    (Some(true), true) => "yes (stale: last deploy rejected)".into(),
                    (Some(false), false) => "no".into(),
                    (Some(false), true) => "no (last deploy rejected)".into(),
                    (None, false) => "-".into(),
                    (None, true) => "- (last deploy rejected)".into(),
                },
            })
            .collect();
        write!(f, "{}", Table::new(rows).with(Style::markdown()))?;
        if !self.server_running {
            write!(
                f,
                "\nServer '{}' is not running; loaded state is unknown until it starts.",
                self.server
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UdfRemoveOutput {
    pub name: String,
    pub server: String,
    pub reloaded: bool,
}

impl fmt::Display for UdfRemoveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Removed UDF {} from server '{}'", self.name, self.server)?;
        if self.reloaded {
            write!(f, " and reloaded functions")
        } else {
            write!(f, "; the change applies on its next start")
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UdfReloadOutput {
    pub server: String,
    /// Executable UDFs the server reports as loaded after the reload.
    pub loaded: Vec<String>,
}

impl fmt::Display for UdfReloadOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Reloaded functions on server '{}'", self.server)?;
        if self.loaded.is_empty() {
            write!(f, "; no executable UDFs are loaded")
        } else {
            write!(f, "; loaded: {}", self.loaded.join(", "))
        }
    }
}

// ── server configs ──────────────────────────────────────────────────────────

/// One always-applied `config.d` layer: its directory and the files it
/// contributes, in merge order.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigLayerOutput {
    pub dir: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerConfigsOutput {
    /// The named-config store `--config` selects from.
    pub dir: String,
    pub configs: Vec<String>,
    /// Applied to every server on this machine, first.
    pub global_config_d: ConfigLayerOutput,
    /// Applied to every server in this project, after the global layer.
    pub project_config_d: ConfigLayerOutput,
}

impl fmt::Display for ServerConfigsOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Applied on every start, in merge order (later files win):"
        )?;
        for (label, layer) in [
            ("Global", &self.global_config_d),
            ("Project", &self.project_config_d),
        ] {
            if layer.files.is_empty() {
                writeln!(f, "  {label} {}: no files", layer.dir)?;
            } else {
                writeln!(f, "  {label} {}:", layer.dir)?;
                for name in &layer.files {
                    writeln!(f, "    {name}")?;
                }
            }
        }
        writeln!(f)?;
        if self.configs.is_empty() {
            writeln!(f, "No named config files in {}", self.dir)?;
            write!(
                f,
                "Create an XML or YAML file in any of these directories with only the settings\n\
                 you want to change. Other settings inherit ClickHouse's built-in defaults.\n\
                 Apply a named config, after both layers, with: \
                 clickhousectl local clickhouse start --config <NAME>"
            )?;
            return Ok(());
        }
        writeln!(f, "Named config files in {}:", self.dir)?;
        for name in &self.configs {
            writeln!(f, "  {name}")?;
        }
        write!(
            f,
            "Apply one, after both layers, with: clickhousectl local clickhouse start --config <NAME>"
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
    pub(crate) guidance: Vec<LocalGuidance>,
    /// The command that produced this list; selects the table and the
    /// empty-list hint.
    #[serde(skip)]
    pub(crate) list: ServerList,
}

/// Which list a [`ServerListOutput`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServerList {
    /// One project's servers.
    Project(ProjectList),
    /// Running ClickHouse servers discovered across projects (`--global`).
    Global,
}

/// A project-scoped list command: one engine's `list`, or `local status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectList {
    Clickhouse,
    Postgres,
    Status,
}

impl ProjectList {
    fn command(self) -> &'static str {
        match self {
            ProjectList::Clickhouse => "clickhousectl local clickhouse list",
            ProjectList::Postgres => "clickhousectl local postgres list",
            ProjectList::Status => "clickhousectl local status",
        }
    }

    /// Only ClickHouse has cross-project discovery.
    fn covers_clickhouse(self) -> bool {
        self != ProjectList::Postgres
    }
}

/// The list that shows the server a startup failure names.
fn startup_list_command(kind: crate::error::StartupKind) -> &'static str {
    match kind {
        crate::error::StartupKind::ClickHouse => ProjectList::Clickhouse.command(),
        crate::error::StartupKind::Postgres => ProjectList::Postgres.command(),
    }
}

/// Hints for an empty project-scoped list.
pub(crate) fn empty_list_guidance(list: ProjectList, kind: ProjectRootKind) -> Vec<LocalGuidance> {
    // Inside a project an empty list is accurate: there is nowhere to change
    // to, only other projects to look in.
    let mut guidance = match kind {
        ProjectRootKind::Marked | ProjectRootKind::Override => Vec::new(),
        ProjectRootKind::Unmarked => project_list_guidance(kind, list.command()),
    };
    if list.covers_clickhouse() {
        guidance.push(LocalGuidance {
            action: LocalGuidanceAction::ListGlobalServers,
            message: "Locate running ClickHouse servers across projects",
            command: Some("clickhousectl local clickhouse list --global"),
        });
    }
    guidance
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
            if let (Some(scope), ServerList::Project(list)) = (&self.project_scope, self.list) {
                writeln!(f, "No servers found in project '{}'.", scope.path)?;
                if scope.kind == LocalProjectScopeKind::CurrentDirectory {
                    writeln!(f, "{NO_PROJECT_FOUND}")?;
                    write!(
                        f,
                        "To reach an existing server, change to a directory inside its project and run `{}`",
                        list.command()
                    )?;
                    if list.covers_clickhouse() {
                        write!(
                            f,
                            ", or use `clickhousectl local clickhouse list --global` to locate running ClickHouse servers in other projects"
                        )?;
                    }
                    return write!(f, ".");
                }
                if list.covers_clickhouse() {
                    write!(
                        f,
                        "Use `clickhousectl local clickhouse list --global` to locate running ClickHouse servers in other projects."
                    )?;
                }
                return Ok(());
            }
            write!(f, "No servers")?;
            return Ok(());
        }

        let status = |running: bool| if running { "running" } else { "stopped" }.to_string();
        let mut table = match self.list {
            ServerList::Global => {
                let rows = self.servers.iter().map(|e| ServerListRowGlobal {
                    name: e.name.clone(),
                    status: status(e.running),
                    pid: or_absent(e.pid),
                    version: or_absent(e.version.as_deref()),
                    http_port: or_absent(e.http_port),
                    tcp_port: or_absent(e.tcp_port),
                    project: or_absent(e.project.as_deref()),
                });
                Table::new(rows)
            }
            ServerList::Project(ProjectList::Clickhouse) => {
                let rows = self.servers.iter().map(|e| ServerListRow {
                    name: e.name.clone(),
                    status: status(e.running),
                    pid: or_absent(e.pid),
                    version: or_absent(e.version.as_deref()),
                    http_port: or_absent(e.http_port),
                    tcp_port: or_absent(e.tcp_port),
                });
                Table::new(rows)
            }
            // Lists that can hold Postgres combine the ClickHouse PID and the
            // Postgres container short ID into one "ID" column.
            ServerList::Project(ProjectList::Postgres | ProjectList::Status) => {
                let rows = self.servers.iter().map(|e| {
                    let id = if e.engine == "postgres" {
                        e.container_id
                            .as_deref()
                            .map(|s| s.chars().take(12).collect::<String>())
                            .unwrap_or_else(|| ABSENT.to_string())
                    } else {
                        or_absent(e.pid)
                    };
                    ServerListRowWithEngine {
                        name: e.name.clone(),
                        engine: e.engine.clone(),
                        status: status(e.running),
                        pid_or_container: id,
                        version: or_absent(e.version.as_deref()),
                        http_port: or_absent(e.http_port),
                        tcp_port: or_absent(e.tcp_port),
                    }
                });
                Table::new(rows)
            }
        };
        table.with(Style::markdown());
        writeln!(f, "{table}")?;
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
            "  Connect:  clickhousectl local postgres client {}",
            self.name
        )
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
    /// Which engine had nothing to stop; `reason` says it to JSON consumers.
    #[serde(skip)]
    pub engine: crate::local::server::Engine,
    pub stopped: bool,
    pub selection: ServerSelection,
    pub reason: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) project_scope: Option<ServerProjectScope>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) guidance: Vec<LocalGuidance>,
}

impl fmt::Display for ServerStopNoopOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::local::server::Engine;
        match self.engine {
            Engine::Clickhouse => write!(f, "No ClickHouse servers found; nothing to stop")?,
            Engine::Postgres => write!(f, "No Postgres instances found; nothing to stop")?,
        }
        if let Some(scope) = &self.project_scope {
            writeln!(f)?;
            writeln!(f, "Searched from '{}'.", scope.path)?;
            writeln!(f, "{NO_PROJECT_FOUND}")?;
            match self.engine {
                Engine::Clickhouse => write!(
                    f,
                    "Change to a directory inside the project where the server was started and run `clickhousectl local clickhouse list`, or use `clickhousectl local clickhouse list --global` to locate running servers in other projects."
                )?,
                Engine::Postgres => write!(
                    f,
                    "Change to a directory inside the project where the instance was started and run `clickhousectl local postgres list`."
                )?,
            }
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

/// JSON only: human mode prints each server's progress line as it stops, so
/// this has no `Display`; print it with [`print_json`].
#[derive(Debug, Clone, Serialize)]
pub struct ServerStopAllOutput {
    pub servers: Vec<ServerStopEntry>,
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

// ── dotenv (ClickHouse and Postgres) ───────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct DotenvOutput {
    pub file: String,
    pub server: String,
    pub vars: Vec<DotenvVar>,
    /// Which `dotenv` wrote the file; selects the human label only.
    #[serde(skip)]
    pub engine: crate::local::server::Engine,
}

#[derive(Debug, Clone, Serialize)]
pub struct DotenvVar {
    pub key: String,
    pub value: String,
}

impl fmt::Display for DotenvOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.engine {
            crate::local::server::Engine::Clickhouse => "server",
            crate::local::server::Engine::Postgres => "postgres",
        };
        writeln!(f, "Wrote to {} ({label} '{}')", self.file, self.server)?;
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
        print_json(output);
    } else {
        println!("{}", output);
    }
}

/// Print output that only has a JSON form.
pub fn print_json(output: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(output).expect("JSON serialization failed")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The project `/project`, found (`Marked`) or standing in (`Unmarked`).
    fn project(kind: ProjectRootKind) -> ProjectRoot {
        ProjectRoot {
            dir: "/project".into(),
            kind,
        }
    }
    use crate::error::BinaryLaunchProblem;

    fn error_json(error: &Error) -> serde_json::Value {
        let body = LocalErrorBody::from_error(error);
        serde_json::to_value(crate::stdout::ErrorEnvelope { error: &body }).unwrap()
    }

    /// Whether `text` names `word` as a whole token (an identifier, not part
    /// of a longer one), so tests can check identifiers without pinning the
    /// sentence around them.
    fn names(text: &str, word: &str) -> bool {
        text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
            .any(|token| token == word)
    }

    /// The numbers in `text`, in order.
    fn numbers(text: &str) -> Vec<&str> {
        text.split(|c: char| !c.is_ascii_digit())
            .filter(|token| !token.is_empty())
            .collect()
    }

    #[test]
    fn local_error_codes_cover_the_stable_vocabulary() {
        let cases = [
            (
                Error::ManagedClient(ManagedClientError {
                    kind: ManagedClientErrorKind::ServerNotFound,
                    project: project(ProjectRootKind::Marked),
                    selection: ManagedClientSelection::Default,
                    server_name: "default".into(),
                    binary_version: None,
                }),
                "managed_client_server_not_found",
            ),
            (
                Error::ManagedClient(ManagedClientError {
                    kind: ManagedClientErrorKind::ServerNotRunning,
                    project: project(ProjectRootKind::Marked),
                    selection: ManagedClientSelection::Named,
                    server_name: "dev".into(),
                    binary_version: None,
                }),
                "managed_client_server_not_running",
            ),
            (
                Error::ManagedClient(ManagedClientError {
                    kind: ManagedClientErrorKind::BinaryNotFound,
                    project: project(ProjectRootKind::Marked),
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
                    project: project(ProjectRootKind::Marked),
                    selection: ManagedClientSelection::Default,
                    server_name: "default".into(),
                    binary_version: None,
                }),
                "managed_client_project_state_unavailable",
            ),
            (
                Error::ProjectServerNotFound(ProjectServerNotFound {
                    command: ProjectServerCommand::Stop,
                    project: project(ProjectRootKind::Marked),
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
                Error::ServerSelectionRequired {
                    engine: crate::local::server::Engine::Postgres,
                    names: vec!["alpha".into(), "beta".into()],
                    usage: "clickhousectl local postgres stop <name>",
                },
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
                    recovery_command: "clickhousectl local use latest".into(),
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
                Error::NotAClickHouseBinary {
                    path: "./clickhouse".into(),
                    reason: "no version".into(),
                },
                "not_a_clickhouse_binary",
            ),
            (
                Error::RepeatedClientQueryUnsupported {
                    version: "24.1.1.1".into(),
                    minimum: "24.2",
                },
                "unsupported_client_version",
            ),
            (
                Error::ClientLoginUnsupported {
                    version: "25.11.2.24".into(),
                    minimum: "25.12",
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
                Error::ServerRunningCannotRemove {
                    name: "dev".into(),
                    command: "clickhousectl local clickhouse stop dev".into(),
                },
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
                Error::PostgresResumePortInUse {
                    name: "dev".into(),
                    port: 5432,
                },
                "port_in_use",
            ),
            (
                Error::PostgresPortNotPublished {
                    name: "dev".into(),
                    port: 5432,
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
            (
                Error::ServerMetadataParse {
                    path: "/work/.clickhouse/servers/default.json".into(),
                    source: serde_json::from_str::<serde_json::Value>("{")
                        .expect_err("invalid fixture must fail to parse"),
                },
                "server_metadata_invalid",
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
            (
                Error::UdfDefinitionParse {
                    path: "clickhouse/udfs/my_fn/udf.json".into(),
                    source: serde_json::from_str::<serde_json::Value>("{")
                        .expect_err("invalid fixture must fail to parse"),
                },
                "udf_definition_invalid",
            ),
            (
                Error::UdfDefinitionInvalid {
                    path: "clickhouse/udfs/my_fn/udf.json".into(),
                    reason: "UDF definition requires functionName".into(),
                },
                "udf_definition_invalid",
            ),
            (
                Error::UdfSourceInvalid {
                    path: "clickhouse/udfs/my_fn".into(),
                    reason: "contains a symbolic link at link.py".into(),
                },
                "udf_source_invalid",
            ),
            (
                Error::UdfNotFound {
                    name: "my_fn".into(),
                    server: "dev".into(),
                },
                "udf_not_found",
            ),
            (
                Error::UdfInterpreterNotFound("No python3.11 or python3 found".into()),
                "udf_interpreter_not_found",
            ),
            (
                Error::UdfQueryFailed {
                    server: "default".into(),
                    details: "Code: 46. DB::Exception: Unknown function".into(),
                },
                "udf_query_failed",
            ),
            (
                Error::UdfServerNotFound {
                    name: "dev".into(),
                    project_dir: "/work".into(),
                    project_has_state: true,
                },
                "server_not_found",
            ),
            (
                Error::UdfServerNotRunning("dev".into()),
                "server_not_running",
            ),
            (
                Error::UdfSourceMissing {
                    path: "clickhouse/udfs/my_fn".into(),
                    name: "my_fn".into(),
                    scaffold_here: true,
                },
                "udf_source_invalid",
            ),
            (
                Error::UdfRuntimeUnsupported("native UDFs are Linux binaries".into()),
                "udf_runtime_unsupported",
            ),
            (
                Error::UdfNotLoaded {
                    name: "my_fn".into(),
                    server: "dev".into(),
                    log_path: "/work/.clickhouse/servers/dev/server.log".into(),
                    deploy_command: "clickhousectl local udf deploy my_fn --server dev".into(),
                },
                "udf_not_loaded",
            ),
            (
                Error::UdfRejected(Box::new(crate::error::UdfRejection {
                    name: "my_fn".into(),
                    server: "dev".into(),
                    source_dir: "clickhouse/udfs/my_fn".into(),
                    deploy_command: "clickhousectl local udf deploy my_fn --server dev".into(),
                    blocking: vec!["my_fn".into()],
                    details: "Code: 50. DB::Exception: Unknown data type".into(),
                })),
                "udf_rejected",
            ),
            (
                Error::UdfServerUnreachable {
                    server: "dev".into(),
                    port: 8123,
                    details: "error sending request".into(),
                },
                "udf_server_unreachable",
            ),
            (
                Error::UdfReloadBlocked {
                    removed: Some("my_fn".into()),
                    server: "dev".into(),
                    blocking: vec!["bad".into()],
                    details: "Code: 50. DB::Exception: Unknown data type".into(),
                },
                "udf_reload_blocked",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error_json(&error)["error"]["code"], expected);
            // Telemetry's `failure_kind` reads the same classification.
            assert_eq!(error_code(&error).as_str(), expected);
        }
    }

    /// Every structured body shares one `project_scope` shape (`kind` always
    /// present) and one `guidance[]` shape (`action` always present) (#1314).
    #[test]
    fn structured_error_bodies_always_carry_scope_kind_and_guidance_action() {
        let managed = |kind: ManagedClientErrorKind, root, selection| {
            Error::ManagedClient(ManagedClientError {
                kind,
                project: project(root),
                selection,
                server_name: "dev".into(),
                binary_version: None,
            })
        };
        let lock_failure = || {
            ManagedClientErrorKind::ProjectStateUnavailable(Box::new(Error::Io(
                std::io::Error::other("lock failed"),
            )))
        };
        let mut errors = Vec::new();
        for root in [ProjectRootKind::Marked, ProjectRootKind::Unmarked] {
            for selection in [
                ManagedClientSelection::Default,
                ManagedClientSelection::Named,
            ] {
                errors.push(managed(
                    ManagedClientErrorKind::ServerNotFound,
                    root,
                    selection,
                ));
                errors.push(managed(
                    ManagedClientErrorKind::ServerNotRunning,
                    root,
                    selection,
                ));
                errors.push(managed(
                    ManagedClientErrorKind::BinaryNotFound,
                    root,
                    selection,
                ));
                errors.push(managed(lock_failure(), root, selection));
            }
            for command in [ProjectServerCommand::Stop, ProjectServerCommand::Remove] {
                errors.push(Error::ProjectServerNotFound(ProjectServerNotFound {
                    command,
                    project: project(root),
                    server_name: "dev".into(),
                }));
            }
        }
        errors.push(Error::ProjectServerStateMissing(
            ProjectServerStateMissing {
                command: ProjectServerCommand::Remove,
                project_dir: "/project".into(),
            },
        ));
        errors.push(Error::ServerMetadataParse {
            path: "/project/.clickhouse/servers/dev.json".into(),
            source: serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
        });

        for error in &errors {
            let json = error_json(error);
            let body = &json["error"];
            if let Some(scope) = body.get("project_scope") {
                assert!(scope["kind"].is_string(), "{body}");
                assert_eq!(scope["path"], "/project", "{body}");
            }
            let guidance = body["guidance"].as_array().expect("guidance list");
            assert!(!guidance.is_empty(), "{body}");
            assert!(
                guidance.iter().all(|step| step["action"].is_string()),
                "{body}"
            );
        }

        // Managed-client scope and actions follow the project lookup and the
        // failure, like the project-server errors.
        let actions = |error: &Error| -> (serde_json::Value, Vec<String>) {
            let json = error_json(error);
            let actions = json["error"]["guidance"]
                .as_array()
                .unwrap()
                .iter()
                .map(|step| step["action"].as_str().unwrap().to_owned())
                .collect();
            (json["error"]["project_scope"]["kind"].clone(), actions)
        };
        let named = ManagedClientSelection::Named;
        assert_eq!(
            actions(&managed(
                ManagedClientErrorKind::ServerNotFound,
                ProjectRootKind::Unmarked,
                named
            )),
            (
                serde_json::json!("current_directory"),
                vec![
                    "list_project_servers".to_owned(),
                    "return_to_project_root".to_owned(),
                    "start_server".to_owned(),
                    "connect_directly".to_owned(),
                ]
            )
        );
        assert_eq!(
            actions(&managed(
                ManagedClientErrorKind::BinaryNotFound,
                ProjectRootKind::Marked,
                named
            )),
            (
                serde_json::json!("project_root"),
                vec![
                    "list_project_servers".to_owned(),
                    "install_version".to_owned(),
                    "connect_directly".to_owned(),
                ]
            )
        );
        assert_eq!(
            actions(&managed(lock_failure(), ProjectRootKind::Marked, named)).1,
            [
                "repair_project_state",
                "list_project_servers",
                "connect_directly"
            ]
        );
    }

    #[test]
    fn udf_rejected_recovery_names_the_broken_function() {
        let rejected = |blocking: &[&str]| {
            error_json(&Error::UdfRejected(Box::new(crate::error::UdfRejection {
                name: "rev".into(),
                server: "dev".into(),
                source_dir: "clickhouse/udfs/rev".into(),
                deploy_command: "clickhousectl local udf deploy rev --server dev".into(),
                blocking: blocking.iter().map(|name| name.to_string()).collect(),
                details: "Code: 50. DB::Exception: Unknown data type".into(),
            })))["error"]
                .clone()
        };
        let own = rejected(&["bad", "rev"]);
        assert_eq!(
            own["command"],
            "clickhousectl local udf remove rev --server dev"
        );
        // The other broken function is named too.
        assert!(names(own["message"].as_str().unwrap(), "bad"));
        let other = rejected(&["bad"]);
        assert_eq!(
            other["command"],
            "clickhousectl local udf remove bad --server dev"
        );
        assert!(
            !other["message"]
                .as_str()
                .unwrap()
                .contains("udf remove rev ")
        );
        let unknown = rejected(&[]);
        assert_eq!(
            unknown["command"],
            "clickhousectl local udf list --server dev"
        );
        assert!(!unknown["message"].as_str().unwrap().contains("udf remove"));
    }

    #[test]
    fn udf_deploy_human_detail_values_share_one_column() {
        let out = UdfDeployOutput {
            name: "my_fn".into(),
            server: "default".into(),
            r#type: "executable".into(),
            runtime: "python3.11".into(),
            reloaded: true,
            loaded: Some(true),
            interpreter: Some("/usr/bin/python3".into()),
            ignored_fields: vec!["poolSize".into(), "memoryLimitMib".into()],
            ignored_files: vec!["amd64/".into()],
            warnings: vec![],
            function_config: "fn.xml".into(),
            scripts_dir: "scripts/my_fn".into(),
        };
        let text = out.to_string();
        let details: Vec<&str> = text.lines().filter(|line| line.starts_with("  ")).collect();
        assert_eq!(details.len(), 5, "{text}");
        assert!(
            details.iter().any(|line| line.ends_with(" amd64/")),
            "{text}"
        );
        let columns: Vec<usize> = details
            .iter()
            .map(|line| {
                let label_end = line.find(':').unwrap() + 1;
                label_end + line[label_end..].len() - line[label_end..].trim_start().len()
            })
            .collect();
        assert!(columns.windows(2).all(|pair| pair[0] == pair[1]), "{text}");
    }

    /// A deploy to a stopped server ends differently from a reloaded one and
    /// names the server that loads the function later.
    #[test]
    fn udf_deploy_human_last_line_follows_the_reload() {
        let last_line = |reloaded| {
            let text = UdfDeployOutput {
                name: "my_fn".into(),
                server: "dev".into(),
                r#type: "executable".into(),
                runtime: "python3.11".into(),
                reloaded,
                loaded: reloaded.then_some(true),
                interpreter: None,
                ignored_fields: vec![],
                ignored_files: vec![],
                warnings: vec![],
                function_config: "fn.xml".into(),
                scripts_dir: "scripts/my_fn".into(),
            }
            .to_string();
            text.lines().last().unwrap().to_string()
        };
        let (reloaded, deferred) = (last_line(true), last_line(false));
        assert_ne!(reloaded, deferred);
        assert!(reloaded.contains("my_fn"), "{reloaded}");
        assert!(deferred.contains("'dev'"), "{deferred}");
    }

    /// Each local UDF failure's recovery command is one that helps: none
    /// where no command adds to the message.
    #[test]
    fn udf_error_hints_name_a_command_that_helps() {
        let command = |error: Error| error_json(&error)["error"]["command"].clone();
        let none = serde_json::Value::Null;
        assert_eq!(
            command(Error::UdfDefinitionParse {
                path: "clickhouse/udfs/my_fn/udf.json".into(),
                source: serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
            }),
            none
        );
        assert_eq!(
            command(Error::UdfDefinitionInvalid {
                path: "clickhouse/udfs/my_fn/udf.json".into(),
                reason: "functionName 'other' does not match".into(),
            }),
            none
        );
        assert_eq!(
            command(Error::UdfSourceInvalid {
                path: "clickhouse/udfs/my_fn".into(),
                reason: "is missing main.py".into(),
            }),
            none
        );
        let missing = |scaffold_here| Error::UdfSourceMissing {
            path: "clickhouse/udfs/my_fn".into(),
            name: "my_fn".into(),
            scaffold_here,
        };
        assert_eq!(command(missing(true)), "clickhousectl local udf init my_fn");
        assert_eq!(command(missing(false)), none);
        assert_eq!(
            command(Error::UdfInterpreterNotFound("not found".into())),
            "clickhousectl local udf deploy --help"
        );
        assert_eq!(
            command(Error::UdfRuntimeUnsupported("Linux only".into())),
            "clickhousectl local udf deploy --help"
        );
        assert_eq!(
            command(Error::UdfQueryFailed {
                server: "dev".into(),
                details: "Code: 46.".into(),
            }),
            none
        );
        let not_running = error_json(&Error::UdfServerNotRunning("dev".into()));
        assert_eq!(not_running["error"]["code"], "server_not_running");
        assert_eq!(
            not_running["error"]["command"],
            "clickhousectl local clickhouse start dev"
        );
        let not_found = |project_has_state| {
            error_json(&Error::UdfServerNotFound {
                name: "dev".into(),
                project_dir: "/work/app".into(),
                project_has_state,
            })["error"]
                .clone()
        };
        let root = not_found(true);
        assert_eq!(root["command"], "clickhousectl local clickhouse start dev");
        assert!(root["message"].as_str().unwrap().contains("'/work/app'"));
        let subdir = not_found(false);
        assert_eq!(
            subdir["command"],
            "clickhousectl local clickhouse list --global"
        );
        let message = subdir["message"].as_str().unwrap();
        assert!(message.contains("'/work/app'"), "{message}");
        assert!(
            !message.contains("clickhousectl local clickhouse start"),
            "{message}"
        );
    }

    #[test]
    fn udf_errors_carry_foreign_text_in_details_and_keep_self_composed_messages() {
        // The message names the file and, when serde knows it, the position;
        // serde's own text goes to `details`.
        let source = serde_json::from_str::<serde_json::Value>("{\n  nope").unwrap_err();
        let serde_text = source.to_string();
        let parse = error_json(&Error::UdfDefinitionParse {
            path: "clickhouse/udfs/my_fn/udf.json".into(),
            source,
        });
        let message = parse["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("'clickhouse/udfs/my_fn/udf.json'"),
            "{message}"
        );
        assert_eq!(numbers(message), ["2", "3"], "{message}");
        assert!(!message.contains(&serde_text), "{message}");
        assert_eq!(parse["error"]["details"], serde_text);
        let positionless = error_json(&Error::UdfDefinitionParse {
            path: "clickhouse/udfs/my_fn/udf.json".into(),
            source: serde_json::Error::io(std::io::Error::other("read failed")),
        });
        let message = positionless["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("'clickhouse/udfs/my_fn/udf.json'"),
            "{message}"
        );
        assert!(numbers(message).is_empty(), "{message}");
        assert_eq!(positionless["error"]["details"], "read failed");

        let query = error_json(&Error::UdfQueryFailed {
            server: "dev".into(),
            details: "Code: 46. DB::Exception: secret-ish server text".into(),
        });
        let message = query["error"]["message"].as_str().unwrap();
        assert!(message.contains("'dev'"), "{message}");
        assert!(!message.contains("secret-ish"), "{message}");
        assert_eq!(
            query["error"]["details"],
            "Code: 46. DB::Exception: secret-ish server text"
        );

        let missing = error_json(&Error::UdfNotFound {
            name: "my_fn".into(),
            server: "dev".into(),
        });
        let message = missing["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("'my_fn'") && message.contains("'dev'"),
            "{message}"
        );
        assert_eq!(
            missing["error"]["command"],
            "clickhousectl local udf list --server dev"
        );

        let not_loaded = error_json(&Error::UdfNotLoaded {
            name: "my_fn".into(),
            server: "dev".into(),
            log_path: "/work/.clickhouse/servers/dev/server.log".into(),
            deploy_command: "clickhousectl local udf deploy my_fn --server dev".into(),
        });
        let message = not_loaded["error"]["message"].as_str().unwrap();
        assert!(
            names(message, "my_fn")
                && names(message, "dev")
                && message.contains("/work/.clickhouse/servers/dev/server.log"),
            "{message}"
        );
        assert_eq!(
            not_loaded["error"]["command"],
            "clickhousectl local udf deploy my_fn --server dev"
        );

        let rejected_error = Error::UdfRejected(Box::new(crate::error::UdfRejection {
            name: "my_fn".into(),
            server: "dev".into(),
            source_dir: "clickhouse/udfs/my_fn".into(),
            deploy_command: "clickhousectl local udf deploy my_fn --server dev".into(),
            blocking: vec!["my_fn".into()],
            details: "Code: 50. DB::Exception: secret-ish server text".into(),
        }));
        assert!(
            rejected_error
                .to_string()
                .contains("secret-ish server text")
        );
        let rejected = error_json(&rejected_error);
        let message = rejected["error"]["message"].as_str().unwrap();
        assert!(!message.contains("secret-ish"));
        assert!(rejected_error.to_string().starts_with(message));
        assert_eq!(
            rejected["error"]["details"],
            "Code: 50. DB::Exception: secret-ish server text"
        );
        assert_eq!(
            rejected["error"]["command"],
            "clickhousectl local udf remove my_fn --server dev"
        );

        let unreachable_error = Error::UdfServerUnreachable {
            server: "dev".into(),
            port: 8123,
            details: "secret-ish client text".into(),
        };
        assert!(
            unreachable_error
                .to_string()
                .contains("secret-ish client text")
        );
        let unreachable = error_json(&unreachable_error);
        let message = unreachable["error"]["message"].as_str().unwrap();
        assert!(message.contains("'dev'"), "{message}");
        assert_eq!(numbers(message), ["8123"], "{message}");
        assert!(!message.contains("secret-ish"), "{message}");
        assert_eq!(unreachable["error"]["details"], "secret-ish client text");
        assert_eq!(
            unreachable["error"]["command"],
            "clickhousectl local clickhouse list"
        );

        let blocked_error = Error::UdfReloadBlocked {
            removed: Some("my_fn".into()),
            server: "dev".into(),
            blocking: Vec::new(),
            details: "Code: 50. DB::Exception: secret-ish server text".into(),
        };
        let blocked = error_json(&blocked_error);
        let message = blocked["error"]["message"].as_str().unwrap();
        let removed_message = message.to_owned();
        assert!(names(message, "my_fn"), "{message}");
        assert!(!message.contains("secret-ish"));
        assert!(blocked_error.to_string().starts_with(message));
        assert_eq!(
            blocked["error"]["details"],
            "Code: 50. DB::Exception: secret-ish server text"
        );
        assert_eq!(
            blocked["error"]["command"],
            "clickhousectl local udf list --server dev"
        );

        // `reload` has no target: nothing was removed and no one is blamed.
        let reload_blocked = error_json(&Error::UdfReloadBlocked {
            removed: None,
            server: "dev".into(),
            blocking: Vec::new(),
            details: "Code: 36. DB::Exception: secret-ish server text".into(),
        });
        let message = reload_blocked["error"]["message"].as_str().unwrap();
        assert!(names(message, "dev"), "{message}");
        assert!(
            message.contains("`clickhousectl local udf list --server dev`"),
            "{message}"
        );
        assert!(!message.contains("udf remove"), "{message}");
        assert_ne!(message, removed_message, "nothing was removed");
        assert!(!message.contains("secret-ish"), "{message}");
        assert_eq!(
            reload_blocked["error"]["details"],
            "Code: 36. DB::Exception: secret-ish server text"
        );
        assert_eq!(
            reload_blocked["error"]["command"],
            "clickhousectl local udf list --server dev"
        );
        let reload_culprit = error_json(&Error::UdfReloadBlocked {
            removed: None,
            server: "dev".into(),
            blocking: vec!["bad".into()],
            details: String::new(),
        });
        let message = reload_culprit["error"]["message"].as_str().unwrap();
        assert!(names(message, "bad"), "{message}");
        assert!(
            message.contains("`clickhousectl local udf remove bad --server dev`"),
            "{message}"
        );
        assert_eq!(
            reload_culprit["error"]["command"],
            "clickhousectl local udf remove bad --server dev"
        );

        // An unquotable path is reported under the code of what it belongs to.
        let mut unquotable_messages = Vec::new();
        for (role, code, command) in [
            (
                crate::error::UdfCommandPath::Interpreter,
                "udf_interpreter_not_found",
                Some("clickhousectl local udf deploy --help"),
            ),
            (
                crate::error::UdfCommandPath::StagedScript,
                "udf_source_invalid",
                None,
            ),
        ] {
            let error = error_json(&Error::UdfPathUnquotable {
                role,
                path: "/work/it's/main.py".into(),
            });
            assert_eq!(error["error"]["code"], code);
            assert_eq!(error["error"]["command"].as_str(), command);
            let message = error["error"]["message"].as_str().unwrap();
            assert!(message.contains("'/work/it's/main.py'"), "{message}");
            unquotable_messages.push(message.to_owned());
        }
        // Each role names which path it is.
        assert_ne!(unquotable_messages[0], unquotable_messages[1]);

        // The validator composes the reason; the message carries it after
        // the path.
        let reason = "is missing main.py, the entrypoint required by runtime python3.11";
        let source = error_json(&Error::UdfSourceInvalid {
            path: "clickhouse/udfs/my_fn".into(),
            reason: reason.into(),
        });
        let message = source["error"]["message"].as_str().unwrap();
        assert!(message.contains("'clickhouse/udfs/my_fn'"), "{message}");
        assert!(message.ends_with(reason), "{message}");
        assert!(source["error"].get("details").is_none());
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
        assert!(
            message.contains(&BinaryLaunchProblem::NotExecutable.to_string()),
            "{message}"
        );
        assert!(
            message.contains("/home/u/.clickhouse/versions/25.12.9.61/clickhouse"),
            "{message}"
        );
        assert_eq!(
            json["error"]["command"],
            "clickhousectl local install --force 25.12.9.61"
        );
    }

    /// A directory at the binary path cannot be fixed by `install --force`
    /// (renaming over a directory fails with EISDIR), so the recovery command
    /// goes through `local remove` first.
    #[test]
    fn directory_at_binary_path_json_error_recommends_remove_then_install() {
        let json = error_json(&Error::BinaryNotLaunchable {
            version: "25.12.9.61".into(),
            problem: BinaryLaunchProblem::NotAFile,
            path: "/home/u/.clickhouse/versions/25.12.9.61/clickhouse".into(),
        });
        let message = json["error"]["message"].as_str().expect("message");
        assert!(
            message.contains(&BinaryLaunchProblem::NotAFile.to_string()),
            "{message}"
        );
        assert!(!message.contains("--force"), "{message}");
        assert_eq!(
            json["error"]["command"],
            "clickhousectl local remove 25.12.9.61 && clickhousectl local install 25.12.9.61"
        );
    }

    #[test]
    fn version_is_default_json_error_explains_both_the_refusal_and_the_way_forward() {
        let json = error_json(&Error::VersionIsDefault {
            version: "25.12.9.61".into(),
            recovery_command: "clickhousectl local use 24.8.14.39".into(),
        });
        let message = json["error"]["message"].as_str().expect("message");

        for required in [
            "25.12.9.61",
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
            json["error"]["command"], "clickhousectl local use 24.8.14.39",
            "the JSON error must name the recovery command"
        );
    }

    /// The fallback keeps its summary as `message` and carries the subprocess
    /// or OS text in `details` (#1072).
    #[test]
    fn fallback_and_wrapped_errors_carry_foreign_text_in_details() {
        let foreign = "No such file or directory (os error 2)";
        for error in [Error::Exec(foreign.into()), Error::Postgres(foreign.into())] {
            let json = error_json(&error);
            assert_eq!(json["error"]["code"], "local_error");
            assert_eq!(json["error"]["details"], foreign);
            let message = json["error"]["message"].as_str().unwrap();
            assert!(!message.contains(foreign), "{message}");
            assert!(json["error"].get("command").is_none());
        }

        let cleanup = "cleanup-note-token";
        let wrapped = Error::PostgresStartupRollback {
            primary: Box::new(Error::StartupExit {
                kind: crate::error::StartupKind::Postgres,
                name: "default".into(),
                details: foreign.into(),
            }),
            cleanup: cleanup.into(),
        };
        // Classified by the primary failure; the cleanup note is dropped.
        let wrapped = error_json(&wrapped);
        assert_eq!(wrapped["error"]["code"], "startup_exit");
        assert_eq!(
            wrapped["error"]["command"],
            "clickhousectl local postgres list"
        );
        assert_eq!(wrapped["error"]["details"], foreign);
        let message = wrapped["error"]["message"].as_str().unwrap();
        assert!(message.contains("'default'"), "{message}");
        assert!(!wrapped.to_string().contains(cleanup));
    }

    /// Name-only errors are raised for either engine, so they point at the
    /// combined list; startup failures know their engine and point at its list.
    #[test]
    fn server_json_errors_point_at_a_list_that_shows_the_server() {
        let command = |error: Error| error_json(&error)["error"]["command"].clone();
        for error in [
            Error::ServerNotFound("app".into()),
            Error::ServerNotRunning("app".into()),
            Error::ServerAlreadyRunning("app".into()),
            Error::InvalidServerName("bad/name".into()),
        ] {
            assert_eq!(
                error_json(&error)["error"]["command"],
                "clickhousectl local status",
                "{error:?}"
            );
        }
        for (kind, list) in [
            (
                crate::error::StartupKind::ClickHouse,
                "clickhousectl local clickhouse list",
            ),
            (
                crate::error::StartupKind::Postgres,
                "clickhousectl local postgres list",
            ),
        ] {
            let exit = command(Error::StartupExit {
                kind,
                name: "default".into(),
                details: String::new(),
            });
            assert_eq!(exit, list, "startup_exit for {kind}");
            let timeout = command(Error::StartupTimeout {
                kind,
                name: "default".into(),
                seconds: 30,
                details: String::new(),
            });
            assert_eq!(timeout, list, "startup_timeout for {kind}");
        }
    }

    #[test]
    fn running_server_remove_json_error_points_at_stopping_that_server() {
        let json = error_json(&Error::ServerRunningCannotRemove {
            name: "dev".into(),
            command: "clickhousectl local clickhouse stop dev".into(),
        });
        assert_eq!(json["error"]["code"], "server_running");
        assert_eq!(
            json["error"]["command"],
            "clickhousectl local clickhouse stop dev"
        );
        let message = json["error"]["message"].as_str().unwrap();
        assert!(message.contains("'dev'"), "{message}");
        assert!(
            message.contains("`clickhousectl local clickhouse stop dev`"),
            "{message}"
        );
        assert_eq!(json["error"].as_object().unwrap().len(), 3);
    }

    #[test]
    fn missing_config_json_error_keeps_the_full_human_detail() {
        let error = Error::ConfigNotFound(
            "config 'does-not-exist' not found in /home/dev/.clickhouse/configs \
             (available: analytics.xml)"
                .to_string(),
        );

        assert_eq!(
            error_json(&error),
            serde_json::json!({
                "error": {
                    "code": "config_not_found",
                    "message": "config 'does-not-exist' not found in /home/dev/.clickhouse/configs (available: analytics.xml)",
                    "command": "clickhousectl local clickhouse configs"
                }
            })
        );
    }

    #[test]
    fn a_version_that_is_not_installed_is_not_reported_as_unavailable_for_download() {
        let missing = error_json(&Error::VersionNotFound("25.12.9.61".into()));
        assert_eq!(missing["error"]["code"], "version_not_installed");
        assert_eq!(missing["error"]["command"], "clickhousectl local list");
        let message = missing["error"]["message"].as_str().unwrap();
        assert!(message.contains("25.12.9.61"), "{message}");
        assert_eq!(missing["error"].as_object().unwrap().len(), 3);

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
            Error::ServerRunningCannotRemove {
                name: "dev".into(),
                command: "clickhousectl local clickhouse stop dev".into(),
            },
            Error::ServerSelectionRequired {
                engine: crate::local::server::Engine::Clickhouse,
                names: vec!["alpha".into(), "beta".into()],
                usage: "clickhousectl local clickhouse stop <name>",
            },
            Error::ServerRemoveSelectionRequired { available: 1 },
            Error::ServerInMultipleProjects {
                name: "dev".into(),
                projects: "/projects/a, /projects/b".into(),
            },
            Error::InvalidServerName("../escape".into()),
            Error::ConfigNotFound("config 'x' not found in /configs (available: y.xml)".into()),
            Error::InvalidConfigName("../etc/passwd".into()),
            Error::VersionNotFound("25.12.9.61".into()),
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
            Error::ClientLoginUnsupported {
                version: "25.11.2.24".into(),
                minimum: "25.12",
            },
            Error::VersionAlreadyInstalled("25.12.9.61".into()),
            Error::VersionInUse {
                version: "25.12.9.61".into(),
                servers: "dev (/project, pid 42)".into(),
            },
            Error::VersionIsDefault {
                version: "25.12.9.61".into(),
                recovery_command: "clickhousectl local use latest".into(),
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
            Error::PostgresResumePortInUse {
                name: "dev".into(),
                port: 5432,
            },
            Error::PostgresPortNotPublished {
                name: "dev".into(),
                port: 5432,
            },
            Error::DockerNotAvailable("Docker socket was not found.\nStart Docker Desktop.".into()),
            Error::ContainerNameConflict("chctl-pg-dev-17".into()),
            Error::PostgresUsage(
                "multiple postgres instances named 'dev' (17, 18); pass --version to select one"
                    .into(),
            ),
            // The missing-`psql` pre-flight (#471): its text is composed by
            // this CLI, so the install hint must survive into JSON.
            Error::PostgresUsage(
                "could not execute psql: not found on PATH (install the PostgreSQL client tools)"
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

    #[test]
    fn sql_input_errors_carry_the_os_cause_in_details_but_not_the_path() {
        let path = "private-path-token.sql";
        let cause = "Permission denied (os error 13)";
        for (error, code) in [
            (
                Error::SqlInputOpen {
                    path: path.into(),
                    source: std::io::Error::other(cause),
                },
                "sql_input_open_failed",
            ),
            (
                Error::SqlInputRead(std::io::Error::other(cause)),
                "sql_input_read_failed",
            ),
        ] {
            let json = error_json(&error);
            assert_eq!(json["error"]["code"], code);
            assert_eq!(json["error"]["details"], cause);
            assert!(!json.to_string().contains(path));
            // A curated summary, not the human text.
            assert_ne!(json["error"]["message"], error.to_string());
            assert_eq!(error.exit_code(), 1);
        }
    }

    /// The complement of the parity rule: text that interpolates foreign output
    /// (subprocess, Docker daemon, OS/serde sources) keeps a summary as
    /// `message`, and the foreign text goes in `details` (#1072).
    #[test]
    fn errors_carrying_foreign_output_put_it_in_details_beside_a_summary() {
        let foreign = "foreign-text-token";
        let path = "private-path-token";
        let json_source = serde_json::from_str::<serde_json::Value>("{")
            .expect_err("invalid fixture must fail to parse");
        let utf8_source = String::from_utf8(vec![0xff]).expect_err("invalid UTF-8 fixture");
        // Each summary is fixed per code, so the identifiers it must carry are
        // listed instead of its wording.
        let cases: Vec<(Error, String, &[&str])> = vec![
            (Error::Exec(foreign.into()), foreign.into(), &[]),
            (Error::DockerError(foreign.into()), foreign.into(), &[]),
            (Error::Postgres(foreign.into()), foreign.into(), &[]),
            (Error::Download(foreign.into()), foreign.into(), &[]),
            (Error::Extract(foreign.into()), foreign.into(), &[]),
            (
                Error::ExtractArchive {
                    archive: path.into(),
                    destination: path.into(),
                    source: std::io::Error::other(foreign),
                },
                foreign.into(),
                &[],
            ),
            (
                Error::Io(std::io::Error::other(foreign)),
                foreign.into(),
                &[],
            ),
            (Error::Json(json_source), String::new(), &[]),
            (
                Error::CreateDir {
                    path: path.into(),
                    source: std::io::Error::other(foreign),
                },
                foreign.into(),
                &[],
            ),
            (
                Error::ServerMetadataPermission {
                    path: path.into(),
                    source: std::io::Error::other(foreign),
                },
                foreign.into(),
                &[],
            ),
            (
                Error::ServerMetadataRead {
                    path: path.into(),
                    source: std::io::Error::other(foreign),
                },
                foreign.into(),
                &[],
            ),
            (
                Error::ServerMetadataUtf8 {
                    path: path.into(),
                    source: utf8_source,
                },
                String::new(),
                &[],
            ),
            (
                Error::ServerMetadataWrite {
                    path: path.into(),
                    source: std::io::Error::other(foreign),
                },
                foreign.into(),
                &[],
            ),
            (
                Error::ServerLock {
                    operation: "open the server metadata lock file",
                    path: path.into(),
                    remediation: "Check access, then retry.",
                    source: std::io::Error::other(foreign),
                },
                foreign.into(),
                &[],
            ),
            (
                Error::StartupExit {
                    kind: crate::error::StartupKind::Postgres,
                    name: "default".into(),
                    details: foreign.into(),
                },
                foreign.into(),
                &["'default'"],
            ),
            (
                Error::StartupTimeout {
                    kind: crate::error::StartupKind::ClickHouse,
                    name: "default".into(),
                    seconds: 30,
                    details: foreign.into(),
                },
                foreign.into(),
                &["'default'", "30"],
            ),
            (
                Error::NotAClickHouseBinary {
                    path: path.into(),
                    reason: foreign.into(),
                },
                foreign.into(),
                &["--binary"],
            ),
        ];

        for (error, expected_details, identifiers) in cases {
            let json = error_json(&error);
            let message = json["error"]["message"].as_str().unwrap().to_owned();
            assert_ne!(message, error.to_string(), "{error:?} must be summarized");
            for identifier in identifiers {
                assert!(message.contains(identifier), "{error:?}: {message}");
            }
            assert!(
                !message.contains(foreign) && !message.contains(path),
                "{error:?}: {message}"
            );
            let details = json["error"]["details"]
                .as_str()
                .unwrap_or_else(|| panic!("{error:?} must carry details"));
            // The serde fixtures' text is serde's own, so only its presence
            // in the human line is asserted.
            if expected_details.is_empty() {
                assert!(error.to_string().contains(details), "{error:?}: {details}");
            } else {
                assert_eq!(details, expected_details, "{error:?}");
            }
            assert!(!details.contains(path), "{error:?}: {details}");
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
        assert_eq!(plain.lines().count(), 1, "{plain}");
        assert!(plain.contains("25.12.5.44"), "{plain}");

        let cleared = RemoveOutput {
            version: "25.12.5.44".to_string(),
            was_default: true,
        }
        .to_string();
        // The same first line, then the warning.
        assert_eq!(cleared.lines().next(), Some(plain.as_str()), "{cleared}");
        assert!(cleared.lines().count() > 1, "{cleared}");
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
            enclosing_project: None,
            project_dir: None,
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
    fn init_json_idempotent_run_reports_no_created_paths() {
        let output = InitOutput {
            paths: vec![],
            already_initialized: true,
            enclosing_project: None,
            project_dir: None,
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["paths"], serde_json::json!([]));
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
            project_scope: Some(server_project_scope(&project(ProjectRootKind::Marked))),
            guidance: Vec::new(),
            list: ServerList::Project(ProjectList::Clickhouse),
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
        assert_eq!(json["project_scope"]["kind"], "project_root");
    }

    #[test]
    fn server_list_json_empty() {
        let output = ServerListOutput {
            servers: vec![],
            total_servers: 0,
            total_running_servers: 0,
            project_scope: Some(server_project_scope(&project(ProjectRootKind::Unmarked))),
            guidance: project_scope_guidance(ProjectRootKind::Unmarked, None),
            list: ServerList::Project(ProjectList::Clickhouse),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["servers"].as_array().unwrap().len(), 0);
        assert_eq!(json["total_servers"], 0);
        assert_eq!(json["total_running_servers"], 0);
        assert_eq!(json["project_scope"]["kind"], "current_directory");
        assert_eq!(json["guidance"][0]["action"], "return_to_project_root");
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
        let render = |already_stopped| {
            ServerStopOutput {
                name: "default".to_string(),
                already_stopped,
                selection: Some(ServerSelection::Implicit),
            }
            .to_string()
        };
        let (noop, stopped) = (render(true), render(false));
        assert_ne!(noop, stopped);
        for text in [&noop, &stopped] {
            assert_eq!(text.lines().count(), 1, "{text}");
            assert!(text.contains("'default'"), "{text}");
        }

        let output = ServerStopOutput {
            name: "default".to_string(),
            already_stopped: true,
            selection: Some(ServerSelection::Implicit),
        };

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

    /// The trimmed cells of the table row whose first cell is `first`.
    fn table_row(text: &str, first: &str) -> Vec<String> {
        let row = text
            .lines()
            .find(|line| line.split('|').nth(1).map(str::trim) == Some(first))
            .unwrap_or_else(|| panic!("no row for {first}: {text}"));
        row.split('|').map(|cell| cell.trim().to_string()).collect()
    }

    #[test]
    fn version_tables_mark_only_the_flagged_row() {
        let installed = ListInstalledOutput {
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
        }
        .to_string();
        assert_eq!(
            table_row(&installed, "Version"),
            ["", "Version", "Default", ""]
        );
        assert_eq!(
            table_row(&installed, "25.12.5.44"),
            ["", "25.12.5.44", "yes", ""]
        );
        assert_eq!(
            table_row(&installed, "25.11.3.22"),
            ["", "25.11.3.22", "", ""]
        );

        let available = ListAvailableOutput {
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
        }
        .to_string();
        assert_eq!(
            table_row(&available, "Version"),
            ["", "Version", "Installed", ""]
        );
        assert_eq!(table_row(&available, "25.12"), ["", "25.12", "yes", ""]);
        assert_eq!(table_row(&available, "25.11"), ["", "25.11", "", ""]);
        assert!(
            available.contains("clickhousectl local install <version>"),
            "{available}"
        );
    }

    #[test]
    fn list_installed_display_empty_suggests_a_runnable_latest_install() {
        use crate::cli::{Cli, Commands};
        use crate::local::cli::{InstallVersionArg, LocalCommands};
        use clap::Parser;

        let text = ListInstalledOutput { versions: vec![] }.to_string();
        let start = text.find("clickhousectl ").expect("a suggested command");
        let argv: Vec<&str> = text[start..].split_whitespace().collect();
        let Commands::Local(local) = Cli::try_parse_from(&argv).unwrap().command else {
            panic!("expected a local command: {argv:?}");
        };
        // Docs and suggestions use `latest`, never `stable`.
        assert!(
            matches!(
                local.command,
                LocalCommands::Install {
                    version: Some(InstallVersionArg::ClickHouse(
                        crate::version_manager::VersionSpec::Latest
                    )),
                    binary: None,
                    force: false,
                }
            ),
            "{argv:?}"
        );
    }

    #[test]
    fn init_display_heads_with_the_run_kind_and_lists_only_scaffolds() {
        let render = |paths: &[&str], already_initialized| {
            InitOutput {
                paths: paths.iter().map(|path| path.to_string()).collect(),
                already_initialized,
                enclosing_project: None,
                project_dir: None,
            }
            .to_string()
        };
        // One headline per run kind.
        let existing = render(&[], true);
        let restored = render(&[".clickhouse/.gitignore"], true);
        let fresh = render(&[".clickhouse/"], false);
        for headline in [&existing, &restored, &fresh] {
            assert_eq!(headline.lines().count(), 1, "{headline}");
        }
        assert!(existing.ends_with(".clickhouse/"), "{existing}");
        assert!(restored.ends_with(".clickhouse/.gitignore"), "{restored}");
        assert!(fresh.ends_with(".clickhouse/"), "{fresh}");
        assert_ne!(existing, restored);
        assert_ne!(existing, fresh);
        assert_ne!(restored, fresh);

        // Then one line per scaffold outside `.clickhouse/`, in order.
        for (paths, already_initialized, headline, scaffolds) in [
            (&["postgres/"][..], true, &existing, &["postgres/"][..]),
            (
                &[".clickhouse/", "clickhouse/", "postgres/"][..],
                false,
                &fresh,
                &["clickhouse/", "postgres/"][..],
            ),
        ] {
            let text = render(paths, already_initialized);
            let lines: Vec<&str> = text.lines().collect();
            assert_eq!(lines[0], headline.as_str(), "{text}");
            assert_eq!(lines.len(), 1 + scaffolds.len(), "{text}");
            for (line, scaffold) in lines[1..].iter().zip(scaffolds) {
                assert!(line.ends_with(&format!(" {scaffold}")), "{text}");
            }
            assert_eq!(
                lines[1].split(scaffolds[0]).next(),
                lines
                    .last()
                    .unwrap()
                    .split(scaffolds.last().unwrap())
                    .next(),
                "{text}"
            );
        }
    }

    #[test]
    fn udf_list_and_remove_human_output_follow_the_server_state() {
        let entry = |name: &str, loaded, last_deploy_rejected| UdfListEntry {
            name: name.to_string(),
            r#type: Some("executable".to_string()),
            runtime: Some("python3.11".to_string()),
            loaded,
            last_deploy_rejected,
        };
        let list = |server_running, udfs| {
            UdfListOutput {
                server: "dev".to_string(),
                server_running,
                udfs,
            }
            .to_string()
        };

        let empty = list(false, Vec::new());
        assert_eq!(empty.lines().count(), 1, "{empty}");
        assert!(empty.contains("'dev'") && !empty.contains('|'), "{empty}");

        let states = [
            (Some(true), false),
            (Some(true), true),
            (Some(false), false),
            (Some(false), true),
            (None, false),
            (None, true),
        ];
        let udfs = || {
            states
                .iter()
                .enumerate()
                .map(|(i, (loaded, rejected))| entry(&format!("f{i}"), *loaded, *rejected))
                .collect()
        };
        let running = list(true, udfs());
        let lines: Vec<&str> = running.lines().collect();
        assert_eq!(lines.len(), 2 + states.len(), "{running}");
        assert!(lines.iter().all(|line| line.starts_with('|')), "{running}");
        let loaded_cells: Vec<&str> = lines[2..]
            .iter()
            .map(|line| line.trim_matches('|').split('|').nth(3).unwrap().trim())
            .collect();
        for (cell, (loaded, rejected)) in loaded_cells.iter().zip(states) {
            let prefix = match loaded {
                Some(true) => "yes",
                Some(false) => "no",
                None => "-",
            };
            assert!(cell.starts_with(prefix), "{cell}");
            assert_eq!(*cell != prefix, rejected, "{cell}");
        }
        let distinct: std::collections::HashSet<_> = loaded_cells.iter().collect();
        assert_eq!(distinct.len(), states.len(), "{loaded_cells:?}");

        // Stopped: the same table, then one trailing note naming the server.
        let stopped = list(false, udfs());
        let note = stopped.strip_prefix(&running).expect("table first");
        assert_eq!(note.lines().filter(|line| !line.is_empty()).count(), 1);
        assert!(note.contains("'dev'"), "{note}");

        let remove = |reloaded| {
            UdfRemoveOutput {
                name: "my_fn".to_string(),
                server: "dev".to_string(),
                reloaded,
            }
            .to_string()
        };
        let (reloaded, deferred) = (remove(true), remove(false));
        assert_ne!(reloaded, deferred);
        for text in [&reloaded, &deferred] {
            assert!(text.contains("my_fn") && text.contains("'dev'"), "{text}");
        }
    }

    #[test]
    fn optional_suffixes_appear_only_when_their_flag_is_set() {
        let install = |set_as_default| {
            InstallOutput {
                version: "25.12.5.44".to_string(),
                set_as_default,
            }
            .to_string()
        };
        let plain = install(false);
        let default = install(true);
        assert!(default.starts_with(&plain) && default.len() > plain.len());

        let stop = |selection| {
            ServerStopOutput {
                name: "default".to_string(),
                already_stopped: false,
                selection,
            }
            .to_string()
        };
        let remove = |selection| {
            ServerRemoveOutput {
                name: "default".to_string(),
                selection,
            }
            .to_string()
        };
        for render in [&stop as &dyn Fn(_) -> String, &remove] {
            let plain = render(None);
            assert_eq!(render(Some(ServerSelection::Explicit)), plain);
            let implicit = render(Some(ServerSelection::Implicit));
            assert!(implicit.starts_with(&plain) && implicit.len() > plain.len());
        }
    }

    #[test]
    fn human_outputs_show_every_value_they_carry() {
        let configs_dir = "/home/user/.clickhouse/configs";
        let global_dir = "/home/user/.clickhouse/config.d";
        let project_dir = "/home/user/app/clickhouse/config.d";
        let strings = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let configs_output = |configs: &[&str], global: &[&str], project: &[&str]| {
            ServerConfigsOutput {
                dir: configs_dir.to_string(),
                configs: strings(configs),
                global_config_d: ConfigLayerOutput {
                    dir: global_dir.to_string(),
                    files: strings(global),
                },
                project_config_d: ConfigLayerOutput {
                    dir: project_dir.to_string(),
                    files: strings(project),
                },
            }
            .to_string()
        };
        for (text, values) in [
            (
                ServerStartOutput {
                    name: "analytics".to_string(),
                    pid: 4242,
                    http_port: 18123,
                    tcp_port: 19000,
                    version: "25.12.5.44".to_string(),
                }
                .to_string(),
                &["analytics", "4242", "18123", "19000", "25.12.5.44"][..],
            ),
            (
                WhichOutput {
                    version: "25.12.5.44".to_string(),
                    binary_path: "/home/user/.clickhouse/versions/25.12.5.44/clickhouse"
                        .to_string(),
                }
                .to_string(),
                &[
                    "25.12.5.44",
                    "/home/user/.clickhouse/versions/25.12.5.44/clickhouse",
                ],
            ),
            (
                UseOutput {
                    version: "25.12.5.44".to_string(),
                }
                .to_string(),
                &["25.12.5.44"],
            ),
            (
                InstallOutput {
                    version: "25.12.5.44".to_string(),
                    set_as_default: false,
                }
                .to_string(),
                &["25.12.5.44"],
            ),
            (
                configs_output(
                    &["dev.xml", "prod.yaml"],
                    &["memory.xml"],
                    &["logging.yaml", "users.xml"],
                ),
                &[
                    configs_dir,
                    "dev.xml",
                    "prod.yaml",
                    global_dir,
                    "memory.xml",
                    project_dir,
                    "logging.yaml",
                    "users.xml",
                ][..],
            ),
            (
                configs_output(&[], &[], &[]),
                &[configs_dir, global_dir, project_dir][..],
            ),
        ] {
            for value in values {
                assert!(text.contains(value), "missing {value:?}: {text}");
            }
        }

        // Empty lists say so instead of printing a header-only table.
        for empty in [
            ListAvailableOutput { versions: vec![] }.to_string(),
            ServerListOutput {
                servers: vec![],
                total_servers: 0,
                total_running_servers: 0,
                project_scope: None,
                guidance: Vec::new(),
                list: ServerList::Project(ProjectList::Clickhouse),
            }
            .to_string(),
        ] {
            assert!(!empty.is_empty() && !empty.contains('|'), "{empty}");
        }
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
            list: ServerList::Project(ProjectList::Clickhouse),
        };
        let text = output.to_string();
        assert_eq!(
            table_row(&text, "Name"),
            [
                "",
                "Name",
                "Status",
                "PID",
                "Version",
                "HTTP Port",
                "TCP Port",
                ""
            ]
        );
        assert_eq!(
            table_row(&text, "default"),
            [
                "",
                "default",
                "running",
                "12345",
                "25.12.5.44",
                "8123",
                "9000",
                ""
            ]
        );
        assert_eq!(
            table_row(&text, "test"),
            ["", "test", "stopped", "-", "-", "-", "-", ""]
        );
        let summary = text.lines().last().unwrap();
        assert!(!summary.contains('|'), "{text}");
        assert_eq!(numbers(summary), ["2", "1"], "{text}");
    }

    #[test]
    fn server_list_display_marks_unavailable_postgres_and_global_fields() {
        let postgres = ServerListOutput {
            servers: vec![ServerListEntry {
                name: "pg".to_string(),
                running: true,
                pid: None,
                version: Some("postgres:18".to_string()),
                http_port: None,
                tcp_port: Some(5432),
                project: None,
                engine: "postgres".to_string(),
                container_id: Some("1234567890abcdef".to_string()),
            }],
            total_servers: 3,
            total_running_servers: 1,
            project_scope: None,
            guidance: Vec::new(),
            list: ServerList::Project(ProjectList::Postgres),
        }
        .to_string();
        let postgres_row = postgres
            .lines()
            .find(|line| line.contains("| pg"))
            .expect("Postgres server row");
        let postgres_cells: Vec<_> = postgres_row.split('|').map(str::trim).collect();
        assert_eq!(
            postgres_cells,
            [
                "",
                "pg",
                "postgres",
                "running",
                "1234567890ab",
                "postgres:18",
                "-",
                "5432",
                ""
            ]
        );
        // The engine-aware table ends with the same totals line.
        let summary = postgres.lines().last().unwrap();
        assert!(!summary.contains('|'), "{postgres}");
        assert_eq!(numbers(summary), ["3", "1"], "{postgres}");

        let global = ServerListOutput {
            servers: vec![ServerListEntry {
                name: "recovered".to_string(),
                running: true,
                pid: Some(42),
                version: None,
                http_port: None,
                tcp_port: None,
                project: Some("/project".to_string()),
                engine: "clickhouse".to_string(),
                container_id: None,
            }],
            total_servers: 1,
            total_running_servers: 1,
            project_scope: None,
            guidance: Vec::new(),
            list: ServerList::Global,
        }
        .to_string();
        let global_row = global
            .lines()
            .find(|line| line.contains("| recovered"))
            .expect("globally discovered server row");
        let global_cells: Vec<_> = global_row.split('|').map(str::trim).collect();
        assert_eq!(
            global_cells,
            [
                "",
                "recovered",
                "running",
                "42",
                "-",
                "-",
                "-",
                "/project",
                ""
            ]
        );
    }

    /// Without a project the hint is to change to one and list there; inside
    /// one only the cross-project list remains. Postgres has no global list.
    #[test]
    fn empty_engine_lists_point_at_their_own_command() {
        for (list, command, global) in [
            (
                ProjectList::Clickhouse,
                "clickhousectl local clickhouse list",
                true,
            ),
            (
                ProjectList::Postgres,
                "clickhousectl local postgres list",
                false,
            ),
            (ProjectList::Status, "clickhousectl local status", true),
        ] {
            for kind in [
                ProjectRootKind::Unmarked,
                ProjectRootKind::Marked,
                ProjectRootKind::Override,
            ] {
                let guidance = empty_list_guidance(list, kind);
                let actions: Vec<_> = guidance.iter().map(|hint| hint.action).collect();
                let mut expected = match kind {
                    ProjectRootKind::Unmarked => vec![
                        LocalGuidanceAction::ReturnToProjectRoot,
                        LocalGuidanceAction::ListProjectServers,
                    ],
                    ProjectRootKind::Marked | ProjectRootKind::Override => Vec::new(),
                };
                if global {
                    expected.push(LocalGuidanceAction::ListGlobalServers);
                }
                assert_eq!(actions, expected, "{list:?} {kind:?}");
                if kind == ProjectRootKind::Unmarked {
                    assert_eq!(guidance[1].command, Some(command));
                }

                let output = ServerListOutput {
                    servers: vec![],
                    total_servers: 0,
                    total_running_servers: 0,
                    project_scope: Some(server_project_scope(&project(kind))),
                    guidance,
                    list: ServerList::Project(list),
                };
                let text = output.to_string();
                assert!(text.contains("'/project'"), "{text}");
                assert_eq!(
                    text.contains(&format!("`{command}`")),
                    kind == ProjectRootKind::Unmarked,
                    "{text}"
                );
                assert_eq!(text.contains("--global"), global, "{text}");
            }
        }
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
            list: ServerList::Project(ProjectList::Clickhouse),
        };
        let text = output.to_string();
        let summary = text.lines().last().unwrap();
        assert!(!summary.contains('|'), "{text}");
        assert_eq!(numbers(summary), ["1", "1"], "{text}");
    }
}
