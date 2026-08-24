//! Exact project-scope diagnostics for local server state (issue #477).

use serde_json::Value;
use std::os::unix::fs::{PermissionsExt, symlink};
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

fn install_fake_clickhouse(home: &Path) {
    let binary = home
        .join(".clickhouse/versions")
        .join(VERSION)
        .join("clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version directory");
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
            &unused_port(),
            "--tcp-port",
            &unused_port(),
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

fn create_stopped_server(project: &Path, name: &str) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(servers.join(name).join("data"))
        .expect("create stopped server data directory");
    std::fs::write(
        servers.join(format!("{name}.json")),
        serde_json::to_vec(&serde_json::json!({
            "name": name,
            "pid": 0,
            "version": "",
            "http_port": 0,
            "tcp_port": 0,
            "started_at": "1700000000",
            "cwd": project.display().to_string(),
            "engine": "clickhouse"
        }))
        .unwrap(),
    )
    .expect("write stopped server metadata");
}

fn assert_scoped_json_error(output: &Output, code: &str, project: &Path) -> Value {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let body: Value = serde_json::from_slice(&output.stderr).expect("parse structured error");
    let project = project.canonicalize().expect("canonical project");
    assert_eq!(body["error"]["code"], code);
    assert_eq!(body["error"]["project"], project.display().to_string());
    assert_eq!(
        body["error"]["command"],
        "clickhousectl local server list --global"
    );
    let message = body["error"]["message"].as_str().unwrap();
    assert!(
        message.contains(&project.display().to_string()),
        "{message}"
    );
    assert!(
        message.contains("parent `.clickhouse` directories are not searched"),
        "{message}"
    );
    assert!(
        message.contains("`clickhousectl local server list --global`"),
        "{message}"
    );
    body
}

fn assert_scoped_human_error(output: &Output, project: &Path) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let project = project.canonicalize().expect("canonical project");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("Error: "), "{stderr}");
    assert!(stderr.contains(&project.display().to_string()), "{stderr}");
    assert!(
        stderr.contains("parent `.clickhouse` directories are not searched"),
        "{stderr}"
    );
    assert!(
        stderr.contains("`clickhousectl local server list --global`"),
        "{stderr}"
    );
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
fn root_child_and_nested_state_use_only_the_exact_project() {
    let workspace = tempfile::tempdir().expect("create workspace tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let root = workspace.path().join("project");
    let child = root.join("child");
    std::fs::create_dir_all(&child).expect("create project child");
    install_fake_clickhouse(home.path());

    let pid = start_fake_server(&root, home.path(), "parent-running");
    let mut process = ProcessGuard::new(pid);
    create_stopped_server(&child, "nested-stopped");

    let list = run(&child, home.path(), &["local", "--json", "server", "list"]);
    assert!(list.status.success());
    let body: Value = serde_json::from_slice(&list.stdout).expect("parse list output");
    assert_eq!(body["total_servers"], 1);
    assert_eq!(body["servers"][0]["name"], "nested-stopped");
    assert_eq!(body["servers"][0]["running"], false);

    let child_stop = run(
        &child,
        home.path(),
        &["local", "--json", "server", "stop", "parent-running"],
    );
    assert_scoped_json_error(&child_stop, "server_not_found", &child);

    let child_remove = run(
        &child,
        home.path(),
        &["local", "server", "remove", "parent-running"],
    );
    assert_scoped_human_error(&child_remove, &child);

    let running_remove = run(
        &root,
        home.path(),
        &["local", "--json", "server", "remove", "parent-running"],
    );
    assert_scoped_json_error(&running_remove, "server_running", &root);

    let stop = run(
        &root,
        home.path(),
        &["local", "--json", "server", "stop", "parent-running"],
    );
    assert!(
        stop.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&stop.stderr)
    );
    process.disarm();

    let stopped = run(
        &root,
        home.path(),
        &["local", "--json", "server", "stop", "parent-running"],
    );
    assert!(stopped.status.success());
    let body: Value = serde_json::from_slice(&stopped.stdout).expect("parse stopped output");
    assert_eq!(body["already_stopped"], true);
}

#[test]
fn symlinked_cwd_reports_the_canonical_project_directory() {
    let workspace = tempfile::tempdir().expect("create workspace tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let project = workspace.path().join("actual-project");
    let link = workspace.path().join("project-link");
    std::fs::create_dir(&project).expect("create project");
    symlink(&project, &link).expect("create project symlink");

    let json = run(
        &link,
        home.path(),
        &["local", "--json", "server", "stop", "missing"],
    );
    assert_scoped_json_error(&json, "server_not_found", &project);

    let human = run(
        &link,
        home.path(),
        &["local", "server", "remove", "missing"],
    );
    assert_scoped_human_error(&human, &project);
    assert!(!String::from_utf8_lossy(&human.stderr).contains(&link.display().to_string()));
}

#[test]
fn list_metadata_errors_identify_the_nested_project_scope() {
    let workspace = tempfile::tempdir().expect("create workspace tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let parent = workspace.path().join("project");
    let nested = parent.join("nested");
    create_stopped_server(&parent, "parent-stopped");
    std::fs::create_dir_all(nested.join(".clickhouse/servers"))
        .expect("create nested metadata directory");
    std::fs::write(nested.join(".clickhouse/servers/broken.json"), b"{")
        .expect("write broken metadata");

    let json = run(&nested, home.path(), &["local", "--json", "server", "list"]);
    assert_scoped_json_error(&json, "server_metadata_invalid", &nested);

    let human = run(&nested, home.path(), &["local", "server", "list"]);
    assert_scoped_human_error(&human, &nested);
}
