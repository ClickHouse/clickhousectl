//! Isolated subprocess coverage for deliberate omitted server selection.

use serde_json::{Value, json};
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
        .env("PATH", "/usr/bin:/bin")
        .current_dir(project)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn body(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("parse JSON output")
}

fn create_stopped_server(project: &Path, name: &str) -> PathBuf {
    let servers = project.join(".clickhouse/servers");
    let directory = servers.join(name);
    std::fs::create_dir_all(directory.join("data")).expect("create server data directory");
    std::fs::write(
        servers.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&json!({
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
    directory
}

fn install_fake_clickhouse(home: &Path) {
    let binary = home.join(format!(".clickhouse/versions/{VERSION}/clickhouse"));
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version directory");
    std::fs::write(
        &binary,
        b"#!/bin/sh\ntrap 'exit 0' TERM INT\nwhile :; do sleep 1; done\n",
    )
    .expect("write fake ClickHouse");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
        .expect("make fake ClickHouse executable");
}

fn unused_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .expect("bind temporary port")
        .local_addr()
        .unwrap()
        .port()
}

struct ProcessGuard {
    pid: u32,
    active: bool,
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
fn omitted_stop_is_a_human_and_json_noop_with_zero_servers() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    let json_output = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert_success(&json_output);
    assert_eq!(
        body(&json_output),
        json!({
            "stopped": false,
            "selection": "implicit",
            "reason": "no_clickhouse_servers"
        })
    );

    let human = run(project.path(), home.path(), &["local", "server", "stop"]);
    assert_success(&human);
    assert_eq!(
        String::from_utf8_lossy(&human.stdout),
        "No ClickHouse servers found; nothing to stop\n"
    );
    assert!(human.stderr.is_empty());
}

#[test]
fn omitted_stop_is_a_noop_with_only_a_postgres_data_remnant() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let postgres_data = project
        .path()
        .join(".clickhouse/servers/analytics-pg18/data");
    std::fs::create_dir_all(&postgres_data).expect("create Postgres data remnant");

    let stop = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );

    assert_success(&stop);
    assert_eq!(
        body(&stop),
        json!({
            "stopped": false,
            "selection": "implicit",
            "reason": "no_clickhouse_servers"
        })
    );
    assert!(postgres_data.is_dir());
}

#[test]
fn omitted_stop_selects_the_sole_running_then_stopped_custom_server() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    install_fake_clickhouse(home.path());
    let http_port = unused_port().to_string();
    let tcp_port = unused_port().to_string();

    let start = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "server",
            "start",
            "dev",
            "--version",
            VERSION,
            "--http-port",
            &http_port,
            "--tcp-port",
            &tcp_port,
            "--no-wait",
        ],
    );
    assert_success(&start);
    let pid = body(&start)["pid"].as_u64().expect("server PID") as u32;
    let mut process = ProcessGuard { pid, active: true };

    let stop = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert_success(&stop);
    process.active = false;
    let stop_body = body(&stop);
    assert_eq!(stop_body["name"], "dev");
    assert_eq!(stop_body["already_stopped"], false);
    assert_eq!(stop_body["selection"], "implicit");

    let metadata = project.path().join(".clickhouse/servers/dev.json");
    let saved: Value = serde_json::from_slice(&std::fs::read(&metadata).unwrap()).unwrap();
    assert_eq!(saved["pid"], 0);
    assert_eq!(saved["version"], "");
    assert!(project.path().join(".clickhouse/servers/dev/data").is_dir());

    let second = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert_success(&second);
    let second_body = body(&second);
    assert_eq!(second_body["name"], "dev");
    assert_eq!(second_body["already_stopped"], true);
    assert_eq!(second_body["selection"], "implicit");
    assert!(metadata.exists(), "idempotent stop must retain identity");
}

#[test]
fn omitted_stop_recognizes_a_sole_legacy_stopped_data_directory() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let legacy = project.path().join(".clickhouse/servers/legacy/data");
    std::fs::create_dir_all(&legacy).expect("create legacy data directory");

    let stop = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );

    assert_success(&stop);
    let stop_body = body(&stop);
    assert_eq!(stop_body["name"], "legacy");
    assert_eq!(stop_body["already_stopped"], true);
    assert_eq!(stop_body["selection"], "implicit");
    assert!(legacy.is_dir());
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/legacy.json")
            .exists()
    );
}

#[test]
fn omitted_commands_prefer_default_without_touching_custom_servers() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let default = create_stopped_server(project.path(), "default");
    let custom = create_stopped_server(project.path(), "dev");

    let stop = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert_success(&stop);
    assert_eq!(body(&stop)["name"], "default");
    assert_eq!(body(&stop)["selection"], "implicit");
    assert!(default.exists());
    assert!(custom.exists());

    let remove = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "remove"],
    );
    assert_success(&remove);
    assert_eq!(body(&remove)["name"], "default");
    assert_eq!(body(&remove)["selection"], "implicit");
    assert!(!default.exists());
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/default.json")
            .exists()
    );
    assert!(custom.exists());
    assert!(project.path().join(".clickhouse/servers/dev.json").exists());
}

#[test]
fn omitted_stop_requires_a_name_or_stop_all_for_many_non_default_servers() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let alpha = create_stopped_server(project.path(), "alpha");
    let beta = create_stopped_server(project.path(), "beta");

    let ambiguous = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert_eq!(ambiguous.status.code(), Some(1));
    assert!(ambiguous.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&ambiguous.stderr).unwrap(),
        json!({
            "error": {
                "code": "server_selection_required",
                "message": "Multiple non-default ClickHouse servers exist (available: 2); specify a name or use stop-all",
                "command": "clickhousectl local server list"
            }
        })
    );

    let human = run(project.path(), home.path(), &["local", "server", "stop"]);
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(stderr.contains("multiple non-default ClickHouse servers exist (available: 2)"));
    assert!(stderr.contains("clickhousectl local server stop <name>"));
    assert!(stderr.contains("clickhousectl local server stop-all"));
    assert!(alpha.exists());
    assert!(beta.exists());

    for command in ["stop", "remove"] {
        let explicit_unknown = run(
            project.path(),
            home.path(),
            &["local", "--json", "server", command, "missing"],
        );
        assert_eq!(explicit_unknown.status.code(), Some(1));
        let error: Value = serde_json::from_slice(&explicit_unknown.stderr).unwrap();
        assert_eq!(error["error"]["code"], "server_not_found");
        assert_eq!(
            error["error"]["message"],
            "Server 'missing' was not found in the current project"
        );
    }
}

#[test]
fn omitted_remove_never_selects_custom_servers() {
    for names in [&[][..], &["dev"][..], &["alpha", "beta"][..]] {
        let project = tempfile::tempdir().expect("create project");
        let home = tempfile::tempdir().expect("create home");
        for name in names {
            create_stopped_server(project.path(), name);
        }

        let remove = run(
            project.path(),
            home.path(),
            &["local", "--json", "server", "remove"],
        );
        assert_eq!(remove.status.code(), Some(1));
        assert!(remove.stdout.is_empty());
        let error: Value = serde_json::from_slice(&remove.stderr).unwrap();
        assert_eq!(error["error"]["code"], "server_selection_required");
        assert_eq!(
            error["error"]["message"],
            format!(
                "The default ClickHouse server does not exist (custom ClickHouse servers available: {}); no server was removed",
                names.len()
            )
        );
        assert_eq!(error["error"]["command"], "clickhousectl local server list");
        for name in names {
            assert!(
                project
                    .path()
                    .join(".clickhouse/servers")
                    .join(name)
                    .exists()
            );
        }
    }

    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let custom = create_stopped_server(project.path(), "dev");
    let human = run(project.path(), home.path(), &["local", "server", "remove"]);
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(stderr.contains("custom ClickHouse servers available: 1"));
    assert!(stderr.contains("clickhousectl local server list"));
    assert!(stderr.contains("clickhousectl local server remove <name>"));
    assert!(custom.exists());

    let explicit = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "remove", "--name", "dev"],
    );
    assert_success(&explicit);
    assert_eq!(body(&explicit)["name"], "dev");
    assert_eq!(body(&explicit)["selection"], "explicit");
    assert!(!custom.exists());
}
