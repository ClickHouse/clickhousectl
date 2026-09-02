//! OS-level process discovery for running ClickHouse servers.
//!
//! Finds ClickHouse processes via `pgrep`, resolves their working directories
//! and command-line arguments to recover server metadata (project root, name,
//! ports, version). Used for orphaned server recovery and global server listing.

use std::{collections::HashMap, process::Command};

/// The name the ClickHouse watchdog rewrites its `argv[0]` to, which is what
/// `ps` reports as the command line of a supervising parent.
const WATCHDOG_PROCESS_NAME: &str = "clickhouse-watchdog";

/// Shortest `argv[0]` still recognisable as the watchdog.
///
/// The watchdog rewrites `argv[0]` in place, so the name it reports is
/// truncated to the length of the path the CLI spawned it with. That path is
/// normally far longer than the new name, but a prefix still has to be
/// accepted, and the bar it has to clear is only the server's own basename:
/// `clickhouse` is itself a prefix of `clickhouse-watchdog`, so one character
/// more is enough to tell the two apart. The only 11-character prefix of the
/// watchdog's name is `clickhouse-`, which no ClickHouse binary is called.
const WATCHDOG_NAME_MIN_LEN: usize = "clickhouse".len() + 1;

/// A ClickHouse process discovered via OS-level process inspection.
#[derive(Debug, Clone)]
pub struct DiscoveredProcess {
    /// The PID to signal to stop this server for good: the ClickHouse watchdog
    /// when the server has one, otherwise the server process itself.
    ///
    /// The watchdog owns the lifetime of the pair: it forwards SIGTERM to the
    /// server it supervises and exits with it, so signalling the watchdog stops
    /// both together. `server start` records that same supervising PID in the
    /// server's metadata, so both views of a server agree (issue #664).
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
    let cwds = get_process_cwds(&pids);

    // The cwd is what decides whether a process was started by this CLI, so it
    // is resolved first: parents are then only looked up for the PIDs that
    // survive the filter.
    let server_pids: Vec<u32> = pids
        .into_iter()
        .filter(|pid| cwds.contains_key(pid))
        .collect();

    let rows = scan_process_rows(&server_pids);
    let supervisors = resolve_watchdog_pids(&server_pids, &rows);
    // The rows already carry each scanned process's command line, so the whole
    // scan stays two flat subprocess calls with no per-PID lookup on top.
    let commands: HashMap<u32, &str> = rows
        .iter()
        .map(|row| (row.pid, row.command.as_str()))
        .collect();

    server_pids
        .iter()
        .filter_map(|pid| {
            // Server metadata always comes from the server process, whose
            // command line still carries the binary path and the port flags.
            // Only the reported PID is the supervisor's.
            let reported_pid = supervisors.get(pid).copied().unwrap_or(*pid);
            let command = commands.get(pid).copied().unwrap_or_default();
            inspect_process(reported_pid, cwds.get(pid)?, command)
        })
        .collect()
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
///
/// `reported_pid` is the PID the result carries, which is the supervising
/// watchdog when there is one; `cwd` and `cmdline` belong to the server process
/// being inspected. A process whose `ps` row was missing arrives with an empty
/// `cmdline`, which leaves every parsed field absent.
fn inspect_process(reported_pid: u32, cwd: &str, cmdline: &str) -> Option<DiscoveredProcess> {
    let (project_root, server_name) = parse_server_cwd(cwd)?;
    let http_port = parse_port_flag(cmdline, "--http_port");
    let tcp_port = parse_port_flag(cmdline, "--tcp_port");
    let version = parse_version_from_cmdline(cmdline);

    Some(DiscoveredProcess {
        pid: reported_pid,
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
    let output = Command::new("lsof")
        .args(["-a", "-d", "cwd", "-Fn", "-p", &pid_list])
        .output();

    let Ok(output) = output else {
        return HashMap::new();
    };

    // `lsof` can report a non-zero status when a selected process exits during
    // inspection. Keep any complete records it emitted for the remaining PIDs.
    parse_lsof_cwds(&String::from_utf8_lossy(&output.stdout))
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

/// A `pid ppid args` row of `ps` output.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessRow {
    pid: u32,
    ppid: u32,
    /// The process's command line, whose first token names the executable.
    command: String,
}

/// Read a `pid ppid args` row for each scanned server PID and for each of
/// their parents.
///
/// ClickHouse starts a watchdog that owns the lifetime of the server it
/// supervises, so the server's own PID is not the one that stops it.
/// `pgrep -x clickhouse` never matches the watchdog (it renames itself), so the
/// parents have to be read separately: one batched `ps` to learn the parents
/// and one to identify them, two calls whatever the number of servers.
/// `get_process_cwds` batches `lsof` for the same reason.
fn scan_process_rows(pids: &[u32]) -> Vec<ProcessRow> {
    let mut rows = ps_process_rows(pids);
    let mut parents = Vec::new();
    for row in &rows {
        // PID 0 is the kernel and has no row to read. PID 1 does, and it is
        // worth reading: in a container the watchdog can be init itself, while
        // an ordinary init simply fails `is_watchdog_command`.
        if row.ppid > 0 && !pids.contains(&row.ppid) && !parents.contains(&row.ppid) {
            parents.push(row.ppid);
        }
    }
    rows.extend(ps_process_rows(&parents));

    rows
}

/// Read `pid ppid args` for `pids` in one `ps` invocation.
fn ps_process_rows(pids: &[u32]) -> Vec<ProcessRow> {
    if pids.is_empty() {
        return Vec::new();
    }

    let pid_list = pids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");

    // `args` rather than `comm`: on Linux `comm` is the thread name the
    // watchdog sets with `prctl` (it has changed between ClickHouse versions),
    // while the rewritten `argv[0]` that `args` reports is the same on Linux
    // and macOS.
    let output = Command::new("ps")
        .args(["-o", "pid=,ppid=,args=", "-p", &pid_list])
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };

    // A non-zero status means at least one selected PID exited during
    // inspection; keep the rows `ps` did emit.
    parse_ps_process_rows(&String::from_utf8_lossy(&output.stdout))
}

/// Parse `ps -o pid=,ppid=,args=` output.
///
/// `ps` right-aligns the numeric columns and `args` is last, so a row is two
/// integers followed by the rest of the line. Lines that do not start with two
/// integers (a header, a warning, or a stand-in tool's output) are ignored.
fn parse_ps_process_rows(output: &str) -> Vec<ProcessRow> {
    output
        .lines()
        .filter_map(|line| {
            let (pid, rest) = split_first_token(line)?;
            let (ppid, command) = split_first_token(rest)?;
            Some(ProcessRow {
                pid: pid.parse().ok()?,
                ppid: ppid.parse().ok()?,
                command: command.trim().to_string(),
            })
        })
        .collect()
}

/// Split leading whitespace and the first token off `text`.
fn split_first_token(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start();
    let end = text.find(char::is_whitespace).unwrap_or(text.len());
    if end == 0 {
        return None;
    }
    Some(text.split_at(end))
}

/// Whether a command line belongs to a ClickHouse watchdog process.
fn is_watchdog_command(command: &str) -> bool {
    let Some(argv0) = command.split_whitespace().next() else {
        return false;
    };
    // macOS reports the full binary path; the watchdog's rewritten name has no
    // path, so take the basename either way.
    let name = argv0.rsplit('/').next().unwrap_or(argv0);

    name.len() >= WATCHDOG_NAME_MIN_LEN && WATCHDOG_PROCESS_NAME.starts_with(name)
}

/// Resolve, for each of `pids`, the PID to report: its parent when that parent
/// is a ClickHouse watchdog, otherwise the PID itself.
///
/// Pure over already-collected `ps` rows, which may describe the scanned
/// processes, their parents, or both.
fn resolve_watchdog_pids(pids: &[u32], rows: &[ProcessRow]) -> HashMap<u32, u32> {
    let by_pid: HashMap<u32, &ProcessRow> = rows.iter().map(|row| (row.pid, row)).collect();

    pids.iter()
        .map(|&pid| {
            let reported = by_pid
                .get(&pid)
                .and_then(|row| by_pid.get(&row.ppid))
                .filter(|parent| is_watchdog_command(&parent.command))
                .map_or(pid, |parent| parent.pid);
            (pid, reported)
        })
        .collect()
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

    // ── watchdog PID resolution tests (issue #664) ─────────────────────

    /// `ps -o pid=,ppid=,args=` as macOS prints it: right-aligned numeric
    /// columns and the server's `argv[0]` a full path. The watchdog row is the
    /// rewritten name alone, padded with the trailing spaces `ps` shows for the
    /// rest of the zeroed `argv` region, because the rewrite ends the macOS
    /// `argv` walk and no argument is reported after it.
    const MACOS_PS_OUTPUT: &str = concat!(
        "95194 95193 /Users/al/.clickhouse/versions/26.9.1.531/clickhouse server -- --path=./ --http_port=8123 --tcp_port=9000\n",
        "95193     1 clickhouse-watchdog     \n",
    );

    /// The same call on Linux: unpadded columns and an `/usr/bin` style path.
    const LINUX_PS_OUTPUT: &str = concat!(
        "4211 4210 /home/al/.clickhouse/versions/26.9.1.531/clickhouse server -- --path=./ --http_port=8123 --tcp_port=9000\n",
        "4210 1 clickhouse-watchdog     \n",
    );

    #[test]
    fn ps_rows_parse_from_macos_output() {
        let rows = parse_ps_process_rows(MACOS_PS_OUTPUT);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].pid, 95194);
        assert_eq!(rows[0].ppid, 95193);
        assert!(
            rows[0]
                .command
                .starts_with("/Users/al/.clickhouse/versions/")
        );
        assert_eq!(rows[1].pid, 95193);
        assert_eq!(rows[1].ppid, 1);
        assert_eq!(
            rows[1].command, "clickhouse-watchdog",
            "the padding `ps` prints after the rewritten name is not an argument"
        );
    }

    #[test]
    fn ps_rows_parse_from_linux_output() {
        let rows = parse_ps_process_rows(LINUX_PS_OUTPUT);

        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].pid, rows[0].ppid), (4211, 4210));
        assert_eq!((rows[1].pid, rows[1].ppid), (4210, 1));
    }

    #[test]
    fn ps_rows_ignore_lines_that_do_not_start_with_two_integers() {
        let output = concat!(
            "  PID  PPID COMMAND\n",
            "ps: warning: something happened\n",
            "\n",
            "  707\n",
            "  808   909 /opt/clickhouse server\n",
        );

        let rows = parse_ps_process_rows(output);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pid, 808);
        assert_eq!(rows[0].command, "/opt/clickhouse server");
    }

    #[test]
    fn a_server_supervised_by_a_watchdog_reports_the_watchdog_pid() {
        let macos = resolve_watchdog_pids(&[95194], &parse_ps_process_rows(MACOS_PS_OUTPUT));
        assert_eq!(macos.get(&95194).copied(), Some(95193));

        let linux = resolve_watchdog_pids(&[4211], &parse_ps_process_rows(LINUX_PS_OUTPUT));
        assert_eq!(linux.get(&4211).copied(), Some(4210));
    }

    #[test]
    fn a_server_whose_parent_is_not_a_watchdog_reports_itself() {
        // CLICKHOUSE_WATCHDOG_ENABLE=0, so the CLI's own child is the server
        // and its parent is a shell.
        let output = concat!(
            "4211 4210 /home/al/.clickhouse/versions/26.9.1.531/clickhouse server\n",
            "4210 1 /bin/zsh\n",
        );

        let resolved = resolve_watchdog_pids(&[4211], &parse_ps_process_rows(output));

        assert_eq!(resolved.get(&4211).copied(), Some(4211));
    }

    #[test]
    fn a_server_whose_parent_line_is_missing_reports_itself() {
        // The parent exited between the two `ps` calls, so only the server's
        // own row is available.
        let output = "4211 4210 /home/al/.clickhouse/versions/26.9.1.531/clickhouse server\n";

        let resolved = resolve_watchdog_pids(&[4211], &parse_ps_process_rows(output));

        assert_eq!(resolved.get(&4211).copied(), Some(4211));
    }

    #[test]
    fn a_server_with_no_row_at_all_reports_itself() {
        let resolved = resolve_watchdog_pids(&[4211], &[]);

        assert_eq!(resolved.get(&4211).copied(), Some(4211));
    }

    #[test]
    fn every_scanned_pid_is_resolved_independently() {
        let output = concat!(
            "10 9 /home/al/.clickhouse/versions/26.9.1.531/clickhouse server\n",
            "9 1 clickhouse-watchdog server\n",
            "20 1 /home/al/.clickhouse/versions/26.9.1.531/clickhouse server\n",
        );

        let resolved = resolve_watchdog_pids(&[10, 20], &parse_ps_process_rows(output));

        assert_eq!(resolved.get(&10).copied(), Some(9));
        assert_eq!(resolved.get(&20).copied(), Some(20));
    }

    #[test]
    fn watchdog_commands_are_recognised_by_their_rewritten_argv0() {
        // The real row: the rewritten name and the padding of the zeroed
        // `argv` region behind it, with no arguments.
        assert!(is_watchdog_command("clickhouse-watchdog     "));
        assert!(is_watchdog_command(
            "clickhouse-watchdog server -- --path=./"
        ));
        assert!(is_watchdog_command("/usr/bin/clickhouse-watchdog server"));
        // The rewrite is truncated to the length of the spawn path, so a
        // prefix counts as long as it is longer than "clickhouse" itself.
        assert!(is_watchdog_command("clickhouse-watc server"));
        assert!(is_watchdog_command("clickhouse- server"));
    }

    #[test]
    fn a_plain_clickhouse_command_is_not_a_watchdog() {
        // "clickhouse" is a prefix of "clickhouse-watchdog", so a name no
        // longer than the server's own basename must not count.
        assert!(!is_watchdog_command("clickhouse server"));
        assert!(!is_watchdog_command("clickhouse local --query 'SELECT 1'"));
        assert!(!is_watchdog_command(
            "/Users/al/.clickhouse/versions/26.9.1.531/clickhouse server -- --path=./"
        ));
        // Long enough, but not a prefix of the watchdog's name.
        assert!(!is_watchdog_command("clickhouse-client --host localhost"));
        assert!(!is_watchdog_command("clickhouse-watchdog-wrapper server"));
        assert!(!is_watchdog_command("/bin/zsh"));
        assert!(!is_watchdog_command("/sbin/launchd"));
        assert!(!is_watchdog_command(""));
    }

    // ── inspect_process over an already-scanned `ps` row ───────────────

    #[test]
    fn a_scanned_row_supplies_every_field_of_the_discovered_process() {
        let proc = inspect_process(
            95193,
            "/Users/al/project/.clickhouse/servers/dev/data",
            "/Users/al/.clickhouse/versions/26.9.1.531/clickhouse server -- --path=./ --http_port=8123 --tcp_port=9000",
        )
        .expect("a CLI-managed cwd is a discovered server");

        assert_eq!(proc.pid, 95193, "the reported PID is the supervisor's");
        assert_eq!(proc.project_root, "/Users/al/project");
        assert_eq!(proc.server_name, "dev");
        assert_eq!(proc.http_port, Some(8123));
        assert_eq!(proc.tcp_port, Some(9000));
        assert_eq!(proc.version.as_deref(), Some("26.9.1.531"));
    }

    #[test]
    fn a_process_with_no_ps_row_keeps_its_cwd_derived_fields() {
        // `ps` dropped the row (the process exited mid-scan), which used to be
        // a failed per-PID lookup and is now an empty command line.
        let proc = inspect_process(4211, "/home/al/app/.clickhouse/servers/default/data", "")
            .expect("the cwd alone still identifies the server");

        assert_eq!(proc.project_root, "/home/al/app");
        assert_eq!(proc.server_name, "default");
        assert_eq!(proc.http_port, None);
        assert_eq!(proc.tcp_port, None);
        assert_eq!(proc.version, None);
    }

    #[test]
    fn a_cwd_outside_a_project_is_not_a_discovered_server() {
        assert!(
            inspect_process(4211, "/var/lib/clickhouse", "/usr/bin/clickhouse server").is_none()
        );
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
