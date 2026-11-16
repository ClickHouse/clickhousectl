//! Subprocess coverage for the watchdog PID: the project-scoped and the
//! `--global` server list must agree on it, and stopping it must take the
//! server it supervises with it (issue #664).
//!
//! ClickHouse spawns a watchdog that owns the lifetime of the server process,
//! so the process `clickhouse start` records is the watchdog while discovery
//! finds the server it supervises by its own `argv[0]`. Both views
//! must report the PID that stops the server for good, and `stop` must leave
//! neither process behind: the watchdog forwards SIGTERM, but a SIGKILL it
//! cannot catch would otherwise strand the server.
//!
//! Strategy: an isolated `HOME` and a compiled fake ClickHouse (see
//! `support/fake_clickhouse.rs`) that really forks a child under a watchdog
//! whose `argv[0]` is rewritten, so there is a genuine parent/child pair for
//! native process discovery to resolve and for `stop` to kill. Discovery also
//! sees the other tests' fixtures running in parallel, so global listings are
//! filtered by this fixture's project.

#[path = "support/fake_clickhouse.rs"]
mod fake_clickhouse;

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const VERSION: &str = "26.9.1.531";
const HTTP_PORT_BASE: u16 = 18900;
const TCP_PORT_BASE: u16 = 19900;
const WAIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Ports for a fixture, derived from its label so two of these tests running in
/// parallel never claim the same pair. Nothing binds them (the fake binary only
/// sleeps), but a shared pair would make two fixtures indistinguishable.
fn ports_for(label: &str) -> (u16, u16) {
    let mut hash: u32 = 2166136261;
    for byte in label.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16777619);
    }
    let offset = (hash % 50) as u16;
    (HTTP_PORT_BASE + offset, TCP_PORT_BASE + offset)
}

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

/// Whether the fake ClickHouse forks a supervised child, as the real watchdog
/// does, or runs the "server" in the process the CLI spawned.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Supervision {
    /// A watchdog that forwards SIGTERM to the server and exits with it, like
    /// ClickHouse's own.
    Watchdog,
    /// A watchdog started with `CLICKHOUSE_WATCHDOG_NO_FORWARD=1`: it neither
    /// handles SIGTERM nor passes it on, so `stop` has to escalate to SIGKILL,
    /// which the watchdog cannot forward either.
    WatchdogIgnoringSigterm,
    /// `CLICKHOUSE_WATCHDOG_ENABLE=0`: the process the CLI spawned is the
    /// server, with no supervising parent.
    None,
}

impl Supervision {
    fn forks_a_child(self) -> bool {
        self != Supervision::None
    }

    fn ignores_sigterm(self) -> bool {
        self == Supervision::WatchdogIgnoringSigterm
    }
}

struct Fixture {
    // Drop the process guard before removing directories used as process CWDs.
    _processes: fake_clickhouse::ProcessGuard,
    _root: tempfile::TempDir,
    home: PathBuf,
    project: PathBuf,
    pid_file: PathBuf,
    path: String,
    supervision: Supervision,
    http_port: u16,
    tcp_port: u16,
}

impl Fixture {
    fn new(label: &str, supervision: Supervision) -> Self {
        let (http_port, tcp_port) = ports_for(label);
        let root = tempfile::Builder::new()
            .prefix(&format!("clickhousectl-watchdog-pid-{label}-"))
            .tempdir()
            .expect("create isolated test root");
        let home = root.path().join("home");
        let project = root.path().join("project");
        let pid_file = root.path().join("fake-clickhouse-pids");
        let binary = home.join(format!(".clickhouse/versions/{VERSION}/clickhouse"));

        std::fs::create_dir_all(&home).expect("create isolated HOME");
        std::fs::create_dir_all(&project).expect("create project directory");
        fake_clickhouse::install(&binary);
        std::fs::write(home.join(".clickhouse/default"), VERSION)
            .expect("select the fake version as latest");

        // `PATH` deliberately omits /usr/local/bin and /opt/homebrew/bin so a
        // `docker` on the developer's machine is unreachable: orphan recovery
        // must not take its Postgres leg during these tests.
        let path = "/usr/bin:/bin:/usr/sbin:/sbin".to_string();

        Self {
            _processes: fake_clickhouse::ProcessGuard {
                pid_file: pid_file.clone(),
            },
            _root: root,
            home,
            project,
            pid_file,
            path,
            supervision,
            http_port,
            tcp_port,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut command = Command::new(clickhousectl_binary());
        command
            // `env_clear` keeps coding-agent detection from switching the
            // subprocess to JSON when a human-output assertion is made.
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            // No update-check network request.
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
            .env("HOME", &self.home)
            .env("PATH", &self.path)
            .env("FAKE_CLICKHOUSE_PID_FILE", &self.pid_file)
            .env(
                "FAKE_CLICKHOUSE_WATCHDOG",
                if self.supervision.forks_a_child() {
                    "1"
                } else {
                    "0"
                },
            )
            .env(
                "FAKE_CLICKHOUSE_IGNORE_SIGTERM",
                if self.supervision.ignores_sigterm() {
                    "1"
                } else {
                    "0"
                },
            )
            .current_dir(&self.project)
            .args(args);
        if self.supervision.ignores_sigterm() {
            // The SIGTERM is ignored, so `stop` always escalates; do not wait
            // out the shipped grace period first.
            command.env("CHCTL_TERM_TIMEOUT_MS", "200");
        }
        command.output().expect("run shipped clickhousectl binary")
    }

    /// Start the fake server and return the PID `start` reported, which is the
    /// PID it recorded in the server's metadata.
    fn start(&self, name: &str) -> u32 {
        let output = self.run(&[
            "local",
            "--json",
            "server",
            "start",
            name,
            "--http-port",
            &self.http_port.to_string(),
            "--tcp-port",
            &self.tcp_port.to_string(),
            "--no-wait",
        ]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "start stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let body: Value = serde_json::from_slice(&output.stdout).expect("parse start JSON");
        let pid = body["pid"].as_u64().expect("start PID") as u32;
        // Wait until the fake has started, and recorded, every process it
        // owns: the watchdog records its server once it would forward a
        // SIGTERM to it.
        let expected_records = if self.supervision.forks_a_child() {
            2
        } else {
            1
        };
        wait_for_records(&self.pid_file, expected_records);
        pid
    }

    /// The PID recorded under `role`, from the fake binary's own records.
    fn recorded_pid(&self, role: &str) -> u32 {
        let records = fake_clickhouse::records(&self.pid_file);
        records
            .iter()
            .find(|record| record.role == role)
            .map(|record| record.pid)
            .unwrap_or_else(|| panic!("no {role} record in {records:?}"))
    }

    /// The PID of this fixture's one server in a list. A global list also
    /// carries the other tests' fixtures, so it is narrowed to this project.
    fn listed_pid(&self, args: &[&str]) -> u32 {
        let output = self.run(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "list stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let body: Value = serde_json::from_slice(&output.stdout).expect("parse list JSON");
        let project = self
            .project
            .canonicalize()
            .expect("canonicalize project")
            .display()
            .to_string();
        let servers: Vec<&Value> = body["servers"]
            .as_array()
            .expect("servers array")
            .iter()
            .filter(|server| {
                server
                    .get("project")
                    .is_none_or(|listed| *listed == project)
            })
            .collect();
        assert_eq!(servers.len(), 1, "unexpected server list: {body}");
        servers[0]["pid"].as_u64().expect("listed PID") as u32
    }

    fn metadata_path(&self, name: &str) -> PathBuf {
        self.project
            .join(".clickhouse/servers")
            .join(format!("{name}.json"))
    }
}

fn wait_for_records(pid_file: &Path, expected: usize) {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while Instant::now() < deadline {
        if fake_clickhouse::records(pid_file).len() >= expected {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("fake ClickHouse never recorded {expected} process(es)");
}

fn is_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Poll for `pid` to be gone. Only the fake watchdog's own ordering needs the
/// timeout: it forwards the signal and exits without waiting for the child to
/// die. The CLI's SIGKILL path verifies both processes itself, so tests of it
/// assert directly.
fn wait_for_exit(pid: u32) -> bool {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while Instant::now() < deadline {
        if !is_alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    !is_alive(pid)
}

const PROJECT_LIST: &[&str] = &["local", "--json", "server", "list"];
const GLOBAL_LIST: &[&str] = &["local", "--json", "server", "list", "--global"];

#[test]
fn both_list_views_report_the_watchdog_pid() {
    let fixture = Fixture::new("agree", Supervision::Watchdog);
    let started = fixture.start("pidcheck");

    let watchdog = fixture.recorded_pid("watchdog");
    let server = fixture.recorded_pid("server");
    assert_ne!(watchdog, server, "the fixture must fork a real child");
    assert_eq!(started, watchdog, "start records the supervising process");

    let project_pid = fixture.listed_pid(PROJECT_LIST);
    let global_pid = fixture.listed_pid(GLOBAL_LIST);

    assert_eq!(
        project_pid, global_pid,
        "the project-scoped and --global views must agree on the PID"
    );
    assert_eq!(global_pid, watchdog);
    assert_ne!(
        global_pid, server,
        "reporting the supervised child hands users a PID whose watchdog \
         outlives it"
    );
}

#[test]
fn orphan_recovery_records_the_watchdog_pid() {
    let fixture = Fixture::new("recovery", Supervision::Watchdog);
    fixture.start("pidcheck");
    let watchdog = fixture.recorded_pid("watchdog");

    // Lose the metadata file, so the next command has to rebuild it from
    // process discovery alone.
    std::fs::remove_file(fixture.metadata_path("pidcheck")).expect("remove server metadata");

    let project_pid = fixture.listed_pid(PROJECT_LIST);

    assert_eq!(
        project_pid, watchdog,
        "recovered metadata must carry the PID that stops the server"
    );
    assert_eq!(project_pid, fixture.listed_pid(GLOBAL_LIST));
}

#[test]
fn a_server_without_a_watchdog_reports_its_own_pid() {
    // CLICKHOUSE_WATCHDOG_ENABLE=0: the process the CLI spawned *is* the
    // server, so there is no parent to promote.
    let fixture = Fixture::new("no-watchdog", Supervision::None);
    let started = fixture.start("pidcheck");
    let server = fixture.recorded_pid("server");

    assert_eq!(started, server);
    assert_eq!(fixture.listed_pid(PROJECT_LIST), server);
    assert_eq!(fixture.listed_pid(GLOBAL_LIST), server);
}

#[test]
fn each_fixture_label_gets_its_own_port_pair() {
    let labels = ["agree", "recovery", "no-watchdog", "stop", "escalate"];
    let mut pairs: Vec<_> = labels.iter().map(|label| ports_for(label)).collect();
    pairs.sort_unstable();
    pairs.dedup();

    assert_eq!(
        pairs.len(),
        labels.len(),
        "two of these tests would claim the same ports: {pairs:?}"
    );
}

#[test]
fn stop_terminates_the_watchdog_and_the_server_it_supervises() {
    let fixture = Fixture::new("stop", Supervision::Watchdog);
    fixture.start("pidcheck");
    let watchdog = fixture.recorded_pid("watchdog");
    let server = fixture.recorded_pid("server");

    let output = fixture.run(&["local", "--json", "server", "stop", "pidcheck"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stop stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse stop JSON");
    assert_eq!(body["already_stopped"], Value::Bool(false), "{body}");

    assert!(wait_for_exit(watchdog), "the signalled watchdog must exit");
    assert!(
        wait_for_exit(server),
        "the watchdog forwards the SIGTERM, so the server goes with it"
    );
}

#[test]
fn stop_escalating_to_sigkill_takes_the_supervised_server_with_it() {
    // The watchdog ignores SIGTERM, so `stop` has to escalate. A SIGKILL is
    // not forwarded either, which used to leave the server running, reparented
    // to init and still holding the ports and the data directory, while the
    // command reported success.
    let fixture = Fixture::new("escalate", Supervision::WatchdogIgnoringSigterm);
    fixture.start("pidcheck");
    let watchdog = fixture.recorded_pid("watchdog");
    let server = fixture.recorded_pid("server");

    let output = fixture.run(&["local", "--json", "server", "stop", "pidcheck"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stop stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse stop JSON");
    assert_eq!(body["already_stopped"], Value::Bool(false), "{body}");

    // Success is only reported once the CLI has verified both processes, so
    // these need no polling.
    assert!(!is_alive(watchdog), "the watchdog must be SIGKILLed");
    assert!(
        !is_alive(server),
        "a SIGKILLed watchdog cannot forward the signal, so the server it \
         supervised has to be killed with it"
    );
}
