//! Subprocess coverage for the cross-project `local remove` running-server
//! guard (issue #600).
//!
//! Strategy: an isolated `HOME` with three fake installed versions and two
//! project directories. The fake servers are compiled stand-ins (see
//! `support/fake_clickhouse.rs`) that native process discovery finds like real
//! ones. A server is started in project B and `local remove` is then run from
//! project A, where the server is invisible to project-scoped metadata.
//!
//! Discovery sees every process on the machine, including the fixtures of the
//! other tests here running in parallel, so each fixture's running version is
//! its own: no test can find, or `--force`-stop, another test's server.

#[path = "support/fake_clickhouse.rs"]
mod fake_clickhouse;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

/// The default version, so the issue-599 default guard never masks this one.
const DEFAULT_VERSION: &str = "25.12.9.61";
/// Installed and used by nothing.
const UNUSED_VERSION: &str = "24.8.1.1";

const WAIT_TIMEOUT: Duration = Duration::from_secs(5);

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

/// The version a fixture's fake server runs on, derived from its label so no
/// two fixtures share one.
fn running_version_for(label: &str) -> String {
    let hash = label.bytes().fold(2166136261u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16777619)
    });
    format!("26.9.{}.217", 1000 + hash % 100_000)
}

struct Fixture {
    // Drop the process guard before removing directories used as process CWDs.
    _processes: fake_clickhouse::ProcessGuard,
    _root: tempfile::TempDir,
    home: PathBuf,
    project_a: PathBuf,
    project_b: PathBuf,
    /// The version the fake server in the other project runs on.
    running_version: String,
    path: String,
    pid_file: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = tempfile::Builder::new()
            .prefix(&format!("clickhousectl-remove-global-{label}-"))
            .tempdir()
            .expect("create isolated test root");
        let home = root.path().join("home");
        let project_a = root.path().join("project-a");
        let project_b = root.path().join("project-b");
        let pid_file = root.path().join("fake-clickhouse-pids");
        let running_version = running_version_for(label);

        std::fs::create_dir_all(home.join(".clickhouse")).expect("create isolated HOME");
        std::fs::create_dir_all(&project_a).expect("create project A");
        std::fs::create_dir_all(&project_b).expect("create project B");

        for version in [running_version.as_str(), DEFAULT_VERSION, UNUSED_VERSION] {
            fake_clickhouse::install(&version_binary(&home, version));
        }

        std::fs::write(home.join(".clickhouse/default"), DEFAULT_VERSION)
            .expect("write default marker");

        // No /usr/local/bin or /opt/homebrew/bin, so a developer's `docker`
        // is unreachable and recovery never takes its Postgres leg.
        let path = "/usr/bin:/bin:/usr/sbin:/sbin".to_string();

        Self {
            _processes: fake_clickhouse::ProcessGuard {
                pid_file: pid_file.clone(),
            },
            _root: root,
            home,
            project_a,
            project_b,
            running_version,
            path,
            pid_file,
        }
    }

    fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(clickhousectl_binary())
            // `env_clear` keeps coding-agent detection from switching the
            // subprocess to JSON, so human-output assertions stay meaningful.
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            // No update-check network request.
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
            .env("HOME", &self.home)
            .env("PATH", &self.path)
            .env("FAKE_CLICKHOUSE_PID_FILE", &self.pid_file)
            .current_dir(cwd)
            .args(args)
            .output()
            .expect("run shipped clickhousectl binary")
    }

    /// Start a fake server on the running version in `project`, returning its
    /// PID.
    fn start_server(&self, project: &Path, name: &str) -> u32 {
        let output = self.run(
            project,
            &[
                "local",
                "--json",
                "server",
                "start",
                name,
                "-v",
                &self.running_version,
                "--no-wait",
            ],
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "start stderr: {}",
            stderr_of(&output)
        );
        let body: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("parse start JSON");
        assert_eq!(body["version"], self.running_version.as_str());
        let pid = body["pid"].as_u64().expect("start PID") as u32;
        wait_for_pid_record(&self.pid_file, pid);
        assert!(process_is_alive(pid));
        pid
    }

    fn version_dir(&self, version: &str) -> PathBuf {
        self.home.join(".clickhouse/versions").join(version)
    }
}

fn version_binary(home: &Path, version: &str) -> PathBuf {
    home.join(".clickhouse/versions")
        .join(version)
        .join("clickhouse")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn process_is_alive(pid: u32) -> bool {
    i32::try_from(pid).is_ok_and(|pid| unsafe { libc::kill(pid, 0) == 0 })
}

fn wait_for_process_exit(pid: u32) {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while process_is_alive(pid) {
        assert!(Instant::now() < deadline, "PID {pid} did not exit");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_pid_record(pid_file: &Path, pid: u32) {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while !fake_clickhouse::records(pid_file)
        .iter()
        .any(|record| record.pid == pid)
    {
        assert!(
            Instant::now() < deadline,
            "fake ClickHouse {pid} never recorded itself"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn canonical(path: &Path) -> String {
    path.canonicalize()
        .expect("canonicalize project path")
        .display()
        .to_string()
}

#[test]
fn removing_a_version_a_server_in_another_project_runs_is_refused() {
    let fixture = Fixture::new("other-project");
    let pid = fixture.start_server(&fixture.project_b, "dev");

    let output = fixture.run(
        &fixture.project_a,
        &["local", "remove", fixture.running_version.as_str()],
    );

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    for required in [
        fixture.running_version.as_str(),
        "'dev'",
        &canonical(&fixture.project_b),
        &format!("PID {pid}"),
        "--force",
    ] {
        assert!(stderr.contains(required), "missing {required:?}: {stderr}");
    }
    assert!(
        fixture.version_dir(&fixture.running_version).exists(),
        "a refused removal must not delete the version directory"
    );
    assert!(
        process_is_alive(pid),
        "a refused removal must not stop the server"
    );
}

#[test]
fn the_structured_refusal_names_the_blocking_project_and_the_global_list() {
    let fixture = Fixture::new("other-project-json");
    let pid = fixture.start_server(&fixture.project_b, "dev");

    let output = fixture.run(
        &fixture.project_a,
        &[
            "local",
            "--json",
            "remove",
            fixture.running_version.as_str(),
        ],
    );

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        output.stdout.is_empty(),
        "runtime errors belong on stderr, got: {}",
        stdout_of(&output)
    );
    let error: serde_json::Value =
        serde_json::from_str(&stderr).expect("one JSON error object on stderr");
    assert_eq!(error["error"]["code"], "server_running");
    let message = error["error"]["message"].as_str().expect("message");
    for required in [
        fixture.running_version.as_str(),
        "'dev'",
        &canonical(&fixture.project_b),
    ] {
        assert!(
            message.contains(required),
            "missing {required:?}: {message}"
        );
    }
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local clickhouse list --global"
    );
    assert!(fixture.version_dir(&fixture.running_version).exists());
    assert!(process_is_alive(pid));
}

#[test]
fn force_stops_the_other_projects_server_before_removing_the_version() {
    let fixture = Fixture::new("other-project-force");
    let pid = fixture.start_server(&fixture.project_b, "dev");

    let output = fixture.run(
        &fixture.project_a,
        &[
            "local",
            "remove",
            fixture.running_version.as_str(),
            "--force",
        ],
    );

    let stderr = stderr_of(&output);
    let stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstderr: {stderr}",
        output.status
    );
    assert!(
        stdout.contains("'dev'") && stdout.contains(&canonical(&fixture.project_b)),
        "--force must report which project's server it stopped: {stdout}"
    );
    wait_for_process_exit(pid);
    assert!(
        !fixture.version_dir(&fixture.running_version).exists(),
        "--force must remove the version directory"
    );
}

#[test]
fn the_same_project_guard_still_refuses_and_keeps_metadata_consistent() {
    let fixture = Fixture::new("same-project");
    let pid = fixture.start_server(&fixture.project_a, "dev");

    let refused = fixture.run(
        &fixture.project_a,
        &["local", "remove", fixture.running_version.as_str()],
    );
    assert_eq!(
        refused.status.code(),
        Some(1),
        "stderr: {}",
        stderr_of(&refused)
    );
    assert!(stderr_of(&refused).contains("'dev'"));
    assert!(process_is_alive(pid));

    let forced = fixture.run(
        &fixture.project_a,
        &[
            "local",
            "remove",
            fixture.running_version.as_str(),
            "--force",
        ],
    );
    assert!(
        forced.status.success(),
        "expected success, got {:?}\nstderr: {}",
        forced.status,
        stderr_of(&forced)
    );
    wait_for_process_exit(pid);
    assert!(!fixture.version_dir(&fixture.running_version).exists());

    // Stopping through this project's metadata keeps the recorded server
    // stopped rather than leaving a stale live PID behind.
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(fixture.project_a.join(".clickhouse/servers/dev.json"))
            .expect("read server metadata"),
    )
    .expect("parse server metadata");
    assert_eq!(metadata["pid"], 0);
    assert_eq!(metadata["version"], "");
}

#[test]
fn a_version_nothing_runs_is_removed_while_another_projects_server_keeps_running() {
    let fixture = Fixture::new("unused-version");
    let pid = fixture.start_server(&fixture.project_b, "dev");

    let output = fixture.run(
        &fixture.project_a,
        &["local", "--json", "remove", UNUSED_VERSION],
    );

    let stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstderr: {}",
        output.status,
        stderr_of(&output)
    );
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("one JSON object");
    assert_eq!(value["version"], UNUSED_VERSION);
    assert!(!fixture.version_dir(UNUSED_VERSION).exists());
    assert!(
        fixture.version_dir(&fixture.running_version).exists(),
        "the in-use version must be untouched"
    );
    assert!(
        process_is_alive(pid),
        "removing an unrelated version must not stop anything"
    );
}
