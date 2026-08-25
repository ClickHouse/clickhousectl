//! Subprocess coverage for local Postgres startup readiness (issue #457).

use serde_json::{Value, json};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

#[derive(Clone, Copy)]
enum StartBehavior {
    DelayedReady { ready_on_probe: usize },
    ImmediateExit,
    StartFailure { cleanup_fails: bool },
    FailOnceThenReady,
}

struct FakeDockerState {
    behavior: StartBehavior,
    project: PathBuf,
    instance_name: String,
    started: AtomicBool,
    starts: AtomicUsize,
    removes: AtomicUsize,
    probes: AtomicUsize,
}

struct FakeDocker {
    socket_path: PathBuf,
    state: Arc<FakeDockerState>,
    stop: Arc<AtomicBool>,
    daemon: Option<JoinHandle<()>>,
}

impl FakeDocker {
    fn spawn(directory: &Path, instance_name: &str, behavior: StartBehavior) -> Self {
        let socket_path = directory.join("docker.sock");
        let listener = UnixListener::bind(&socket_path).expect("bind fake Docker socket");
        listener
            .set_nonblocking(true)
            .expect("make fake Docker socket nonblocking");
        let state = Arc::new(FakeDockerState {
            behavior,
            project: directory.to_path_buf(),
            instance_name: instance_name.to_string(),
            started: AtomicBool::new(false),
            starts: AtomicUsize::new(0),
            removes: AtomicUsize::new(0),
            probes: AtomicUsize::new(0),
        });
        let stop = Arc::new(AtomicBool::new(false));
        let daemon_state = Arc::clone(&state);
        let daemon_stop = Arc::clone(&stop);
        let daemon = thread::spawn(move || {
            while !daemon_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_nonblocking(true)
                            .expect("make fake Docker connection nonblocking");
                        if let Some(request) = read_request(&mut stream) {
                            stream
                                .set_nonblocking(false)
                                .expect("make fake Docker response blocking");
                            respond(&mut stream, &request, &daemon_state);
                        }
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept fake Docker connection: {error}"),
                }
            }
        });
        Self {
            socket_path,
            state,
            stop,
            daemon: Some(daemon),
        }
    }

    fn probes(&self) -> usize {
        self.state.probes.load(Ordering::SeqCst)
    }

    fn starts(&self) -> usize {
        self.state.starts.load(Ordering::SeqCst)
    }

    fn removes(&self) -> usize {
        self.state.removes.load(Ordering::SeqCst)
    }

    fn finish(mut self) {
        self.stop_daemon();
    }

    fn stop_daemon(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(daemon) = self.daemon.take() {
            let joined = daemon.join();
            if !thread::panicking() {
                joined.expect("join fake Docker daemon");
            }
        }
    }
}

impl Drop for FakeDocker {
    fn drop(&mut self) {
        self.stop_daemon();
    }
}

fn read_request(stream: &mut UnixStream) -> Option<String> {
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut request = Vec::new();
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        if !read_request_bytes(stream, &mut request, deadline) {
            return None;
        }
    }

    let header_end = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("Docker header terminator")
        + 4;
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .unwrap_or(0);
    while request.len() - header_end < content_length {
        if !read_request_bytes(stream, &mut request, deadline) {
            return None;
        }
    }
    Some(String::from_utf8(request).expect("Docker request is UTF-8"))
}

fn read_request_bytes(stream: &mut UnixStream, request: &mut Vec<u8>, deadline: Instant) -> bool {
    let mut buffer = [0_u8; 2048];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => return false,
            Ok(bytes) => {
                request.extend_from_slice(&buffer[..bytes]);
                return true;
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(1));
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => return false,
            Err(error) => panic!("read Docker request: {error}"),
        }
    }
}

fn write_response(stream: &mut UnixStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    if let Err(error) = stream.write_all(response.as_bytes())
        && error.kind() != ErrorKind::BrokenPipe
    {
        panic!("write fake Docker response: {error}");
    }
}

fn ok_json(stream: &mut UnixStream, body: &str) {
    write_response(stream, "200 OK", "application/json", body);
}

fn inspect_body(state: &FakeDockerState) -> String {
    let running = state.started.load(Ordering::SeqCst)
        && matches!(
            state.behavior,
            StartBehavior::DelayedReady { .. } | StartBehavior::FailOnceThenReady
        );
    let status = if running { "running" } else { "exited" };
    let exit_code = if matches!(state.behavior, StartBehavior::ImmediateExit)
        && state.started.load(Ordering::SeqCst)
    {
        7
    } else {
        0
    };
    json!({
        "Id": "test-container",
        "State": {
            "Status": status,
            "Running": running,
            "ExitCode": exit_code
        },
        "Config": {
            "Env": [
                "POSTGRES_USER=test-user",
                "POSTGRES_PASSWORD=test-password",
                "POSTGRES_DB=test-db"
            ]
        }
    })
    .to_string()
}

fn write_partial_pgdata(state: &FakeDockerState) {
    let data_dir = state
        .project
        .join(".clickhouse/servers")
        .join(format!("{}-pg18/data", state.instance_name));
    std::fs::create_dir_all(&data_dir).expect("create simulated partial PGDATA");
    std::fs::write(data_dir.join("PG_VERSION"), "partial").expect("write simulated partial PGDATA");
}

fn respond(stream: &mut UnixStream, request: &str, state: &FakeDockerState) {
    let request_line = request.lines().next().expect("Docker request line");
    if request_line.starts_with("GET /_ping ") {
        write_response(stream, "200 OK", "text/plain", "OK");
    } else if request_line.starts_with("GET ") && request_line.contains("/containers/json?") {
        ok_json(stream, "[]");
    } else if request_line.starts_with("GET ") && request_line.contains("/images/") {
        ok_json(stream, "{}");
    } else if request_line.starts_with("GET ")
        && request_line.contains("/containers/clickhousectl-pg-")
    {
        write_response(
            stream,
            "404 Not Found",
            "application/json",
            r#"{"message":"No such container"}"#,
        );
    } else if request_line.starts_with("POST ") && request_line.contains("/containers/create?") {
        assert!(request.contains("POSTGRES_USER=test-user"));
        assert!(request.contains("POSTGRES_DB=test-db"));
        ok_json(stream, r#"{"Id":"test-container","Warnings":[]}"#);
    } else if request_line.starts_with("POST ")
        && request_line.contains("/containers/test-container/start ")
    {
        let attempt = state.starts.fetch_add(1, Ordering::SeqCst) + 1;
        let start_fails = matches!(state.behavior, StartBehavior::StartFailure { .. })
            || matches!(state.behavior, StartBehavior::FailOnceThenReady) && attempt == 1;
        if start_fails {
            write_partial_pgdata(state);
            write_response(
                stream,
                "500 Internal Server Error",
                "application/json",
                r#"{"message":"simulated start failure"}"#,
            );
        } else {
            state.started.store(true, Ordering::SeqCst);
            ok_json(stream, "");
        }
    } else if request_line.starts_with("GET ")
        && request_line.contains("/containers/test-container/logs?")
    {
        write_response(
            stream,
            "200 OK",
            "text/plain",
            "database system was interrupted during startup\n",
        );
    } else if request_line.starts_with("GET ")
        && request_line.contains("/containers/test-container/json ")
    {
        ok_json(stream, &inspect_body(state));
    } else if request_line.starts_with("POST ")
        && request_line.contains("/containers/test-container/exec ")
    {
        assert!(request.contains("pg_isready"));
        assert!(request.contains("test-user"));
        assert!(request.contains("test-db"));
        let probe = state.probes.fetch_add(1, Ordering::SeqCst) + 1;
        ok_json(stream, &format!(r#"{{"Id":"probe-{probe}"}}"#));
    } else if request_line.starts_with("POST ") && request_line.contains("/exec/probe-") {
        assert!(request.contains(r#""Detach":true"#));
        ok_json(stream, "");
    } else if request_line.starts_with("GET ") && request_line.contains("/exec/probe-") {
        let probe = request_line
            .split("/exec/probe-")
            .nth(1)
            .and_then(|tail| tail.split('/').next())
            .and_then(|probe| probe.parse::<usize>().ok())
            .expect("probe number");
        let ready = matches!(
            state.behavior,
            StartBehavior::DelayedReady { ready_on_probe } if probe >= ready_on_probe
        ) || matches!(state.behavior, StartBehavior::FailOnceThenReady);
        ok_json(
            stream,
            &format!(
                r#"{{"Running":false,"ExitCode":{}}}"#,
                if ready { 0 } else { 1 }
            ),
        );
    } else if request_line.starts_with("POST ")
        && request_line.contains("/containers/test-container/stop?")
    {
        state.started.store(false, Ordering::SeqCst);
        ok_json(stream, "");
    } else if request_line.starts_with("DELETE ")
        && request_line.contains("/containers/test-container?")
    {
        state.removes.fetch_add(1, Ordering::SeqCst);
        if matches!(
            state.behavior,
            StartBehavior::StartFailure {
                cleanup_fails: true
            }
        ) {
            write_response(
                stream,
                "500 Internal Server Error",
                "application/json",
                r#"{"message":"simulated cleanup failure"}"#,
            );
        } else {
            state.started.store(false, Ordering::SeqCst);
            ok_json(stream, "");
        }
    } else {
        panic!("unexpected fake Docker request: {request_line}");
    }
}

fn run_start(project: &Path, docker: &FakeDocker, name: &str, json: bool, agent: bool) -> Output {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project)
        .env(
            "DOCKER_HOST",
            format!("unix://{}", docker.socket_path.display()),
        )
        .current_dir(project)
        .arg("local")
        .args(json.then_some("--json"))
        .args([
            "postgres",
            "start",
            "--name",
            name,
            "--version",
            "18",
            "--user",
            "test-user",
            "--password",
            "test-password",
            "--database",
            "test-db",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if agent {
        command.env("AGENT", "opencode");
    }
    let mut child = command.spawn().expect("run clickhousectl");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().expect("poll clickhousectl").is_some() {
            return child
                .wait_with_output()
                .expect("collect clickhousectl output");
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill timed out clickhousectl");
            let output = child.wait_with_output().expect("collect timed out output");
            panic!(
                "clickhousectl timed out\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn write_stopped_metadata(project: &Path) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(servers.join("resume-pg18/data"))
        .expect("create resumed Postgres data dir");
    let metadata = json!({
        "name": "resume-pg18",
        "pid": 0,
        "version": "postgres:18",
        "http_port": 0,
        "tcp_port": 5432,
        "started_at": "test",
        "cwd": project.canonicalize().unwrap().display().to_string(),
        "engine": "postgres",
        "container_id": "test-container"
    });
    std::fs::write(
        servers.join("resume-pg18.json"),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .expect("write stopped Postgres metadata");
}

#[test]
fn fresh_start_waits_for_delayed_postgres_readiness() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let docker = FakeDocker::spawn(
        project.path(),
        "fresh",
        StartBehavior::DelayedReady { ready_on_probe: 2 },
    );

    let output = run_start(project.path(), &docker, "fresh", true, false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(docker.probes(), 2);
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse start JSON");
    assert_eq!(body["name"], "fresh");
    docker.finish();
}

#[test]
fn resumed_start_waits_for_delayed_postgres_readiness() {
    let project = tempfile::tempdir().expect("create project tempdir");
    write_stopped_metadata(project.path());
    let docker = FakeDocker::spawn(
        project.path(),
        "resume",
        StartBehavior::DelayedReady { ready_on_probe: 2 },
    );

    let output = run_start(project.path(), &docker, "resume", true, false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(docker.probes(), 2);
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse start JSON");
    assert_eq!(body["name"], "resume");
    docker.finish();
}

#[test]
fn immediate_container_exit_is_a_failed_start_with_logs() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let docker = FakeDocker::spawn(project.path(), "crashed", StartBehavior::ImmediateExit);

    let output = run_start(project.path(), &docker, "crashed", false, false);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "start printed a success payload");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exited before PostgreSQL became ready"));
    assert!(stderr.contains("exit code 7"));
    assert!(stderr.contains("database system was interrupted during startup"));
    assert_eq!(docker.probes(), 0);
    assert_eq!(docker.removes(), 1);
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/crashed-pg18.json")
            .exists(),
        "failed fresh start retained success metadata"
    );
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/crashed-pg18")
            .exists(),
        "failed fresh start retained its data directory"
    );
    docker.finish();
}

#[test]
fn immediate_container_exit_is_structured_in_json_and_agent_modes() {
    for (json, agent) in [(true, false), (false, true)] {
        let project = tempfile::tempdir().expect("create project tempdir");
        let docker = FakeDocker::spawn(project.path(), "crashed", StartBehavior::ImmediateExit);

        let output = run_start(project.path(), &docker, "crashed", json, agent);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty(), "start printed a success payload");
        let body: Value = serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
            panic!(
                "parse structured startup error: {error}; stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        assert_eq!(body["error"]["code"], "startup_exit");
        assert_eq!(
            body["error"]["message"],
            "Server exited before startup completed"
        );
        assert_eq!(body["error"]["command"], "clickhousectl local server list");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("exit code 7"));
        docker.finish();
    }
}

#[test]
fn create_success_start_failure_rolls_back_container_and_partial_pgdata() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let docker = FakeDocker::spawn(
        project.path(),
        "start-failure",
        StartBehavior::StartFailure {
            cleanup_fails: false,
        },
    );

    let output = run_start(project.path(), &docker, "start-failure", false, false);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "start printed a success payload");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("simulated start failure"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(docker.starts(), 1);
    assert_eq!(docker.removes(), 1);
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/start-failure-pg18")
            .exists(),
        "start failure retained partial PGDATA"
    );
    docker.finish();
}

#[test]
fn metadata_failure_uses_the_same_fresh_start_rollback() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let metadata_path = project
        .path()
        .join(".clickhouse/servers/metadata-failure-pg18.json");
    std::fs::create_dir_all(&metadata_path).expect("create metadata path collision");
    let docker = FakeDocker::spawn(
        project.path(),
        "metadata-failure",
        StartBehavior::DelayedReady { ready_on_probe: 1 },
    );

    let output = run_start(project.path(), &docker, "metadata-failure", false, false);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("IO error"), "stderr: {stderr}");
    assert!(
        stderr.contains("failed to remove metadata"),
        "stderr: {stderr}"
    );
    assert_eq!(docker.starts(), 1);
    assert_eq!(docker.removes(), 1);
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/metadata-failure-pg18")
            .exists(),
        "metadata failure retained fresh PGDATA"
    );
    docker.finish();
}

#[test]
fn failed_fresh_start_can_retry_from_clean_pgdata() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let docker = FakeDocker::spawn(project.path(), "retry", StartBehavior::FailOnceThenReady);

    let first = run_start(project.path(), &docker, "retry", true, false);
    assert_eq!(first.status.code(), Some(1));
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/retry-pg18")
            .exists(),
        "first attempt retained partial PGDATA"
    );

    let second = run_start(project.path(), &docker, "retry", true, false);
    assert!(
        second.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    let body: Value = serde_json::from_slice(&second.stdout).expect("parse retry JSON");
    assert_eq!(body["name"], "retry");
    assert_eq!(docker.starts(), 2);
    assert_eq!(docker.removes(), 1);
    docker.finish();
}

#[test]
fn cleanup_failure_preserves_primary_error_and_retains_pgdata() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let docker = FakeDocker::spawn(
        project.path(),
        "cleanup-failure",
        StartBehavior::StartFailure {
            cleanup_fails: true,
        },
    );

    let output = run_start(project.path(), &docker, "cleanup-failure", false, false);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("simulated start failure"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("Postgres startup rollback diagnostics"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("simulated cleanup failure"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("retained Postgres data"),
        "stderr: {stderr}"
    );
    assert_eq!(docker.removes(), 1);
    assert!(
        project
            .path()
            .join(".clickhouse/servers/cleanup-failure-pg18/data/PG_VERSION")
            .exists(),
        "PGDATA was removed while its container remained"
    );
    docker.finish();
}

#[test]
fn fresh_failure_retains_data_that_predated_the_attempt() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let marker = project
        .path()
        .join(".clickhouse/servers/preexisting-pg18/data/existing-data");
    std::fs::create_dir_all(marker.parent().unwrap()).expect("create pre-existing data dir");
    std::fs::write(&marker, "keep").expect("write pre-existing PGDATA marker");
    let docker = FakeDocker::spawn(
        project.path(),
        "preexisting",
        StartBehavior::StartFailure {
            cleanup_fails: false,
        },
    );

    let output = run_start(project.path(), &docker, "preexisting", false, false);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("simulated start failure"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("retained Postgres data"),
        "stderr: {stderr}"
    );
    assert_eq!(docker.removes(), 1);
    assert!(marker.exists(), "failure removed pre-existing PGDATA");
    docker.finish();
}

#[test]
fn resume_failure_stops_container_without_removing_existing_data() {
    let project = tempfile::tempdir().expect("create project tempdir");
    write_stopped_metadata(project.path());
    let marker = project
        .path()
        .join(".clickhouse/servers/resume-pg18/data/existing-data");
    std::fs::write(&marker, "keep").expect("write existing PGDATA marker");
    let docker = FakeDocker::spawn(project.path(), "resume", StartBehavior::ImmediateExit);

    let output = run_start(project.path(), &docker, "resume", false, false);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("exited before PostgreSQL became ready")
    );
    assert_eq!(docker.removes(), 0);
    assert!(marker.exists(), "resume failure removed existing PGDATA");
    docker.finish();
}
