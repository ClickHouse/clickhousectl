//! End-to-end coverage for the project-local ClickHouse server state machine.
//!
//! The fake servers are compiled stand-ins (see `support/fake_clickhouse.rs`)
//! that native process discovery finds like real ones. Discovery also sees the
//! other scenarios' fixtures running in parallel, so every global lookup here
//! is filtered by this fixture's project.

#[path = "support/fake_clickhouse.rs"]
mod fake_clickhouse;

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const VERSION: &str = "latest";
const WAIT_TIMEOUT: Duration = Duration::from_secs(5);

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

struct Fixture {
    // Drop the process guard before removing directories used as process CWDs.
    _processes: fake_clickhouse::ProcessGuard,
    _root: tempfile::TempDir,
    home: PathBuf,
    project: PathBuf,
    child: PathBuf,
    pid_file: PathBuf,
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

        std::fs::create_dir_all(&home).expect("create isolated HOME");
        std::fs::create_dir_all(&child).expect("create project child directory");
        fake_clickhouse::install(&binary);
        std::fs::write(home.join(".clickhouse/default"), VERSION)
            .expect("select fake latest version");

        // No /usr/local/bin or /opt/homebrew/bin, so a developer's `docker`
        // is unreachable and recovery never takes its Postgres leg.
        let path = "/usr/bin:/bin:/usr/sbin:/sbin".to_string();

        Self {
            _processes: fake_clickhouse::ProcessGuard {
                pid_file: pid_file.clone(),
            },
            _root: root,
            home,
            project,
            child,
            pid_file,
            path,
        }
    }

    fn command(&self, cwd: &Path) -> Command {
        let mut command = Command::new(clickhousectl_binary());
        command
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            // No update-check network request.
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
            .env("HOME", &self.home)
            .env("PATH", &self.path)
            .env("FAKE_CLICKHOUSE_PID_FILE", &self.pid_file)
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

/// JSON is compared field by field with every `message` reduced to "is a
/// string"; human text by the values and commands it must name. Wording is
/// unit-tested beside the output types.
enum ExpectedOutput {
    JsonSuccess(Value),
    JsonFailure(Value),
    HumanSuccess(Vec<String>),
    HumanFailure(Vec<String>),
}

const ANY_MESSAGE: &str = "<message>";

fn without_messages(mut value: Value) -> Value {
    fn walk(value: &mut Value) {
        match value {
            Value::Object(map) => {
                for (key, child) in map.iter_mut() {
                    if key == "message" && child.is_string() {
                        *child = ANY_MESSAGE.into();
                    } else {
                        walk(child);
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut value);
    value
}

fn assert_names_all(label: &str, text: &str, fragments: &[String]) {
    for fragment in fragments {
        assert!(
            text.contains(fragment.as_str()),
            "{label} missing {fragment:?}: {text}"
        );
    }
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
                    without_messages(
                        serde_json::from_slice::<Value>(&output.stdout)
                            .unwrap_or_else(|error| panic!("{label} JSON stdout: {error}"))
                    ),
                    without_messages(expected.clone()),
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
                    without_messages(
                        serde_json::from_slice::<Value>(&output.stderr)
                            .unwrap_or_else(|error| panic!("{label} JSON stderr: {error}"))
                    ),
                    without_messages(expected.clone()),
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
                assert_names_all(label, &String::from_utf8_lossy(&output.stdout), expected);
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
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(stderr.starts_with("Error: "), "{label}: {stderr}");
                assert_names_all(label, &stderr, expected);
            }
        }
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
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

/// An expected error envelope; its `message` is compared only as a string.
fn error(code: &str, command: Option<&str>) -> Value {
    let mut detail = serde_json::Map::new();
    detail.insert("code".into(), code.into());
    detail.insert("message".into(), ANY_MESSAGE.into());
    if let Some(command) = command {
        detail.insert("command".into(), command.into());
    }
    json!({ "error": detail })
}

/// Whether `project` holds a `.clickhouse/` marker, which decides its scope
/// kind and guidance. Read when the expectation is built.
fn is_marked(project: &Path) -> bool {
    project.join(".clickhouse").is_dir()
}

fn project_scope(project: &Path) -> Value {
    json!({
        "kind": if is_marked(project) { "project_root" } else { "current_directory" },
        "path": canonical(project)
    })
}

/// Changing directory is advised only when no project was found.
fn scope_guidance(project: &Path, include_global_stop: bool) -> Value {
    let mut guidance = Vec::new();
    if !is_marked(project) {
        guidance.push(json!({
            "action": "return_to_project_root",
            "message": ANY_MESSAGE,
            "command": "cd <project-root>"
        }));
    }
    guidance.push(json!({
        "action": "list_project_servers",
        "message": ANY_MESSAGE,
        "command": "clickhousectl local clickhouse list"
    }));
    guidance.push(json!({
        "action": "list_global_servers",
        "message": ANY_MESSAGE,
        "command": "clickhousectl local clickhouse list --global"
    }));
    if include_global_stop {
        guidance.push(json!({
            "action": "stop_global_project_server",
            "message": ANY_MESSAGE,
            "command": "clickhousectl local clickhouse stop <name> --global --project <project-root>"
        }));
    }
    Value::Array(guidance)
}

fn project_not_found(project: &Path, name: &str, stop: bool) -> Value {
    json!({
        "error": {
            "code": "server_not_found",
            "message": ANY_MESSAGE,
            "project_scope": project_scope(project),
            "server": { "name": name },
            "guidance": scope_guidance(project, stop)
        }
    })
}

fn human_project_not_found(project: &Path, name: &str, command: &str) -> Vec<String> {
    let mut fragments = vec![
        format!("'{name}'"),
        format!("'{}'", canonical(project)),
        "`clickhousectl local clickhouse list`".into(),
        "`clickhousectl local clickhouse list --global`".into(),
    ];
    if command == "stop" {
        fragments.push(
            "`clickhousectl local clickhouse stop <name> --global --project <project-root>`".into(),
        );
    }
    fragments
}

/// Inside a project an empty list only points at other projects.
fn empty_server_list(project: &Path) -> Value {
    let guidance = if is_marked(project) {
        json!([{
            "action": "list_global_servers",
            "message": ANY_MESSAGE,
            "command": "clickhousectl local clickhouse list --global"
        }])
    } else {
        scope_guidance(project, false)
    };
    json!({
        "servers": [],
        "total_servers": 0,
        "total_running_servers": 0,
        "project_scope": project_scope(project),
        "guidance": guidance
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
        "project_scope": project_scope(project)
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
            // The fresh list above created no `.clickhouse/` (#1077), so the
            // project still has no state and the no-op explains its scope.
            expected: ExpectedOutput::JsonSuccess(json!({
                "stopped": false,
                "selection": "implicit",
                "reason": "no_clickhouse_servers",
                "project_scope": project_scope(&fixture.project),
                "guidance": scope_guidance(&fixture.project, true)
            })),
        },
        CommandCase {
            label: "fresh omitted stop human output".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "server", "stop"]),
            expected: ExpectedOutput::HumanSuccess(vec![
                format!("'{}'", canonical(&fixture.project)),
                "`clickhousectl local clickhouse list`".into(),
                "`clickhousectl local clickhouse list --global`".into(),
            ]),
        },
        CommandCase {
            label: "fresh omitted remove".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "remove"]),
            expected: ExpectedOutput::JsonFailure(json!({
                "error": {
                    "code": "server_selection_required",
                    "message": ANY_MESSAGE,
                    "project_scope": project_scope(&fixture.project),
                    "guidance": scope_guidance(&fixture.project, false)
                }
            })),
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
                Some("clickhousectl local status"),
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
            expected: ExpectedOutput::HumanFailure(vec![
                "'default'".into(),
                "`clickhousectl local clickhouse stop default`".into(),
            ]),
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
                Some("clickhousectl local clickhouse list"),
            )),
        },
        CommandCase {
            label: "running custom explicit remove".into(),
            cwd: TestCwd::Root,
            args: strings(&["local", "--json", "server", "remove", "dev"]),
            expected: ExpectedOutput::JsonFailure(error(
                "server_running",
                Some("clickhousectl local clickhouse stop dev"),
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
        Some("clickhousectl local clickhouse list"),
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
                Some("clickhousectl local clickhouse list"),
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
            args: strings(&["local", "--json", "server", "stop", "--name", "beta"]),
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

/// A subdirectory acts on the enclosing project; `--global --project` still
/// selects a project explicitly from anywhere.
fn child_cwd_and_explicit_project(fixture: &Fixture) {
    let scoped = fixture.start(Some("scoped"));
    fixture.write_marker("scoped", "scoped-data");
    fixture.run_cases(&[
        CommandCase {
            label: "child list".into(),
            cwd: TestCwd::Child,
            args: strings(&["local", "--json", "server", "list"]),
            expected: ExpectedOutput::JsonSuccess(server_list(
                &fixture.project,
                &[ListedServer::Running(&scoped)],
            )),
        },
        CommandCase {
            label: "child unknown stop".into(),
            cwd: TestCwd::Child,
            args: strings(&["local", "--json", "server", "stop", "missing"]),
            expected: ExpectedOutput::JsonFailure(project_not_found(
                &fixture.project,
                "missing",
                true,
            )),
        },
        CommandCase {
            label: "child unknown remove".into(),
            cwd: TestCwd::Child,
            args: strings(&["local", "server", "remove", "missing"]),
            expected: ExpectedOutput::HumanFailure(human_project_not_found(
                &fixture.project,
                "missing",
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

/// Run one scenario in its own fixture, then check the invariants every
/// scenario must leave behind.
fn run_scenario(name: &str, run: fn(&Fixture)) {
    let fixture = Fixture::new(name);
    run(&fixture);
    fixture.assert_no_server_instances(&fixture.project);
    fixture.assert_telemetry_disabled();
    assert!(
        !fixture.home.join(".clickhouse/servers").exists(),
        "{name} wrote project server state under HOME"
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
        "{name} left a fake ClickHouse process running"
    );
}

#[test]
fn state_machine_fresh_project() {
    run_scenario("fresh", fresh_project);
}

#[test]
fn state_machine_default_lifecycle() {
    run_scenario("default-lifecycle", default_lifecycle);
}

#[test]
fn state_machine_custom_name_lifecycle() {
    run_scenario("custom-name-lifecycle", custom_name_lifecycle);
}

#[test]
fn state_machine_selection_matrix() {
    run_scenario("selection-matrix", omitted_and_explicit_selection);
}

#[test]
fn state_machine_child_cwd_and_explicit_project() {
    run_scenario("child-cwd", child_cwd_and_explicit_project);
}
