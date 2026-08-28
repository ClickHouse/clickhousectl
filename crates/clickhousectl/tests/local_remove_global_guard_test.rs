//! Subprocess coverage for the cross-project `local remove` running-server
//! guard (issue #600).
//!
//! Strategy: an isolated `HOME` with three fake installed versions, two project
//! directories, and deterministic `pgrep`/`lsof`/`ps` stand-ins on `PATH` (the
//! same technique as `local_server_state_machine_test.rs`) so process discovery
//! only ever sees this fixture's fake servers. A server is started in project B
//! and `local remove` is then run from project A, where the server is invisible
//! to project-scoped metadata.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The version the fake server in the other project runs on.
const RUNNING_VERSION: &str = "26.9.1.217";
/// The default version, so the issue-599 default guard never masks this one.
const DEFAULT_VERSION: &str = "25.12.9.61";
/// Installed and used by nothing.
const UNUSED_VERSION: &str = "24.8.1.1";

const WAIT_TIMEOUT: Duration = Duration::from_secs(5);

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

struct ProcessGuard {
    pid_file: PathBuf,
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let Ok(contents) = std::fs::read_to_string(&self.pid_file) else {
            return;
        };
        for pid in contents
            .lines()
            .filter_map(|line| line.split('|').next()?.parse::<i32>().ok())
        {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }
}

struct Fixture {
    // Drop the process guard before removing directories used as process CWDs.
    _processes: ProcessGuard,
    _root: tempfile::TempDir,
    home: PathBuf,
    project_a: PathBuf,
    project_b: PathBuf,
    pid_file: PathBuf,
    path: String,
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
        let tools = root.path().join("fake-tools");

        std::fs::create_dir_all(home.join(".clickhouse")).expect("create isolated HOME");
        std::fs::create_dir_all(&project_a).expect("create project A");
        std::fs::create_dir_all(&project_b).expect("create project B");
        std::fs::create_dir_all(&tools).expect("create fake process tools directory");

        for version in [RUNNING_VERSION, DEFAULT_VERSION, UNUSED_VERSION] {
            let binary = version_binary(&home, version);
            std::fs::create_dir_all(binary.parent().expect("fake binary parent"))
                .expect("create fake version directory");
            // Records `<pid>|<cwd>|<version>` for the fake process tools, then
            // blocks so the server stays "running" for the whole test.
            write_executable(
                &binary,
                &format!(
                    "#!/bin/sh\nprintf '%s|%s|%s\\n' \"$$\" \"$PWD\" '{version}' >> \"$FAKE_CLICKHOUSE_PID_FILE\"\nexec /bin/sleep 300\n"
                ),
            );
        }

        std::fs::write(home.join(".clickhouse/default"), DEFAULT_VERSION)
            .expect("write default marker");
        seed_update_cache(&home);

        // Deterministic stand-ins: they expose only this fixture's live fake
        // processes, so the global scan cannot see (or kill) anything else on
        // the machine, and does not depend on platform process names.
        write_executable(
            &tools.join("pgrep"),
            "#!/bin/sh\n[ -f \"$FAKE_CLICKHOUSE_PID_FILE\" ] || exit 1\nfound=1\nwhile IFS='|' read -r pid cwd version; do\n  if kill -0 \"$pid\" 2>/dev/null; then\n    printf '%s\\n' \"$pid\"\n    found=0\n  fi\ndone < \"$FAKE_CLICKHOUSE_PID_FILE\"\nexit \"$found\"\n",
        );
        write_executable(
            &tools.join("lsof"),
            "#!/bin/sh\n[ -f \"$FAKE_CLICKHOUSE_PID_FILE\" ] || exit 1\nwhile IFS='|' read -r pid cwd version; do\n  if kill -0 \"$pid\" 2>/dev/null; then\n    printf 'p%s\\nfcwd\\nn%s\\n' \"$pid\" \"$cwd\"\n  fi\ndone < \"$FAKE_CLICKHOUSE_PID_FILE\"\n",
        );
        // `ps -o args= -p <pid>`: the version is what the guard matches on, so
        // it has to come from the requested PID's own record.
        write_executable(
            &tools.join("ps"),
            "#!/bin/sh\nfor arg in \"$@\"; do target=\"$arg\"; done\n[ -f \"$FAKE_CLICKHOUSE_PID_FILE\" ] || exit 1\nwhile IFS='|' read -r pid cwd version; do\n  if [ \"$pid\" = \"$target\" ]; then\n    printf '%s\\n' \"$HOME/.clickhouse/versions/$version/clickhouse server --http_port=8123 --tcp_port=9000\"\n    exit 0\n  fi\ndone < \"$FAKE_CLICKHOUSE_PID_FILE\"\nexit 1\n",
        );
        let path = format!("{}:/usr/bin:/bin:/usr/sbin:/sbin", tools.display());

        Self {
            _processes: ProcessGuard {
                pid_file: pid_file.clone(),
            },
            _root: root,
            home,
            project_a,
            project_b,
            pid_file,
            path,
        }
    }

    fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(clickhousectl_binary())
            // `env_clear` keeps coding-agent detection from switching the
            // subprocess to JSON, so human-output assertions stay meaningful.
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &self.home)
            .env("PATH", &self.path)
            .env("FAKE_CLICKHOUSE_PID_FILE", &self.pid_file)
            .current_dir(cwd)
            .args(args)
            .output()
            .expect("run shipped clickhousectl binary")
    }

    /// Start a fake server on `RUNNING_VERSION` in `project`, returning its PID.
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
                RUNNING_VERSION,
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
        assert_eq!(body["version"], RUNNING_VERSION);
        let pid = body["pid"].as_u64().expect("start PID") as u32;
        // The guard reads the process's own command line, so wait until the
        // fake binary has recorded itself for the stand-in tools.
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

fn write_executable(path: &Path, contents: &str) {
    std::fs::write(path, contents).expect("write fake executable");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("make fake executable executable");
}

fn seed_update_cache(home: &Path) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_secs();
    std::fs::write(
        home.join(".clickhouse/last_update_check"),
        format!("{now}\n{}", env!("CARGO_PKG_VERSION")),
    )
    .expect("seed fresh update cache");
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
    loop {
        let recorded = std::fs::read_to_string(pid_file).unwrap_or_default();
        if recorded
            .lines()
            .any(|line| line.split('|').next() == Some(&pid.to_string()))
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "fake ClickHouse {pid} never recorded itself: {recorded}"
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

    let output = fixture.run(&fixture.project_a, &["local", "remove", RUNNING_VERSION]);

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    for required in [
        RUNNING_VERSION,
        "'dev'",
        &canonical(&fixture.project_b),
        &format!("PID {pid}"),
        "--force",
    ] {
        assert!(stderr.contains(required), "missing {required:?}: {stderr}");
    }
    assert!(
        fixture.version_dir(RUNNING_VERSION).exists(),
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
        &["local", "--json", "remove", RUNNING_VERSION],
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
    for required in [RUNNING_VERSION, "'dev'", &canonical(&fixture.project_b)] {
        assert!(
            message.contains(required),
            "missing {required:?}: {message}"
        );
    }
    assert_eq!(
        error["error"]["command"],
        "clickhousectl local server list --global"
    );
    assert!(fixture.version_dir(RUNNING_VERSION).exists());
    assert!(process_is_alive(pid));
}

#[test]
fn force_stops_the_other_projects_server_before_removing_the_version() {
    let fixture = Fixture::new("other-project-force");
    let pid = fixture.start_server(&fixture.project_b, "dev");

    let output = fixture.run(
        &fixture.project_a,
        &["local", "remove", RUNNING_VERSION, "--force"],
    );

    let stderr = stderr_of(&output);
    let stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstderr: {stderr}",
        output.status
    );
    assert!(
        stdout.contains("Stopped server 'dev' in")
            && stdout.contains(&canonical(&fixture.project_b)),
        "--force must report which project's server it stopped: {stdout}"
    );
    wait_for_process_exit(pid);
    assert!(
        !fixture.version_dir(RUNNING_VERSION).exists(),
        "--force must remove the version directory"
    );
}

#[test]
fn the_same_project_guard_still_refuses_and_keeps_metadata_consistent() {
    let fixture = Fixture::new("same-project");
    let pid = fixture.start_server(&fixture.project_a, "dev");

    let refused = fixture.run(&fixture.project_a, &["local", "remove", RUNNING_VERSION]);
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
        &["local", "remove", RUNNING_VERSION, "--force"],
    );
    assert!(
        forced.status.success(),
        "expected success, got {:?}\nstderr: {}",
        forced.status,
        stderr_of(&forced)
    );
    wait_for_process_exit(pid);
    assert!(!fixture.version_dir(RUNNING_VERSION).exists());

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
        fixture.version_dir(RUNNING_VERSION).exists(),
        "the in-use version must be untouched"
    );
    assert!(
        process_is_alive(pid),
        "removing an unrelated version must not stop anything"
    );
}
