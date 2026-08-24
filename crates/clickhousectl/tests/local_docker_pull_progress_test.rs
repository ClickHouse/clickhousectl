//! Subprocess coverage for non-TTY Docker pull reporting.

use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
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

fn read_request(stream: &mut UnixStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let bytes = stream.read(&mut buffer).expect("read Docker request");
        assert!(bytes > 0, "Docker request ended before its headers");
        request.extend_from_slice(&buffer[..bytes]);
    }
    String::from_utf8(request).expect("Docker request is UTF-8")
}

fn accept_connection(listener: &UnixListener, operation: &str) -> UnixStream {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(false)
                    .expect("make Docker connection blocking");
                return stream;
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("accept Docker {operation}: {error}"),
        }
    }
}

fn spawn_fake_docker(socket_path: &Path, pull_response: String) -> JoinHandle<()> {
    let listener = UnixListener::bind(socket_path).expect("bind fake Docker socket");
    listener
        .set_nonblocking(true)
        .expect("make fake Docker socket nonblocking");
    thread::spawn(move || {
        let mut ping = accept_connection(&listener, "ping");
        let request = read_request(&mut ping);
        assert!(
            request.starts_with("GET /_ping "),
            "unexpected request: {request}"
        );
        write_response(&mut ping, "text/plain", "OK");

        let mut pull = accept_connection(&listener, "pull");
        let request = read_request(&mut pull);
        assert!(
            request.starts_with("POST /images/create?")
                && request.contains("fromImage=postgres%3A"),
            "unexpected request: {request}"
        );
        write_response(&mut pull, "application/json", &pull_response);
    })
}

fn run_install(tag: &str, pull_response: &str, json: bool) -> Output {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let socket_path = tempdir.path().join("docker.sock");
    let daemon = spawn_fake_docker(&socket_path, pull_response.to_string());

    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", tempdir.path())
        .env("DOCKER_HOST", format!("unix://{}", socket_path.display()))
        .arg("local");
    if json {
        command.arg("--json");
    }
    command
        .args(["install", &format!("postgres@{tag}"), "--force"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("run clickhousectl");
    for _ in 0..100 {
        if child.try_wait().expect("poll clickhousectl").is_some() {
            let output = child
                .wait_with_output()
                .expect("collect clickhousectl output");
            daemon.join().expect("fake Docker daemon");
            return output;
        }
        thread::sleep(Duration::from_millis(50));
    }
    child.kill().expect("kill timed out clickhousectl");
    let output = child.wait_with_output().expect("collect timed out output");
    panic!(
        "clickhousectl timed out\nstderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn non_tty_pull_collapses_repeated_layer_events() {
    let output = run_install(
        "18",
        concat!(
            "{\"status\":\"Pulling fs layer\",\"id\":\"layer-a\"}\n",
            "{\"status\":\"Downloading\",\"id\":\"layer-a\",\"progressDetail\":{\"current\":1,\"total\":10}}\n",
            "{\"status\":\"Pulling fs layer\",\"id\":\"layer-b\"}\n",
            "{\"status\":\"Extracting\",\"id\":\"layer-b\",\"progressDetail\":{\"current\":5,\"total\":10}}\n",
            "{\"status\":\"Pull complete\",\"id\":\"layer-a\"}\n",
            "{\"status\":\"Pull complete\",\"id\":\"layer-b\"}\n"
        ),
        false,
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Pulling postgres:18... done\n"
    );
}

#[test]
fn non_tty_pull_reports_failure_once_and_preserves_diagnostics() {
    let output = run_install(
        "18-missing",
        "{\"errorDetail\":{\"message\":\"manifest for postgres:18-missing not found\"}}\n",
        false,
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        stderr.starts_with("Pulling postgres:18-missing... failed\n"),
        "unexpected stderr: {stderr}"
    );
    assert!(
        stderr.contains("Docker stream error: manifest for postgres:18-missing not found"),
        "missing Docker diagnostics: {stderr}"
    );
    assert!(
        !stderr.contains("Pulling fs layer"),
        "progress leaked: {stderr}"
    );
}

#[test]
fn structured_pull_failure_is_one_redacted_error_object() {
    let output = run_install(
        "18-secret",
        "{\"errorDetail\":{\"message\":\"registry password=hunter2\"}}\n",
        true,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        concat!(
            "{\n",
            "  \"error\": {\n",
            "    \"code\": \"download_failed\",\n",
            "    \"message\": \"Download failed\",\n",
            "    \"command\": \"clickhousectl local install --help\"\n",
            "  }\n",
            "}\n"
        )
    );
}
