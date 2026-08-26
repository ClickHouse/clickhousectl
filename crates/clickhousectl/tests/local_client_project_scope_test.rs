//! Exact project-scope diagnostics for managed local clients (issue #467).

use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};

const VERSION: &str = "25.12.9.61";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn command(project: &Path, home: &Path) -> Command {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project);
    command
}

fn run(project: &Path, home: &Path, args: &[&str]) -> Output {
    command(project, home)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn write_server_metadata(project: &Path, name: &str, pid: u32, version: &str) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(servers.join(name).join("data")).expect("create server data directory");
    std::fs::write(
        servers.join(format!("{name}.json")),
        serde_json::to_vec(&serde_json::json!({
            "name": name,
            "pid": pid,
            "version": version,
            "http_port": 8123,
            "tcp_port": 9000,
            "started_at": "1700000000",
            "cwd": project.display().to_string(),
            "engine": "clickhouse"
        }))
        .unwrap(),
    )
    .expect("write server metadata");
}

struct ProcessGuard(Child);

impl ProcessGuard {
    fn id(&self) -> u32 {
        self.0.id()
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_managed_clickhouse(project: &Path, name: &str) -> ProcessGuard {
    let data = project.join(".clickhouse/servers").join(name).join("data");
    std::fs::create_dir_all(&data).expect("create server data directory");
    let binary = project.join("clickhouse");
    std::fs::write(
        &binary,
        b"#!/bin/sh\ntrap 'exit 0' TERM INT\nwhile :; do sleep 1; done\n",
    )
    .expect("write fake ClickHouse binary");
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&binary, permissions).expect("make fake ClickHouse executable");
    ProcessGuard(
        Command::new(binary)
            .current_dir(data)
            .spawn()
            .expect("spawn fake managed ClickHouse"),
    )
}

fn assert_managed_json_error(output: &Output, code: &str, project: &Path) -> Value {
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
    assert_eq!(body["error"]["mode"], "managed");
    assert_eq!(body["error"]["project"], project.display().to_string());
    assert_eq!(body["error"]["command"], "clickhousectl local server list");
    let message = body["error"]["message"].as_str().unwrap();
    assert!(message.starts_with("Managed local client:"), "{message}");
    assert!(
        message.contains(&project.display().to_string()),
        "{message}"
    );
    assert!(
        message.contains("parent `.clickhouse` directories are not searched"),
        "{message}"
    );
    assert!(
        message.contains("`clickhousectl local server list`"),
        "{message}"
    );
    assert!(
        message.contains("`clickhousectl local client --host localhost"),
        "{message}"
    );
    body
}

#[test]
fn empty_project_reports_managed_mode_scope_and_safe_recovery() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");

    let json = run(project.path(), home.path(), &["local", "--json", "client"]);
    let body = assert_managed_json_error(&json, "server_not_found", project.path());
    let message = body["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("return to that project root")
            && message.contains("`clickhousectl local server start [name]`"),
        "{message}"
    );

    let human = run(project.path(), home.path(), &["local", "client"]);
    assert_eq!(human.status.code(), Some(1));
    assert!(human.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(
        stderr.starts_with("Error: Managed local client:"),
        "{stderr}"
    );
    assert_eq!(stderr.lines().count(), 4, "human error should stay concise");
}

#[test]
fn child_of_valid_project_does_not_search_the_parent() {
    let workspace = tempfile::tempdir().expect("create workspace tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let root = workspace.path().join("project");
    let child = root.join("child");
    std::fs::create_dir_all(&child).expect("create child directory");
    write_server_metadata(&root, "default", 0, VERSION);

    let output = command(&child, home.path())
        .env("AGENT", "opencode")
        .args(["local", "client"])
        .output()
        .expect("run clickhousectl");
    let body = assert_managed_json_error(&output, "server_not_found", &child);
    assert_ne!(
        body["error"]["project"],
        root.canonicalize().unwrap().display().to_string()
    );
}

#[test]
fn explicit_wrong_name_and_stopped_metadata_have_distinct_codes() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    write_server_metadata(project.path(), "dev", 0, VERSION);

    let missing = run(
        project.path(),
        home.path(),
        &["local", "--json", "client", "--name", "wrong"],
    );
    let missing_body = assert_managed_json_error(&missing, "server_not_found", project.path());
    assert!(
        missing_body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Server 'wrong' not found")
    );

    let stopped = run(
        project.path(),
        home.path(),
        &["local", "--json", "client", "--name", "dev"],
    );
    let stopped_body = assert_managed_json_error(&stopped, "server_not_running", project.path());
    let message = stopped_body["error"]["message"].as_str().unwrap();
    assert!(message.contains("Server 'dev' is not running"), "{message}");
    assert!(
        message.contains("`clickhousectl local server start [name]`"),
        "{message}"
    );
}

#[test]
fn selected_missing_binary_keeps_version_code_and_managed_scope() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let process = spawn_managed_clickhouse(project.path(), "dev");
    write_server_metadata(project.path(), "dev", process.id(), VERSION);

    let output = run(
        project.path(),
        home.path(),
        &["local", "--json", "client", "--name", "dev"],
    );
    let body = assert_managed_json_error(&output, "version_unavailable", project.path());
    let message = body["error"]["message"].as_str().unwrap();
    assert!(message.contains(&format!("Version {VERSION} not found")));
    assert!(
        message.contains("`clickhousectl local install <version>`")
            && message.contains("--version <installed-version>"),
        "{message}"
    );
}

#[test]
fn invalid_lock_directory_keeps_managed_scope_in_json_and_human_errors() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let servers = project.path().join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).expect("create servers directory");
    std::fs::write(servers.join(".locks"), b"not a directory").expect("create invalid locks path");

    let json = run(project.path(), home.path(), &["local", "--json", "client"]);
    let body = assert_managed_json_error(&json, "server_lock", project.path());
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Could not create server lifecycle lock directory"),
        "{body}"
    );

    let human = run(project.path(), home.path(), &["local", "client"]);
    assert_eq!(human.status.code(), Some(1));
    assert!(human.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(
        stderr.starts_with(
            "Error: Managed local client: Could not create server lifecycle lock directory"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains(&project.path().canonicalize().unwrap().display().to_string()),
        "{stderr}"
    );
    assert!(
        stderr.contains("`clickhousectl local client --host localhost`"),
        "{stderr}"
    );
}
