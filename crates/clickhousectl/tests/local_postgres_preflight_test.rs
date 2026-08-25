//! Subprocess coverage that keeps invalid Postgres starts away from Docker.

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
            Err(error) => panic!("read Docker request: {error}"),
        };
        assert!(bytes > 0, "Docker request ended before its headers");
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
    daemon: JoinHandle<()>,
}

impl FakeDocker {
    fn spawn(socket_path: &Path) -> Self {
        let listener = UnixListener::bind(socket_path).expect("bind fake Docker socket");
        listener
            .set_nonblocking(true)
            .expect("make fake Docker socket nonblocking");
        let requests = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let started = Arc::new(AtomicBool::new(false));
        let daemon_requests = Arc::clone(&requests);
        let daemon_stop = Arc::clone(&stop);
        let daemon_started = Arc::clone(&started);
        let daemon = thread::spawn(move || {
            while !daemon_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .expect("make Docker connection blocking");
                        let Some(request) = read_request(&mut stream) else {
                            continue;
                        };
                        daemon_requests.fetch_add(1, Ordering::SeqCst);
                        if request.contains("/_ping ") {
                            write_response(&mut stream, "text/plain", "OK");
                        } else if request.contains("/containers/json") {
                            write_response(&mut stream, "application/json", "[]");
                        } else if request.contains("/containers/existing-container/start") {
                            daemon_started.store(true, Ordering::SeqCst);
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
                                    "Running": daemon_started.load(Ordering::SeqCst)
                                }
                            });
                            write_response(&mut stream, "application/json", &body.to_string());
                        } else {
                            write_response(&mut stream, "application/json", "{}");
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
            requests,
            stop,
            daemon,
        }
    }

    fn finish(self) -> usize {
        self.stop.store(true, Ordering::SeqCst);
        self.daemon.join().expect("join fake Docker daemon");
        self.requests.load(Ordering::SeqCst)
    }
}

fn run_invalid_start(args: &[&str], expected_error: &str) -> Output {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let socket_path = tempdir.path().join("docker.sock");
    let docker = FakeDocker::spawn(&socket_path);

    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", tempdir.path())
        .env("DOCKER_HOST", format!("unix://{}", socket_path.display()))
        .current_dir(tempdir.path())
        .args(["local", "postgres", "start"])
        .args(args)
        .output()
        .expect("run clickhousectl");

    let requests = docker.finish();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains(expected_error),
        "expected '{expected_error}' in stderr: {stderr}"
    );
    assert_eq!(requests, 0, "invalid start contacted Docker");
    assert!(
        !tempdir.path().join(".clickhouse/servers").exists(),
        "invalid start created Postgres project state"
    );
    output
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

#[test]
fn invalid_definition_options_make_zero_docker_requests() {
    for (args, expected_error) in [
        (vec!["--name", "../invalid"], "Invalid server name"),
        (vec!["--version", "16"], "not supported"),
        (vec!["--version", "18garbage"], "not supported"),
        (vec!["--port", "0"], "--port 0 is not allowed"),
        (vec!["-e", "NO_EQUALS"], "expected KEY=VALUE"),
        (vec!["-e", "=value"], "KEY must not be empty"),
    ] {
        run_invalid_start(&args, expected_error);
    }
}

#[test]
fn bound_explicit_port_is_rejected_without_docker_requests() {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind temporary port");
    let port = listener.local_addr().unwrap().port().to_string();

    run_invalid_start(&["--port", &port], "already in use");
}

#[test]
fn exhausted_auto_port_range_does_not_block_resume() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let stored_port = 6543;
    write_stopped_postgres_metadata(tempdir.path(), stored_port);

    let socket_path = tempdir.path().join("docker.sock");
    let docker = FakeDocker::spawn(&socket_path);
    let _listeners: Vec<_> = (5432..=5532)
        .filter_map(
            |port| match std::net::TcpListener::bind(("127.0.0.1", port)) {
                Ok(listener) => Some(listener),
                Err(error) if error.kind() == ErrorKind::AddrInUse => None,
                Err(error) => panic!("bind Postgres port {port}: {error}"),
            },
        )
        .collect();

    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", tempdir.path())
        .env("DOCKER_HOST", format!("unix://{}", socket_path.display()))
        .current_dir(tempdir.path())
        .args(["local", "--json", "postgres", "start"])
        .output()
        .expect("run clickhousectl");

    let requests = docker.finish();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(requests > 0, "resume did not contact Docker");
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse start JSON");
    assert_eq!(body["port"], stored_port);
    assert_eq!(body["container_id"], "existing-container");
}
