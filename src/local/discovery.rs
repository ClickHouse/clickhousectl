//! OS-level process discovery for running ClickHouse servers.
//!
//! Finds ClickHouse processes via `pgrep`, resolves their working directories
//! and command-line arguments to recover server metadata (project root, name,
//! ports, version). Used for orphaned server recovery and global server listing.

use std::process::Command;

/// A ClickHouse process discovered via OS-level process inspection.
#[derive(Debug, Clone)]
pub struct DiscoveredProcess {
    pub pid: u32,
    pub project_root: String,
    pub server_name: String,
    pub http_port: Option<u16>,
    pub tcp_port: Option<u16>,
    pub version: Option<String>,
}

/// Find all running ClickHouse processes started by the CLI and parse their metadata.
///
/// Only returns processes whose cwd matches the `.clickhouse/servers/<name>/data/` pattern,
/// meaning they were started by this CLI. Other ClickHouse processes are ignored.
pub fn discover_clickhouse_processes() -> Vec<DiscoveredProcess> {
    let pids = find_clickhouse_pids();
    let mut discovered = Vec::new();

    for pid in pids {
        if let Some(proc) = inspect_process(pid) {
            discovered.push(proc);
        }
    }

    discovered
}

/// Find PIDs of running `clickhouse` processes.
fn find_clickhouse_pids() -> Vec<u32> {
    let output = Command::new("pgrep").arg("-x").arg("clickhouse").output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect(),
        _ => Vec::new(),
    }
}

/// Inspect a single process to extract server metadata from its cwd and cmdline.
fn inspect_process(pid: u32) -> Option<DiscoveredProcess> {
    let cwd = get_process_cwd(pid)?;
    let (project_root, server_name) = parse_server_cwd(&cwd)?;
    let cmdline = get_process_cmdline(pid).unwrap_or_default();
    let http_port = parse_port_flag(&cmdline, "--http_port");
    let tcp_port = parse_port_flag(&cmdline, "--tcp_port");
    let version = parse_version_from_cmdline(&cmdline);

    Some(DiscoveredProcess {
        pid,
        project_root,
        server_name,
        http_port,
        tcp_port,
        version,
    })
}

/// Get the current working directory of a process (macOS).
#[cfg(target_os = "macos")]
fn get_process_cwd(pid: u32) -> Option<String> {
    let output = Command::new("lsof")
        .args(["-d", "cwd", "-Fn", "-p", &pid.to_string()])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix('n') {
            return Some(path.to_string());
        }
    }
    None
}

/// Get the current working directory of a process (Linux).
#[cfg(target_os = "linux")]
fn get_process_cwd(pid: u32) -> Option<String> {
    std::fs::read_link(format!("/proc/{}/cwd", pid))
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
}

/// Get the command-line string of a process.
fn get_process_cmdline(pid: u32) -> Option<String> {
    let output = Command::new("ps")
        .args(["-o", "args=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Parse a cwd path matching `<project_root>/.clickhouse/servers/<name>/data`
/// to extract the project root and server name.
///
/// Returns `None` if the path doesn't match the expected pattern.
pub fn parse_server_cwd(cwd: &str) -> Option<(String, String)> {
    let marker = "/.clickhouse/servers/";
    let idx = cwd.find(marker)?;
    let project_root = &cwd[..idx];
    let rest = &cwd[idx + marker.len()..];

    // rest should be "<name>/data" or "<name>/data/"
    let name = rest
        .strip_suffix("/data/")
        .or_else(|| rest.strip_suffix("/data"))
        .unwrap_or(rest);

    if name.is_empty() || name.contains('/') {
        return None;
    }

    Some((project_root.to_string(), name.to_string()))
}

/// Parse a port value from command-line flags like `--http_port=8123`.
pub fn parse_port_flag(cmdline: &str, flag: &str) -> Option<u16> {
    let prefix = format!("{}=", flag);
    cmdline
        .split_whitespace()
        .find_map(|arg| arg.strip_prefix(&prefix).and_then(|v| v.parse::<u16>().ok()))
}

/// Extract the ClickHouse version from the binary path in the command line.
///
/// Binary paths look like: `~/.clickhouse/versions/<version>/clickhouse`
pub fn parse_version_from_cmdline(cmdline: &str) -> Option<String> {
    let marker = "/.clickhouse/versions/";
    let idx = cmdline.find(marker)?;
    let rest = &cmdline[idx + marker.len()..];
    let version = rest.split('/').next()?;
    if version.is_empty() {
        return None;
    }
    Some(version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_server_cwd tests ─────────────────────────────────────────

    #[test]
    fn parse_cwd_standard_path() {
        let cwd = "/Users/al/project-a/.clickhouse/servers/default/data";
        let (root, name) = parse_server_cwd(cwd).unwrap();
        assert_eq!(root, "/Users/al/project-a");
        assert_eq!(name, "default");
    }

    #[test]
    fn parse_cwd_trailing_slash() {
        let cwd = "/Users/al/project-a/.clickhouse/servers/default/data/";
        let (root, name) = parse_server_cwd(cwd).unwrap();
        assert_eq!(root, "/Users/al/project-a");
        assert_eq!(name, "default");
    }

    #[test]
    fn parse_cwd_custom_name() {
        let cwd = "/home/user/myapp/.clickhouse/servers/bold-crane/data";
        let (root, name) = parse_server_cwd(cwd).unwrap();
        assert_eq!(root, "/home/user/myapp");
        assert_eq!(name, "bold-crane");
    }

    #[test]
    fn parse_cwd_deep_project_root() {
        let cwd = "/Users/al/code/projects/web/.clickhouse/servers/test/data";
        let (root, name) = parse_server_cwd(cwd).unwrap();
        assert_eq!(root, "/Users/al/code/projects/web");
        assert_eq!(name, "test");
    }

    #[test]
    fn parse_cwd_not_cli_managed() {
        // Process not started by the CLI — no matching pattern
        assert!(parse_server_cwd("/var/lib/clickhouse").is_none());
    }

    #[test]
    fn parse_cwd_missing_data_suffix() {
        // cwd is the server dir but not the data subdir — ambiguous, still works
        // because we strip /data suffix only if present
        let cwd = "/Users/al/project/.clickhouse/servers/default";
        // This doesn't end in /data, so "default" is treated as the rest
        // Since "default" doesn't contain '/' and isn't empty, it's accepted
        let (root, name) = parse_server_cwd(cwd).unwrap();
        assert_eq!(root, "/Users/al/project");
        assert_eq!(name, "default");
    }

    #[test]
    fn parse_cwd_empty_name() {
        let cwd = "/Users/al/project/.clickhouse/servers/";
        assert!(parse_server_cwd(cwd).is_none());
    }

    // ── parse_port_flag tests ──────────────────────────────────────────

    #[test]
    fn parse_http_port() {
        let cmdline =
            "/home/user/.clickhouse/versions/25.12.5.44/clickhouse server --http_port=8123 --tcp_port=9000";
        assert_eq!(parse_port_flag(cmdline, "--http_port"), Some(8123));
    }

    #[test]
    fn parse_tcp_port() {
        let cmdline =
            "/home/user/.clickhouse/versions/25.12.5.44/clickhouse server --http_port=8124 --tcp_port=9001";
        assert_eq!(parse_port_flag(cmdline, "--tcp_port"), Some(9001));
    }

    #[test]
    fn parse_port_custom() {
        let cmdline = "clickhouse server --http_port=18123 --tcp_port=19000";
        assert_eq!(parse_port_flag(cmdline, "--http_port"), Some(18123));
        assert_eq!(parse_port_flag(cmdline, "--tcp_port"), Some(19000));
    }

    #[test]
    fn parse_port_missing() {
        let cmdline = "clickhouse server";
        assert_eq!(parse_port_flag(cmdline, "--http_port"), None);
    }

    #[test]
    fn parse_port_invalid_value() {
        let cmdline = "clickhouse server --http_port=abc";
        assert_eq!(parse_port_flag(cmdline, "--http_port"), None);
    }

    // ── parse_version_from_cmdline tests ───────────────────────────────

    #[test]
    fn parse_version_standard() {
        let cmdline =
            "/Users/al/.clickhouse/versions/25.12.5.44/clickhouse server --http_port=8123";
        assert_eq!(
            parse_version_from_cmdline(cmdline),
            Some("25.12.5.44".to_string())
        );
    }

    #[test]
    fn parse_version_linux_path() {
        let cmdline = "/home/user/.clickhouse/versions/24.8.1.1/clickhouse server";
        assert_eq!(
            parse_version_from_cmdline(cmdline),
            Some("24.8.1.1".to_string())
        );
    }

    #[test]
    fn parse_version_not_managed() {
        let cmdline = "/usr/bin/clickhouse server";
        assert_eq!(parse_version_from_cmdline(cmdline), None);
    }

    #[test]
    fn parse_version_empty_version() {
        let cmdline = "/home/user/.clickhouse/versions//clickhouse server";
        assert_eq!(parse_version_from_cmdline(cmdline), None);
    }
}
