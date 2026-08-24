//! Subprocess coverage that keeps invalid Postgres starts away from Docker.

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

fn read_request(stream: &mut UnixStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set fake Docker read timeout");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let bytes = stream.read(&mut buffer).expect("read Docker request");
        assert!(bytes > 0, "Docker request ended before its headers");
        request.extend_from_slice(&buffer[..bytes]);
    }
    String::from_utf8(request).expect("Docker request is UTF-8")
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
        let daemon_requests = Arc::clone(&requests);
        let daemon_stop = Arc::clone(&stop);
        let daemon = thread::spawn(move || {
            while !daemon_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        daemon_requests.fetch_add(1, Ordering::SeqCst);
                        let request = read_request(&mut stream);
                        if request.contains("/_ping ") {
                            write_response(&mut stream, "text/plain", "OK");
                        } else if request.contains("/containers/json") {
                            write_response(&mut stream, "application/json", "[]");
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
