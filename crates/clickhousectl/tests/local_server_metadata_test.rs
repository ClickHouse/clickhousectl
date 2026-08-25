//! End-to-end coverage for selected local server metadata failures (#472).

use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn run(project: &Path, home: &Path, json: bool) -> Output {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project)
        .arg("local");
    if json {
        command.arg("--json");
    }
    command
        .args(["server", "stop", "default"])
        .output()
        .expect("run clickhousectl")
}

fn metadata_path(project: &Path) -> PathBuf {
    project.join(".clickhouse/servers/default.json")
}

fn lock_directory(project: &Path) -> PathBuf {
    project.join(".clickhouse/servers/.locks")
}

fn lock_path(project: &Path) -> PathBuf {
    lock_directory(project).join("default.lock")
}

fn prepare_project(project: &Path) {
    std::fs::create_dir_all(project.join(".clickhouse/servers/default/data"))
        .expect("create server data directory");
}

fn valid_metadata(project: &Path) -> Vec<u8> {
    serde_json::to_vec_pretty(&serde_json::json!({
        "name": "default",
        "pid": std::process::id(),
        "version": "26.8.1.1",
        "http_port": 8123,
        "tcp_port": 9000,
        "started_at": "1700000000",
        "cwd": project.display().to_string(),
        "engine": "clickhouse"
    }))
    .unwrap()
}

fn assert_json_error(output: &Output, code: &str, message_fragment: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let body: Value = serde_json::from_slice(&output.stderr).expect("parse structured error");
    assert_eq!(body["error"]["code"], code);
    assert_eq!(body["error"]["command"], "clickhousectl local server list");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains(message_fragment),
        "{body}"
    );
}

fn assert_lock_error(output: &Output, operation: &str, path: &Path) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let body: Value = serde_json::from_slice(&output.stderr).expect("parse structured lock error");
    assert_eq!(body["error"]["code"], "server_lock");
    assert_eq!(body["error"]["command"], "clickhousectl local server list");
    let message = body["error"]["message"].as_str().unwrap();
    assert!(message.contains(operation), "{message}");
    assert!(message.contains(&path.display().to_string()), "{message}");
    assert!(message.contains("retry"), "{message}");
    assert!(!message.contains("metadata"), "{message}");
    assert!(!message.contains("default.json"), "{message}");
}

#[test]
fn lock_directory_creation_failure_reports_the_lock_directory() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let locks = lock_directory(project.path());
    std::fs::create_dir_all(locks.parent().unwrap()).expect("create servers directory");
    std::fs::write(&locks, b"blocks lock directory creation").expect("block lock directory");

    let json = run(project.path(), home.path(), true);
    assert_lock_error(&json, "create server lifecycle lock directory", &locks);

    let human = run(project.path(), home.path(), false);
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(
        stderr.starts_with("Error: Could not create server lifecycle lock directory"),
        "{stderr}"
    );
    assert!(stderr.contains(&locks.display().to_string()), "{stderr}");
    assert!(!stderr.contains("metadata"), "{stderr}");
    assert!(!stderr.contains("default.json"), "{stderr}");
}

#[test]
fn lock_file_open_failure_reports_the_lock_file() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let lock = lock_path(project.path());
    std::fs::create_dir_all(&lock).expect("create directory at lock file path");

    let json = run(project.path(), home.path(), true);
    assert_lock_error(&json, "open server lifecycle lock file", &lock);

    let human = run(project.path(), home.path(), false);
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(
        stderr.starts_with("Error: Could not open server lifecycle lock file"),
        "{stderr}"
    );
    assert!(stderr.contains(&lock.display().to_string()), "{stderr}");
    assert!(!stderr.contains("metadata"), "{stderr}");
    assert!(!stderr.contains("default.json"), "{stderr}");
}

#[test]
fn selected_partial_json_and_invalid_utf8_are_parse_errors() {
    for (label, contents) in [
        ("partial JSON", br#"{"name":"default","pid":1"#.as_slice()),
        ("invalid UTF-8", b"{\"name\":\xff}".as_slice()),
    ] {
        let project = tempfile::tempdir().expect("create project tempdir");
        let home = tempfile::tempdir().expect("create home tempdir");
        prepare_project(project.path());
        std::fs::write(metadata_path(project.path()), contents).expect("write invalid metadata");

        let json = run(project.path(), home.path(), true);
        assert_json_error(
            &json,
            "server_metadata_invalid",
            "Metadata for server 'default'",
        );

        let human = run(project.path(), home.path(), false);
        assert_eq!(human.status.code(), Some(1), "case: {label}");
        let stderr = String::from_utf8_lossy(&human.stderr);
        assert!(
            stderr.starts_with("Error: Metadata for server 'default'"),
            "case: {label}; stderr: {stderr}"
        );
        assert!(stderr.contains("Repair or remove the metadata file, then retry."));
    }
}

#[test]
fn selected_metadata_read_failure_is_not_reported_as_stopped() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    prepare_project(project.path());
    std::fs::create_dir(metadata_path(project.path())).expect("create unreadable metadata shape");

    let json = run(project.path(), home.path(), true);
    assert_json_error(
        &json,
        "server_metadata_read",
        "Could not read metadata for server 'default'",
    );

    let human = run(project.path(), home.path(), false);
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(stderr.starts_with("Error: Could not read metadata for server 'default'"));
    assert!(stderr.contains("Check that the file is readable and retry."));
}

#[test]
fn selected_metadata_permission_failure_has_its_own_action() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    prepare_project(project.path());
    let path = metadata_path(project.path());
    std::fs::write(&path, valid_metadata(project.path())).expect("write metadata");
    let original = std::fs::metadata(&path).unwrap().permissions();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("remove metadata permissions");

    let json = run(project.path(), home.path(), true);
    let human = run(project.path(), home.path(), false);
    std::fs::set_permissions(&path, original).expect("restore metadata permissions");

    assert_json_error(
        &json,
        "server_metadata_permission",
        "Permission denied accessing metadata for server 'default'",
    );
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(stderr.starts_with("Error: Permission denied accessing metadata for server 'default'"));
    assert!(stderr.contains("Restore access to the file and its parent directory, then retry."));
}
