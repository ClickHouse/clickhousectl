//! Regression coverage for stopped ClickHouse server metadata (issue #324).

use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn write_server_metadata(project: &Path, pid: u32) -> PathBuf {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(servers.join("default/data")).expect("create server data dir");
    let metadata = servers.join("default.json");
    std::fs::write(
        &metadata,
        serde_json::to_vec_pretty(&json!({
            "name": "default",
            "pid": pid,
            "version": "25.12.9.61",
            "http_port": 8123,
            "tcp_port": 9000,
            "started_at": "1700000000",
            "cwd": project.display().to_string(),
            "engine": "clickhouse"
        }))
        .unwrap(),
    )
    .expect("write server metadata");
    metadata
}

fn run(project: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn install_fake_clickhouse(home: &Path) {
    let binary = home.join(".clickhouse/versions/25.12.9.61/clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version dir");
    std::fs::write(
        &binary,
        b"#!/bin/sh\ntrap 'exit 0' TERM INT\nwhile :; do sleep 1; done\n",
    )
    .expect("write fake ClickHouse");
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(binary, permissions).expect("make fake ClickHouse executable");
}

fn assert_stopped_list(output: &Output) {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse list JSON");
    assert_eq!(body["total_servers"], 1);
    assert_eq!(body["total_running_servers"], 0);
    let server = &body["servers"][0];
    assert_eq!(server["name"], "default");
    assert_eq!(server["running"], false);
    assert!(server.get("pid").is_none());
    assert!(server.get("version").is_none());
    assert!(server.get("http_port").is_none());
    assert!(server.get("tcp_port").is_none());
}

struct ProcessGuard {
    pid: u32,
    active: bool,
}

impl ProcessGuard {
    fn new(pid: u32) -> Self {
        Self { pid, active: true }
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                libc::kill(self.pid as i32, libc::SIGKILL);
            }
        }
    }
}

#[test]
fn stopping_clickhouse_retains_metadata_and_lists_it_as_stopped() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path());

    let start = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "server",
            "start",
            "--version",
            "25.12.9.61",
            "--no-wait",
        ],
    );
    assert!(
        start.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&start.stderr)
    );
    let start_body: Value = serde_json::from_slice(&start.stdout).expect("parse start JSON");
    let pid = start_body["pid"].as_u64().expect("start PID") as u32;
    let mut process = ProcessGuard::new(pid);
    let metadata = project.path().join(".clickhouse/servers/default.json");

    let stop = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert!(
        stop.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&stop.stderr)
    );
    process.disarm();
    let body: Value = serde_json::from_slice(&stop.stdout).expect("parse stop JSON");
    assert_eq!(body["name"], "default");
    assert_eq!(body["already_stopped"], false);

    let saved: Value = serde_json::from_slice(&std::fs::read(&metadata).unwrap()).unwrap();
    assert_eq!(saved["pid"], 0);
    assert_eq!(saved["version"], "");
    assert_eq!(saved["http_port"], 0);
    assert_eq!(saved["tcp_port"], 0);
    assert_stopped_list(&run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "list"],
    ));
}

#[test]
fn stale_clickhouse_metadata_is_retained_as_stopped() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let metadata = write_server_metadata(project.path(), u32::MAX);

    let first = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "list"],
    );
    assert_stopped_list(&first);

    let saved: Value = serde_json::from_slice(&std::fs::read(&metadata).unwrap()).unwrap();
    assert_eq!(saved["pid"], 0);
    assert_eq!(saved["version"], "");
    assert_eq!(saved["http_port"], 0);
    assert_eq!(saved["tcp_port"], 0);

    let second = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "list"],
    );
    assert_stopped_list(&second);
    assert!(metadata.exists());
}

#[test]
fn stopped_server_connection_commands_fail_without_using_saved_ports() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    write_server_metadata(project.path(), 0);

    let client = run(
        project.path(),
        home.path(),
        &["local", "client", "--name", "default"],
    );
    assert_eq!(client.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&client.stderr).contains("Server 'default' is not running"));

    let dotenv = run(
        project.path(),
        home.path(),
        &["local", "server", "dotenv", "--name", "default"],
    );
    assert_eq!(dotenv.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&dotenv.stderr).contains("Server 'default' is not running"));
    assert!(!project.path().join(".env").exists());
}
