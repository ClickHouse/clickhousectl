//! Structured output types for local commands.
//!
//! Successful output types support both JSON serialization and human-readable
//! display. Runtime failures use the redacted stable envelope below.

use crate::error::{
    Error, ManagedClientError, ManagedClientErrorKind, ManagedClientSelection, NetworkStage,
    PortKind,
};
use serde::Serialize;
use std::fmt;
use std::io::Write;
use tabled::{Table, Tabled, settings::Style};

/// Stable codes for local runtime failures. New codes may be added, but
/// existing spellings and meanings are part of the machine-output contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocalErrorCode {
    ManagedClientServerNotFound,
    ManagedClientServerNotRunning,
    ManagedClientBinaryNotFound,
    ServerNotFound,
    ServerSelectionRequired,
    ServerNotRunning,
    ServerRunning,
    InvalidVersion,
    VersionUnavailable,
    PortInUse,
    StartupExit,
    StartupTimeout,
    DownloadFailed,
    IoError,
    LocalError,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalErrorDetail {
    code: LocalErrorCode,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocalClientMode {
    Managed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocalProjectScopeKind {
    ExactCurrentProject,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalProjectScope {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocalGuidanceAction {
    ListProjectServers,
    ReturnToProjectRoot,
    StartDefaultServer,
    StartNamedServer,
    InstallSelectedVersion,
    UseDirectHost,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalGuidance {
    action: LocalGuidanceAction,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<&'static str>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ManagedClientErrorDetail {
    code: LocalErrorCode,
    message: &'static str,
    mode: LocalClientMode,
    project_scope: LocalProjectScope,
    server: LocalManagedServer,
    guidance: Vec<LocalGuidance>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
enum LocalErrorBody {
    General(LocalErrorDetail),
    ManagedClient(ManagedClientErrorDetail),
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct LocalErrorOutput {
    error: LocalErrorBody,
}

impl LocalErrorOutput {
    fn from_error(error: &Error) -> Self {
        if let Error::ManagedClient(error) = error {
            return Self {
                error: LocalErrorBody::ManagedClient(ManagedClientErrorDetail::from_error(error)),
            };
        }
        let detail = match error {
            Error::ServerNotFound(name) => LocalErrorDetail {
                code: LocalErrorCode::ServerNotFound,
                message: format!("Server '{name}' not found"),
                command: Some("clickhousectl local server list"),
            },
            Error::ServerStopSelectionRequired { available } => LocalErrorDetail {
                code: LocalErrorCode::ServerSelectionRequired,
                message: format!(
                    "Multiple non-default ClickHouse servers exist (available: {available}); specify a name or use stop-all"
                ),
                command: Some("clickhousectl local server list"),
            },
            Error::ServerRemoveSelectionRequired { available } => LocalErrorDetail {
                code: LocalErrorCode::ServerSelectionRequired,
                message: format!(
                    "The default ClickHouse server does not exist (custom ClickHouse servers available: {available}); no server was removed"
                ),
                command: Some("clickhousectl local server list"),
            },
            Error::ServerNotRunning(name) => LocalErrorDetail {
                code: LocalErrorCode::ServerNotRunning,
                message: format!("Server '{name}' is not running"),
                command: Some("clickhousectl local server list"),
            },
            Error::ServerAlreadyRunning(name) => LocalErrorDetail {
                code: LocalErrorCode::ServerRunning,
                message: format!("Server '{name}' is already running"),
                command: Some("clickhousectl local server list"),
            },
            Error::ServerRunningCannotRemove(name) => LocalErrorDetail {
                code: LocalErrorCode::ServerRunning,
                message: format!("Server '{name}' is running"),
                command: Some("clickhousectl local server list"),
            },
            Error::VersionInUse { .. } => LocalErrorDetail {
                code: LocalErrorCode::ServerRunning,
                message: "A running server is using this version".to_string(),
                command: Some("clickhousectl local server list"),
            },
            Error::InvalidVersion(_) => LocalErrorDetail {
                code: LocalErrorCode::InvalidVersion,
                message: "Invalid version".to_string(),
                command: Some("clickhousectl local install --help"),
            },
            Error::VersionNotFound(_)
            | Error::NoVersionsInstalled
            | Error::NoDefaultVersion
            | Error::NoClientVersionInstalled
            | Error::AmbiguousClientVersion
            | Error::StaleDefaultVersion(_)
            | Error::ClientVersionNotInstalled(_)
            | Error::RepeatedClientQueryUnsupported { .. }
            | Error::NoMatchingVersion(_)
            | Error::ExactVersionUnavailable { .. }
            | Error::UnknownVersionChannel(_)
            | Error::VersionResolutionFallback { .. } => LocalErrorDetail {
                code: LocalErrorCode::VersionUnavailable,
                message: "Requested version is unavailable".to_string(),
                command: Some("clickhousectl local list --remote"),
            },
            Error::PortInUse { kind, port } => LocalErrorDetail {
                code: LocalErrorCode::PortInUse,
                message: format!("{kind} port {port} is already in use"),
                command: Some(match kind {
                    PortKind::Postgres => "clickhousectl local postgres start --help",
                    PortKind::Http | PortKind::Tcp => "clickhousectl local server start --help",
                }),
            },
            Error::PortUnavailable(kind) => LocalErrorDetail {
                code: LocalErrorCode::PortInUse,
                message: format!("No free {kind} port is available"),
                command: Some(match kind {
                    PortKind::Postgres => "clickhousectl local postgres start --help",
                    PortKind::Http | PortKind::Tcp => "clickhousectl local server start --help",
                }),
            },
            Error::StartupExit { kind, name, .. } => LocalErrorDetail {
                code: LocalErrorCode::StartupExit,
                message: format!("{kind} server '{name}' exited before becoming ready"),
                command: Some("clickhousectl local server list"),
            },
            Error::StartupTimeout {
                kind,
                name,
                seconds,
                ..
            } => LocalErrorDetail {
                code: LocalErrorCode::StartupTimeout,
                message: format!(
                    "{kind} server '{name}' did not become ready within {seconds} seconds"
                ),
                command: Some("clickhousectl local server list"),
            },
            Error::Download(_) | Error::Extract(_) => LocalErrorDetail {
                code: LocalErrorCode::DownloadFailed,
                message: "Download failed".to_string(),
                command: None,
            },
            Error::Network(failure)
                if matches!(
                    failure.stage,
                    NetworkStage::DownloadHeaders
                        | NetworkStage::DownloadBody
                        | NetworkStage::Download
                ) =>
            {
                LocalErrorDetail {
                    code: LocalErrorCode::DownloadFailed,
                    message: "Download failed".to_string(),
                    command: None,
                }
            }
            Error::Network(_) => LocalErrorDetail {
                code: LocalErrorCode::VersionUnavailable,
                message: "Requested version is unavailable".to_string(),
                command: Some("clickhousectl local list --remote"),
            },
            Error::Io(_)
            | Error::Json(_)
            | Error::CreateDir { .. }
            | Error::ServerMetadataRead { .. }
            | Error::ServerMetadataUtf8 { .. }
            | Error::ServerMetadataParse { .. }
            | Error::ServerMetadataWrite { .. } => LocalErrorDetail {
                code: LocalErrorCode::IoError,
                message: "Local I/O operation failed".to_string(),
                command: None,
            },
            Error::PostgresStartupRollback { primary, .. } => {
                return Self::from_error(primary);
            }
            _ => LocalErrorDetail {
                code: LocalErrorCode::LocalError,
                message: "Local command failed".to_string(),
                command: None,
            },
        };
        Self {
            error: LocalErrorBody::General(detail),
        }
    }
}

impl ManagedClientErrorDetail {
    fn from_error(error: &ManagedClientError) -> Self {
        let (code, message) = match error.kind {
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
        };
        let selection = match error.selection {
            ManagedClientSelection::Default => LocalServerSelection::Default,
            ManagedClientSelection::Named => LocalServerSelection::Named,
        };
        let mut guidance = vec![LocalGuidance {
            action: LocalGuidanceAction::ListProjectServers,
            message: "List managed servers in this exact project",
            command: Some("clickhousectl local server list"),
        }];
        match error.kind {
            ManagedClientErrorKind::ServerNotFound => {
                guidance.push(LocalGuidance {
                    action: LocalGuidanceAction::ReturnToProjectRoot,
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
                    action: LocalGuidanceAction::InstallSelectedVersion,
                    message: "Install the version selected by the managed server metadata",
                    command: Some("clickhousectl local install <version>"),
                });
            }
        }
        guidance.push(LocalGuidance {
            action: LocalGuidanceAction::UseDirectHost,
            message: "Bypass managed project lookup and connect directly",
            command: Some("clickhousectl local client --host <host> --port <port>"),
        });

        Self {
            code,
            message,
            mode: LocalClientMode::Managed,
            project_scope: LocalProjectScope {
                kind: LocalProjectScopeKind::ExactCurrentProject,
                path: error.project_dir.display().to_string(),
                parent_projects_searched: false,
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

fn start_guidance(selection: ManagedClientSelection) -> LocalGuidance {
    match selection {
        ManagedClientSelection::Default => LocalGuidance {
            action: LocalGuidanceAction::StartDefaultServer,
            message: "Start the default managed server in this project",
            command: Some("clickhousectl local server start"),
        },
        ManagedClientSelection::Named => LocalGuidance {
            action: LocalGuidanceAction::StartNamedServer,
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
}

impl fmt::Display for RemoveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Removed version {}", self.version)
    }
}

// ── init ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct InitOutput {
    pub path: String,
}

impl fmt::Display for InitOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Initialized ClickHouse project in {}", self.path)
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
}

impl fmt::Display for ServerStopNoopOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No ClickHouse servers found; nothing to stop")
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
            (Error::ServerNotFound("default".into()), "server_not_found"),
            (
                Error::ServerStopSelectionRequired { available: 2 },
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
                Error::InvalidVersion("unsafe input".into()),
                "invalid_version",
            ),
            (
                Error::VersionNotFound("25.12.9.61".into()),
                "version_unavailable",
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
        ];

        for (error, expected) in cases {
            assert_eq!(error_json(&error)["error"]["code"], expected);
        }
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
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["version"], "25.12.5.44");
    }

    #[test]
    fn init_json() {
        let output = InitOutput {
            path: ".clickhouse/".to_string(),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["path"], ".clickhouse/");
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
    }

    #[test]
    fn server_list_json_empty() {
        let output = ServerListOutput {
            servers: vec![],
            total_servers: 0,
            total_running_servers: 0,
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["servers"].as_array().unwrap().len(), 0);
        assert_eq!(json["total_servers"], 0);
        assert_eq!(json["total_running_servers"], 0);
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
        };
        assert_eq!(output.to_string(), "Removed version 25.12.5.44");
    }

    #[test]
    fn init_display() {
        let output = InitOutput {
            path: ".clickhouse/".to_string(),
        };
        assert_eq!(
            output.to_string(),
            "Initialized ClickHouse project in .clickhouse/"
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
        };
        assert_eq!(output.to_string(), "No servers");
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
