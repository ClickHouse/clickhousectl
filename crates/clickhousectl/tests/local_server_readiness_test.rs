//! Regression coverage for ClickHouse startup readiness (issue #330).

use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn install_fake_clickhouse(home: &Path, script: &str) {
    let binary = home.join(".clickhouse/versions/25.12.9.61/clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version dir");
    std::fs::write(&binary, script).expect("write fake ClickHouse");
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(binary, permissions).expect("make fake ClickHouse executable");
}

fn run_start(project: &Path, home: &Path, http_port: u16, tcp_port: u16) -> Output {
    let http_port_arg = http_port.to_string();
    let tcp_port_arg = tcp_port.to_string();
    Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("FAKE_CLICKHOUSE_HTTP_PORT", &http_port_arg)
        .env("FAKE_CLICKHOUSE_PORT", tcp_port.to_string())
        .current_dir(project)
        .args([
            "local",
            "--json",
            "server",
            "start",
            "--version",
            "25.12.9.61",
            "--http-port",
            &http_port_arg,
            "--tcp-port",
            &tcp_port_arg,
        ])
        .output()
        .expect("run clickhousectl")
}

fn unused_port() -> u16 {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind temporary port");
    listener.local_addr().unwrap().port()
}

struct ProcessGuard(u32);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        unsafe {
            libc::kill(self.0 as i32, libc::SIGKILL);
        }
    }
}

#[test]
fn fake_clickhouse_process() {
    let Ok(http_port) = std::env::var("FAKE_CLICKHOUSE_HTTP_PORT") else {
        return;
    };
    let Ok(port) = std::env::var("FAKE_CLICKHOUSE_PORT") else {
        return;
    };
    std::thread::sleep(Duration::from_millis(700));
    let tcp_listener = std::net::TcpListener::bind(("127.0.0.1", port.parse::<u16>().unwrap()))
        .expect("bind fake ClickHouse port");
    std::thread::spawn(move || {
        tcp_listener.accept().expect("accept readiness probe");
        std::thread::sleep(Duration::from_secs(30));
    });
    let http_listener =
        std::net::TcpListener::bind(("127.0.0.1", http_port.parse::<u16>().unwrap()))
            .expect("bind fake ClickHouse HTTP port");
    let (mut request, _) = http_listener.accept().expect("accept HTTP health check");
    let mut buffer = [0; 1024];
    let bytes_read = request.read(&mut buffer).expect("read HTTP health check");
    assert!(bytes_read > 0, "HTTP health check should not be empty");
    request
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nOk.\n")
        .expect("write HTTP health response");
    std::thread::sleep(Duration::from_secs(30));
}

#[test]
fn background_start_waits_for_http_and_tcp_readiness() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let test_binary = std::env::current_exe().expect("locate test binary");
    install_fake_clickhouse(
        home.path(),
        &format!(
            "#!/bin/sh\nexec '{}' --exact fake_clickhouse_process --nocapture\n",
            test_binary.display()
        ),
    );
    let http_port = unused_port();
    let tcp_port = unused_port();

    let started = Instant::now();
    let output = run_start(project.path(), home.path(), http_port, tcp_port);
    let elapsed = started.elapsed();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        elapsed >= Duration::from_millis(600),
        "start returned before readiness after {elapsed:?}"
    );

    let body: Value = serde_json::from_slice(&output.stdout).expect("parse start JSON");
    let pid = body["pid"].as_u64().expect("start PID") as u32;
    let _process = ProcessGuard(pid);
    std::net::TcpStream::connect(("127.0.0.1", tcp_port))
        .expect("server should accept connections after start returns");
}

#[test]
fn failed_start_captures_log_without_exposing_its_path_in_json() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(
        home.path(),
        "#!/bin/sh\nprintf 'synthetic startup failure\\n' >&2\nexit 7\n",
    );

    let output = run_start(project.path(), home.path(), unused_port(), unused_port());
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).expect("parse startup error JSON");
    assert_eq!(error["error"]["code"], "startup_exit");
    assert_eq!(
        error["error"]["message"],
        "ClickHouse server 'default' exited before becoming ready"
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("server.log"));

    let log = project
        .path()
        .join(".clickhouse/servers/default/server.log");
    assert_eq!(
        std::fs::read_to_string(log).expect("read server log"),
        "synthetic startup failure\n"
    );
    let metadata: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/servers/default.json"))
            .expect("read metadata"),
    )
    .expect("parse metadata");
    assert_eq!(metadata["pid"], 0);
}
