//! Subprocess coverage for PID agreement between the project-scoped and the
//! `--global` server list (issue #664).
//!
//! ClickHouse spawns a watchdog that re-execs the server if it dies, so the
//! process `server start` records is the watchdog while `pgrep -x clickhouse`
//! only ever matches the server it supervises. Both views must report the PID
//! that stops the server for good.
//!
//! Strategy: an isolated `HOME`, a fake ClickHouse that really forks a child so
//! there is a genuine parent/child pair to resolve, and deterministic
//! `pgrep`/`lsof`/`ps` stand-ins on `PATH` (the same technique as
//! `local_server_state_machine_test.rs`) so process discovery only ever sees
//! this fixture's processes and does not depend on platform process names.

use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const VERSION: &str = "26.9.1.531";
const HTTP_PORT: u16 = 18923;
const TCP_PORT: u16 = 19923;
const WAIT_TIMEOUT: Duration = Duration::from_secs(5);

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

/// Whether the fake ClickHouse forks a supervised child, as the real watchdog
/// does, or runs the "server" in the process the CLI spawned.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Supervision {
    Watchdog,
    None,
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
            .filter_map(|line| line.split('|').nth(1)?.parse::<i32>().ok())
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
    project: PathBuf,
    pid_file: PathBuf,
    path: String,
    supervision: Supervision,
}

impl Fixture {
    fn new(label: &str, supervision: Supervision) -> Self {
        let root = tempfile::Builder::new()
            .prefix(&format!("clickhousectl-watchdog-pid-{label}-"))
            .tempdir()
            .expect("create isolated test root");
        let home = root.path().join("home");
        let project = root.path().join("project");
        let pid_file = root.path().join("fake-clickhouse-pids");
        let tools = root.path().join("fake-tools");
        let binary = home.join(format!(".clickhouse/versions/{VERSION}/clickhouse"));

        std::fs::create_dir_all(&home).expect("create isolated HOME");
        std::fs::create_dir_all(&project).expect("create project directory");
        std::fs::create_dir_all(binary.parent().expect("fake binary parent"))
            .expect("create fake version directory");
        std::fs::create_dir_all(&tools).expect("create fake process tools directory");

        // Records `<role>|<pid>|<ppid>|<cwd>` for the stand-in tools. With a
        // watchdog the spawned process stays alive as the parent of the
        // "server" child, exactly like ClickHouse's own watchdog.
        write_executable(
            &binary,
            r#"#!/bin/sh
if [ "$FAKE_CLICKHOUSE_WATCHDOG" = 1 ]; then
  /bin/sleep 300 &
  child=$!
  {
    printf 'watchdog|%s|1|%s\n' "$$" "$PWD"
    printf 'server|%s|%s|%s\n' "$child" "$$" "$PWD"
  } >> "$FAKE_CLICKHOUSE_PID_FILE"
  wait "$child"
else
  printf 'server|%s|1|%s\n' "$$" "$PWD" >> "$FAKE_CLICKHOUSE_PID_FILE"
  exec /bin/sleep 300
fi
"#,
        );
        std::fs::write(home.join(".clickhouse/default"), VERSION)
            .expect("select the fake version as latest");
        seed_update_cache(&home);

        // `pgrep -x clickhouse` matches the server, never the watchdog, which
        // is the whole reason the two views disagreed.
        write_executable(
            &tools.join("pgrep"),
            r#"#!/bin/sh
[ -f "$FAKE_CLICKHOUSE_PID_FILE" ] || exit 1
found=1
while IFS='|' read -r role pid ppid cwd; do
  if [ "$role" = server ] && kill -0 "$pid" 2>/dev/null; then
    printf '%s\n' "$pid"
    found=0
  fi
done < "$FAKE_CLICKHOUSE_PID_FILE"
exit "$found"
"#,
        );
        write_executable(
            &tools.join("lsof"),
            r#"#!/bin/sh
[ -f "$FAKE_CLICKHOUSE_PID_FILE" ] || exit 1
while IFS='|' read -r role pid ppid cwd; do
  if [ "$role" = server ] && kill -0 "$pid" 2>/dev/null; then
    printf 'p%s\nfcwd\nn%s\n' "$pid" "$cwd"
  fi
done < "$FAKE_CLICKHOUSE_PID_FILE"
"#,
        );
        // Two shapes are used by discovery: `ps -o args= -p <pid>` for one
        // process's command line, and `ps -o pid=,ppid=,args= -p <pid,...>`
        // for batched parent resolution. The watchdog's `argv[0]` is the
        // rewritten name the real one uses; the server's is its binary path.
        write_executable(
            &tools.join("ps"),
            &format!(
                r#"#!/bin/sh
fields=""
targets=""
while [ $# -gt 0 ]; do
  case "$1" in
    -o) fields="$2"; shift 2 ;;
    -p) targets="$2"; shift 2 ;;
    *) shift ;;
  esac
done
[ -f "$FAKE_CLICKHOUSE_PID_FILE" ] || exit 1
found=1
while IFS='|' read -r role pid ppid cwd; do
  case ",$targets," in
    *",$pid,"*) ;;
    *) continue ;;
  esac
  if [ "$role" = watchdog ]; then
    args="clickhouse-watchdog server -- --path=./ --http_port={HTTP_PORT} --tcp_port={TCP_PORT}"
  else
    args="$HOME/.clickhouse/versions/{VERSION}/clickhouse server -- --path=./ --http_port={HTTP_PORT} --tcp_port={TCP_PORT}"
  fi
  if [ "$fields" = "args=" ]; then
    printf '%s\n' "$args"
  else
    printf '%5s %5s %s\n' "$pid" "$ppid" "$args"
  fi
  found=0
done < "$FAKE_CLICKHOUSE_PID_FILE"
exit "$found"
"#
            ),
        );
        let path = format!("{}:/usr/bin:/bin:/usr/sbin:/sbin", tools.display());

        Self {
            _processes: ProcessGuard {
                pid_file: pid_file.clone(),
            },
            _root: root,
            home,
            project,
            pid_file,
            path,
            supervision,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(clickhousectl_binary())
            // `env_clear` keeps coding-agent detection from switching the
            // subprocess to JSON when a human-output assertion is made.
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &self.home)
            .env("PATH", &self.path)
            .env("FAKE_CLICKHOUSE_PID_FILE", &self.pid_file)
            .env(
                "FAKE_CLICKHOUSE_WATCHDOG",
                if self.supervision == Supervision::Watchdog {
                    "1"
                } else {
                    "0"
                },
            )
            .current_dir(&self.project)
            .args(args)
            .output()
            .expect("run shipped clickhousectl binary")
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
            &HTTP_PORT.to_string(),
            "--tcp-port",
            &TCP_PORT.to_string(),
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
        // Discovery reads the stand-ins' records, so wait until the fake
        // binary has registered every process it owns.
        let expected_records = if self.supervision == Supervision::Watchdog {
            2
        } else {
            1
        };
        wait_for_records(&self.pid_file, expected_records);
        pid
    }

    /// The PID recorded under `role`, from the fake binary's own records.
    fn recorded_pid(&self, role: &str) -> u32 {
        let contents = std::fs::read_to_string(&self.pid_file).expect("read PID records");
        contents
            .lines()
            .find_map(|line| {
                let mut fields = line.split('|');
                (fields.next()? == role).then(|| fields.next()?.parse::<u32>().ok())?
            })
            .unwrap_or_else(|| panic!("no {role} record in {contents}"))
    }

    fn listed_pid(&self, args: &[&str]) -> u32 {
        let output = self.run(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "list stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let body: Value = serde_json::from_slice(&output.stdout).expect("parse list JSON");
        let servers = body["servers"].as_array().expect("servers array");
        assert_eq!(servers.len(), 1, "unexpected server list: {body}");
        servers[0]["pid"].as_u64().expect("listed PID") as u32
    }

    fn metadata_path(&self, name: &str) -> PathBuf {
        self.project
            .join(".clickhouse/servers")
            .join(format!("{name}.json"))
    }
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

fn wait_for_records(pid_file: &Path, expected: usize) {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while Instant::now() < deadline {
        if std::fs::read_to_string(pid_file)
            .map(|contents| contents.lines().count() >= expected)
            .unwrap_or(false)
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("fake ClickHouse never recorded {expected} process(es)");
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
        "reporting the supervised child hands users a PID that restarts itself"
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
