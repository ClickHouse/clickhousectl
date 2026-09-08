//! SQL input and exit-status coverage for the managed Docker psql fallback.

use serde_json::{Value, json};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Execution {
    config: Option<Value>,
    input: Vec<u8>,
}

struct Fixture {
    project: tempfile::TempDir,
    home: tempfile::TempDir,
    execution: Arc<Mutex<Execution>>,
    stop: Arc<AtomicBool>,
    daemon: Option<JoinHandle<()>>,
}

impl Fixture {
    fn new(exit_code: i32, early_exit: bool) -> Self {
        let project = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write_metadata(project.path(), "pg-id");
        let listener = UnixListener::bind(home.path().join("docker.sock")).unwrap();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let execution = Arc::new(Mutex::new(Execution::default()));
        let thread_execution = execution.clone();
        let daemon = thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("accept: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let (path, body) = read_request(&mut stream);
                if path == "/_ping" {
                    respond(&mut stream, "OK");
                } else if path.starts_with("/containers/json") {
                    respond(&mut stream, "[]");
                } else if path == "/containers/pg-id/json" {
                    respond(
                        &mut stream,
                        r#"{"Id":"pg-id","State":{"Running":true},"Config":{"Env":["POSTGRES_USER=postgres","POSTGRES_DB=postgres"]}}"#,
                    );
                } else if path == "/containers/pg-id/exec" {
                    thread_execution.lock().unwrap().config =
                        Some(serde_json::from_slice(&body).unwrap());
                    respond(&mut stream, r#"{"Id":"psql-exec"}"#);
                } else if path == "/exec/psql-exec/start" {
                    stream.write_all(b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n").unwrap();
                    let attach = thread_execution.lock().unwrap().config.as_ref().unwrap()["AttachStdin"]
                        == true;
                    if attach && !early_exit {
                        let mut input = Vec::new();
                        stream
                            .read_to_end(&mut input)
                            .expect("SQL input must reach EOF");
                        thread_execution.lock().unwrap().input = input;
                    }
                    write_frame(&mut stream, 1, b"query output\n");
                    write_frame(&mut stream, 2, b"psql diagnostic\n");
                } else if path == "/exec/psql-exec/json" {
                    respond(
                        &mut stream,
                        &json!({"ExitCode":exit_code,"Running":false}).to_string(),
                    );
                } else {
                    panic!("unexpected Docker request: {path}");
                }
            }
        });
        Self {
            project,
            home,
            execution,
            stop,
            daemon: Some(daemon),
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = client_command(self.project.path());
        command
            .env("HOME", self.home.path())
            .env(
                "DOCKER_HOST",
                format!("unix://{}", self.home.path().join("docker.sock").display()),
            )
            .args(args);
        command
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(daemon) = self.daemon.take() {
            let result = daemon.join();
            if !std::thread::panicking() {
                result.unwrap();
            }
        }
    }
}

fn client_command(project: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clickhousectl"));
    command
        .env("DO_NOT_TRACK", "1")
        // An empty directory forces fallback regardless of the host's tools.
        .env("PATH", project.join("empty-path"))
        .current_dir(project)
        .args(["local", "postgres", "client"]);
    command
}

fn write_metadata(project: &Path, container: &str) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).unwrap();
    std::fs::create_dir_all(project.join("empty-path")).unwrap();
    std::fs::write(
        servers.join("default-pg18.json"),
        json!({
            "name":"default-pg18", "pid":0, "version":"postgres:18", "http_port":0,
            "tcp_port":5432, "started_at":"1700000000", "cwd":project,
            "engine":"postgres", "container_id":container
        })
        .to_string(),
    )
    .unwrap();
}

fn read_request(stream: &mut UnixStream) -> (String, Vec<u8>) {
    let mut headers = Vec::new();
    while !headers.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        headers.push(byte[0]);
    }
    let headers = String::from_utf8(headers).unwrap();
    let path = headers
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_string();
    let count: usize = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|s| s.trim().parse().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; count];
    stream.read_exact(&mut body).unwrap();
    (path, body)
}

fn respond(stream: &mut UnixStream, body: &str) {
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
}

fn write_frame(stream: &mut UnixStream, channel: u8, body: &[u8]) {
    stream.write_all(&[channel, 0, 0, 0]).unwrap();
    stream
        .write_all(&(body.len() as u32).to_be_bytes())
        .unwrap();
    stream.write_all(body).unwrap();
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn relative_and_absolute_host_files_stream_in_query_order() {
    for absolute in [false, true] {
        let fixture = Fixture::new(0, false);
        let sql = "INSERT INTO data VALUES ('Grüße');\n".repeat(4096);
        let file = fixture.project.path().join("input with spaces.sql");
        std::fs::write(&file, &sql).unwrap();
        let path = if absolute {
            file.to_str().unwrap()
        } else {
            "input with spaces.sql"
        };
        let output = fixture
            .command(&[
                "--query",
                "CREATE TABLE data (value text)",
                "--queries-file",
                path,
            ])
            .output()
            .unwrap();
        assert_success(&output);
        assert_eq!(output.stdout, b"query output\n");
        assert!(String::from_utf8_lossy(&output.stderr).contains("psql diagnostic"));
        let execution = fixture.execution.lock().unwrap();
        assert_eq!(execution.input, sql.as_bytes());
        let config = execution.config.as_ref().unwrap();
        assert_eq!(
            config["Cmd"],
            json!([
                "psql",
                "-U",
                "postgres",
                "-d",
                "postgres",
                "-c",
                "CREATE TABLE data (value text)",
                "-f",
                "-"
            ])
        );
        assert_eq!(config["AttachStdin"], true);
        assert_eq!(config["Tty"], false);
    }
}

#[test]
fn explicit_stdin_and_empty_input_reach_eof() {
    for sql in ["INSERT INTO data VALUES (2);\n", ""] {
        let fixture = Fixture::new(0, false);
        let mut child = fixture
            .command(&["--queries-file", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(sql.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert_success(&output);
        assert_eq!(fixture.execution.lock().unwrap().input, sql.as_bytes());
    }
}

#[test]
fn file_and_stdin_preserve_psql_error_status() {
    for stdin in [false, true] {
        let fixture = Fixture::new(3, false);
        std::fs::write(fixture.project.path().join("bad.sql"), "BAD SQL;\n").unwrap();
        let mut child = fixture
            .command(&[
                "--queries-file",
                if stdin { "-" } else { "bad.sql" },
                "--",
                "-v",
                "ON_ERROR_STOP=1",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"BAD SQL;\n")
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert!(String::from_utf8_lossy(&output.stderr).contains("psql diagnostic"));
    }
}

#[test]
fn missing_host_file_fails_before_executing_query() {
    let fixture = Fixture::new(0, false);
    let output = fixture
        .command(&[
            "--query",
            "DROP TABLE data",
            "--queries-file",
            "missing.sql",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(fixture.execution.lock().unwrap().config.is_none());
}

#[test]
fn query_only_keeps_stdin_detached() {
    let fixture = Fixture::new(0, false);
    assert_success(&fixture.command(&["--query", "SELECT 1"]).output().unwrap());
    assert_eq!(
        fixture.execution.lock().unwrap().config.as_ref().unwrap()["AttachStdin"],
        false
    );
}

#[test]
fn early_psql_exit_does_not_wait_for_open_stdin() {
    let fixture = Fixture::new(3, true);
    let mut child = fixture
        .command(&["--queries-file", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _held_open = child.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert_eq!(status.code(), Some(3));
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("psql exit blocked on an upstream pipe that stayed open");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// Exercises actual database effects. Requires Docker and postgres:18 (pulled
/// on demand); creates one uniquely named disposable container with tmpfs data.
#[test]
#[ignore = "requires a running Docker daemon"]
fn real_docker_file_and_stdin_apply_sql_and_propagate_errors() {
    struct Container(String);
    impl Drop for Container {
        fn drop(&mut self) {
            let output = Command::new("docker")
                .args(["rm", "-f", &self.0])
                .output()
                .unwrap();
            assert_success(&output);
        }
    }
    let container = Container(format!("chctl-760-{}", uuid::Uuid::new_v4()));
    let output = Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            &container.0,
            "--tmpfs",
            "/var/lib/postgresql",
            "-e",
            "POSTGRES_HOST_AUTH_METHOD=trust",
            "postgres:18",
        ])
        .output()
        .unwrap();
    assert_success(&output);
    let project = tempfile::tempdir().unwrap();
    write_metadata(project.path(), &container.0);
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if Command::new("docker")
            // The image's temporary initdb server only listens on a Unix
            // socket. Wait for TCP so we cannot race its shutdown/restart.
            .args([
                "exec",
                &container.0,
                "pg_isready",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
            ])
            .output()
            .unwrap()
            .status
            .success()
        {
            break;
        }
        assert!(Instant::now() < deadline, "Postgres readiness timeout");
        thread::sleep(Duration::from_millis(200));
    }
    let run = |args: &[&str], input: Option<&str>| {
        let mut child = client_command(project.path())
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        if let Some(input) = input {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
        }
        child.wait_with_output().unwrap()
    };
    let file = project.path().join("seed file.sql");
    std::fs::write(&file, "INSERT INTO qa_files VALUES (1);\n").unwrap();
    // The INSERT depends on the preceding --query in this same psql session.
    assert_success(&run(
        &[
            "--query",
            "CREATE TABLE qa_files (id integer)",
            "--queries-file",
            "seed file.sql",
        ],
        None,
    ));
    std::fs::write(&file, "INSERT INTO qa_files VALUES (2);\n").unwrap();
    assert_success(&run(&["--queries-file", file.to_str().unwrap()], None));
    assert_success(&run(
        &["--queries-file", "-"],
        Some("INSERT INTO qa_files VALUES (3);\n"),
    ));
    let rows = run(
        &[
            "--query",
            "SELECT string_agg(id::text, ',' ORDER BY id) FROM qa_files",
            "--",
            "-tA",
        ],
        None,
    );
    assert_success(&rows);
    assert_eq!(String::from_utf8_lossy(&rows.stdout).trim(), "1,2,3");
    for path in [file.to_str().unwrap(), "-"] {
        let bad_sql = "THIS IS INVALID SQL;\nINSERT INTO qa_files VALUES (99);\n";
        std::fs::write(&file, bad_sql).unwrap();
        let output = run(
            &["--queries-file", path, "--", "-v", "ON_ERROR_STOP=1"],
            Some(bad_sql),
        );
        assert_eq!(output.status.code(), Some(3), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("ERROR"));
    }
    let rows = run(
        &["--query", "SELECT count(*) FROM qa_files", "--", "-tA"],
        None,
    );
    assert_success(&rows);
    assert_eq!(String::from_utf8_lossy(&rows.stdout).trim(), "3");
}
