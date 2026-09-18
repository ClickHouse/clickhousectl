//! End-to-end coverage for the project-local ClickHouse server state machine.

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const VERSION: &str = "latest";
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
        for pid in contents.lines().filter_map(|line| {
            line.split_once('|')
                .and_then(|(pid, _)| pid.parse::<i32>().ok())
        }) {
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
    child: PathBuf,
    pid_file: PathBuf,
    runner: PathBuf,
    path: String,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = tempfile::Builder::new()
            .prefix(&format!("clickhousectl-state-machine-{label}-"))
            .tempdir()
            .expect("create isolated test root");
        let home = root.path().join("home");
        let project = root.path().join("project");
        let child = project.join("child");
        let pid_file = root.path().join("fake-clickhouse-pids");
        let binary = home.join(format!(".clickhouse/versions/{VERSION}/clickhouse"));
        let runner = PathBuf::from("/bin/sleep");
        let tools = root.path().join("fake-tools");

        std::fs::create_dir_all(&home).expect("create isolated HOME");
        std::fs::create_dir_all(&child).expect("create project child directory");
        std::fs::create_dir_all(binary.parent().expect("fake binary parent"))
            .expect("create fake version directory");
        std::fs::create_dir_all(&tools).expect("create fake process tools directory");

        std::fs::write(
            &binary,
            b"#!/bin/sh\nprintf '%s|%s\\n' \"$$\" \"$PWD\" >> \"$FAKE_CLICKHOUSE_PID_FILE\"\nexec \"$FAKE_CLICKHOUSE_RUNNER\" 300\n",
        )
        .expect("install fake ClickHouse wrapper");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .expect("make fake ClickHouse wrapper executable");
        std::fs::write(home.join(".clickhouse/default"), VERSION)
            .expect("select fake latest version");
        seed_update_cache(&home);

        // Process discovery is part of the shipped lifecycle. Deterministic
        // stand-ins expose only this fixture's live fake processes, avoiding
        // dependence on platform process names or unrelated user processes.
        write_executable(
            &tools.join("pgrep"),
            // `-P <ppid>` is the child lookup in the SIGKILL escalation path.
            // These fake servers `exec` a sleep and have no children, so there
            // is nothing to report.
            r#"#!/bin/sh
[ "$1" = "-P" ] && exit 1
[ -f "$FAKE_CLICKHOUSE_PID_FILE" ] || exit 1
found=1
while IFS='|' read -r pid cwd; do
  if kill -0 "$pid" 2>/dev/null; then
    printf '%s\n' "$pid"
    found=0
  fi
done < "$FAKE_CLICKHOUSE_PID_FILE"
exit "$found"
"#,
        );
        write_executable(
            &tools.join("lsof"),
            "#!/bin/sh\n[ -f \"$FAKE_CLICKHOUSE_PID_FILE\" ] || exit 1\nwhile IFS='|' read -r pid cwd; do\n  if kill -0 \"$pid\" 2>/dev/null; then\n    printf 'p%s\\nfcwd\\nn%s\\n' \"$pid\" \"$cwd\"\n  fi\ndone < \"$FAKE_CLICKHOUSE_PID_FILE\"\n",
        );
        // `ps -o pid=,ppid=,args= -p <pid,...>`: one batched call is the only
        // shape discovery uses. The fake servers are reparented to init once
        // the CLI that spawned them exits, so their PPID is 1.
        write_executable(
            &tools.join("ps"),
            &format!(
                r#"#!/bin/sh
targets=""
while [ $# -gt 0 ]; do
  case "$1" in
    -p) targets="$2"; shift 2 ;;
    *) shift ;;
  esac
done
[ -f "$FAKE_CLICKHOUSE_PID_FILE" ] || exit 1
found=1
while IFS='|' read -r pid cwd; do
  case ",$targets," in
    *",$pid,"*) ;;
    *) continue ;;
  esac
  printf '%5s %5s %s\n' "$pid" 1 "$HOME/.clickhouse/versions/{VERSION}/clickhouse server"
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
            child,
            pid_file,
            runner,
            path,
        }
    }

    fn command(&self, cwd: &Path) -> Command {
        let mut command = Command::new(clickhousectl_binary());
        command
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &self.home)
            .env("PATH", &self.path)
            .env("FAKE_CLICKHOUSE_PID_FILE", &self.pid_file)
            .env("FAKE_CLICKHOUSE_RUNNER", &self.runner)
            .current_dir(cwd);
        command
    }

    fn run(&self, cwd: &Path, args: &[String]) -> Output {
        self.command(cwd)
            .args(args)
            .output()
            .expect("run shipped clickhousectl binary")
    }

    fn run_cases(&self, cases: &[CommandCase]) {
        for case in cases {
            let cwd = match case.cwd {
                TestCwd::Root => &self.project,
                TestCwd::Child => &self.child,
            };
            let output = self.run(cwd, &case.args);
            case.expected.assert(&case.label, &output);
        }
    }

    fn start(&self, name: Option<&str>) -> RunningServer {
        self.start_with_expected_name(name, Some(name.unwrap_or("default")))
    }

    fn start_generated(&self) -> RunningServer {
        self.start_with_expected_name(None, None)
    }

    fn start_with_expected_name(
        &self,
        name: Option<&str>,
        expected_name: Option<&str>,
    ) -> RunningServer {
        let mut args = strings(&["local", "--json", "server", "start"]);
        if let Some(name) = name {
            args.push(name.to_string());
        }
        args.push("--no-wait".into());
        let output = self.run(&self.project, &args);
        let log = expected_name
            .and_then(|name| std::fs::read_to_string(self.server_dir(name).join("server.log")).ok())
            .unwrap_or_default();
        assert_eq!(
            output.status.code(),
            Some(0),
            "start {name:?} stderr: {} server log: {log}",
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(output.stderr.is_empty(), "JSON start wrote to stderr");
        let body: Value = serde_json::from_slice(&output.stdout).expect("parse start JSON");
        let actual_name = body["name"].as_str().expect("start name");
        if let Some(expected_name) = expected_name {
            assert_eq!(actual_name, expected_name, "unexpected server name");
        } else {
            assert_ne!(actual_name, "default", "repeated start reused default name");
            let (adjective, noun) = actual_name
                .split_once('-')
                .expect("generated name uses adjective-noun form");
            assert!(!adjective.is_empty() && !noun.is_empty());
        }
        let pid = body["pid"].as_u64().expect("start PID") as u32;
        let http_port = body["http_port"].as_u64().expect("start HTTP port") as u16;
        let tcp_port = body["tcp_port"].as_u64().expect("start TCP port") as u16;
        assert_ne!(pid, 0);
        assert_ne!(http_port, 0);
        assert_ne!(tcp_port, 0);
        assert_eq!(
            body,
            json!({
                "name": actual_name,
                "pid": pid,
                "http_port": http_port,
                "tcp_port": tcp_port,
                "version": VERSION
            }),
            "unexpected exact start response"
        );

        self.wait_for_pid_record(pid);
        wait_for_process(pid, true);
        let metadata = read_json(&self.metadata_path(actual_name));
        let started_at = metadata["started_at"]
            .as_str()
            .expect("metadata timestamp")
            .to_string();
        assert!(
            started_at.parse::<u64>().is_ok_and(|value| value > 0),
            "invalid persisted timestamp: {started_at}"
        );
        let server = RunningServer {
            name: actual_name.to_string(),
            pid,
            http_port,
            tcp_port,
            started_at,
        };
        self.assert_running_metadata(&server);
        assert!(self.data_dir(actual_name).is_dir());
        server
    }

    fn assert_list(&self, cwd: TestCwd, servers: &[ListedServer<'_>]) {
        let project = match cwd {
            TestCwd::Root => &self.project,
            TestCwd::Child => &self.child,
        };
        self.run_cases(&[CommandCase {
            label: "server list".into(),
            cwd,
            args: strings(&["local", "--json", "server", "list"]),
            expected: ExpectedOutput::JsonSuccess(server_list(project, servers)),
        }]);
    }

    fn assert_running_metadata(&self, server: &RunningServer) {
        assert_eq!(
            read_json(&self.metadata_path(&server.name)),
            json!({
                "name": server.name,
                "pid": server.pid,
                "version": VERSION,
                "http_port": server.http_port,
                "tcp_port": server.tcp_port,
                "started_at": server.started_at,
                "cwd": canonical(&self.project),
                "engine": "clickhouse"
            }),
            "unexpected running metadata for {}",
            server.name
        );
        assert!(process_is_alive(server.pid));
        assert!(self.data_dir(&server.name).is_dir());
    }

    fn assert_stopped_metadata(&self, server: &RunningServer) {
        assert_eq!(
            read_json(&self.metadata_path(&server.name)),
            json!({
                "name": server.name,
                "pid": 0,
                "version": "",
                "http_port": 0,
                "tcp_port": 0,
                "started_at": server.started_at,
                "cwd": canonical(&self.project),
                "engine": "clickhouse"
            }),
            "unexpected stopped metadata for {}",
            server.name
        );
        assert!(!process_is_alive(server.pid));
        assert!(self.data_dir(&server.name).is_dir());
    }

    fn write_marker(&self, name: &str, contents: &str) {
        std::fs::write(self.data_dir(name).join("state-marker"), contents)
            .expect("write persistent data marker");
    }

    fn assert_marker(&self, name: &str, contents: &str) {
        assert_eq!(
            std::fs::read_to_string(self.data_dir(name).join("state-marker"))
                .expect("read persistent data marker"),
            contents
        );
    }

    fn assert_removed(&self, name: &str) {
        assert!(!self.metadata_path(name).exists());
        assert!(!self.server_dir(name).exists());
    }

    fn assert_no_server_instances(&self, project: &Path) {
        let servers = project.join(".clickhouse/servers");
        if !servers.exists() {
            return;
        }
        for entry in std::fs::read_dir(servers).expect("read project server state") {
            let entry = entry.expect("read project server entry");
            let path = entry.path();
            assert_ne!(
                path.extension().and_then(|value| value.to_str()),
                Some("json"),
                "unexpected persisted metadata: {}",
                path.display()
            );
            assert!(
                !path.is_dir() || !path.join("data").exists(),
                "unexpected persisted data directory: {}",
                path.display()
            );
        }
    }

    fn assert_telemetry_disabled(&self) {
        assert!(
            !self.home.join(".clickhouse/telemetry.json").exists(),
            "telemetry touched isolated HOME"
        );
    }

    fn wait_for_pid_record(&self, pid: u32) {
        let deadline = Instant::now() + WAIT_TIMEOUT;
        let expected = pid.to_string();
        loop {
            let recorded = std::fs::read_to_string(&self.pid_file)
                .unwrap_or_default()
                .lines()
                .filter_map(|line| line.split_once('|').map(|(pid, _)| pid))
                .any(|pid| pid == expected);
            if recorded {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "fake ClickHouse PID {pid} was not recorded"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_for_global_server(&self, server: &RunningServer) {
        let project = canonical(&self.project);
        let args = strings(&["local", "--json", "server", "list", "--global"]);
        let deadline = Instant::now() + WAIT_TIMEOUT;
        loop {
            let output = self.run(&self.child, &args);
            assert_eq!(
                output.status.code(),
                Some(0),
                "global list stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let body: Value =
                serde_json::from_slice(&output.stdout).expect("parse global list JSON");
            let discovered = body["servers"].as_array().is_some_and(|servers| {
                servers.iter().any(|entry| {
                    entry["name"] == server.name
                        && entry["pid"] == server.pid
                        && entry["project"] == project
                })
            });
            if discovered {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "global discovery did not find {} in {}",
                server.name,
                project
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn server_dir(&self, name: &str) -> PathBuf {
        self.project.join(".clickhouse/servers").join(name)
    }

    fn data_dir(&self, name: &str) -> PathBuf {
        self.server_dir(name).join("data")
    }

    fn metadata_path(&self, name: &str) -> PathBuf {
        self.project
            .join(".clickhouse/servers")
            .join(format!("{name}.json"))
    }
}

#[derive(Clone)]
struct RunningServer {
    name: String,
    pid: u32,
    http_port: u16,
    tcp_port: u16,
    started_at: String,
}

#[derive(Clone, Copy)]
enum TestCwd {
    Root,
    Child,
}

enum ListedServer<'a> {
    Running(&'a RunningServer),
    Stopped(&'a str),
}

struct CommandCase {
    label: String,
    cwd: TestCwd,
    args: Vec<String>,
    expected: ExpectedOutput,
}

enum ExpectedOutput {
    JsonSuccess(Value),
    JsonFailure(Value),
    HumanSuccess(String),
    HumanFailure(String),
}

impl ExpectedOutput {
    fn assert(&self, label: &str, output: &Output) {
        match self {
            Self::JsonSuccess(expected) => {
                assert_eq!(
                    output.status.code(),
                    Some(0),
                    "{label} stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(
                    output.stderr.is_empty(),
                    "{label} wrote JSON success stderr"
                );
                assert_eq!(
                    serde_json::from_slice::<Value>(&output.stdout)
                        .unwrap_or_else(|error| panic!("{label} JSON stdout: {error}")),
                    *expected,
                    "{label} JSON stdout"
                );
            }
            Self::JsonFailure(expected) => {
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{label} stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(output.stdout.is_empty(), "{label} failure wrote stdout");
                assert_eq!(
                    serde_json::from_slice::<Value>(&output.stderr)
                        .unwrap_or_else(|error| panic!("{label} JSON stderr: {error}")),
                    *expected,
                    "{label} JSON stderr"
                );
            }
            Self::HumanSuccess(expected) => {
                assert_eq!(
                    output.status.code(),
                    Some(0),
                    "{label} stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(
                    output.stderr.is_empty(),
                    "{label} wrote human success stderr"
                );
                assert_eq!(String::from_utf8_lossy(&output.stdout), expected.as_str());
            }
            Self::HumanFailure(expected) => {
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{label} stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert!(
                    output.stdout.is_empty(),
                    "{label} human failure wrote stdout"
                );
                assert_eq!(String::from_utf8_lossy(&output.stderr), expected.as_str());
            }
        }
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn write_executable(path: &Path, contents: &str) {
    std::fs::write(path, contents).expect("write fake process tool");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("make fake process tool executable");
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

fn canonical(path: &Path) -> String {
    path.canonicalize()
        .expect("canonicalize project path")
        .display()
        .to_string()
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read persisted JSON"))
        .expect("parse persisted JSON")
}

fn process_is_alive(pid: u32) -> bool {
    i32::try_from(pid).is_ok_and(|pid| unsafe { libc::kill(pid, 0) == 0 })
}

fn wait_for_process(pid: u32, alive: bool) {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    loop {
        if process_is_alive(pid) == alive {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "PID {pid} did not become {}",
            if alive { "running" } else { "stopped" }
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn error(code: &str, message: impl Into<String>, command: Option<&str>) -> Value {
    let mut detail = serde_json::Map::new();
    detail.insert("code".into(), code.into());
    let message: String = message.into();
    detail.insert("message".into(), message.into());
    if let Some(command) = command {
        detail.insert("command".into(), command.into());
    }
    json!({ "error": detail })
}

fn scope_guidance(include_global_stop: bool) -> Value {
    let mut guidance = vec![
        json!({
            "action": "return_to_project_root",
            "message": "Change to the local project root where the server was started",
            "command": "cd <project-root>"
        }),
        json!({
            "action": "list_project_servers",
            "message": "List servers after returning to that exact project",
            "command": "clickhousectl local server list"
        }),
        json!({
            "action": "list_global_servers",
            "message": "Locate running ClickHouse servers across projects",
            "command": "clickhousectl local server list --global"
        }),
    ];
    if include_global_stop {
        guidance.push(json!({
            "action": "stop_global_project_server",
            "message": "After confirming the project, stop the server with explicit global project selection",
            "command": "clickhousectl local server stop <name> --global --project <project-root>"
        }));
    }
    Value::Array(guidance)
}

fn project_not_found(project: &Path, name: &str, stop: bool) -> Value {
    json!({
        "error": {
            "code": "server_not_found",
            "message": format!("Server '{name}' was not found in the current project"),
            "project_scope": {
                "kind": "exact_current_project",
                "path": canonical(project),
                "parent_projects_searched": false
            },
            "server": { "name": name },
            "guidance": scope_guidance(stop)
        }
    })
}

fn human_project_not_found(project: &Path, name: &str, command: &str) -> String {
    let global_stop = if command == "stop" {
        "; after confirming the project, use `clickhousectl local server stop <name> --global --project <project-root>`"
    } else {
        ""
    };
    format!(
        "Error: Server '{name}' was not found in project '{}'.\n\
         Project-local server {command} uses the exact current working directory; parent `.clickhouse` directories are not searched.\n\
         Return to the local project root where the server was started and run `clickhousectl local server list`; use `clickhousectl local server list --global` to locate running servers in other projects{global_stop}.\n",
        canonical(project)
    )
}

fn empty_server_list(project: &Path) -> Value {
    json!({
        "servers": [],
        "total_servers": 0,
        "total_running_servers": 0,
        "project_scope": {
            "kind": "exact_current_project",
            "path": canonical(project),
            "parent_projects_searched": false
        },
        "guidance": scope_guidance(false)
    })
}

fn server_list(project: &Path, servers: &[ListedServer<'_>]) -> Value {
    if servers.is_empty() {
        return empty_server_list(project);
    }
    let entries: Vec<_> = servers
        .iter()
        .map(|server| match server {
            ListedServer::Running(server) => json!({
                "name": server.name,
                "running": true,
                "pid": server.pid,
                "version": VERSION,
                "http_port": server.http_port,
                "tcp_port": server.tcp_port,
                "engine": "clickhouse"
            }),
            ListedServer::Stopped(name) => json!({
                "name": name,
                "running": false,
                "engine": "clickhouse"
            }),
        })
        .collect();
    let running = servers
        .iter()
        .filter(|server| matches!(server, ListedServer::Running(_)))
        .count();
    json!({
        "servers": entries,
        "total_servers": servers.len(),
        "total_running_servers": running,
        "project_scope": {
            "kind": "exact_current_project",
            "path": canonical(project),
            "parent_projects_searched": false
        }
    })
}

fn fresh_project(fixture: &Fixture) {
    fixture.run_cases(&[
        CommandCase {
            label: "fresh list".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "list"]),
            expected: ExpectedOutput::JsonSuccess(empty_server_list(&fixture.project)),
        },
        CommandCase {
            label: "fresh omitted stop".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "stop"]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "stopped": false,
                "selection": "implicit",
                "reason": "no_clickhouse_servers"
            })),
        },
        CommandCase {
            label: "fresh omitted stop human output".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "server", "stop"]),
            expected: ExpectedOutput::HumanSuccess(
                "No ClickHouse servers found; nothing to stop\n".into(),
            ),
        },
        CommandCase {
            label: "fresh omitted remove".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "remove"]),
            expected: ExpectedOutput::JsonFailure(error(
                "server_selection_required",
                "No server name was provided and the default ClickHouse server does not exist \
                 (custom ClickHouse servers available: 0); no server was removed. Inspect them with \
                 `clickhousectl local server list`; to remove one, pass its name with \
                 `clickhousectl local server remove <name>`.",
                Some("clickhousectl local server list"),
            )),
        },
        CommandCase {
            label: "fresh explicit unknown stop".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "stop", "missing"]),
            expected: ExpectedOutput::JsonFailure(project_not_found(
                &fixture.project,
                "missing",
                true,
            )),
        },
        CommandCase {
            label: "fresh explicit unknown remove".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "server", "remove", "missing"]),
            expected: ExpectedOutput::HumanFailure(human_project_not_found(
                &fixture.project,
                "missing",
                "remove",
            )),
        },
    ]);
    fixture.assert_no_server_instances(&fixture.project);
}

fn default_lifecycle(fixture: &Fixture) {
    let server = fixture.start(None);
    fixture.write_marker("default", "default-data");
    let generated = fixture.start_generated();
    fixture.run_cases(&[CommandCase {
        label: "generated server explicit stop".into(),
        cwd: TestCwd::Root,
        args: vec![
            "local".into(),
            "--json".into(),
            "server".into(),
            "stop".into(),
            generated.name.clone(),
        ],
        expected: ExpectedOutput::JsonSuccess(json!({
            "name": generated.name,
            "already_stopped": false,
            "selection": "explicit"
        })),
    }]);
    wait_for_process(generated.pid, false);
    fixture.assert_stopped_metadata(&generated);
    fixture.run_cases(&[CommandCase {
        label: "generated server explicit remove".into(),
        cwd: TestCwd::Root,
        args: vec![
            "local".into(),
            "--json".into(),
            "server".into(),
            "remove".into(),
            generated.name.clone(),
        ],
        expected: ExpectedOutput::JsonSuccess(json!({
            "name": generated.name,
            "selection": "explicit"
        })),
    }]);
    fixture.assert_removed(&generated.name);
    fixture.run_cases(&[
        CommandCase {
            label: "repeated default start".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "start", "default", "--no-wait"]),
            expected: ExpectedOutput::JsonFailure(error(
                "server_running",
                "Server 'default' is already running",
                Some("clickhousectl local server list"),
            )),
        },
        CommandCase {
            label: "running default list".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "list"]),
            expected: ExpectedOutput::JsonSuccess(server_list(
                &fixture.project,
                &[ListedServer::Running(&server)],
            )),
        },
        CommandCase {
            label: "running default remove".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "server", "remove"]),
            expected: ExpectedOutput::HumanFailure(
                "Error: Server 'default' is running; stop it first with `clickhousectl local server stop default`\n"
                    .into(),
            ),
        },
    ]);
    fixture.assert_running_metadata(&server);
    fixture.assert_marker("default", "default-data");

    fixture.run_cases(&[CommandCase {
        label: "omitted default stop".into(),
        cwd: TestCwd::Root,
        args: strings(&["local", "--json", "server", "stop"]),
        expected: ExpectedOutput::JsonSuccess(json!({
            "name": "default",
            "already_stopped": false,
            "selection": "implicit"
        })),
    }]);
    wait_for_process(server.pid, false);
    fixture.assert_stopped_metadata(&server);
    fixture.assert_marker("default", "default-data");

    fixture.run_cases(&[
        CommandCase {
            label: "repeated omitted default stop".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "stop"]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "name": "default",
                "already_stopped": true,
                "selection": "implicit"
            })),
        },
        CommandCase {
            label: "explicit stopped default stop".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "stop", "--name", "default"]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "name": "default",
                "already_stopped": true,
                "selection": "explicit"
            })),
        },
        CommandCase {
            label: "omitted stopped default remove".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "remove"]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "name": "default",
                "selection": "implicit"
            })),
        },
        CommandCase {
            label: "post-default-remove list".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "list"]),
            expected: ExpectedOutput::JsonSuccess(empty_server_list(&fixture.project)),
        },
    ]);
    fixture.assert_removed("default");
}

fn custom_name_lifecycle(fixture: &Fixture) {
    let first = fixture.start(Some("dev"));
    fixture.write_marker("dev", "custom-data");
    fixture.assert_list(TestCwd::Root, &[ListedServer::Running(&first)]);

    fixture.run_cases(&[CommandCase {
        label: "sole custom omitted stop".into(),
        cwd: TestCwd::Root,
        args: strings(&["local", "--json", "server", "stop"]),
        expected: ExpectedOutput::JsonSuccess(json!({
            "name": "dev",
            "already_stopped": false,
            "selection": "implicit"
        })),
    }]);
    wait_for_process(first.pid, false);
    fixture.assert_stopped_metadata(&first);
    fixture.assert_marker("dev", "custom-data");
    fixture.assert_list(TestCwd::Root, &[ListedServer::Stopped("dev")]);

    let second = fixture.start(Some("dev"));
    fixture.assert_marker("dev", "custom-data");
    fixture.run_cases(&[
        CommandCase {
            label: "sole custom omitted remove".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "remove"]),
            expected: ExpectedOutput::JsonFailure(error(
                "server_selection_required",
                "No server name was provided and the default ClickHouse server does not exist \
                 (custom ClickHouse servers available: 1); no server was removed. Inspect them with \
                 `clickhousectl local server list`; to remove one, pass its name with \
                 `clickhousectl local server remove <name>`.",
                Some("clickhousectl local server list"),
            )),
        },
        CommandCase {
            label: "running custom explicit remove".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "remove", "dev"]),
            expected: ExpectedOutput::JsonFailure(error(
                "server_running",
                "Server 'dev' is running; stop it first with `clickhousectl local server stop dev`",
                Some("clickhousectl local server stop dev"),
            )),
        },
        CommandCase {
            label: "custom explicit flag stop".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "stop", "--name", "dev"]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "name": "dev",
                "already_stopped": false,
                "selection": "explicit"
            })),
        },
    ]);
    wait_for_process(second.pid, false);
    fixture.assert_stopped_metadata(&second);
    fixture.assert_marker("dev", "custom-data");

    fixture.run_cases(&[
        CommandCase {
            label: "repeated custom explicit stop".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "stop", "dev"]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "name": "dev",
                "already_stopped": true,
                "selection": "explicit"
            })),
        },
        CommandCase {
            label: "custom explicit remove".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "remove", "--name", "dev"]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "name": "dev",
                "selection": "explicit"
            })),
        },
    ]);
    fixture.assert_removed("dev");
    fixture.assert_list(TestCwd::Root, &[]);
}

fn omitted_and_explicit_selection(fixture: &Fixture) {
    let alpha = fixture.start(Some("alpha"));
    let beta = fixture.start(Some("beta"));
    let default = fixture.start(None);
    fixture.write_marker("alpha", "alpha-data");
    fixture.write_marker("beta", "beta-data");
    fixture.write_marker("default", "default-data");
    fixture.assert_list(
        TestCwd::Root,
        &[
            ListedServer::Running(&alpha),
            ListedServer::Running(&beta),
            ListedServer::Running(&default),
        ],
    );

    fixture.run_cases(&[CommandCase {
        label: "many servers omitted stop prefers default".into(),
        cwd: TestCwd::Root,
        args: strings(&["local", "--json", "server", "stop"]),
        expected: ExpectedOutput::JsonSuccess(json!({
            "name": "default",
            "already_stopped": false,
            "selection": "implicit"
        })),
    }]);
    wait_for_process(default.pid, false);
    fixture.assert_stopped_metadata(&default);
    fixture.assert_running_metadata(&alpha);
    fixture.assert_running_metadata(&beta);
    fixture.assert_list(
        TestCwd::Root,
        &[
            ListedServer::Running(&alpha),
            ListedServer::Running(&beta),
            ListedServer::Stopped("default"),
        ],
    );

    fixture.run_cases(&[CommandCase {
        label: "many servers omitted remove prefers default".into(),
        cwd: TestCwd::Root,
        args: strings(&["local", "--json", "server", "remove"]),
        expected: ExpectedOutput::JsonSuccess(json!({
            "name": "default",
            "selection": "implicit"
        })),
    }]);
    fixture.assert_removed("default");
    fixture.assert_running_metadata(&alpha);
    fixture.assert_running_metadata(&beta);

    let ambiguous_stop = error(
        "server_selection_required",
        "No server name was provided and multiple non-default ClickHouse servers exist \
         (available: 2). Pass a name with `clickhousectl local server stop <name>`, or stop every \
         server with `clickhousectl local server stop-all`.",
        Some("clickhousectl local server list"),
    );
    fixture.run_cases(&[
        CommandCase {
            label: "many custom-only omitted stop".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "stop"]),
            expected: ExpectedOutput::JsonFailure(ambiguous_stop.clone()),
        },
        CommandCase {
            label: "many custom-only omitted remove".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "remove"]),
            expected: ExpectedOutput::JsonFailure(error(
                "server_selection_required",
                "No server name was provided and the default ClickHouse server does not exist \
                 (custom ClickHouse servers available: 2); no server was removed. Inspect them with \
                 `clickhousectl local server list`; to remove one, pass its name with \
                 `clickhousectl local server remove <name>`.",
                Some("clickhousectl local server list"),
            )),
        },
        CommandCase {
            label: "many custom-only explicit positional stop".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "stop", "alpha"]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "name": "alpha",
                "already_stopped": false,
                "selection": "explicit"
            })),
        },
        CommandCase {
            label: "many custom-only explicit flag stop".into(),
            cwd: TestCwd::Root,
            args: strings(&[
                "local", "--json", "server", "stop", "--name", "beta",
            ]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "name": "beta",
                "already_stopped": false,
                "selection": "explicit"
            })),
        },
    ]);
    wait_for_process(alpha.pid, false);
    wait_for_process(beta.pid, false);
    fixture.assert_stopped_metadata(&alpha);
    fixture.assert_stopped_metadata(&beta);
    fixture.assert_marker("alpha", "alpha-data");
    fixture.assert_marker("beta", "beta-data");

    fixture.run_cases(&[
        CommandCase {
            label: "many stopped custom-only omitted stop".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "stop"]),
            expected: ExpectedOutput::JsonFailure(ambiguous_stop),
        },
        CommandCase {
            label: "explicit positional remove alpha".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "remove", "alpha"]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "name": "alpha",
                "selection": "explicit"
            })),
        },
        CommandCase {
            label: "explicit flag remove beta".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "remove", "--name", "beta"]),
            expected: ExpectedOutput::JsonSuccess(json!({
                "name": "beta",
                "selection": "explicit"
            })),
        },
    ]);
    fixture.assert_removed("alpha");
    fixture.assert_removed("beta");
    fixture.assert_list(TestCwd::Root, &[]);
}

fn exact_cwd_and_explicit_project(fixture: &Fixture) {
    let scoped = fixture.start(Some("scoped"));
    fixture.write_marker("scoped", "scoped-data");
    fixture.run_cases(&[
        CommandCase {
            label: "child exact-CWD list".into(),
            cwd: TestCwd::Child,
            args: strings(&["local", "--json", "server", "list"]),
            expected: ExpectedOutput::JsonSuccess(empty_server_list(&fixture.child)),
        },
        CommandCase {
            label: "child exact-CWD stop".into(),
            cwd: TestCwd::Child,
            args: strings(&["local", "--json", "server", "stop", "scoped"]),
            expected: ExpectedOutput::JsonFailure(project_not_found(
                &fixture.child,
                "scoped",
                true,
            )),
        },
        CommandCase {
            label: "child exact-CWD remove".into(),
            cwd: TestCwd::Child,
            args: strings(&["local", "server", "remove", "scoped"]),
            expected: ExpectedOutput::HumanFailure(human_project_not_found(
                &fixture.child,
                "scoped",
                "remove",
            )),
        },
    ]);
    fixture.assert_no_server_instances(&fixture.child);
    fixture.assert_running_metadata(&scoped);
    fixture.assert_marker("scoped", "scoped-data");

    fixture.wait_for_global_server(&scoped);
    let project = canonical(&fixture.project);
    fixture.run_cases(&[CommandCase {
        label: "explicit global project stop from child".into(),
        cwd: TestCwd::Child,
        args: vec![
            "local".into(),
            "--json".into(),
            "server".into(),
            "stop".into(),
            "scoped".into(),
            "--global".into(),
            "--project".into(),
            project,
        ],
        expected: ExpectedOutput::JsonSuccess(json!({
            "name": "scoped",
            "already_stopped": false,
            "selection": "explicit"
        })),
    }]);
    wait_for_process(scoped.pid, false);

    // The global stop kills by discovered PID. A root list performs the normal
    // persisted transition to stopped metadata before removal.
    fixture.assert_list(TestCwd::Root, &[ListedServer::Stopped("scoped")]);
    fixture.assert_stopped_metadata(&scoped);
    fixture.assert_marker("scoped", "scoped-data");
    fixture.run_cases(&[CommandCase {
        label: "root remove after explicit project stop".into(),
        cwd: TestCwd::Root,
        args: strings(&["local", "--json", "server", "remove", "scoped"]),
        expected: ExpectedOutput::JsonSuccess(json!({
            "name": "scoped",
            "selection": "explicit"
        })),
    }]);
    fixture.assert_removed("scoped");
    fixture.assert_no_server_instances(&fixture.child);
}

struct Scenario {
    name: &'static str,
    run: fn(&Fixture),
}

#[test]
fn local_server_state_machine() {
    let scenarios = [
        Scenario {
            name: "fresh",
            run: fresh_project,
        },
        Scenario {
            name: "default-lifecycle",
            run: default_lifecycle,
        },
        Scenario {
            name: "custom-name-lifecycle",
            run: custom_name_lifecycle,
        },
        Scenario {
            name: "selection-matrix",
            run: omitted_and_explicit_selection,
        },
        Scenario {
            name: "exact-cwd",
            run: exact_cwd_and_explicit_project,
        },
    ];

    for scenario in scenarios {
        let fixture = Fixture::new(scenario.name);
        (scenario.run)(&fixture);
        fixture.assert_no_server_instances(&fixture.project);
        fixture.assert_telemetry_disabled();
        assert!(
            !fixture.home.join(".clickhouse/servers").exists(),
            "{} wrote project server state under HOME",
            scenario.name
        );

        let recorded: BTreeSet<_> = std::fs::read_to_string(&fixture.pid_file)
            .unwrap_or_default()
            .lines()
            .filter_map(|line| {
                line.split_once('|')
                    .and_then(|(pid, _)| pid.parse::<u32>().ok())
            })
            .collect();
        assert!(
            recorded.iter().all(|pid| !process_is_alive(*pid)),
            "{} left a fake ClickHouse process running",
            scenario.name
        );
    }
}
