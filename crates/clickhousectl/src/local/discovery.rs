//! OS-level process discovery for running ClickHouse servers.
//!
//! Finds ClickHouse processes in the native process table (see
//! [`super::procinfo`]), then reads their working directories, command-line
//! arguments and managed config files to recover server metadata (project
//! root, name, ports, version). Used for orphaned server recovery and global
//! server listing.

use std::{collections::HashMap, io, path::Path};

use super::procinfo::{self, ProcessEntry};

/// The kernel process name of a ClickHouse server. A macOS watchdog has it
/// too, so a candidate is only a server once its `argv[0]` is not the
/// watchdog's.
const SERVER_PROCESS_NAME: &str = "clickhouse";

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
    /// both together. `clickhouse start` records that same supervising PID in the
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
///
/// An error means the process table could not be read at all, which is not
/// the same as "no server is running" (issue #1301). A single process that
/// exits or cannot be inspected mid-scan is skipped.
pub fn discover_clickhouse_processes() -> io::Result<Vec<DiscoveredProcess>> {
    let table = procinfo::process_table()?;
    Ok(discover_in(
        &table,
        |pid| procinfo::process_cwd(pid).ok(),
        |pid| procinfo::process_args(pid).ok(),
    ))
}

/// [`discover_clickhouse_processes`] over an already-read process table, with
/// the per-process reads passed in.
fn discover_in(
    table: &[ProcessEntry],
    cwd_of: impl Fn(u32) -> Option<std::path::PathBuf>,
    args_of: impl Fn(u32) -> Option<Vec<String>>,
) -> Vec<DiscoveredProcess> {
    let by_pid: HashMap<u32, &ProcessEntry> =
        table.iter().map(|entry| (entry.pid, entry)).collect();

    // The cwd is what decides whether a process was started by this CLI, so it
    // is resolved first: parents are then only looked up for the PIDs that
    // survive the filter.
    let servers: Vec<(u32, String)> = table
        .iter()
        .filter(|entry| entry.name == SERVER_PROCESS_NAME)
        .filter_map(|entry| {
            let cwd = cwd_of(entry.pid)?;
            Some((entry.pid, cwd.to_str()?.to_string()))
        })
        .collect();
    let server_pids: Vec<u32> = servers.iter().map(|(pid, _)| *pid).collect();

    let rows = process_rows(&server_pids, &by_pid, &args_of);
    let supervisors = resolve_watchdog_pids(&server_pids, &rows);
    let commands: HashMap<u32, &str> = rows
        .iter()
        .map(|row| (row.pid, row.command.as_str()))
        .collect();

    servers
        .iter()
        // On macOS the watchdog keeps the kernel name `clickhouse` and the
        // server's cwd, so only its rewritten `argv[0]` tells it apart. It is
        // the supervisor its server reports, never a server of its own.
        .filter(|(pid, _)| {
            !commands
                .get(pid)
                .is_some_and(|command| is_watchdog_command(command))
        })
        .filter_map(|(pid, cwd)| {
            // Server metadata always comes from the server process, whose
            // command line carries the binary path and whose cwd holds the
            // managed config.
            // Only the reported PID is the supervisor's.
            let reported_pid = supervisors.get(pid).copied().unwrap_or(*pid);
            let command = commands.get(pid).copied().unwrap_or_default();
            inspect_process(reported_pid, cwd, command)
        })
        .collect()
}

/// Inspect a single process to extract server metadata from its cwd and cmdline.
///
/// `reported_pid` is the PID the result carries, which is the supervising
/// watchdog when there is one; `cwd` and `cmdline` belong to the server process
/// being inspected. A process whose arguments could not be read arrives with an
/// empty `cmdline`, which leaves every parsed field absent.
fn inspect_process(reported_pid: u32, cwd: &str, cmdline: &str) -> Option<DiscoveredProcess> {
    let (project_root, server_name) = parse_server_cwd(cwd)?;
    // Servers started by this CLI take their ports from the managed config
    // file in their data directory; older releases passed them as flags.
    let (file_http_port, file_tcp_port) = super::config::read_managed_ports(Path::new(cwd));
    let http_port = parse_port_flag(cmdline, "--http_port").or(file_http_port);
    let tcp_port = parse_port_flag(cmdline, "--tcp_port").or(file_tcp_port);
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

/// A scanned process's PID, parent and command line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessRow {
    pid: u32,
    ppid: u32,
    /// The process's arguments joined by spaces, whose first token names the
    /// executable.
    command: String,
}

/// A row for each server PID and for each of their parents.
///
/// ClickHouse starts a watchdog that owns the lifetime of the server it
/// supervises, so the server's own PID is not the one that stops it. On Linux
/// the watchdog renames its kernel process, so it is not among the scanned
/// PIDs and has to be read separately as a parent; either way it is
/// identified by the rewritten `argv[0]` its row carries.
fn process_rows(
    pids: &[u32],
    table: &HashMap<u32, &ProcessEntry>,
    args_of: &impl Fn(u32) -> Option<Vec<String>>,
) -> Vec<ProcessRow> {
    let row = |pid: u32| {
        let entry = table.get(&pid)?;
        // A process whose arguments cannot be read keeps its row with an empty
        // command line: its cwd still identifies the server.
        let command = args_of(pid).unwrap_or_default().join(" ");
        Some(ProcessRow {
            pid,
            ppid: entry.ppid,
            command,
        })
    };

    let mut rows: Vec<ProcessRow> = pids.iter().filter_map(|&pid| row(pid)).collect();
    let mut parents = Vec::new();
    for row in &rows {
        // PID 0 is the kernel and has no row to read. PID 1 does, and it is
        // worth reading: in a container the watchdog can be init itself, while
        // an ordinary init simply fails `is_watchdog_command`.
        if row.ppid > 0 && !pids.contains(&row.ppid) && !parents.contains(&row.ppid) {
            parents.push(row.ppid);
        }
    }
    rows.extend(parents.into_iter().filter_map(row));

    rows
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
/// Pure over already-collected rows, which may describe the scanned
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

    // ── watchdog PID resolution over a process table (issue #664) ──────

    const SERVER_ARGS: &str = "/Users/al/.clickhouse/versions/26.9.1.531/clickhouse server -- --path=./ --http_port=8123 --tcp_port=9000";

    fn entry(pid: u32, ppid: u32, name: &str) -> ProcessEntry {
        ProcessEntry {
            pid,
            ppid,
            name: name.to_string(),
        }
    }

    /// A process table with per-process cwds and arguments, standing in for
    /// the native reads.
    struct FakeProcesses {
        table: Vec<ProcessEntry>,
        cwds: HashMap<u32, &'static str>,
        args: HashMap<u32, Vec<String>>,
    }

    impl FakeProcesses {
        fn new() -> Self {
            Self {
                table: Vec::new(),
                cwds: HashMap::new(),
                args: HashMap::new(),
            }
        }

        fn process(mut self, pid: u32, ppid: u32, name: &str, args: &str) -> Self {
            self.table.push(entry(pid, ppid, name));
            self.args
                .insert(pid, args.split(' ').map(str::to_string).collect());
            self
        }

        fn server(self, pid: u32, ppid: u32, cwd: &'static str) -> Self {
            let mut this = self.process(pid, ppid, "clickhouse", SERVER_ARGS);
            this.cwds.insert(pid, cwd);
            this
        }

        fn discover(&self) -> Vec<DiscoveredProcess> {
            discover_in(
                &self.table,
                |pid| self.cwds.get(&pid).map(std::path::PathBuf::from),
                |pid| self.args.get(&pid).cloned(),
            )
        }
    }

    const DEV_CWD: &str = "/Users/al/project/.clickhouse/servers/dev/data";

    #[test]
    fn each_server_reports_its_watchdog_parent_or_else_itself() {
        for (case, processes, expected) in [
            (
                // The rewritten `argv[0]` and the empty arguments left by the
                // zeroed argument area behind it.
                "watchdog parent",
                FakeProcesses::new()
                    .process(9, 1, "clickhouse-watc", "clickhouse-watchdog   ")
                    .server(10, 9, DEV_CWD),
                vec![9],
            ),
            // CLICKHOUSE_WATCHDOG_ENABLE=0, so the CLI's own child is the
            // server and its parent is a shell.
            (
                "parent is not a watchdog",
                FakeProcesses::new()
                    .process(9, 1, "zsh", "/bin/zsh")
                    .server(10, 9, DEV_CWD),
                vec![10],
            ),
            // The parent exited between reading the table and its arguments.
            (
                "parent arguments unreadable",
                {
                    let mut processes = FakeProcesses::new()
                        .process(9, 1, "clickhouse-watc", "clickhouse-watchdog")
                        .server(10, 9, DEV_CWD);
                    processes.args.remove(&9);
                    processes
                },
                vec![10],
            ),
            (
                "parent not in the table",
                FakeProcesses::new().server(10, 9, DEV_CWD),
                vec![10],
            ),
            (
                "each PID resolved independently",
                FakeProcesses::new()
                    .process(9, 1, "clickhouse-watc", "clickhouse-watchdog server")
                    .server(10, 9, DEV_CWD)
                    .server(20, 1, "/Users/al/other/.clickhouse/servers/dev/data"),
                vec![9, 20],
            ),
        ] {
            let pids: Vec<u32> = processes.discover().iter().map(|proc| proc.pid).collect();
            assert_eq!(pids, expected, "{case}");
        }
    }

    #[test]
    fn a_macos_watchdog_named_clickhouse_is_the_supervisor_not_a_second_server() {
        // macOS keeps the kernel name `clickhouse` for the watchdog, and it
        // shares the server's cwd: only its rewritten `argv[0]` tells it apart.
        let mut processes = FakeProcesses::new()
            .process(9, 1, "clickhouse", "clickhouse-watchdog ")
            .server(10, 9, DEV_CWD);
        processes.cwds.insert(9, DEV_CWD);

        let pids: Vec<u32> = processes.discover().iter().map(|proc| proc.pid).collect();

        assert_eq!(pids, [9], "one server, reported by its watchdog's PID");
    }

    #[test]
    fn only_clickhouse_processes_with_a_managed_readable_cwd_are_discovered() {
        let mut processes = FakeProcesses::new()
            // A Linux watchdog renames its kernel process, so it is not a
            // candidate even though it shares the server's cwd.
            .process(9, 1, "clickhouse-watc", "clickhouse-watchdog")
            .server(10, 9, DEV_CWD)
            .server(11, 1, "/var/lib/clickhouse")
            .server(12, 1, "/Users/al/gone/.clickhouse/servers/dev/data")
            .process(13, 1, "sleep", SERVER_ARGS);
        processes.cwds.insert(9, DEV_CWD);
        processes.cwds.insert(13, DEV_CWD);
        // Exited (or another user's) between the table read and the cwd read.
        processes.cwds.remove(&12);

        let found = processes.discover();

        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].pid, 9);
        assert_eq!(found[0].project_root, "/Users/al/project");
        assert_eq!(found[0].server_name, "dev");
        assert_eq!(found[0].version.as_deref(), Some("26.9.1.531"));
        assert_eq!(found[0].http_port, Some(8123));
    }

    #[test]
    fn a_server_whose_arguments_are_unreadable_keeps_its_cwd_derived_fields() {
        let mut processes = FakeProcesses::new().server(10, 1, DEV_CWD);
        processes.args.clear();

        let found = processes.discover();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].pid, 10);
        assert_eq!(found[0].server_name, "dev");
        assert_eq!(found[0].version, None);
    }

    /// The scan end to end over the real process table: a process named
    /// `clickhouse`, started from a managed data directory, is discovered with
    /// its project, name and version.
    #[test]
    fn the_native_scan_discovers_a_live_process_named_clickhouse() {
        use std::os::unix::process::CommandExt;

        let root = tempfile::tempdir().unwrap();
        let version_dir = root.path().join(".clickhouse/versions/26.9.1.531");
        let data_dir = root.path().join("project/.clickhouse/servers/dev/data");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::create_dir_all(&data_dir).unwrap();
        let binary = version_dir.join("clickhouse");
        let sleep = ["/bin/sleep", "/usr/bin/sleep"]
            .into_iter()
            .find(|path| Path::new(path).exists())
            .expect("a sleep binary");
        std::fs::copy(sleep, &binary).unwrap();

        // The kernel name comes from the executed file, so the copy runs as
        // `clickhouse`. `argv[0]` still has to say `sleep`: a multicall
        // coreutils (uutils on Ubuntu 26.04) picks the program from it and
        // exits at once for `clickhouse`. It keeps the versions directory, so
        // the version is still parsed from the command line.
        let argv0 = version_dir.join("sleep");

        // Another test thread forking while the copy was open can hold the
        // file open for writing until its child execs (ETXTBSY), so retry.
        let mut attempts = 0;
        let mut child = loop {
            match std::process::Command::new(&binary)
                .arg0(&argv0)
                .arg("30")
                .current_dir(&data_dir)
                .spawn()
            {
                Err(error)
                    if error.kind() == std::io::ErrorKind::ExecutableFileBusy && attempts < 50 =>
                {
                    attempts += 1;
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                spawned => break spawned.expect("spawn the stand-in server"),
            }
        };
        let pid = child.id();
        let found = discover_clickhouse_processes();
        let _ = child.kill();
        let _ = child.wait();

        let found = found.expect("read the process table");
        let proc = found
            .iter()
            .find(|proc| proc.pid == pid)
            .unwrap_or_else(|| panic!("PID {pid} not discovered in {found:?}"));
        let project = root.path().join("project").canonicalize().unwrap();
        assert_eq!(Path::new(&proc.project_root), project);
        assert_eq!(proc.server_name, "dev");
        assert_eq!(proc.version.as_deref(), Some("26.9.1.531"));
    }

    #[test]
    fn watchdog_commands_are_recognised_by_their_rewritten_argv0() {
        for (command, watchdog) in [
            // The real row: the rewritten name and the padding of the zeroed
            // `argv` region behind it, with no arguments.
            ("clickhouse-watchdog     ", true),
            ("clickhouse-watchdog server -- --path=./", true),
            ("/usr/bin/clickhouse-watchdog server", true),
            // The rewrite is truncated to the length of the spawn path, so a
            // prefix counts as long as it is longer than "clickhouse" itself.
            ("clickhouse-watc server", true),
            ("clickhouse- server", true),
            // "clickhouse" is a prefix of "clickhouse-watchdog", so a name no
            // longer than the server's own basename must not count.
            ("clickhouse server", false),
            ("clickhouse local --query 'SELECT 1'", false),
            (
                "/Users/al/.clickhouse/versions/26.9.1.531/clickhouse server -- --path=./",
                false,
            ),
            // Long enough, but not a prefix of the watchdog's name.
            ("clickhouse-client --host localhost", false),
            ("clickhouse-watchdog-wrapper server", false),
            ("/bin/zsh", false),
            ("/sbin/launchd", false),
            ("", false),
        ] {
            assert_eq!(is_watchdog_command(command), watchdog, "{command:?}");
        }
    }

    // ── inspect_process over an already-scanned row ────────────────────

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
    fn a_process_with_no_arguments_keeps_its_cwd_derived_fields() {
        // The arguments could not be read (the process exited mid-scan), which
        // leaves an empty command line.
        let proc = inspect_process(4211, "/home/al/app/.clickhouse/servers/default/data", "")
            .expect("the cwd alone still identifies the server");

        assert_eq!(proc.project_root, "/home/al/app");
        assert_eq!(proc.server_name, "default");
        assert_eq!(proc.http_port, None);
        assert_eq!(proc.tcp_port, None);
        assert_eq!(proc.version, None);
    }

    #[test]
    fn command_line_ports_from_older_releases_win_over_the_managed_config() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("app/.clickhouse/servers/dev/data");
        crate::local::config::write_managed_config(&data_dir, 18123, 19000).unwrap();
        let binary = "/home/al/.clickhouse/versions/26.9.1.531/clickhouse server";

        for (cmdline, expected) in [
            (binary.to_string(), (Some(18123), Some(19000))),
            (
                format!("{binary} -- --path=./ --http_port=8123 --tcp_port=9000"),
                (Some(8123), Some(9000)),
            ),
        ] {
            let proc = inspect_process(4211, data_dir.to_str().unwrap(), &cmdline)
                .expect("a CLI-managed cwd is a discovered server");
            assert_eq!((proc.http_port, proc.tcp_port), expected, "{cmdline}");
        }
    }

    #[test]
    fn a_cwd_outside_a_project_is_not_a_discovered_server() {
        assert!(
            inspect_process(4211, "/var/lib/clickhouse", "/usr/bin/clickhouse server").is_none()
        );
    }

    // ── cwd, port and version parsers ──────────────────────────────────

    #[test]
    fn parse_server_cwd_splits_project_root_and_name_or_rejects_unmanaged_paths() {
        for (cwd, expected) in [
            (
                "/Users/al/project-a/.clickhouse/servers/default/data",
                Some(("/Users/al/project-a", "default")),
            ),
            (
                "/Users/al/project-a/.clickhouse/servers/default/data/",
                Some(("/Users/al/project-a", "default")),
            ),
            (
                "/home/user/myapp/.clickhouse/servers/bold-crane/data",
                Some(("/home/user/myapp", "bold-crane")),
            ),
            (
                "/Users/al/code/projects/web/.clickhouse/servers/test/data",
                Some(("/Users/al/code/projects/web", "test")),
            ),
            // The server dir itself, without `/data`: the suffix is only
            // stripped when present, and "default" is still a valid name.
            (
                "/Users/al/project/.clickhouse/servers/default",
                Some(("/Users/al/project", "default")),
            ),
            // Not started by the CLI.
            ("/var/lib/clickhouse", None),
            ("/Users/al/project/.clickhouse/servers/", None),
        ] {
            let expected = expected.map(|(root, name)| (root.to_string(), name.to_string()));
            assert_eq!(parse_server_cwd(cwd), expected, "{cwd}");
        }
    }

    #[test]
    fn parse_port_flag_reads_the_named_flag_value() {
        let managed = "/home/user/.clickhouse/versions/25.12.5.44/clickhouse server";
        for (cmdline, flag, expected) in [
            (
                format!("{managed} --http_port=8123 --tcp_port=9000"),
                "--http_port",
                Some(8123),
            ),
            (
                format!("{managed} --http_port=8124 --tcp_port=9001"),
                "--tcp_port",
                Some(9001),
            ),
            (
                "clickhouse server --http_port=18123 --tcp_port=19000".to_string(),
                "--http_port",
                Some(18123),
            ),
            (
                "clickhouse server --http_port=18123 --tcp_port=19000".to_string(),
                "--tcp_port",
                Some(19000),
            ),
            ("clickhouse server".to_string(), "--http_port", None),
            (
                "clickhouse server --http_port=abc".to_string(),
                "--http_port",
                None,
            ),
        ] {
            assert_eq!(
                parse_port_flag(&cmdline, flag),
                expected,
                "{cmdline} {flag}"
            );
        }
    }

    #[test]
    fn parse_version_from_cmdline_reads_the_managed_versions_directory() {
        for (cmdline, expected) in [
            (
                "/Users/al/.clickhouse/versions/25.12.5.44/clickhouse server --http_port=8123",
                Some("25.12.5.44"),
            ),
            (
                "/home/user/.clickhouse/versions/24.8.1.1/clickhouse server",
                Some("24.8.1.1"),
            ),
            ("/usr/bin/clickhouse server", None),
            ("/home/user/.clickhouse/versions//clickhouse server", None),
        ] {
            assert_eq!(
                parse_version_from_cmdline(cmdline).as_deref(),
                expected,
                "{cmdline}"
            );
        }
    }
}
