//! Subprocess coverage for local Postgres readiness through a fake Docker API.

use std::collections::{HashMap, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

#[derive(Clone, Copy)]
enum ContainerOutcome {
    Running,
    ImmediateExit,
}

struct DockerScenario {
    existing: bool,
    outcome: ContainerOutcome,
    readiness_exit_codes: Vec<i64>,
    logs: Vec<String>,
}

#[derive(Clone, Debug)]
struct DockerRequest {
    method: String,
    path: String,
    body: String,
}

struct FakeDocker {
    stop: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<DockerRequest>>>,
    thread: Option<JoinHandle<()>>,
}

impl FakeDocker {
    fn start(socket_path: &Path, scenario: DockerScenario) -> Self {
        let listener = UnixListener::bind(socket_path).expect("bind fake Docker socket");
        listener
            .set_nonblocking(true)
            .expect("make fake Docker socket nonblocking");
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let thread_stop = Arc::clone(&stop);
        let thread_requests = Arc::clone(&requests);
        let thread = thread::spawn(move || {
            let mut started = false;
            let mut next_exec = 0_usize;
            let mut readiness_exit_codes: VecDeque<i64> = scenario.readiness_exit_codes.into();
            let mut exec_exit_codes = HashMap::new();

            while !thread_stop.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("accept fake Docker request: {error}"),
                };
                stream
                    .set_nonblocking(false)
                    .expect("make fake Docker connection blocking");
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .expect("set fake Docker read timeout");
                let request = read_request(&mut stream);
                thread_requests.lock().unwrap().push(request.clone());

                match (request.method.as_str(), request.path.as_str()) {
                    ("GET", "/_ping") => write_response(&mut stream, 200, "text/plain", b"OK"),
                    ("GET", path) if path.starts_with("/containers/json?") => {
                        write_json(&mut stream, 200, "[]")
                    }
                    ("GET", "/images/postgres:18/json") => write_json(&mut stream, 200, "{}"),
                    ("GET", path)
                        if path.starts_with("/containers/clickhousectl-pg-default-18/json") =>
                    {
                        write_json(&mut stream, 404, r#"{"message":"No such container"}"#)
                    }
                    ("POST", path) if path.starts_with("/containers/create?") => {
                        assert!(!scenario.existing, "resumed start created a new container");
                        write_json(&mut stream, 201, r#"{"Id":"pg-id","Warnings":[]}"#);
                    }
                    ("POST", path) if path.starts_with("/containers/pg-id/start") => {
                        started = true;
                        write_response(&mut stream, 204, "application/json", b"");
                    }
                    ("GET", path) if path.starts_with("/containers/pg-id/json") => {
                        let running =
                            started && matches!(scenario.outcome, ContainerOutcome::Running);
                        let state = if running {
                            r#"{"Status":"running","Running":true,"Paused":false,"ExitCode":0,"OOMKilled":false}"#
                        } else if started {
                            r#"{"Status":"exited","Running":false,"Paused":false,"ExitCode":1,"OOMKilled":false}"#
                        } else {
                            r#"{"Status":"exited","Running":false,"Paused":false,"ExitCode":0,"OOMKilled":false}"#
                        };
                        let body = format!(
                            r#"{{"Id":"pg-id","State":{state},"Config":{{"Env":["POSTGRES_USER=postgres","POSTGRES_PASSWORD=stored-secret","POSTGRES_DB=postgres"]}}}}"#
                        );
                        write_json(&mut stream, 200, &body);
                    }
                    ("POST", "/containers/pg-id/exec") => {
                        let exit_code = readiness_exit_codes
                            .pop_front()
                            .expect("unexpected extra pg_isready probe");
                        let exec_id = format!("exec-{next_exec}");
                        next_exec += 1;
                        exec_exit_codes.insert(exec_id.clone(), exit_code);
                        write_json(&mut stream, 201, &format!(r#"{{"Id":"{exec_id}"}}"#));
                    }
                    ("POST", path) if path.starts_with("/exec/") && path.ends_with("/start") => {
                        write_response(&mut stream, 200, "application/json", b"");
                    }
                    ("GET", path) if path.starts_with("/exec/") && path.ends_with("/json") => {
                        let exec_id = path.trim_start_matches("/exec/").trim_end_matches("/json");
                        let exit_code = exec_exit_codes
                            .get(exec_id)
                            .expect("inspect unknown fake exec");
                        write_json(
                            &mut stream,
                            200,
                            &format!(
                                r#"{{"ID":"{exec_id}","Running":false,"ExitCode":{exit_code}}}"#
                            ),
                        );
                    }
                    ("GET", path) if path.starts_with("/containers/pg-id/logs?") => {
                        write_response(
                            &mut stream,
                            200,
                            "application/vnd.docker.raw-stream",
                            &docker_log_stream(&scenario.logs),
                        );
                    }
                    ("POST", path) if path.starts_with("/containers/pg-id/stop?") => {
                        write_response(&mut stream, 204, "application/json", b"")
                    }
                    ("DELETE", path) if path.starts_with("/containers/pg-id?") => {
                        write_response(&mut stream, 204, "application/json", b"")
                    }
                    _ => panic!("unexpected fake Docker request: {request:?}"),
                }
                let _ = stream.shutdown(Shutdown::Both);
            }
        });
        Self {
            stop,
            requests,
            thread: Some(thread),
        }
    }

    fn requests(&self) -> Vec<DockerRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for FakeDocker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().expect("join fake Docker daemon");
        }
    }
}

fn read_request(stream: &mut UnixStream) -> DockerRequest {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let count = stream.read(&mut buffer).expect("read fake Docker request");
        assert!(count > 0, "Docker request ended before its headers");
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8(bytes[..header_end].to_vec()).expect("HTTP headers are UTF-8");
    let content_length = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(str::trim)
                .map(str::parse::<usize>)
        })
        .transpose()
        .expect("valid Content-Length")
        .unwrap_or(0);
    while bytes.len() - header_end < content_length {
        let count = stream.read(&mut buffer).expect("read fake Docker body");
        assert!(count > 0, "Docker request ended before its body");
        bytes.extend_from_slice(&buffer[..count]);
    }
    let request_line = headers.lines().next().expect("HTTP request line");
    let mut request_parts = request_line.split_whitespace();
    DockerRequest {
        method: request_parts.next().expect("HTTP method").to_string(),
        path: request_parts.next().expect("HTTP path").to_string(),
        body: String::from_utf8_lossy(&bytes[header_end..header_end + content_length]).into_owned(),
    }
}

fn write_json(stream: &mut UnixStream, status: u16, body: &str) {
    write_response(stream, status, "application/json", body.as_bytes());
}

fn write_response(stream: &mut UnixStream, status: u16, content_type: &str, body: &[u8]) {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        404 => "Not Found",
        _ => "Response",
    };
    let headers = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .and_then(|()| stream.write_all(body))
        .expect("write fake Docker response");
}

fn docker_log_stream(lines: &[String]) -> Vec<u8> {
    let mut body = Vec::new();
    for line in lines {
        let message = format!("{line}\n");
        body.extend_from_slice(&[2, 0, 0, 0]);
        body.extend_from_slice(&(message.len() as u32).to_be_bytes());
        body.extend_from_slice(message.as_bytes());
    }
    body
}

fn reserve_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .expect("reserve Postgres port")
        .local_addr()
        .expect("reserved address")
        .port()
}

fn write_resumed_server(project: &Path) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).expect("create resumed server metadata directory");
    let cwd = project.canonicalize().expect("canonical project path");
    let metadata = serde_json::json!({
        "name": "default-pg18",
        "pid": 0,
        "version": "postgres:18",
        "http_port": 0,
        "tcp_port": reserve_port(),
        "started_at": "before-resume",
        "cwd": cwd,
        "engine": "postgres",
        "container_id": "pg-id"
    });
    std::fs::write(
        servers.join("default-pg18.json"),
        serde_json::to_vec_pretty(&metadata).unwrap(),
    )
    .expect("write resumed server metadata");
}

fn run_start(
    scenario: DockerScenario,
    resumed: bool,
    telemetry_debug: bool,
) -> (Output, Vec<DockerRequest>) {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    if resumed {
        write_resumed_server(project.path());
    }
    if telemetry_debug {
        let telemetry_dir = home.path().join(".clickhouse");
        std::fs::create_dir_all(&telemetry_dir).expect("create telemetry state directory");
        std::fs::write(
            telemetry_dir.join("telemetry.json"),
            r#"{"disabled":false}"#,
        )
        .expect("enable telemetry");
    }
    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(&socket_path, scenario);
    let port = reserve_port().to_string();
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("HOME", home.path())
        .env("DOCKER_HOST", format!("unix://{}", socket_path.display()))
        .current_dir(project.path())
        .args([
            "local",
            "--json",
            "postgres",
            "start",
            "--wait-timeout",
            "2",
        ]);
    if resumed {
        command.args(["--name", "default"]);
    } else {
        command.args(["--port", &port, "--password", "fresh-secret"]);
    }
    if telemetry_debug {
        command.env("CHCTL_TELEMETRY_DEBUG", "1");
    } else {
        command.env("DO_NOT_TRACK", "1");
    }
    let output = command.output().expect("run clickhousectl");
    let requests = docker.requests();
    drop(docker);
    (output, requests)
}

fn readiness_requests(requests: &[DockerRequest]) -> Vec<&DockerRequest> {
    requests
        .iter()
        .filter(|request| request.path == "/containers/pg-id/exec")
        .collect()
}

#[test]
fn fresh_start_waits_for_delayed_postgres_readiness_without_exposing_password() {
    let (output, requests) = run_start(
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::Running,
            readiness_exit_codes: vec![1, 0],
            logs: vec![],
        },
        false,
        false,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).expect("start JSON");
    assert_eq!(result["container_id"], "pg-id");
    let probes = readiness_requests(&requests);
    assert_eq!(probes.len(), 2);
    for probe in probes {
        let body: serde_json::Value = serde_json::from_str(&probe.body).expect("exec body JSON");
        assert_eq!(
            body["Cmd"],
            serde_json::json!([
                "pg_isready",
                "--quiet",
                "--host",
                "127.0.0.1",
                "--port",
                "5432",
                "--timeout",
                "1"
            ])
        );
        assert!(!probe.body.contains("fresh-secret"));
        assert!(body["Env"].is_null());
    }
}

#[test]
fn resumed_start_also_waits_for_postgres_readiness() {
    let (output, requests) = run_start(
        DockerScenario {
            existing: true,
            outcome: ContainerOutcome::Running,
            readiness_exit_codes: vec![1, 0],
            logs: vec![],
        },
        true,
        false,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(readiness_requests(&requests).len(), 2);
    assert!(requests.iter().any(|request| {
        request.method == "POST" && request.path.starts_with("/containers/pg-id/start")
    }));
    assert!(
        !requests
            .iter()
            .any(|request| request.path.starts_with("/containers/create"))
    );
}

#[test]
fn immediate_exit_reports_bounded_logs_and_error_telemetry_without_setup_success() {
    let mut logs: Vec<String> = (0..80)
        .map(|index| format!("startup line {index}: {}", "x".repeat(300)))
        .collect();
    logs.push("FATAL: startup failed before readiness".to_string());
    let (output, requests) = run_start(
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::ImmediateExit,
            readiness_exit_codes: vec![],
            logs,
        },
        false,
        true,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "setup success leaked to stdout");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("exited before PostgreSQL became ready"),
        "{stderr}"
    );
    assert!(stderr.contains("exit code: 1"), "{stderr}");
    assert!(
        stderr.contains("FATAL: startup failed before readiness"),
        "{stderr}"
    );
    assert!(
        stderr.contains("[earlier log output truncated]"),
        "{stderr}"
    );
    assert!(stderr.len() < 18_000, "diagnostics were not byte-bounded");
    assert!(
        stderr.contains(r#""command":"local postgres start""#),
        "{stderr}"
    );
    assert!(stderr.contains(r#""exit_code":1"#), "{stderr}");
    assert!(stderr.contains(r#""outcome":"error""#), "{stderr}");
    assert!(readiness_requests(&requests).is_empty());
}
