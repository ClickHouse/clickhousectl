//! Isolated subprocess coverage for omitted ClickHouse server selection (#473).

use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VERSION: &str = "25.12.9.61";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn run(project: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn create_stopped_server(project: &Path, name: &str) {
    std::fs::create_dir_all(project.join(".clickhouse/servers").join(name).join("data"))
        .expect("create stopped server data dir");
}

fn create_stopped_postgres_server(project: &Path) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(servers.join("default-pg18/data"))
        .expect("create stopped Postgres data dir");
    std::fs::write(
        servers.join("default-pg18.json"),
        serde_json::to_vec(&serde_json::json!({
            "name": "default-pg18",
            "pid": 0,
            "version": "postgres:18",
            "http_port": 0,
            "tcp_port": 5432,
            "started_at": "1700000000",
            "cwd": project.display().to_string(),
            "engine": "postgres"
        }))
        .unwrap(),
    )
    .expect("write stopped Postgres metadata");
}

fn install_fake_clickhouse(home: &Path) {
    let binary = home
        .join(".clickhouse/versions")
        .join(VERSION)
        .join("clickhouse");
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

fn unused_port() -> String {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .expect("bind temporary port")
        .local_addr()
        .unwrap()
        .port()
        .to_string()
}

fn start_fake_server(project: &Path, home: &Path, name: &str) -> u32 {
    let http_port = unused_port();
    let tcp_port = unused_port();
    let output = run(
        project,
        home,
        &[
            "local",
            "--json",
            "server",
            "start",
            name,
            "--version",
            VERSION,
            "--http-port",
            &http_port,
            "--tcp-port",
            &tcp_port,
            "--no-wait",
        ],
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice::<Value>(&output.stdout).unwrap()["pid"]
        .as_u64()
        .expect("server pid") as u32
}

fn assert_json_error(output: &Output, message: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let body: Value = serde_json::from_slice(&output.stderr).expect("parse structured error");
    assert_eq!(body["error"]["code"], "server_not_found");
    assert_eq!(body["error"]["message"], message);
    assert_eq!(body["error"]["command"], "clickhousectl local server list");
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
fn omitted_stop_is_a_clear_successful_noop_with_no_servers() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    create_stopped_postgres_server(project.path());

    let json = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert!(json.status.success());
    assert!(json.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).unwrap(),
        serde_json::json!({"stopped": false, "reason": "no_servers"})
    );

    let human = run(project.path(), home.path(), &["local", "server", "stop"]);
    assert!(human.status.success());
    assert_eq!(human.stdout, b"No ClickHouse servers to stop\n");
    assert!(human.stderr.is_empty());
}

#[test]
fn omitted_stop_selects_a_sole_stopped_or_running_custom_server() {
    let stopped_project = tempfile::tempdir().expect("create stopped project tempdir");
    let stopped_home = tempfile::tempdir().expect("create stopped home tempdir");
    create_stopped_server(stopped_project.path(), "dev");

    let stopped = run(
        stopped_project.path(),
        stopped_home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert!(stopped.status.success());
    let body: Value = serde_json::from_slice(&stopped.stdout).unwrap();
    assert_eq!(body["name"], "dev");
    assert_eq!(body["already_stopped"], true);

    let running_project = tempfile::tempdir().expect("create running project tempdir");
    let running_home = tempfile::tempdir().expect("create running home tempdir");
    install_fake_clickhouse(running_home.path());
    let pid = start_fake_server(running_project.path(), running_home.path(), "dev");
    let mut guard = ProcessGuard::new(pid);

    let running = run(
        running_project.path(),
        running_home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert!(
        running.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&running.stderr)
    );
    let body: Value = serde_json::from_slice(&running.stdout).unwrap();
    assert_eq!(body["name"], "dev");
    assert_eq!(body["already_stopped"], false);
    guard.disarm();
}

#[test]
fn omitted_stop_prefers_default_and_rejects_multiple_non_default_servers() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    create_stopped_server(project.path(), "analytics");
    create_stopped_server(project.path(), "dev");

    let ambiguous = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );
    let message = "No server name was provided and multiple non-default ClickHouse servers exist. Pass a name or run `clickhousectl local server stop-all`; use `clickhousectl local server list` to see available servers.";
    assert_json_error(&ambiguous, message);

    let human = run(project.path(), home.path(), &["local", "server", "stop"]);
    assert_eq!(human.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&human.stderr),
        format!("Error: {message}\n")
    );

    create_stopped_server(project.path(), "default");
    let preferred = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert!(preferred.status.success());
    let body: Value = serde_json::from_slice(&preferred.stdout).unwrap();
    assert_eq!(body["name"], "default");
    assert_eq!(body["already_stopped"], true);
}

#[test]
fn omitted_remove_never_guesses_custom_servers() {
    let empty_project = tempfile::tempdir().expect("create empty project tempdir");
    let empty_home = tempfile::tempdir().expect("create empty home tempdir");
    let empty = run(
        empty_project.path(),
        empty_home.path(),
        &["local", "--json", "server", "remove"],
    );
    assert_json_error(
        &empty,
        "No removable 'default' ClickHouse server exists, and no custom ClickHouse servers are available. Run `clickhousectl local server list` to inspect local server state.",
    );

    let project = tempfile::tempdir().expect("create custom project tempdir");
    let home = tempfile::tempdir().expect("create custom home tempdir");
    create_stopped_server(project.path(), "dev");

    let one = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "remove"],
    );
    let message = "No removable 'default' ClickHouse server exists. Run `clickhousectl local server list`, then pass a custom server name explicitly with `clickhousectl local server remove <name>`.";
    assert_json_error(&one, message);
    assert!(project.path().join(".clickhouse/servers/dev/data").exists());

    create_stopped_server(project.path(), "analytics");
    let many = run(project.path(), home.path(), &["local", "server", "remove"]);
    assert_eq!(many.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&many.stderr),
        format!("Error: {message}\n")
    );
    assert!(project.path().join(".clickhouse/servers/dev/data").exists());
    assert!(
        project
            .path()
            .join(".clickhouse/servers/analytics/data")
            .exists()
    );

    create_stopped_server(project.path(), "default");
    let default = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "remove"],
    );
    assert!(default.status.success());
    let body: Value = serde_json::from_slice(&default.stdout).unwrap();
    assert_eq!(body["name"], "default");
    assert!(!project.path().join(".clickhouse/servers/default").exists());
    assert!(project.path().join(".clickhouse/servers/dev/data").exists());
}

#[test]
fn omitted_remove_refuses_running_default_and_explicit_unknown_stays_a_typo() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path());
    let pid = start_fake_server(project.path(), home.path(), "default");
    let mut guard = ProcessGuard::new(pid);

    let remove = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "remove"],
    );
    assert_eq!(remove.status.code(), Some(1));
    let body: Value = serde_json::from_slice(&remove.stderr).unwrap();
    assert_eq!(body["error"]["code"], "server_running");
    assert_eq!(
        body["error"]["message"],
        "Server 'default' is running; stop it first with `clickhousectl local server stop default`"
    );

    let stop = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop", "default"],
    );
    assert!(stop.status.success());
    guard.disarm();

    let unknown = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "remove", "--name", "missing"],
    );
    assert_json_error(&unknown, "Server 'missing' not found");
}
