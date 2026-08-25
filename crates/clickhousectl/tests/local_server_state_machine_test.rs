//! Table-driven subprocess coverage for the local ClickHouse server state machine.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const VERSION: &str = "latest";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

#[derive(Debug)]
enum ExpectedName {
    Exact(String),
    Generated,
}

#[derive(Debug)]
struct ListEntry {
    name: String,
    running: bool,
}

#[derive(Debug)]
enum ExpectedOutput {
    Start(ExpectedName),
    List(Vec<ListEntry>),
    Stop {
        name: String,
        already_stopped: bool,
    },
    StopNoop,
    Remove(String),
    Error {
        code: &'static str,
        command: &'static str,
        scoped: bool,
        message_fragments: Vec<String>,
    },
}

#[derive(Debug)]
struct Step {
    label: &'static str,
    cwd: PathBuf,
    args: Vec<String>,
    expected: ExpectedOutput,
}

impl Step {
    fn new(label: &'static str, cwd: &Path, args: &[&str], expected: ExpectedOutput) -> Self {
        Self {
            label,
            cwd: cwd.to_path_buf(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            expected,
        }
    }
}

#[derive(Debug)]
struct ServerState {
    pid: u32,
    running: bool,
    http_port: u16,
    tcp_port: u16,
}

struct Harness {
    _temp: tempfile::TempDir,
    home: PathBuf,
    projects: BTreeMap<PathBuf, BTreeMap<String, ServerState>>,
    active_pids: BTreeSet<u32>,
}

impl Harness {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("create state-machine tempdir");
        let home = temp.path().join("home");
        std::fs::create_dir(&home).expect("create temporary HOME");
        install_fake_clickhouse(&home);
        seed_update_cache(&home);
        Self {
            _temp: temp,
            home,
            projects: BTreeMap::new(),
            active_pids: BTreeSet::new(),
        }
    }

    fn project(&mut self, name: &str) -> PathBuf {
        let project = self._temp.path().join(name);
        std::fs::create_dir(&project).expect("create temporary project");
        let project = project.canonicalize().expect("canonicalize project");
        self.projects.insert(project.clone(), BTreeMap::new());
        project
    }

    fn child_project(&mut self, parent: &Path, name: &str) -> PathBuf {
        let child = parent.join(name);
        std::fs::create_dir(&child).expect("create child working directory");
        let child = child.canonicalize().expect("canonicalize child directory");
        self.projects.insert(child.clone(), BTreeMap::new());
        child
    }

    fn run_steps(&mut self, steps: Vec<Step>) -> Vec<Option<String>> {
        steps.into_iter().map(|step| self.run_step(step)).collect()
    }

    fn run_step(&mut self, step: Step) -> Option<String> {
        let project = step.cwd.canonicalize().expect("canonicalize command cwd");
        assert!(
            self.projects.contains_key(&project),
            "{}: unregistered project {}",
            step.label,
            project.display()
        );
        let output = self.run_command(&step);
        let expected_error = matches!(&step.expected, ExpectedOutput::Error { .. });
        assert_eq!(
            output.status.code(),
            Some(if expected_error { 1 } else { 0 }),
            "{}: unexpected status\nstdout: {}\nstderr: {}",
            step.label,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let selected_name = match step.expected {
            ExpectedOutput::Error {
                code,
                command,
                scoped,
                message_fragments,
            } => {
                assert!(
                    output.stdout.is_empty(),
                    "{}: errors must not write stdout: {}",
                    step.label,
                    String::from_utf8_lossy(&output.stdout)
                );
                let body: Value = serde_json::from_slice(&output.stderr)
                    .unwrap_or_else(|error| panic!("{}: parse error JSON: {error}", step.label));
                let error = &body["error"];
                assert_eq!(error["code"], code, "{}", step.label);
                assert_eq!(error["command"], command, "{}", step.label);
                assert_eq!(
                    error.get("project").and_then(Value::as_str),
                    scoped.then(|| project.to_str().expect("UTF-8 project path")),
                    "{}",
                    step.label
                );
                assert!(error.get("mode").is_none(), "{}", step.label);
                let message = error["message"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{}: error message is a string", step.label));
                for fragment in message_fragments {
                    assert!(
                        message.contains(&fragment),
                        "{}: expected {fragment:?} in {message:?}",
                        step.label
                    );
                }
                assert!(!command.contains(['\n', '\r']), "{}", step.label);
                None
            }
            expected => {
                assert!(
                    output.stderr.is_empty(),
                    "{}: success wrote stderr: {}",
                    step.label,
                    String::from_utf8_lossy(&output.stderr)
                );
                let body: Value = serde_json::from_slice(&output.stdout)
                    .unwrap_or_else(|error| panic!("{}: parse success JSON: {error}", step.label));
                self.assert_success(step.label, &project, expected, &body)
            }
        };

        self.assert_all_project_state(step.label);
        assert!(
            !self.home.join(".clickhouse/telemetry.json").exists(),
            "{}: telemetry must remain disabled",
            step.label
        );
        selected_name
    }

    fn run_command(&self, step: &Step) -> Output {
        Command::new(clickhousectl_binary())
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &self.home)
            .env("XDG_CACHE_HOME", self.home.join(".cache"))
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .current_dir(&step.cwd)
            .args(["local", "--json", "server"])
            .args(&step.args)
            .output()
            .unwrap_or_else(|error| panic!("{}: run clickhousectl: {error}", step.label))
    }

    fn assert_success(
        &mut self,
        label: &str,
        project: &Path,
        expected: ExpectedOutput,
        body: &Value,
    ) -> Option<String> {
        match expected {
            ExpectedOutput::Start(expected_name) => {
                let pid = u32::try_from(
                    body["pid"]
                        .as_u64()
                        .unwrap_or_else(|| panic!("{label}: start PID is an integer")),
                )
                .expect("start PID fits u32");
                assert_ne!(pid, 0, "{label}");
                self.active_pids.insert(pid);
                let name = body["name"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{label}: start name is a string"));
                match expected_name {
                    ExpectedName::Exact(expected) => assert_eq!(name, expected, "{label}"),
                    ExpectedName::Generated => {
                        assert_ne!(name, "default", "{label}");
                        assert!(name.contains('-'), "{label}: generated name was {name:?}");
                    }
                }
                let http_port = u16::try_from(
                    body["http_port"]
                        .as_u64()
                        .unwrap_or_else(|| panic!("{label}: HTTP port is an integer")),
                )
                .expect("HTTP port fits u16");
                let tcp_port = u16::try_from(
                    body["tcp_port"]
                        .as_u64()
                        .unwrap_or_else(|| panic!("{label}: TCP port is an integer")),
                )
                .expect("TCP port fits u16");
                assert_ne!(http_port, 0, "{label}");
                assert_ne!(tcp_port, 0, "{label}");
                assert_eq!(body["version"], VERSION, "{label}");
                assert_eq!(body.as_object().expect("start object").len(), 5, "{label}");
                self.projects
                    .get_mut(project)
                    .expect("registered project")
                    .insert(
                        name.to_string(),
                        ServerState {
                            pid,
                            running: true,
                            http_port,
                            tcp_port,
                        },
                    );
                Some(name.to_string())
            }
            ExpectedOutput::List(expected) => {
                let servers = body["servers"]
                    .as_array()
                    .unwrap_or_else(|| panic!("{label}: list servers is an array"));
                assert_eq!(servers.len(), expected.len(), "{label}");
                assert_eq!(body["total_servers"], expected.len(), "{label}");
                assert_eq!(
                    body["total_running_servers"],
                    expected.iter().filter(|server| server.running).count(),
                    "{label}"
                );
                for (actual, expected) in servers.iter().zip(expected) {
                    assert_eq!(actual["name"], expected.name, "{label}");
                    assert_eq!(actual["running"], expected.running, "{label}");
                    assert_eq!(actual["engine"], "clickhouse", "{label}");
                    assert!(actual.get("project").is_none(), "{label}");
                    assert!(actual.get("container_id").is_none(), "{label}");
                    if expected.running {
                        let state = &self.projects[project][&expected.name];
                        assert_eq!(actual["pid"], state.pid, "{label}");
                        assert_eq!(actual["version"], VERSION, "{label}");
                        assert_eq!(actual["http_port"], state.http_port, "{label}");
                        assert_eq!(actual["tcp_port"], state.tcp_port, "{label}");
                    } else {
                        for field in ["pid", "version", "http_port", "tcp_port"] {
                            assert!(actual.get(field).is_none(), "{label}: found {field}");
                        }
                    }
                }
                None
            }
            ExpectedOutput::Stop {
                name,
                already_stopped,
            } => {
                assert_eq!(body["name"], name, "{label}");
                assert_eq!(body["already_stopped"], already_stopped, "{label}");
                assert_eq!(body.as_object().expect("stop object").len(), 2, "{label}");
                let state = self
                    .projects
                    .get_mut(project)
                    .expect("registered project")
                    .get_mut(&name)
                    .unwrap_or_else(|| panic!("{label}: modeled server {name} exists"));
                assert_eq!(state.running, !already_stopped, "{label}");
                state.running = false;
                self.active_pids.remove(&state.pid);
                assert!(
                    !process_alive(state.pid),
                    "{label}: PID {} is alive",
                    state.pid
                );
                Some(name)
            }
            ExpectedOutput::StopNoop => {
                assert_eq!(body["stopped"], false, "{label}");
                assert_eq!(body["reason"], "no_servers", "{label}");
                assert_eq!(body.as_object().expect("stop noop object").len(), 2);
                None
            }
            ExpectedOutput::Remove(name) => {
                assert_eq!(body["name"], name, "{label}");
                assert_eq!(body.as_object().expect("remove object").len(), 1, "{label}");
                let state = self
                    .projects
                    .get_mut(project)
                    .expect("registered project")
                    .remove(&name)
                    .unwrap_or_else(|| panic!("{label}: modeled server {name} exists"));
                assert!(!state.running, "{label}: removed modeled running server");
                assert!(!process_alive(state.pid), "{label}: removed PID is alive");
                Some(name)
            }
            ExpectedOutput::Error { .. } => unreachable!(),
        }
    }

    fn assert_all_project_state(&self, label: &str) {
        for (project, expected) in &self.projects {
            let servers_dir = project.join(".clickhouse/servers");
            let mut metadata_names = BTreeSet::new();
            let mut data_names = BTreeSet::new();
            if let Ok(entries) = std::fs::read_dir(&servers_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if path.is_file()
                        && let Some(stem) = name.strip_suffix(".json")
                    {
                        metadata_names.insert(stem.to_string());
                    }
                    if path.is_dir() && path.join("data").is_dir() {
                        data_names.insert(name);
                    }
                }
            }
            let expected_names: BTreeSet<_> = expected.keys().cloned().collect();
            assert_eq!(
                metadata_names,
                expected_names,
                "{label}: metadata in {}",
                project.display()
            );
            assert_eq!(
                data_names,
                expected_names,
                "{label}: data in {}",
                project.display()
            );

            for (name, state) in expected {
                let metadata_path = servers_dir.join(format!("{name}.json"));
                let metadata: Value =
                    serde_json::from_slice(&std::fs::read(&metadata_path).unwrap_or_else(
                        |error| panic!("{label}: read {}: {error}", metadata_path.display()),
                    ))
                    .unwrap_or_else(|error| {
                        panic!("{label}: parse {}: {error}", metadata_path.display())
                    });
                assert_eq!(metadata["name"], name.as_str(), "{label}");
                assert_eq!(metadata["engine"], "clickhouse", "{label}");
                assert!(metadata.get("container_id").is_none(), "{label}");
                assert!(
                    metadata["started_at"]
                        .as_str()
                        .and_then(|value| value.parse::<u64>().ok())
                        .is_some(),
                    "{label}: invalid started_at"
                );
                let metadata_cwd = Path::new(
                    metadata["cwd"]
                        .as_str()
                        .unwrap_or_else(|| panic!("{label}: metadata cwd is a string")),
                )
                .canonicalize()
                .unwrap_or_else(|error| panic!("{label}: canonicalize metadata cwd: {error}"));
                assert_eq!(metadata_cwd, *project, "{label}");
                assert_eq!(
                    process_alive(state.pid),
                    state.running,
                    "{label}: PID {}",
                    state.pid
                );
                if state.running {
                    assert_eq!(metadata["pid"], state.pid, "{label}");
                    assert_eq!(metadata["version"], VERSION, "{label}");
                    assert_eq!(metadata["http_port"], state.http_port, "{label}");
                    assert_eq!(metadata["tcp_port"], state.tcp_port, "{label}");
                } else {
                    assert_eq!(metadata["pid"], 0, "{label}");
                    assert_eq!(metadata["version"], "", "{label}");
                    assert_eq!(metadata["http_port"], 0, "{label}");
                    assert_eq!(metadata["tcp_port"], 0, "{label}");
                }
            }
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        for pid in &self.active_pids {
            unsafe {
                libc::kill(*pid as i32, libc::SIGKILL);
            }
        }
    }
}

fn install_fake_clickhouse(home: &Path) {
    let binary = home
        .join(".clickhouse/versions")
        .join(VERSION)
        .join("clickhouse");
    std::fs::create_dir_all(binary.parent().expect("fake binary parent"))
        .expect("create fake version directory");
    std::fs::write(&binary, b"#!/bin/sh\nexec sleep 300\n").expect("write fake ClickHouse");
    let mut permissions = std::fs::metadata(&binary)
        .expect("read fake ClickHouse metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&binary, permissions).expect("make fake ClickHouse executable");
    std::fs::write(home.join(".clickhouse/default"), VERSION).expect("write default version");
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

fn process_alive(pid: u32) -> bool {
    pid != 0 && unsafe { libc::kill(pid as i32, 0) == 0 }
}

fn exact(name: &str) -> ExpectedName {
    ExpectedName::Exact(name.to_string())
}

fn list(entries: &[(&str, bool)]) -> ExpectedOutput {
    ExpectedOutput::List(
        entries
            .iter()
            .map(|(name, running)| ListEntry {
                name: (*name).to_string(),
                running: *running,
            })
            .collect(),
    )
}

fn stop(name: &str, already_stopped: bool) -> ExpectedOutput {
    ExpectedOutput::Stop {
        name: name.to_string(),
        already_stopped,
    }
}

fn scoped_error(code: &'static str, fragments: &[&str]) -> ExpectedOutput {
    ExpectedOutput::Error {
        code,
        command: "clickhousectl local server list --global",
        scoped: true,
        message_fragments: fragments
            .iter()
            .map(|fragment| (*fragment).to_string())
            .collect(),
    }
}

fn start_args(name: Option<&str>) -> Vec<&str> {
    let mut args = vec!["start"];
    if let Some(name) = name {
        args.push(name);
    }
    args.push("--no-wait");
    args
}

#[test]
fn local_server_state_machine() {
    let mut harness = Harness::new();

    let fresh = harness.project("fresh");
    harness.run_steps(vec![
        Step::new("fresh list", &fresh, &["list"], list(&[])),
        Step::new(
            "fresh omitted stop",
            &fresh,
            &["stop"],
            ExpectedOutput::StopNoop,
        ),
        Step::new(
            "fresh omitted remove",
            &fresh,
            &["remove"],
            scoped_error(
                "server_not_found",
                &[
                    "No removable 'default' ClickHouse server exists",
                    "no custom ClickHouse servers are available",
                    "parent `.clickhouse` directories are not searched",
                ],
            ),
        ),
    ]);
    assert!(
        !fresh.join(".clickhouse").exists(),
        "read-only fresh transitions should not initialize a project"
    );
    harness.run_steps(vec![
        Step::new(
            "fresh explicit stop",
            &fresh,
            &["stop", "unknown"],
            scoped_error(
                "server_not_found",
                &[
                    "Server 'unknown' not found",
                    "Run `clickhousectl local server list --global`",
                ],
            ),
        ),
        Step::new(
            "fresh explicit remove",
            &fresh,
            &["remove", "unknown"],
            scoped_error(
                "server_not_found",
                &[
                    "Server 'unknown' not found",
                    "change to the intended project directory",
                ],
            ),
        ),
    ]);

    let default_project = harness.project("default-lifecycle");
    harness.run_steps(vec![
        Step::new(
            "default start",
            &default_project,
            &start_args(None),
            ExpectedOutput::Start(exact("default")),
        ),
        Step::new(
            "explicit repeated default start",
            &default_project,
            &start_args(Some("default")),
            ExpectedOutput::Error {
                code: "server_running",
                command: "clickhousectl local server list",
                scoped: false,
                message_fragments: vec!["Server 'default' is already running".to_string()],
            },
        ),
    ]);
    let generated = harness
        .run_steps(vec![Step::new(
            "omitted repeated start generates a name",
            &default_project,
            &start_args(None),
            ExpectedOutput::Start(ExpectedName::Generated),
        )])
        .pop()
        .flatten()
        .expect("generated server name");
    let mut running_names = ["default".to_string(), generated.clone()];
    running_names.sort();
    harness.run_steps(vec![
        Step::new(
            "running default cannot be removed",
            &default_project,
            &["remove"],
            scoped_error(
                "server_running",
                &[
                    "Server 'default' is running and cannot be removed",
                    "stop it by name before retrying",
                ],
            ),
        ),
        Step::new(
            "list repeated-start state",
            &default_project,
            &["list"],
            list(&[(&running_names[0], true), (&running_names[1], true)]),
        ),
        Step::new(
            "omitted stop prefers default",
            &default_project,
            &["stop"],
            stop("default", false),
        ),
        Step::new(
            "repeated default stop",
            &default_project,
            &["stop", "default"],
            stop("default", true),
        ),
        Step::new(
            "omitted stopped default remove",
            &default_project,
            &["remove"],
            ExpectedOutput::Remove("default".to_string()),
        ),
        Step::new(
            "post-default-remove list",
            &default_project,
            &["list"],
            list(&[(&generated, true)]),
        ),
        Step::new(
            "omitted stop selects sole custom server",
            &default_project,
            &["stop"],
            stop(&generated, false),
        ),
        Step::new(
            "repeated omitted sole-custom stop",
            &default_project,
            &["stop"],
            stop(&generated, true),
        ),
        Step::new(
            "omitted remove never infers sole custom server",
            &default_project,
            &["remove"],
            scoped_error(
                "server_not_found",
                &[
                    "No removable 'default' ClickHouse server exists",
                    "explicit custom server name",
                ],
            ),
        ),
        Step::new(
            "explicit generated remove",
            &default_project,
            &["remove", &generated],
            ExpectedOutput::Remove(generated.clone()),
        ),
        Step::new(
            "post-remove empty list",
            &default_project,
            &["list"],
            list(&[]),
        ),
    ]);

    let custom = harness.project("custom-lifecycle");
    harness.run_steps(vec![
        Step::new(
            "custom start",
            &custom,
            &start_args(Some("analytics")),
            ExpectedOutput::Start(exact("analytics")),
        ),
        Step::new(
            "custom running list",
            &custom,
            &["list"],
            list(&[("analytics", true)]),
        ),
        Step::new(
            "custom running remove",
            &custom,
            &["remove", "analytics"],
            scoped_error(
                "server_running",
                &["Server 'analytics' is running", "stop it by name"],
            ),
        ),
        Step::new(
            "custom-only omitted stop",
            &custom,
            &["stop"],
            stop("analytics", false),
        ),
        Step::new(
            "custom repeated explicit stop",
            &custom,
            &["stop", "analytics"],
            stop("analytics", true),
        ),
        Step::new(
            "custom stopped list",
            &custom,
            &["list"],
            list(&[("analytics", false)]),
        ),
        Step::new(
            "custom omitted remove",
            &custom,
            &["remove"],
            scoped_error("server_not_found", &["explicit custom server name"]),
        ),
        Step::new(
            "custom explicit remove",
            &custom,
            &["remove", "analytics"],
            ExpectedOutput::Remove("analytics".to_string()),
        ),
        Step::new("custom post-remove list", &custom, &["list"], list(&[])),
    ]);

    let many = harness.project("many-custom");
    harness.run_steps(vec![
        Step::new(
            "many alpha start",
            &many,
            &start_args(Some("alpha")),
            ExpectedOutput::Start(exact("alpha")),
        ),
        Step::new(
            "many beta start",
            &many,
            &start_args(Some("beta")),
            ExpectedOutput::Start(exact("beta")),
        ),
        Step::new(
            "many running list",
            &many,
            &["list"],
            list(&[("alpha", true), ("beta", true)]),
        ),
        Step::new(
            "many custom omitted stop",
            &many,
            &["stop"],
            scoped_error(
                "server_not_found",
                &[
                    "multiple non-default ClickHouse servers exist",
                    "Pass a name or run `clickhousectl local server stop-all`",
                ],
            ),
        ),
        Step::new(
            "many custom explicit alpha stop",
            &many,
            &["stop", "alpha"],
            stop("alpha", false),
        ),
        Step::new(
            "many mixed-state list",
            &many,
            &["list"],
            list(&[("beta", true), ("alpha", false)]),
        ),
        Step::new(
            "many remains ambiguous with one running",
            &many,
            &["stop"],
            scoped_error(
                "server_not_found",
                &["multiple non-default ClickHouse servers"],
            ),
        ),
        Step::new(
            "many custom explicit beta stop",
            &many,
            &["stop", "beta"],
            stop("beta", false),
        ),
        Step::new(
            "many custom omitted remove",
            &many,
            &["remove"],
            scoped_error("server_not_found", &["explicit custom server name"]),
        ),
        Step::new(
            "many custom explicit alpha remove",
            &many,
            &["remove", "alpha"],
            ExpectedOutput::Remove("alpha".to_string()),
        ),
        Step::new(
            "sole custom still needs explicit remove",
            &many,
            &["remove"],
            scoped_error("server_not_found", &["explicit custom server name"]),
        ),
        Step::new(
            "many custom explicit beta remove",
            &many,
            &["remove", "beta"],
            ExpectedOutput::Remove("beta".to_string()),
        ),
    ]);

    let root = harness.project("cwd-scope");
    let child = harness.child_project(&root, "child");
    harness.run_steps(vec![
        Step::new(
            "root server start",
            &root,
            &start_args(Some("rooted")),
            ExpectedOutput::Start(exact("rooted")),
        ),
        Step::new(
            "child does not inherit root list",
            &child,
            &["list"],
            list(&[]),
        ),
        Step::new(
            "child omitted stop is independent",
            &child,
            &["stop"],
            ExpectedOutput::StopNoop,
        ),
        Step::new(
            "child explicit stop does not reach root",
            &child,
            &["stop", "rooted"],
            scoped_error(
                "server_not_found",
                &[
                    "Server 'rooted' not found",
                    "parent `.clickhouse` directories are not searched",
                ],
            ),
        ),
        Step::new(
            "child explicit remove does not reach root",
            &child,
            &["remove", "rooted"],
            scoped_error("server_not_found", &["Server 'rooted' not found"]),
        ),
        Step::new(
            "root server remains running",
            &root,
            &["list"],
            list(&[("rooted", true)]),
        ),
        Step::new(
            "root explicit stop",
            &root,
            &["stop", "rooted"],
            stop("rooted", false),
        ),
        Step::new(
            "root explicit remove",
            &root,
            &["remove", "rooted"],
            ExpectedOutput::Remove("rooted".to_string()),
        ),
    ]);

    assert!(
        !harness.home.join(".clickhouse/servers").exists(),
        "project server state must never be written under HOME"
    );
}
