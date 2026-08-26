//! Subprocess coverage for local Postgres start preflight validation.

use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
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
        let thread_requests = Arc::clone(&requests);
        let thread_stop = Arc::clone(&stop);
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
                thread_requests.fetch_add(1, Ordering::Relaxed);
                stream
                    .set_read_timeout(Some(Duration::from_millis(250)))
                    .expect("set fake Docker read timeout");
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(bytes) => request.extend_from_slice(&buffer[..bytes]),
                        Err(error)
                            if matches!(
                                error.kind(),
                                ErrorKind::WouldBlock | ErrorKind::TimedOut
                            ) =>
                        {
                            break;
                        }
                        Err(error) => panic!("read fake Docker request: {error}"),
                    }
                }
                let ping = request.starts_with(b"GET /_ping ");
                let (content_type, body) = if ping {
                    ("text/plain", "OK")
                } else {
                    ("application/json", "[]")
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
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
