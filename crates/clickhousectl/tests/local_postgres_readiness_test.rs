//! Subprocess coverage for local Postgres readiness through a fake Docker API.

use std::collections::{HashMap, VecDeque};
use std::fs::OpenOptions;
use std::io::{ErrorKind, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

static START_COMMAND_LOCK: Mutex<()> = Mutex::new(());

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
    start_statuses: Vec<u16>,
    remove_statuses: Vec<u16>,
    readiness_exit_codes: Vec<i64>,
    readiness_create_errors: usize,
    logs: Vec<String>,
    write_partial_data: bool,
    create_metadata_directory_on_start: bool,
}

#[derive(Clone, Debug)]
struct DockerRequest {
    method: String,
    path: String,
    body: String,
    metadata_lock_available: bool,
}

struct FakeDocker {
    stop: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<DockerRequest>>>,
    thread: Option<JoinHandle<()>>,
}

impl FakeDocker {
    fn start(socket_path: &Path, project_path: &Path, scenario: DockerScenario) -> Self {
        let listener = UnixListener::bind(socket_path).expect("bind fake Docker socket");
        listener
            .set_nonblocking(true)
            .expect("make fake Docker socket nonblocking");
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let thread_stop = Arc::clone(&stop);
        let thread_requests = Arc::clone(&requests);
        let project = project_path.to_path_buf();
        let partial_data_path =
            project_path.join(".clickhouse/servers/default-pg18/data/partial-init");
        let thread = thread::spawn(move || {
            let DockerScenario {
                existing,
                outcome,
                start_statuses,
                remove_statuses,
                readiness_exit_codes,
                readiness_create_errors,
                logs,
                write_partial_data,
                create_metadata_directory_on_start,
            } = scenario;
            let mut started = false;
            let mut next_exec = 0_usize;
            let mut start_statuses: VecDeque<u16> = start_statuses.into();
            let mut remove_statuses: VecDeque<u16> = remove_statuses.into();
            let mut readiness_create_errors = readiness_create_errors;
            let mut readiness_exit_codes: VecDeque<i64> = readiness_exit_codes.into();
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
                let mut request = read_request(&mut stream);
                request.metadata_lock_available = metadata_lock_available(&project);
                thread_requests.lock().unwrap().push(request.clone());

                match (request.method.as_str(), request.path.as_str()) {
                    ("GET", "/_ping") => write_response(&mut stream, 200, "text/plain", b"OK"),
                    ("GET", path) if path.starts_with("/containers/json?") => {
                        write_json(&mut stream, 200, "[]")
                    }
                    ("GET", "/images/postgres:18/json") => {
                        inject_metadata_during_image_inspect(&project);
                        write_json(&mut stream, 200, "{}");
                    }
                    ("GET", "/images/alpine:latest/json") => write_json(&mut stream, 200, "{}"),
                    ("GET", path)
                        if path.starts_with("/containers/clickhousectl-pg-default-18/json") =>
                    {
                        write_json(&mut stream, 404, r#"{"message":"No such container"}"#)
                    }
                    ("GET", path) if path.starts_with("/containers/concurrent-id/json") => {
                        write_json(&mut stream, 404, r#"{"message":"No such container"}"#)
                    }
                    ("POST", path) if path.starts_with("/containers/create?") => {
                        let body: serde_json::Value = serde_json::from_str(&request.body)
                            .expect("container create body JSON");
                        if body["Image"] == "alpine:latest" {
                            let instance_dir = project.join(".clickhouse/servers/default-pg18");
                            match std::fs::remove_dir_all(instance_dir) {
                                Ok(()) => {}
                                Err(error) if error.kind() == ErrorKind::NotFound => {}
                                Err(error) => panic!("simulate privileged data cleanup: {error}"),
                            }
                            write_json(&mut stream, 201, r#"{"Id":"cleanup-id","Warnings":[]}"#);
                        } else {
                            assert!(!existing, "resumed start created a new container");
                            started = false;
                            write_json(&mut stream, 201, r#"{"Id":"pg-id","Warnings":[]}"#);
                        }
                    }
                    ("POST", path) if path.starts_with("/containers/pg-id/start") => {
                        let status = start_statuses.pop_front().unwrap_or(204);
                        if status == 204 {
                            started = true;
                            let data_dir = project.join(".clickhouse/servers/default-pg18/data");
                            std::fs::create_dir_all(&data_dir).expect("create simulated PGDATA");
                            std::fs::write(data_dir.join("PG_VERSION"), "18")
                                .expect("write simulated PGDATA marker");
                            if write_partial_data {
                                std::fs::write(&partial_data_path, "partial PGDATA")
                                    .expect("write partial PGDATA marker");
                            }
                            if create_metadata_directory_on_start {
                                let instance_dir =
                                    partial_data_path.parent().unwrap().parent().unwrap();
                                std::fs::create_dir(instance_dir.with_extension("json"))
                                    .expect("create metadata failure directory");
                            }
                            write_response(&mut stream, 204, "application/json", b"");
                        } else {
                            write_json(
                                &mut stream,
                                status,
                                r#"{"message":"start failed by test"}"#,
                            );
                        }
                    }
                    ("GET", path) if path.starts_with("/containers/pg-id/json") => {
                        let running = started && matches!(outcome, ContainerOutcome::Running);
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
                    ("GET", path) if path.starts_with("/containers/dotenv-id/json") => {
                        write_json(
                            &mut stream,
                            200,
                            r#"{"Id":"dotenv-id","State":{"Running":true},"Config":{"Env":["POSTGRES_USER=dotenv-user","POSTGRES_PASSWORD=dotenv-secret","POSTGRES_DB=dotenv-database"]}}"#,
                        );
                    }
                    ("POST", "/containers/pg-id/exec") => {
                        if readiness_create_errors > 0 {
                            readiness_create_errors -= 1;
                            write_json(
                                &mut stream,
                                500,
                                r#"{"message":"temporary create_exec failure"}"#,
                            );
                            continue;
                        }
                        let exit_code = readiness_exit_codes.pop_front().unwrap_or(1);
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
                            &docker_log_stream(&logs),
                        );
                    }
                    ("POST", path) if path.starts_with("/containers/pg-id/stop?") => {
                        write_response(&mut stream, 204, "application/json", b"")
                    }
                    ("POST", path) if path.starts_with("/containers/cleanup-id/start") => {
                        write_response(&mut stream, 204, "application/json", b"")
                    }
                    ("POST", path) if path.starts_with("/containers/cleanup-id/wait") => {
                        write_json(&mut stream, 200, r#"{"StatusCode":0}"#)
                    }
                    ("DELETE", path) if path.starts_with("/containers/pg-id?") => {
                        let status = remove_statuses.pop_front().unwrap_or(204);
                        if status == 204 {
                            started = false;
                            write_response(&mut stream, 204, "application/json", b"")
                        } else {
                            write_json(
                                &mut stream,
                                status,
                                r#"{"message":"remove failed by test"}"#,
                            )
                        }
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
        metadata_lock_available: false,
    }
}

fn metadata_lock_available(project: &Path) -> bool {
    let path = project.join(".clickhouse/servers/.metadata.lock");
    let Ok(file) = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
    else {
        return false;
    };
    file.try_lock().is_ok()
}

fn inject_metadata_during_image_inspect(project: &Path) {
    let marker = project.join("inject-metadata-during-image-inspect");
    if !marker.exists() {
        return;
    }

    let servers = project.join(".clickhouse/servers");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(servers.join(".metadata.lock"))
        .expect("open metadata lock for concurrent write");
    lock.lock()
        .expect("acquire metadata lock for concurrent write");
    let metadata = serde_json::json!({
        "name": "default-pg18",
        "pid": 0,
        "version": "postgres:18",
        "http_port": 0,
        "tcp_port": 5432,
        "started_at": "concurrent",
        "cwd": project.canonicalize().unwrap(),
        "engine": "postgres",
        "container_id": "concurrent-id"
    });
    std::fs::write(
        servers.join("default-pg18.json"),
        serde_json::to_vec_pretty(&metadata).unwrap(),
    )
    .expect("write concurrent metadata");
    std::fs::remove_file(marker).expect("remove metadata injection marker");
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
        500 => "Internal Server Error",
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
    std::fs::create_dir_all(servers.join("default-pg18/data"))
        .expect("create resumed server data directory");
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

fn write_dotenv_server(project: &Path) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(servers.join("default-pg18/data"))
        .expect("create dotenv server data directory");
    let metadata = serde_json::json!({
        "name": "default-pg18",
        "pid": 0,
        "version": "postgres:18",
        "http_port": 0,
        "tcp_port": 5432,
        "started_at": "before-dotenv",
        "cwd": project.canonicalize().unwrap(),
        "engine": "postgres",
        "container_id": "dotenv-id"
    });
    std::fs::write(
        servers.join("default-pg18.json"),
        serde_json::to_vec_pretty(&metadata).unwrap(),
    )
    .expect("write dotenv server metadata");
}

fn run_start(
    scenario: DockerScenario,
    resumed: bool,
    telemetry_debug: bool,
    wait_timeout: u16,
    preexisting_data: bool,
) -> (Output, Vec<DockerRequest>, tempfile::TempDir) {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    if resumed {
        write_resumed_server(project.path());
    }
    if preexisting_data {
        let marker = project
            .path()
            .join(".clickhouse/servers/default-pg18/data/existing-data");
        std::fs::create_dir_all(marker.parent().unwrap())
            .expect("create pre-existing Postgres data directory");
        std::fs::write(marker, "keep").expect("write pre-existing data marker");
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
    let docker = FakeDocker::start(&socket_path, project.path(), scenario);
    let output = run_start_command(
        home.path(),
        project.path(),
        &socket_path,
        resumed,
        telemetry_debug,
        wait_timeout,
    );
    let requests = docker.requests();
    drop(docker);
    (output, requests, project)
}

fn run_start_command(
    home: &Path,
    project: &Path,
    socket_path: &Path,
    resumed: bool,
    telemetry_debug: bool,
    wait_timeout: u16,
) -> Output {
    let _guard = START_COMMAND_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let port = reserve_port().to_string();
    let wait_timeout = wait_timeout.to_string();
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("HOME", home)
        .env("DOCKER_HOST", format!("unix://{}", socket_path.display()))
        .current_dir(project)
        .args([
            "local",
            "--json",
            "postgres",
            "start",
            "--wait-timeout",
            &wait_timeout,
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
    command.output().expect("run clickhousectl")
}

fn readiness_requests(requests: &[DockerRequest]) -> Vec<&DockerRequest> {
    requests
        .iter()
        .filter(|request| request.path == "/containers/pg-id/exec")
        .collect()
}

#[test]
fn fresh_start_waits_for_delayed_postgres_readiness_without_exposing_password() {
    let (output, requests, _project) = run_start(
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::Running,
            start_statuses: vec![204],
            remove_statuses: vec![],
            readiness_exit_codes: vec![1, 0],
            readiness_create_errors: 1,
            logs: vec![],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
        false,
        false,
        2,
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
    assert_eq!(probes.len(), 3);
    for probe in probes {
        assert!(
            probe.metadata_lock_available,
            "metadata lock was held during readiness request: {probe:?}"
        );
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
    let image_inspect = requests
        .iter()
        .find(|request| request.path == "/images/postgres:18/json")
        .expect("fresh image inspection request");
    assert!(
        image_inspect.metadata_lock_available,
        "metadata lock was held during image inspection: {image_inspect:?}"
    );
}

#[test]
fn postgres_dotenv_releases_metadata_lock_before_docker_credentials_read() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    write_dotenv_server(project.path());
    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(
        &socket_path,
        project.path(),
        DockerScenario {
            existing: true,
            outcome: ContainerOutcome::Running,
            start_statuses: vec![],
            remove_statuses: vec![],
            readiness_exit_codes: vec![],
            readiness_create_errors: 0,
            logs: vec![],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
    );

    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home.path())
        .env("DOCKER_HOST", format!("unix://{}", socket_path.display()))
        .current_dir(project.path())
        .args(["local", "--json", "postgres", "dotenv", "--name", "default"])
        .output()
        .expect("run postgres dotenv");
    let requests = docker.requests();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let inspections: Vec<_> = requests
        .iter()
        .filter(|request| request.path == "/containers/dotenv-id/json")
        .collect();
    assert!(inspections.len() >= 2, "requests: {requests:?}");
    assert!(
        inspections.last().unwrap().metadata_lock_available,
        "metadata lock was held during credential read: {inspections:?}"
    );
}

#[test]
fn postgres_start_revalidates_metadata_after_image_inspection() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    std::fs::write(
        project.path().join("inject-metadata-during-image-inspect"),
        b"inject",
    )
    .expect("write metadata injection marker");
    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(
        &socket_path,
        project.path(),
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::Running,
            start_statuses: vec![],
            remove_statuses: vec![],
            readiness_exit_codes: vec![],
            readiness_create_errors: 0,
            logs: vec![],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
    );
    let port = reserve_port().to_string();

    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home.path())
        .env("DOCKER_HOST", format!("unix://{}", socket_path.display()))
        .current_dir(project.path())
        .args([
            "local",
            "postgres",
            "start",
            "--name",
            "default",
            "--version",
            "18",
            "--port",
            &port,
        ])
        .output()
        .expect("run postgres start");
    let requests = docker.requests();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("container is gone"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !requests
            .iter()
            .any(|request| request.path.starts_with("/containers/create")),
        "start ignored concurrently committed metadata: {requests:?}"
    );
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/servers/default-pg18.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(metadata["container_id"], "concurrent-id");
}

#[test]
fn resumed_start_also_waits_for_postgres_readiness() {
    let (output, requests, _project) = run_start(
        DockerScenario {
            existing: true,
            outcome: ContainerOutcome::Running,
            start_statuses: vec![204],
            remove_statuses: vec![],
            readiness_exit_codes: vec![1, 0],
            readiness_create_errors: 0,
            logs: vec![],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
        true,
        false,
        2,
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
fn wall_clock_timeout_fails_and_rolls_back_fresh_data() {
    let started = std::time::Instant::now();
    let (output, requests, project) = run_start(
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::Running,
            start_statuses: vec![204],
            remove_statuses: vec![204],
            readiness_exit_codes: vec![],
            readiness_create_errors: 0,
            logs: vec![],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
        false,
        false,
        1,
        false,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(started.elapsed() < Duration::from_secs(4));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "startup_timeout");
    assert_eq!(
        error["error"]["message"],
        "Postgres server 'default' did not become ready within 1 seconds"
    );
    assert!(readiness_requests(&requests).len() >= 2);
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/default-pg18")
            .exists(),
        "wall-clock timeout retained PGDATA created by this attempt"
    );
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/default-pg18.json")
            .exists(),
        "wall-clock timeout retained success metadata"
    );
}

#[test]
fn immediate_exit_redacts_bounded_logs_without_setup_success_or_telemetry_noise() {
    let mut logs: Vec<String> = (0..80)
        .map(|index| format!("startup line {index}: {}", "x".repeat(300)))
        .collect();
    logs.push("FATAL: startup failed before readiness".to_string());
    let (output, requests, project) = run_start(
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::ImmediateExit,
            start_statuses: vec![204],
            remove_statuses: vec![204],
            readiness_exit_codes: vec![],
            readiness_create_errors: 0,
            logs,
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
        false,
        true,
        2,
        false,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "setup success leaked to stdout");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    let error: serde_json::Value =
        serde_json::from_str(&stderr).expect("stderr is exactly one JSON error");
    assert_eq!(error["error"]["code"], "startup_exit");
    assert_eq!(
        error["error"]["message"],
        "Postgres server 'default' exited before becoming ready"
    );
    assert!(!stderr.contains("FATAL: startup failed before readiness"));
    assert!(!stderr.contains("[earlier log output truncated]"));
    assert!(readiness_requests(&requests).is_empty());
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/default-pg18")
            .exists(),
        "failed fresh start retained PGDATA created by this attempt"
    );
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/default-pg18.json")
            .exists(),
        "failed fresh start retained success metadata"
    );
}

#[test]
fn failed_fresh_start_preserves_postgres_identity_without_polluting_clickhouse_selection() {
    let (output, requests, project) = run_start(
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::ImmediateExit,
            start_statuses: vec![204],
            remove_statuses: vec![204],
            readiness_exit_codes: vec![],
            readiness_create_errors: 0,
            logs: vec![],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
        false,
        false,
        2,
        true,
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "startup_exit");
    assert!(!stderr.contains("directory contained data before this start attempt"));
    assert!(!stderr.contains("recovery metadata retained"));
    assert!(
        project
            .path()
            .join(".clickhouse/servers/default-pg18/data/existing-data")
            .exists(),
        "failed start removed pre-existing data"
    );
    assert!(
        project
            .path()
            .join(".clickhouse/servers/default-pg18.json")
            .exists(),
        "failed start did not retain recovery metadata"
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == "DELETE")
            .count(),
        1
    );

    let clickhouse_data = project.path().join(".clickhouse/servers/dev/data");
    std::fs::create_dir_all(&clickhouse_data).expect("create ClickHouse data directory");
    let home = tempfile::tempdir().expect("create selection home");
    let stop = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home.path())
        .env("PATH", "/usr/bin:/bin")
        .current_dir(project.path())
        .args(["local", "--json", "server", "stop"])
        .output()
        .expect("run omitted ClickHouse stop");

    assert!(
        stop.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&stop.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&stop.stdout).expect("stop JSON");
    assert_eq!(result["name"], "dev");
    assert_eq!(result["already_stopped"], true);
    assert_eq!(result["selection"], "implicit");
    assert!(clickhouse_data.is_dir());
}

#[test]
fn incomplete_container_cleanup_retains_pgdata_and_recovery_metadata() {
    let (output, requests, project) = run_start(
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::ImmediateExit,
            start_statuses: vec![204],
            remove_statuses: vec![500],
            readiness_exit_codes: vec![],
            readiness_create_errors: 0,
            logs: vec![],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
        false,
        false,
        2,
        false,
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "startup_exit");
    assert!(!stderr.contains("remove failed by test"));
    assert!(!stderr.contains("recovery metadata retained"));
    assert!(
        project
            .path()
            .join(".clickhouse/servers/default-pg18/data/PG_VERSION")
            .exists(),
        "PGDATA was removed while its container remained"
    );
    assert!(
        project
            .path()
            .join(".clickhouse/servers/default-pg18.json")
            .exists(),
        "incomplete cleanup did not retain recovery metadata"
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == "DELETE")
            .count(),
        1
    );
}

fn fresh_instance_dir(project: &Path) -> PathBuf {
    project.join(".clickhouse/servers/default-pg18")
}

fn metadata_path(project: &Path) -> PathBuf {
    project.join(".clickhouse/servers/default-pg18.json")
}

fn request_index(requests: &[DockerRequest], method: &str, path_fragment: &str) -> usize {
    requests
        .iter()
        .position(|request| request.method == method && request.path.contains(path_fragment))
        .unwrap_or_else(|| panic!("missing {method} request containing {path_fragment}"))
}

#[test]
fn create_success_start_failure_rolls_back_exact_container_and_fresh_data() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(
        &socket_path,
        project.path(),
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::Running,
            start_statuses: vec![500],
            remove_statuses: vec![204],
            readiness_exit_codes: vec![],
            readiness_create_errors: 0,
            logs: vec![],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
    );

    let output = run_start_command(home.path(), project.path(), &socket_path, false, false, 2);
    let requests = docker.requests();

    assert_eq!(output.status.code(), Some(1));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "docker_error");
    assert_eq!(error["error"]["message"], "Docker operation failed");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("start failed by test"));
    let create = request_index(&requests, "POST", "/containers/create?");
    let start = request_index(&requests, "POST", "/containers/pg-id/start");
    let remove = request_index(&requests, "DELETE", "/containers/pg-id?");
    assert!(create < start && start < remove);
    assert!(!fresh_instance_dir(project.path()).exists());
    assert!(!metadata_path(project.path()).exists());
}

#[test]
fn initialization_timeout_removes_partial_pgdata() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(
        &socket_path,
        project.path(),
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::Running,
            start_statuses: vec![204],
            remove_statuses: vec![204],
            readiness_exit_codes: vec![1; 100],
            readiness_create_errors: 0,
            logs: vec!["database system is starting up".to_string()],
            write_partial_data: true,
            create_metadata_directory_on_start: false,
        },
    );

    let output = run_start_command(home.path(), project.path(), &socket_path, false, false, 1);
    let requests = docker.requests();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "startup_timeout");
    assert_eq!(
        error["error"]["message"],
        "Postgres server 'default' did not become ready within 1 seconds"
    );
    assert!(!stderr.contains("database system is starting up"));
    request_index(&requests, "DELETE", "/containers/pg-id?");
    assert!(!fresh_instance_dir(project.path()).exists());
    assert!(!metadata_path(project.path()).exists());
}

#[test]
fn metadata_failure_uses_the_fresh_start_rollback() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");

    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(
        &socket_path,
        project.path(),
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::Running,
            start_statuses: vec![204],
            remove_statuses: vec![204],
            readiness_exit_codes: vec![],
            readiness_create_errors: 0,
            logs: vec![],
            write_partial_data: true,
            create_metadata_directory_on_start: true,
        },
    );

    let output = run_start_command(home.path(), project.path(), &socket_path, false, false, 2);
    let requests = docker.requests();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "io_error");
    assert!(!stderr.contains("Failed to durably update server metadata"));
    assert!(!stderr.contains("failed to remove metadata"));
    request_index(&requests, "DELETE", "/containers/pg-id?");
    assert!(readiness_requests(&requests).is_empty());
    assert!(!fresh_instance_dir(project.path()).exists());
    assert!(metadata_path(project.path()).is_dir());
}

#[test]
fn retry_after_rolled_back_start_failure_succeeds_cleanly() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(
        &socket_path,
        project.path(),
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::Running,
            start_statuses: vec![500, 204],
            remove_statuses: vec![204],
            readiness_exit_codes: vec![0],
            readiness_create_errors: 0,
            logs: vec![],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
    );

    let first = run_start_command(home.path(), project.path(), &socket_path, false, false, 2);
    assert_eq!(first.status.code(), Some(1));
    assert!(!fresh_instance_dir(project.path()).exists());

    let second = run_start_command(home.path(), project.path(), &socket_path, false, false, 2);
    let requests = docker.requests();

    assert!(
        second.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == "POST"
                && request.path.starts_with("/containers/create?")
                && request.body.contains(r#""Image":"postgres:18""#))
            .count(),
        2
    );
    assert!(fresh_instance_dir(project.path()).exists());
    assert!(metadata_path(project.path()).is_file());
}

#[test]
fn resume_failure_preserves_existing_container_metadata_and_data() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    write_resumed_server(project.path());
    let marker = fresh_instance_dir(project.path()).join("data/user-data");
    std::fs::create_dir_all(marker.parent().unwrap()).expect("create resumed data directory");
    std::fs::write(&marker, "keep me").expect("write resumed data marker");

    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(
        &socket_path,
        project.path(),
        DockerScenario {
            existing: true,
            outcome: ContainerOutcome::ImmediateExit,
            start_statuses: vec![204],
            remove_statuses: vec![],
            readiness_exit_codes: vec![],
            readiness_create_errors: 0,
            logs: vec!["resume failed".to_string()],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
    );

    let output = run_start_command(home.path(), project.path(), &socket_path, true, false, 2);
    let requests = docker.requests();

    assert_eq!(output.status.code(), Some(1));
    assert!(marker.is_file());
    assert!(metadata_path(project.path()).is_file());
    assert!(requests.iter().any(|request| {
        request.method == "POST" && request.path.starts_with("/containers/pg-id/stop?")
    }));
    assert!(!requests.iter().any(|request| request.method == "DELETE"));
}

#[test]
fn cleanup_failure_preserves_rollback_behavior_but_redacts_json_diagnostics() {
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = tempfile::tempdir().expect("create project tempdir");
    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(
        &socket_path,
        project.path(),
        DockerScenario {
            existing: false,
            outcome: ContainerOutcome::Running,
            start_statuses: vec![500],
            remove_statuses: vec![500],
            readiness_exit_codes: vec![],
            readiness_create_errors: 0,
            logs: vec![],
            write_partial_data: false,
            create_metadata_directory_on_start: false,
        },
    );

    let output = run_start_command(home.path(), project.path(), &socket_path, false, false, 2);
    let requests = docker.requests();

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "docker_error");
    assert_eq!(error["error"]["message"], "Docker operation failed");
    assert!(!stderr.contains("start failed by test"));
    assert!(!stderr.contains("Postgres startup rollback incomplete"));
    assert!(!stderr.contains("remove failed by test"));
    request_index(&requests, "DELETE", "/containers/pg-id?");
    assert!(fresh_instance_dir(project.path()).exists());
    assert!(metadata_path(project.path()).is_file());
}
