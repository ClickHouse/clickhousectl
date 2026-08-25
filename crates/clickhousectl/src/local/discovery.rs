//! OS-level process discovery for running ClickHouse servers.
//!
//! Finds ClickHouse processes via `pgrep`, resolves their working directories
//! and command-line arguments to recover server metadata (project root, name,
//! ports, version). Used for orphaned server recovery and global server listing.

use std::{collections::HashMap, io, path::Path, process::Command};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedProcessIdentity {
    Managed,
    NotManaged,
}

#[derive(Debug)]
pub struct ProcessInspectionError {
    pub operation: &'static str,
    pub source: io::Error,
}

impl ProcessInspectionError {
    fn new(operation: &'static str, source: io::Error) -> Self {
        Self { operation, source }
    }
}

fn process_command(standard_path: &'static str, fallback: &'static str) -> Command {
    Command::new(if Path::new(standard_path).is_file() {
        standard_path
    } else {
        fallback
    })
}

/// Find all running ClickHouse processes started by the CLI and parse their metadata.
///
/// Only returns processes whose cwd matches the `.clickhouse/servers/<name>/data/` pattern,
/// meaning they were started by this CLI. Other ClickHouse processes are ignored.
pub fn discover_clickhouse_processes() -> Vec<DiscoveredProcess> {
    let pids = find_clickhouse_pids().unwrap_or_default();
    let cwds = get_process_cwds(&pids);
    let mut discovered = Vec::new();

    for pid in pids {
        let Some(cwd) = cwds.get(&pid) else {
            continue;
        };

        if let Some(proc) = inspect_process(pid, cwd) {
            discovered.push(proc);
        }
    }

    discovered
}

/// Determine whether a PID is a ClickHouse process in the managed data
/// directory for the recorded project and server name.
pub fn inspect_managed_clickhouse_process(
    pid: u32,
    project_root: &str,
    server_name: &str,
) -> Result<ManagedProcessIdentity, ProcessInspectionError> {
    if pid == 0 || i32::try_from(pid).is_err() {
        return Ok(ManagedProcessIdentity::NotManaged);
    }
    if !process_exists(pid).map_err(|source| {
        ProcessInspectionError::new("check whether the recorded process exists", source)
    })? {
        return Ok(ManagedProcessIdentity::NotManaged);
    }

    inject_inspection_failure("command", "inspect the recorded process executable")?;
    let is_clickhouse = match (find_clickhouse_pids(), get_process_cmdline(pid)) {
        (Ok(pids), cmdline) => {
            pids.contains(&pid) || cmdline.is_ok_and(|cmdline| cmdline_is_clickhouse(&cmdline))
        }
        (Err(_), Ok(cmdline)) => cmdline_is_clickhouse(&cmdline),
        (Err(pgrep_error), Err(ps_error)) => {
            if !process_exists(pid).map_err(|source| {
                ProcessInspectionError::new("recheck whether the recorded process exists", source)
            })? {
                return Ok(ManagedProcessIdentity::NotManaged);
            }
            return Err(ProcessInspectionError::new(
                "inspect the recorded process executable",
                io::Error::other(format!(
                    "pgrep failed: {pgrep_error}; ps failed: {ps_error}"
                )),
            ));
        }
    };
    if !is_clickhouse {
        return Ok(ManagedProcessIdentity::NotManaged);
    }

    inject_inspection_failure("cwd", "read the recorded process working directory")?;
    let cwd = match get_process_cwd(pid) {
        Ok(Some(cwd)) => cwd,
        Ok(None) => {
            if !process_exists(pid).map_err(|source| {
                ProcessInspectionError::new("recheck whether the recorded process exists", source)
            })? {
                return Ok(ManagedProcessIdentity::NotManaged);
            }
            return Err(ProcessInspectionError::new(
                "read the recorded process working directory",
                io::Error::other("process inspection returned no working directory"),
            ));
        }
        Err(source) => {
            if !process_exists(pid).map_err(|recheck_source| {
                ProcessInspectionError::new(
                    "recheck whether the recorded process exists",
                    recheck_source,
                )
            })? {
                return Ok(ManagedProcessIdentity::NotManaged);
            }
            return Err(ProcessInspectionError::new(
                "read the recorded process working directory",
                source,
            ));
        }
    };
    let expected_cwd = Path::new(project_root)
        .join(".clickhouse")
        .join("servers")
        .join(server_name)
        .join("data");

    inject_inspection_failure("canonicalize", "resolve the managed process paths")?;
    let actual = Path::new(&cwd).canonicalize().map_err(|source| {
        ProcessInspectionError::new("resolve the recorded process working directory", source)
    })?;
    let expected = expected_cwd.canonicalize().map_err(|source| {
        ProcessInspectionError::new("resolve the expected managed data directory", source)
    })?;

    Ok(if actual == expected {
        ManagedProcessIdentity::Managed
    } else {
        ManagedProcessIdentity::NotManaged
    })
}

/// Find PIDs of running `clickhouse` processes.
fn find_clickhouse_pids() -> io::Result<Vec<u32>> {
    let output = process_command("/usr/bin/pgrep", "pgrep")
        .arg("-x")
        .arg("clickhouse")
        .output()?;

    match output.status.code() {
        Some(0) => Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect()),
        Some(1) => Ok(Vec::new()),
        _ => Err(command_error("pgrep", &output)),
    }
}

fn cmdline_is_clickhouse(cmdline: &str) -> bool {
    cmdline.split_whitespace().take(2).any(|arg| {
        Path::new(arg)
            .file_name()
            .is_some_and(|name| name == "clickhouse")
    })
}

fn process_exists(pid: u32) -> io::Result<bool> {
    let pid = i32::try_from(pid).map_err(io::Error::other)?;
    if unsafe { libc::kill(pid, 0) } == 0 {
        return Ok(true);
    }

    let source = io::Error::last_os_error();
    match source.raw_os_error() {
        Some(libc::ESRCH) => Ok(false),
        Some(libc::EPERM) => Ok(true),
        _ => Err(source),
    }
}

fn command_error(command: &str, output: &std::process::Output) -> io::Error {
    let stderr = String::from_utf8_lossy(&output.stderr);
    io::Error::other(format!(
        "{command} exited with {}{}",
        output.status,
        if stderr.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", stderr.trim())
        }
    ))
}

#[cfg(test)]
fn inject_inspection_failure(
    failure: &str,
    operation: &'static str,
) -> Result<(), ProcessInspectionError> {
    if std::env::var("CHCTL_TEST_PROCESS_INSPECTION_FAILURE").as_deref() == Ok(failure) {
        return Err(ProcessInspectionError::new(
            operation,
            io::Error::other(format!("injected {failure} inspection failure")),
        ));
    }
    Ok(())
}

#[cfg(not(test))]
fn inject_inspection_failure(
    _failure: &str,
    _operation: &'static str,
) -> Result<(), ProcessInspectionError> {
    Ok(())
}

/// Inspect a single process to extract server metadata from its cwd and cmdline.
fn inspect_process(pid: u32, cwd: &str) -> Option<DiscoveredProcess> {
    let (project_root, server_name) = parse_server_cwd(cwd)?;
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

/// Get the current working directories of processes (macOS).
///
/// Resolving every PID in one `lsof` invocation avoids paying its relatively
/// high startup cost once per ClickHouse process on the machine.
#[cfg(target_os = "macos")]
fn get_process_cwds(pids: &[u32]) -> HashMap<u32, String> {
    if pids.is_empty() {
        return HashMap::new();
    }

    let pid_list = pids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");

    // -a is required to AND the conditions; without it macOS lsof OR's
    // -d and -p, returning the cwd of every process on the system.
    let output = process_command("/usr/sbin/lsof", "lsof")
        .args(["-a", "-d", "cwd", "-Fn", "-p", &pid_list])
        .output();

    let Ok(output) = output else {
        return HashMap::new();
    };

    // `lsof` can report a non-zero status when a selected process exits during
    // inspection. Keep any complete records it emitted for the remaining PIDs.
    parse_lsof_cwds(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "macos")]
fn get_process_cwd(pid: u32) -> io::Result<Option<String>> {
    let pid_arg = pid.to_string();
    let output = process_command("/usr/sbin/lsof", "lsof")
        .args(["-a", "-d", "cwd", "-Fn", "-p", &pid_arg])
        .output()?;
    let cwd = parse_lsof_cwds(&String::from_utf8_lossy(&output.stdout)).remove(&pid);
    if cwd.is_some() || output.status.success() {
        Ok(cwd)
    } else {
        Err(command_error("lsof", &output))
    }
}

/// Parse `lsof -Fn` output into process working directories.
///
/// `p<pid>` starts a process record and `n<path>` names its cwd. Other fields
/// (such as `fcwd`) are intentionally ignored.
#[cfg(any(target_os = "macos", test))]
fn parse_lsof_cwds(output: &str) -> HashMap<u32, String> {
    let mut current_pid = None;
    let mut cwds = HashMap::new();

    for line in output.lines() {
        if let Some(pid) = line.strip_prefix('p') {
            current_pid = pid.parse().ok();
        } else if let (Some(pid), Some(path)) = (current_pid, line.strip_prefix('n')) {
            cwds.insert(pid, path.to_string());
        }
    }

    cwds
}

/// Get the current working directories of processes (Linux).
#[cfg(target_os = "linux")]
fn get_process_cwds(pids: &[u32]) -> HashMap<u32, String> {
    pids.iter()
        .filter_map(|&pid| {
            std::fs::read_link(format!("/proc/{pid}/cwd"))
                .ok()
                .and_then(|path| path.to_str().map(|path| (pid, path.to_string())))
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn get_process_cwd(pid: u32) -> io::Result<Option<String>> {
    let path = std::fs::read_link(format!("/proc/{pid}/cwd"))?;
    path.into_os_string().into_string().map(Some).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "process working directory is not valid UTF-8",
        )
    })
}

/// Get the command-line string of a process.
fn get_process_cmdline(pid: u32) -> io::Result<String> {
    let output = process_command("/bin/ps", "ps")
        .args(["-o", "args=", "-p", &pid.to_string()])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(command_error("ps", &output))
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
    cmdline.split_whitespace().find_map(|arg| {
        arg.strip_prefix(&prefix)
            .and_then(|v| v.parse::<u16>().ok())
    })
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

    // ── parse_lsof_cwds tests ─────────────────────────────────────────

    #[test]
    fn parse_lsof_cwds_attributes_each_path_to_its_pid() {
        let output = concat!(
            "p101\n",
            "fcwd\n",
            "n/Users/al/project one/.clickhouse/servers/default/data\n",
            "p202\n",
            "fcwd\n",
            "n/Users/al/project-two/.clickhouse/servers/test/data\n",
        );

        let cwds = parse_lsof_cwds(output);

        assert_eq!(cwds.len(), 2);
        assert_eq!(
            cwds.get(&101).map(String::as_str),
            Some("/Users/al/project one/.clickhouse/servers/default/data")
        );
        assert_eq!(
            cwds.get(&202).map(String::as_str),
            Some("/Users/al/project-two/.clickhouse/servers/test/data")
        );
    }

    #[test]
    fn parse_lsof_cwds_ignores_incomplete_and_malformed_records() {
        let output = concat!(
            "n/path-before-any-pid\n",
            "pnot-a-pid\n",
            "n/path-for-invalid-pid\n",
            "p303\n",
            "fcwd\n",
            "p404\n",
            "n/path-for-404\n",
        );

        let cwds = parse_lsof_cwds(output);

        assert_eq!(cwds.len(), 1);
        assert_eq!(cwds.get(&404).map(String::as_str), Some("/path-for-404"));
    }

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
        let cmdline = "/home/user/.clickhouse/versions/25.12.5.44/clickhouse server --http_port=8123 --tcp_port=9000";
        assert_eq!(parse_port_flag(cmdline, "--http_port"), Some(8123));
    }

    #[test]
    fn parse_tcp_port() {
        let cmdline = "/home/user/.clickhouse/versions/25.12.5.44/clickhouse server --http_port=8124 --tcp_port=9001";
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
