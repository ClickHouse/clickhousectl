//! Concurrency coverage for Postgres start's per-instance lifecycle lock.

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn write_response(stream: &mut UnixStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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

fn expect_request(listener: &UnixListener, operation: &str, prefix: &str) -> UnixStream {
    let mut stream = accept_connection(listener, operation);
    let request = read_request(&mut stream);
    assert!(
        request.starts_with(prefix),
        "unexpected Docker {operation} request: {request}"
    );
    stream
}

fn spawn_fake_docker(
    socket_path: &Path,
    pull_started: SyncSender<()>,
    release_pull: Receiver<()>,
) -> JoinHandle<()> {
    let listener = UnixListener::bind(socket_path).expect("bind fake Docker socket");
    listener
        .set_nonblocking(true)
        .expect("make fake Docker socket nonblocking");

    thread::spawn(move || {
        let mut recovery_ping = expect_request(&listener, "recovery ping", "GET /_ping ");
        write_response(&mut recovery_ping, "200 OK", "text/plain", "OK");

        let mut recovery_list = expect_request(
            &listener,
            "recovery container list",
            "GET /containers/json?",
        );
        write_response(&mut recovery_list, "200 OK", "application/json", "[]");

        let mut start_ping = expect_request(&listener, "start ping", "GET /_ping ");
        write_response(&mut start_ping, "200 OK", "text/plain", "OK");

        let mut image_inspect =
            expect_request(&listener, "image inspect", "GET /images/postgres:18/json ");
        write_response(
            &mut image_inspect,
            "404 Not Found",
            "application/json",
            r#"{"message":"No such image"}"#,
        );

        let mut pull = expect_request(&listener, "image pull", "POST /images/create?");
        pull_started.send(()).expect("signal image pull");
        release_pull
            .recv_timeout(Duration::from_secs(5))
            .expect("release image pull");
        write_response(
            &mut pull,
            "200 OK",
            "application/json",
            "{\"status\":\"Pull complete\"}\n",
        );
    })
}

fn try_lock_instance(path: &Path) -> std::io::Result<File> {
    std::fs::create_dir_all(path.parent().expect("lock parent"))?;
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)?;
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        Ok(file)
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn unlock_instance(file: &File) {
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
    assert_eq!(
        result,
        0,
        "unlock instance: {}",
        std::io::Error::last_os_error()
    );
}

fn wait_for_output(mut child: Child) -> Output {
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
                "clickhousectl timed out\nstderr: {}\nstdout: {}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn postgres_start_pulls_before_lock_and_revalidates_before_create() {
    let project = tempfile::tempdir().expect("create project");
    let socket_path = project.path().join("docker.sock");
    let (pull_started_tx, pull_started_rx) = mpsc::sync_channel(0);
    let (release_pull_tx, release_pull_rx) = mpsc::sync_channel(0);
    let daemon = spawn_fake_docker(&socket_path, pull_started_tx, release_pull_rx);

    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project.path())
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
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("run clickhousectl");

    pull_started_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("wait for image pull");

    let lock_path = project
        .path()
        .join(".clickhouse/servers/.locks/default-pg18.lock");
    let instance_lock = match try_lock_instance(&lock_path) {
        Ok(lock) => lock,
        Err(error) => {
            release_pull_tx.send(()).expect("release image pull");
            child.kill().expect("kill clickhousectl");
            child.wait().expect("reap clickhousectl");
            daemon.join().expect("fake Docker daemon");
            panic!("instance lock was held during image pull: {error}");
        }
    };

    let metadata_path = project.path().join(".clickhouse/servers/default-pg18.json");
    let metadata = serde_json::json!({
        "name": "default-pg18",
        "pid": 0,
        "version": "postgres:18",
        "http_port": 0,
        "tcp_port": 5432,
        "started_at": "concurrent",
        "cwd": project.path().canonicalize().unwrap(),
        "engine": "postgres"
    });
    std::fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&metadata).unwrap(),
    )
    .expect("write concurrent metadata");

    release_pull_tx.send(()).expect("release image pull");
    thread::sleep(Duration::from_millis(250));
    assert!(
        child.try_wait().expect("poll blocked start").is_none(),
        "start did not wait for the lifecycle lock before revalidation"
    );

    unlock_instance(&instance_lock);
    let output = wait_for_output(child);
    daemon.join().expect("fake Docker daemon");

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("has metadata but the container is gone"),
        "start did not revalidate concurrent metadata: {stderr}"
    );
    assert!(metadata_path.exists(), "concurrent metadata was mutated");
}
