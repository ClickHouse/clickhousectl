//! Concurrency coverage for Postgres dotenv's per-instance lifecycle lock.

use serde_json::json;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
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

fn write_response(stream: &mut UnixStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write fake Docker response");
}

struct ReleaseGate {
    released: Mutex<bool>,
    ready: Condvar,
}

impl ReleaseGate {
    fn new() -> Self {
        Self {
            released: Mutex::new(false),
            ready: Condvar::new(),
        }
    }

    fn wait(&self) {
        let mut released = self.released.lock().expect("lock release gate");
        while !*released {
            released = self.ready.wait(released).expect("wait for release");
        }
    }

    fn release(&self) {
        *self.released.lock().expect("lock release gate") = true;
        self.ready.notify_all();
    }
}

struct FakeDocker {
    credential_release: Arc<ReleaseGate>,
    removed: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    daemon: JoinHandle<()>,
}

impl FakeDocker {
    fn spawn(
        socket_path: &Path,
        credential_started: SyncSender<()>,
        second_recovery_finished: SyncSender<()>,
        container_removed: SyncSender<()>,
    ) -> Self {
        let listener = UnixListener::bind(socket_path).expect("bind fake Docker socket");
        listener
            .set_nonblocking(true)
            .expect("make fake Docker socket nonblocking");

        let credential_release = Arc::new(ReleaseGate::new());
        let removed = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let daemon_release = Arc::clone(&credential_release);
        let daemon_removed = Arc::clone(&removed);
        let daemon_stop = Arc::clone(&stop);
        let daemon = thread::spawn(move || {
            let inspect_count = Arc::new(AtomicUsize::new(0));
            let list_count = Arc::new(AtomicUsize::new(0));
            let mut handlers = Vec::new();

            while !daemon_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .expect("make Docker connection blocking");
                        let request = read_request(&mut stream);
                        let inspect_count = Arc::clone(&inspect_count);
                        let list_count = Arc::clone(&list_count);
                        let credential_release = Arc::clone(&daemon_release);
                        let removed = Arc::clone(&daemon_removed);
                        let credential_started = credential_started.clone();
                        let second_recovery_finished = second_recovery_finished.clone();
                        let container_removed = container_removed.clone();

                        handlers.push(thread::spawn(move || {
                            if request.contains("/_ping ") {
                                write_response(&mut stream, "200 OK", "text/plain", "OK");
                            } else if request.contains("/containers/json?") {
                                let list = list_count.fetch_add(1, Ordering::SeqCst);
                                write_response(&mut stream, "200 OK", "application/json", "[]");
                                if list == 1 {
                                    let _ = second_recovery_finished.send(());
                                }
                            } else if request.contains("GET /containers/existing-container/json ") {
                                let inspect = inspect_count.fetch_add(1, Ordering::SeqCst);
                                if inspect == 1 {
                                    let _ = credential_started.send(());
                                    credential_release.wait();
                                    if removed.load(Ordering::SeqCst) {
                                        write_response(
                                            &mut stream,
                                            "404 Not Found",
                                            "application/json",
                                            r#"{"message":"No such container"}"#,
                                        );
                                    } else {
                                        write_container_inspect(&mut stream, true);
                                    }
                                } else {
                                    write_container_inspect(&mut stream, inspect == 0);
                                }
                            } else if request.contains("POST /containers/existing-container/stop") {
                                write_response(&mut stream, "204 No Content", "text/plain", "");
                            } else if request.contains("DELETE /containers/existing-container?") {
                                removed.store(true, Ordering::SeqCst);
                                write_response(&mut stream, "204 No Content", "text/plain", "");
                                let _ = container_removed.send(());
                            } else {
                                panic!("unexpected Docker request: {request}");
                            }
                        }));
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept fake Docker connection: {error}"),
                }
            }

            for handler in handlers {
                handler.join().expect("fake Docker request handler");
            }
        });

        Self {
            credential_release,
            removed,
            stop,
            daemon,
        }
    }

    fn release_credentials(&self) {
        self.credential_release.release();
    }

    fn finish(self) {
        assert!(
            self.removed.load(Ordering::SeqCst),
            "serialized remove did not remove the container"
        );
        self.stop.store(true, Ordering::SeqCst);
        self.daemon.join().expect("fake Docker daemon");
    }
}

fn write_container_inspect(stream: &mut UnixStream, running: bool) {
    let body = json!({
        "Id": "existing-container",
        "Config": {
            "Env": [
                "POSTGRES_USER=stored-user",
                "POSTGRES_PASSWORD=stored-password",
                "POSTGRES_DB=stored-database"
            ]
        },
        "State": { "Running": running }
    });
    write_response(stream, "200 OK", "application/json", &body.to_string());
}

fn command(project: &Path, socket_path: &Path) -> Command {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project)
        .env("DOCKER_HOST", format!("unix://{}", socket_path.display()))
        .current_dir(project)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
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

fn write_postgres_metadata(project: &Path) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(servers.join("default-pg18/data"))
        .expect("create Postgres data directory");
    std::fs::write(
        servers.join("default-pg18.json"),
        serde_json::to_vec_pretty(&json!({
            "name": "default-pg18",
            "pid": 0,
            "version": "postgres:18",
            "http_port": 0,
            "tcp_port": 5432,
            "started_at": "1700000000",
            "cwd": project.canonicalize().unwrap(),
            "engine": "postgres",
            "container_id": "existing-container"
        }))
        .unwrap(),
    )
    .expect("write Postgres metadata");
}

fn receive_with_timeout(receiver: &Receiver<()>, operation: &str) {
    receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_else(|error| panic!("wait for {operation}: {error}"));
}

#[test]
fn postgres_dotenv_holds_lock_through_container_credential_read() {
    let project = tempfile::tempdir().expect("create project");
    write_postgres_metadata(project.path());
    let original_dotenv =
        "APP_SETTING=keep\nPOSTGRES_USER=old-user\nPOSTGRES_PASSWORD=old-password\n";
    std::fs::write(project.path().join(".env"), original_dotenv).expect("write original .env");

    let socket_path = project.path().join("docker.sock");
    let (credential_started_tx, credential_started_rx) = mpsc::sync_channel(1);
    let (remove_recovery_tx, remove_recovery_rx) = mpsc::sync_channel(1);
    let (container_removed_tx, container_removed_rx) = mpsc::sync_channel(1);
    let docker = FakeDocker::spawn(
        &socket_path,
        credential_started_tx,
        remove_recovery_tx,
        container_removed_tx,
    );

    let mut dotenv_command = command(project.path(), &socket_path);
    dotenv_command.args([
        "local",
        "postgres",
        "dotenv",
        "--name",
        "default",
        "--version",
        "18",
    ]);
    let dotenv = dotenv_command.spawn().expect("run postgres dotenv");
    receive_with_timeout(&credential_started_rx, "paused credential read");

    let mut remove_command = command(project.path(), &socket_path);
    remove_command.args(["local", "postgres", "remove", "default", "--version", "18"]);
    let remove = remove_command.spawn().expect("run postgres remove");
    receive_with_timeout(&remove_recovery_rx, "remove recovery");

    let removed_during_credential_read = container_removed_rx
        .recv_timeout(Duration::from_millis(500))
        .is_ok();
    docker.release_credentials();

    let dotenv_output = wait_for_output(dotenv);
    let remove_output = wait_for_output(remove);
    docker.finish();

    assert!(
        !removed_during_credential_read,
        "remove deleted the container while dotenv was reading credentials"
    );
    assert!(
        dotenv_output.status.success(),
        "dotenv stderr: {}",
        String::from_utf8_lossy(&dotenv_output.stderr)
    );
    assert!(
        remove_output.status.success(),
        "remove stderr: {}",
        String::from_utf8_lossy(&remove_output.stderr)
    );

    let dotenv = std::fs::read_to_string(project.path().join(".env")).expect("read .env");
    assert_eq!(
        dotenv,
        "APP_SETTING=keep\nPOSTGRES_USER=stored-user\nPOSTGRES_PASSWORD=stored-password\nPOSTGRES_HOST=127.0.0.1\nPOSTGRES_PORT=5432\nPOSTGRES_DATABASE=stored-database\n"
    );
    assert!(!dotenv.contains("POSTGRES_PASSWORD=\"\""));
}
