//! Regression coverage for atomic, concurrency-safe server metadata (issue #472).

use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const VERSION: &str = "25.12.9.61";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn command(project: &Path, home: &Path) -> Command {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .env(
            "FAKE_CLICKHOUSE_PID_FILE",
            project.join("fake-clickhouse-pids"),
        )
        .current_dir(project);
    command
}

fn run(project: &Path, home: &Path, args: &[&str]) -> Output {
    command(project, home)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn servers_dir(project: &Path) -> PathBuf {
    project.join(".clickhouse/servers")
}

fn write_metadata(project: &Path, bytes: &[u8]) -> PathBuf {
    let directory = servers_dir(project);
    std::fs::create_dir_all(directory.join("default/data")).expect("create server data dir");
    let path = directory.join("default.json");
    std::fs::write(&path, bytes).expect("write metadata");
    path
}

fn valid_metadata(project: &Path, pid: u32) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({
        "name": "default",
        "pid": pid,
        "version": VERSION,
        "http_port": 8123,
        "tcp_port": 9000,
        "started_at": "1700000000",
        "cwd": project.display().to_string(),
        "engine": "clickhouse"
    }))
    .unwrap()
}

fn install_fake_clickhouse(home: &Path) {
    let binary = home.join(format!(".clickhouse/versions/{VERSION}/clickhouse"));
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version dir");
    std::fs::write(
        &binary,
        b"#!/bin/sh\ncase \"$1\" in\nserver) printf '%s\\n' \"$$\" >> \"$FAKE_CLICKHOUSE_PID_FILE\"; trap 'exit 0' TERM INT; while :; do sleep 0.1; done ;;\nclient) exit 0 ;;\nesac\n",
    )
    .expect("write fake ClickHouse");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
        .expect("make fake ClickHouse executable");
}

struct ProcessFileGuard(PathBuf);

impl Drop for ProcessFileGuard {
    fn drop(&mut self) {
        let Ok(contents) = std::fs::read_to_string(&self.0) else {
            return;
        };
        for pid in contents.lines().filter_map(|line| line.parse::<i32>().ok()) {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }
}

#[test]
fn selected_partial_json_is_not_reported_as_not_running() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    write_metadata(project.path(), br#"{"name":"default","pid":123"#);

    let output = run(
        project.path(),
        home.path(),
        &["local", "client", "--name", "default"],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The parser's own diagnostic reaches the human error, naming the file.
    let source = serde_json::from_slice::<Value>(br#"{"name":"default","pid":123"#).unwrap_err();
    assert!(stderr.contains(&source.to_string()), "stderr: {stderr}");
    assert!(stderr.contains("default.json"), "stderr: {stderr}");
    assert_json_code(
        project.path(),
        home.path(),
        &["local", "--json", "client", "--name", "default"],
        "managed_client_project_state_unavailable",
    );
}

/// Runs `args` and returns the JSON error code, so a human-mode test can
/// confirm which error branch the same case takes.
fn assert_json_code(project: &Path, home: &Path, args: &[&str], code: &str) {
    let output = run(project, home, args);
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).expect("parse JSON error");
    assert_eq!(error["error"]["code"], code, "{error}");
}

#[test]
fn selected_invalid_utf8_is_actionable() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    write_metadata(project.path(), &[0xff, 0xfe, 0xfd]);

    let output = run(
        project.path(),
        home.path(),
        &["local", "client", "--name", "default"],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let source = String::from_utf8(vec![0xff, 0xfe, 0xfd]).unwrap_err();
    assert!(stderr.contains(&source.to_string()), "stderr: {stderr}");
    assert!(stderr.contains("default.json"), "stderr: {stderr}");
    assert_json_code(
        project.path(),
        home.path(),
        &["local", "--json", "client", "--name", "default"],
        "managed_client_project_state_unavailable",
    );
}

#[cfg(unix)]
#[test]
fn selected_permission_failure_is_actionable() {
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let metadata = write_metadata(project.path(), &valid_metadata(project.path(), 0));
    std::fs::set_permissions(&metadata, std::fs::Permissions::from_mode(0o000))
        .expect("remove metadata permissions");

    let output = run(
        project.path(),
        home.path(),
        &["local", "client", "--name", "default"],
    );

    let json = run(
        project.path(),
        home.path(),
        &["local", "--json", "client", "--name", "default"],
    );
    std::fs::set_permissions(&metadata, std::fs::Permissions::from_mode(0o600))
        .expect("restore metadata permissions");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let source = std::io::Error::from_raw_os_error(libc::EACCES);
    assert!(stderr.contains(&source.to_string()), "stderr: {stderr}");
    assert!(stderr.contains("default.json"), "stderr: {stderr}");
    assert_eq!(json.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&json.stderr).expect("parse JSON error");
    assert_eq!(
        error["error"]["code"], "managed_client_project_state_unavailable",
        "{error}"
    );
}

#[test]
fn list_ignores_stale_temp_but_rejects_corrupt_live_entry() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let directory = servers_dir(project.path());
    std::fs::create_dir_all(&directory).expect("create servers dir");
    std::fs::write(
        directory.join(".metadata-interrupted-write"),
        br#"{"name":"default""#,
    )
    .expect("write stale temp");

    let absent = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "list"],
    );
    assert!(
        absent.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&absent.stderr)
    );
    let body: Value = serde_json::from_slice(&absent.stdout).expect("parse list JSON");
    assert_eq!(body["total_servers"], 0);

    std::fs::write(
        directory.join("default.json"),
        b"{ private SQL and password=hunter2",
    )
    .expect("write corrupt live entry");
    let corrupt = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "list"],
    );
    assert_eq!(corrupt.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&corrupt.stderr).expect("parse metadata error JSON");
    let error = &error["error"];
    assert_eq!(error["code"], "server_metadata_invalid");
    assert!(error["message"].is_string(), "{error}");
    assert_eq!(
        error["path"],
        json!(directory.canonicalize().unwrap().join("default.json"))
    );
    let guidance = error["guidance"].as_array().expect("guidance list");
    assert!(guidance.iter().all(|item| item["message"].is_string()));
    let commands: Vec<&Value> = guidance.iter().map(|item| &item["command"]).collect();
    assert_eq!(
        commands,
        [
            &Value::Null,
            &json!("clickhousectl local clickhouse list --global"),
            &Value::Null,
            &json!("clickhousectl local clickhouse list"),
        ]
    );
    let stderr = String::from_utf8_lossy(&corrupt.stderr);
    assert!(!stderr.contains("private SQL"));
    assert!(!stderr.contains("hunter2"));
}

#[test]
fn corrupt_unrelated_metadata_does_not_block_start() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path());
    let _processes = ProcessFileGuard(project.path().join("fake-clickhouse-pids"));
    let metadata = write_metadata(project.path(), b"not json");

    let output = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "server",
            "start",
            "--name",
            "unrelated",
            "--version",
            VERSION,
            "--no-wait",
        ],
    );

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read(metadata).unwrap(), b"not json");
}

fn assert_success_or_codes(output: &Output, codes: &[&str]) {
    if output.status.success() {
        return;
    }
    let error: Value = serde_json::from_slice(&output.stderr).expect("parse JSON error");
    let code = error["error"]["code"].as_str().expect("error code");
    assert!(codes.contains(&code), "{error}");
}

#[test]
fn concurrent_start_client_and_stop_leave_valid_metadata() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path());
    let _processes = ProcessFileGuard(project.path().join("fake-clickhouse-pids"));

    let initial = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "server",
            "start",
            "--name",
            "default",
            "--version",
            VERSION,
            "--no-wait",
        ],
    );
    assert!(
        initial.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&initial.stderr)
    );
    let initial_body: Value = serde_json::from_slice(&initial.stdout).expect("parse start JSON");
    let initial_pid = initial_body["pid"].as_u64().expect("initial PID") as u32;

    let directory = servers_dir(project.path());
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join(".metadata.lock"))
        .expect("open metadata lock");
    lock.lock().expect("hold metadata lock");

    let mut restart = command(project.path(), home.path());
    restart
        .args([
            "local",
            "--json",
            "server",
            "start",
            "--name",
            "default",
            "--version",
            VERSION,
            "--no-wait",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let restart = restart.spawn().expect("spawn concurrent restart");

    let mut client = command(project.path(), home.path());
    client
        .args([
            "local", "--json", "client", "--name", "default", "--query", "SELECT 1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let client = client.spawn().expect("spawn concurrent client");

    let mut stop = command(project.path(), home.path());
    stop.args(["local", "server", "stop", "default"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let stop = stop.spawn().expect("spawn concurrent stop");

    let keep_reading = Arc::new(AtomicBool::new(true));
    let reader_flag = Arc::clone(&keep_reading);
    let metadata = directory.join("default.json");
    let reader = std::thread::spawn(move || {
        while reader_flag.load(Ordering::Acquire) {
            let bytes = std::fs::read(&metadata).expect("read live metadata");
            serde_json::from_slice::<Value>(&bytes).expect("live metadata must stay complete");
            std::thread::yield_now();
        }
    });

    drop(lock);
    let restart = restart.wait_with_output().expect("wait for restart");
    let client = client.wait_with_output().expect("wait for client");
    let stop = stop.wait_with_output().expect("wait for stop");
    keep_reading.store(false, Ordering::Release);
    reader.join().expect("join metadata reader");

    // Losing a race is allowed; a metadata error is not.
    assert_success_or_codes(&restart, &["server_running", "local_error"]);
    assert_success_or_codes(&client, &["managed_client_server_not_running"]);
    assert!(
        stop.status.success(),
        "stop stderr: {}",
        String::from_utf8_lossy(&stop.stderr)
    );
    for output in [&restart, &client, &stop] {
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("metadata"),
            "unexpected metadata error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let final_metadata: Value = serde_json::from_slice(
        &std::fs::read(directory.join("default.json")).expect("read final metadata"),
    )
    .expect("parse final metadata");
    let final_pid = final_metadata["pid"].as_u64().unwrap_or(0) as u32;
    for pid in [initial_pid, final_pid] {
        if pid != 0 {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
    }
}
