//! Subprocess coverage for local Postgres start preflight validation.

use serde_json::{Value, json};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn read_request(stream: &mut UnixStream) -> Option<String> {
    stream
        .set_nonblocking(false)
        .expect("make fake Docker connection blocking");
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set fake Docker read timeout");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let bytes = match stream.read(&mut buffer) {
            Ok(bytes) => bytes,
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return None;
            }
            Err(error) => panic!("read fake Docker request: {error}"),
        };
        if bytes == 0 {
            return None;
        }
        request.extend_from_slice(&buffer[..bytes]);
    }
    Some(String::from_utf8(request).expect("Docker request is UTF-8"))
}

fn write_response(stream: &mut UnixStream, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write fake Docker response");
}

struct FakeDocker {
    requests: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FakeDocker {
    fn start(socket_path: &Path) -> Self {
        let listener = UnixListener::bind(socket_path).expect("bind fake Docker socket");
        listener
            .set_nonblocking(true)
            .expect("make fake Docker socket nonblocking");
        let requests = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let started = Arc::new(AtomicBool::new(false));
        let thread_requests = Arc::clone(&requests);
        let thread_stop = Arc::clone(&stop);
        let thread_started = Arc::clone(&started);
        let thread = thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("accept fake Docker request: {error}"),
                };
                let Some(request) = read_request(&mut stream) else {
                    continue;
                };
                thread_requests.fetch_add(1, Ordering::Relaxed);
                if request.contains("/_ping ") {
                    write_response(&mut stream, "text/plain", "OK");
                } else if request.contains("/containers/json") {
                    write_response(&mut stream, "application/json", "[]");
                } else if request.contains("/containers/existing-container/start") {
                    thread_started.store(true, Ordering::Relaxed);
                    write_response(&mut stream, "application/json", "");
                } else if request.contains("/containers/existing-container/json") {
                    let body = json!({
                        "Id": "existing-container",
                        "Config": {
                            "Env": [
                                "POSTGRES_USER=stored-user",
                                "POSTGRES_PASSWORD=stored-password",
                                "POSTGRES_DB=stored-database"
                            ]
                        },
                        "State": {
                            "Running": thread_started.load(Ordering::Relaxed)
                        }
                    });
                    write_response(&mut stream, "application/json", &body.to_string());
                } else {
                    write_response(&mut stream, "application/json", "{}");
                }
            }
        });
        Self {
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.load(Ordering::Relaxed)
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

fn run_invalid_start(args: &[&str]) -> (Output, usize, bool) {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(&socket_path);
    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home.path())
        .env("DOCKER_HOST", format!("unix://{}", socket_path.display()))
        .current_dir(project.path())
        .args(["local", "--json", "postgres", "start"])
        .args(args)
        .output()
        .expect("run clickhousectl");
    let requests = docker.request_count();
    let project_state_created = project.path().join(".clickhouse").exists();
    (output, requests, project_state_created)
}

fn write_stopped_postgres_metadata(project: &Path, port: u16) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).expect("create servers directory");
    std::fs::write(
        servers.join("default-pg18.json"),
        serde_json::to_vec_pretty(&json!({
            "name": "default-pg18",
            "pid": 0,
            "version": "postgres:18",
            "http_port": 0,
            "tcp_port": port,
            "started_at": "1700000000",
            "cwd": project.display().to_string(),
            "engine": "postgres",
            "container_id": "existing-container"
        }))
        .unwrap(),
    )
    .expect("write Postgres metadata");
}

fn run_resume(project: &Path, home: &Path, socket_path: &Path, args: &[&str]) -> Output {
    Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("DOCKER_HOST", format!("unix://{}", socket_path.display()))
        .current_dir(project)
        .args(["local", "--json", "postgres", "start"])
        .args(args)
        .output()
        .expect("run clickhousectl")
}

#[test]
fn invalid_start_inputs_make_zero_docker_requests_or_project_state() {
    for args in [
        vec!["--name", "../unsafe"],
        vec!["--version", "18garbage"],
        vec!["--port", "0"],
        vec!["--env", "NO_EQUALS"],
        vec!["--env", "POSTGRES_USER=admin"],
        vec!["--env", "APP_MODE=dev", "--env", "APP_MODE=test"],
        vec![
            "--password",
            "from-flag",
            "--env",
            "POSTGRES_PASSWORD=from-env",
        ],
        vec![
            "--env",
            "POSTGRES_PASSWORD=first",
            "--env",
            "POSTGRES_PASSWORD=second",
        ],
    ] {
        let (output, requests, project_state_created) = run_invalid_start(&args);
        assert!(
            !output.status.success(),
            "arguments unexpectedly passed: {args:?}"
        );
        assert_eq!(
            output.status.code(),
            Some(2),
            "wrong exit code for {args:?}"
        );
        assert_eq!(requests, 0, "Docker was contacted for {args:?}");
        assert!(
            !project_state_created,
            "project state was created for {args:?}"
        );
    }
}

#[test]
fn bound_explicit_port_fails_locally_without_docker_or_project_state() {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind occupied port");
    let port = listener.local_addr().expect("occupied port address").port();
    let port_arg = port.to_string();

    let (output, requests, project_state_created) = run_invalid_start(&["--port", &port_arg]);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains(&format!("Postgres port {port} is already in use")));
    assert!(stderr.contains("omit --port to auto-select a free port"));
    assert_eq!(requests, 0);
    assert!(!project_state_created);
}

#[test]
fn exhausted_auto_port_range_does_not_block_resume() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let stored_port = 6543;
    write_stopped_postgres_metadata(project.path(), stored_port);

    let socket_path = home.path().join("docker.sock");
    let docker = FakeDocker::start(&socket_path);
    let _listeners: Vec<_> = (5432..=5532)
        .filter_map(
            |port| match std::net::TcpListener::bind(("127.0.0.1", port)) {
                Ok(listener) => Some(listener),
                Err(error) if error.kind() == ErrorKind::AddrInUse => None,
                Err(error) => panic!("bind Postgres port {port}: {error}"),
            },
        )
        .collect();

    let output = run_resume(project.path(), home.path(), &socket_path, &[]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(docker.request_count() > 0, "resume did not contact Docker");
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse start JSON");
    assert_eq!(body["port"], stored_port);
    assert_eq!(body["container_id"], "existing-container");
}

#[test]
fn password_env_override_reports_stored_settings_on_resume() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    write_stopped_postgres_metadata(project.path(), 6543);

    let socket_path = home.path().join("docker.sock");
    let _docker = FakeDocker::start(&socket_path);
    let output = run_resume(
        project.path(),
        home.path(),
        &socket_path,
        &["--env", "POSTGRES_PASSWORD=ignored"],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains("resuming with stored settings"), "{stderr}");
}
