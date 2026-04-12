//! Structured output types for local commands.
//!
//! Each type supports both JSON serialization (via serde) and human-readable
//! display (via `fmt::Display`). The `--json` flag switches between the two.

use serde::Serialize;
use std::fmt;
use tabled::{Table, Tabled, settings::Style};

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
        let table = Table::new(rows).with(Style::rounded()).to_string();
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
        let table = Table::new(rows).with(Style::rounded()).to_string();
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
            let table = Table::new(rows).with(Style::rounded()).to_string();
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
            let table = Table::new(rows).with(Style::rounded()).to_string();
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

// ── server stop ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServerStopOutput {
    pub name: String,
}

impl fmt::Display for ServerStopOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Server '{}' stopped", self.name)
    }
}

// ── server stop-all ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ServerStopEntry {
    pub name: String,
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
            if s.stopped {
                writeln!(f, "Stopping '{}'... stopped", s.name)?;
            } else {
                writeln!(
                    f,
                    "Stopping '{}'... error: {}",
                    s.name,
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
}

impl fmt::Display for ServerRemoveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Server '{}' removed", self.name)
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
        let json: serde_json::Value = serde_json::from_str(
            &serde_json::to_string_pretty(&output).unwrap(),
        )
        .unwrap();

        assert_eq!(json["versions"][0]["version"], "25.12.5.44");
        assert_eq!(json["versions"][0]["default"], true);
        assert_eq!(json["versions"][1]["version"], "25.11.3.22");
        assert_eq!(json["versions"][1]["default"], false);
        assert_eq!(json["versions"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn list_installed_json_empty() {
        let output = ListInstalledOutput {
            versions: vec![],
        };
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
                },
                ServerListEntry {
                    name: "test".to_string(),
                    running: false,
                    pid: None,
                    version: None,
                    http_port: None,
                    tcp_port: None,
                    project: None,
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
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["name"], "default");
    }

    #[test]
    fn server_stop_all_json() {
        let output = ServerStopAllOutput {
            servers: vec![
                ServerStopEntry {
                    name: "default".to_string(),
                    stopped: true,
                    error: None,
                },
                ServerStopEntry {
                    name: "test".to_string(),
                    stopped: false,
                    error: Some("process not found".to_string()),
                },
            ],
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["servers"][0]["name"], "default");
        assert_eq!(json["servers"][0]["stopped"], true);
        assert!(json["servers"][0].get("error").is_none());
        assert_eq!(json["servers"][1]["name"], "test");
        assert_eq!(json["servers"][1]["stopped"], false);
        assert_eq!(json["servers"][1]["error"], "process not found");
    }

    #[test]
    fn server_stop_all_json_empty() {
        let output = ServerStopAllOutput {
            servers: vec![],
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["servers"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn server_remove_json() {
        let output = ServerRemoveOutput {
            name: "test".to_string(),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string_pretty(&output).unwrap()).unwrap();

        assert_eq!(json["name"], "test");
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
        let output = ListInstalledOutput {
            versions: vec![],
        };
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
        let output = ListAvailableOutput {
            versions: vec![],
        };
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
                },
                ServerListEntry {
                    name: "test".to_string(),
                    running: false,
                    pid: None,
                    version: None,
                    http_port: None,
                    tcp_port: None,
                    project: None,
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
        };
        assert_eq!(output.to_string(), "Server 'default' stopped");
    }

    #[test]
    fn server_stop_all_display() {
        let output = ServerStopAllOutput {
            servers: vec![
                ServerStopEntry {
                    name: "default".to_string(),
                    stopped: true,
                    error: None,
                },
                ServerStopEntry {
                    name: "test".to_string(),
                    stopped: false,
                    error: Some("process not found".to_string()),
                },
            ],
        };
        let text = output.to_string();
        assert!(text.contains("Stopping 'default'... stopped"));
        assert!(text.contains("Stopping 'test'... error: process not found"));
        assert!(text.contains("Done"));
    }

    #[test]
    fn server_stop_all_display_empty() {
        let output = ServerStopAllOutput {
            servers: vec![],
        };
        assert_eq!(output.to_string(), "No running servers");
    }

    #[test]
    fn server_remove_display() {
        let output = ServerRemoveOutput {
            name: "test".to_string(),
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
