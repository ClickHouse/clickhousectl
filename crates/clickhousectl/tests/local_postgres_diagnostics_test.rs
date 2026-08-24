//! Subprocess coverage for local Postgres and Docker diagnostics (issue #465).

use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

#[derive(Clone, Copy)]
enum PingFailure {
    PermissionDenied,
    DaemonDown,
}

struct FakeDocker {
    socket_path: PathBuf,
    stop: Arc<AtomicBool>,
    daemon: Option<JoinHandle<()>>,
}

impl FakeDocker {
    fn spawn(directory: &Path, failure: PingFailure) -> Self {
        let socket_path = directory.join("docker.sock");
        let listener = UnixListener::bind(&socket_path).expect("bind fake Docker socket");
        listener
            .set_nonblocking(true)
            .expect("make fake Docker socket nonblocking");
        let stop = Arc::new(AtomicBool::new(false));
        let daemon_stop = Arc::clone(&stop);
        let daemon = thread::spawn(move || {
            while !daemon_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .expect("make fake Docker connection blocking");
                        respond_to_ping(&mut stream, failure);
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
            stop,
            daemon: Some(daemon),
        }
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

fn respond_to_ping(stream: &mut UnixStream, failure: PingFailure) {
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
    let request = String::from_utf8(request).expect("Docker request is UTF-8");
    assert!(
        request.starts_with("GET /_ping "),
        "unexpected fake Docker request: {request}"
    );

    let (status, body) = match failure {
        PingFailure::PermissionDenied => (
            "403 Forbidden",
            r#"{"message":"permission denied while trying to connect to the Docker daemon socket"}"#,
        ),
        PingFailure::DaemonDown => (
            "503 Service Unavailable",
            r#"{"message":"Docker daemon is shutting down"}"#,
        ),
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write fake Docker response");
}

fn run_postgres_start(project: &Path, docker_host: &str) -> Output {
    Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project)
        .env("DOCKER_HOST", docker_host)
        .current_dir(project)
        .args([
            "local",
            "postgres",
            "start",
            "--name",
            "diagnostics",
            "--version",
            "18",
        ])
        .output()
        .expect("run clickhousectl")
}

fn assert_setup_guidance(stderr: &str) {
    assert!(stderr.contains("Docker setup guidance"), "stderr: {stderr}");
    assert!(stderr.contains("Docker CLI contexts"), "stderr: {stderr}");
    assert!(stderr.contains("Docker socket"), "stderr: {stderr}");
    assert!(stderr.contains("DOCKER_HOST"), "stderr: {stderr}");

    match std::env::consts::OS {
        "macos" | "windows" => assert!(stderr.contains("Docker Desktop"), "stderr: {stderr}"),
        "linux" => {
            assert!(stderr.contains("Docker Engine"), "stderr: {stderr}");
            assert!(stderr.contains("Docker Desktop"), "stderr: {stderr}");
            assert!(stderr.contains("rootless Docker"), "stderr: {stderr}");
        }
        _ => assert!(
            stderr.contains("Install and start Docker"),
            "stderr: {stderr}"
        ),
    }
}

#[test]
fn missing_docker_socket_has_constructor_guidance() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let socket_path = project.path().join("missing-docker.sock");
    let output = run_postgres_start(project.path(), &format!("unix://{}", socket_path.display()));

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("Docker socket was not found"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains(&socket_path.display().to_string()));
    assert_setup_guidance(&stderr);
}

#[test]
fn docker_socket_permission_failure_has_ping_guidance() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let docker = FakeDocker::spawn(project.path(), PingFailure::PermissionDenied);
    let output = run_postgres_start(
        project.path(),
        &format!("unix://{}", docker.socket_path.display()),
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("Permission denied while contacting the Docker daemon"),
        "stderr: {stderr}"
    );
    assert_setup_guidance(&stderr);
}

#[test]
fn daemon_down_failure_has_ping_guidance() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let docker = FakeDocker::spawn(project.path(), PingFailure::DaemonDown);
    let output = run_postgres_start(
        project.path(),
        &format!("unix://{}", docker.socket_path.display()),
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("Docker daemon is not responding at the selected endpoint"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("Docker daemon is shutting down"));
    assert_setup_guidance(&stderr);
}

#[test]
fn psql_launch_failure_is_postgres_specific() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let empty_path = project.path().join("empty-path");
    std::fs::create_dir(&empty_path).expect("create empty PATH directory");
    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project.path())
        .env("PATH", &empty_path)
        .current_dir(project.path())
        .args(["local", "postgres", "client", "--host", "127.0.0.1"])
        .output()
        .expect("run clickhousectl");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(
        stderr.contains("Postgres operation failed: could not execute psql"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains("Failed to execute ClickHouse"));
}
