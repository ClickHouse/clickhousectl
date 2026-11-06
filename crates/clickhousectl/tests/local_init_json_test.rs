//! Subprocess coverage for `local init`: the `--json` payload must report the
//! full set of paths the command created (issue #609), and the runtime ignore
//! file is created, preserved or reported as an error on disk. The human
//! wording is unit-tested beside `InitOutput` and `init_output`.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn run(project: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("HOME", home)
        .current_dir(project)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn stdout_json(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout is a single JSON object")
}

#[test]
fn init_json_reports_clickhouse_dir_and_both_scaffolds_on_first_run() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    let output = run(project.path(), home.path(), &["local", "init", "--json"]);
    let json = stdout_json(&output);

    assert_eq!(
        json["paths"],
        serde_json::json!([".clickhouse/", "clickhouse/", "postgres/"])
    );

    assert!(project.path().join(".clickhouse").is_dir());
    assert!(project.path().join("clickhouse/tables").is_dir());
    assert!(project.path().join("postgres/tables").is_dir());
}

#[test]
fn init_json_on_second_run_reports_no_created_paths() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    run(project.path(), home.path(), &["local", "init", "--json"]);
    let output = run(project.path(), home.path(), &["local", "init", "--json"]);
    let json = stdout_json(&output);

    assert_eq!(json["paths"], serde_json::json!([]));
}

#[test]
fn init_json_reports_runtime_gitignore_repair() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    run(project.path(), home.path(), &["local", "init", "--json"]);
    std::fs::remove_file(project.path().join(".clickhouse/.gitignore"))
        .expect("remove runtime gitignore");
    let output = run(project.path(), home.path(), &["local", "init", "--json"]);
    let json = stdout_json(&output);

    assert_eq!(json["paths"], serde_json::json!([".clickhouse/.gitignore"]));
    assert_eq!(
        std::fs::read_to_string(project.path().join(".clickhouse/.gitignore")).unwrap(),
        "*\n"
    );
}

/// Without `--json` the dispatcher renders the same result as text on stdout,
/// one line per created path.
#[test]
fn init_human_output_is_text_on_stdout_naming_each_created_path() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    let output = run(project.path(), home.path(), &["local", "init"]);
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(
        serde_json::from_str::<serde_json::Value>(&stdout).is_err(),
        "{stdout}"
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "{stdout}");
    for (line, path) in lines
        .iter()
        .zip([".clickhouse/", "clickhouse/", "postgres/"])
    {
        assert!(line.ends_with(&format!(" {path}")), "{stdout}");
    }
}

#[test]
fn init_after_server_list_creates_runtime_gitignore() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    let list = run(project.path(), home.path(), &["local", "server", "list"]);
    assert!(
        list.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    // Listing no longer creates project state (#1077), but releases before it
    // left the metadata lock behind without an ignore file. Init must repair
    // that partially initialized directory.
    assert!(!project.path().join(".clickhouse").exists());
    let servers = project.path().join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).expect("create legacy servers directory");
    std::fs::write(servers.join(".metadata.lock"), "").expect("create legacy metadata lock");

    let init = run(project.path(), home.path(), &["local", "init"]);
    assert!(
        init.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(project.path().join(".clickhouse/.gitignore")).unwrap(),
        "*\n"
    );

    let git_init = Command::new("git")
        .arg("init")
        .current_dir(project.path())
        .output()
        .expect("initialize temporary Git repository");
    assert!(git_init.status.success());
    let status = Command::new("git")
        .args(["status", "--short", "--untracked-files=all"])
        .current_dir(project.path())
        .output()
        .expect("inspect temporary Git repository");
    assert!(status.status.success());
    assert!(!String::from_utf8_lossy(&status.stdout).contains(".metadata.lock"));
}

#[test]
fn init_preserves_existing_runtime_gitignore() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let runtime_dir = project.path().join(".clickhouse");
    std::fs::create_dir(&runtime_dir).unwrap();
    std::fs::write(runtime_dir.join(".gitignore"), "custom-entry\n").unwrap();

    let first = run(project.path(), home.path(), &["local", "init"]);
    assert!(first.status.success());
    let second = run(project.path(), home.path(), &["local", "init"]);
    assert!(second.status.success());
    assert_eq!(
        std::fs::read_to_string(runtime_dir.join(".gitignore")).unwrap(),
        "custom-entry\n"
    );
}

#[test]
fn init_propagates_runtime_gitignore_write_failure() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let runtime_dir = project.path().join(".clickhouse");
    std::fs::create_dir(&runtime_dir).unwrap();
    std::fs::set_permissions(&runtime_dir, std::fs::Permissions::from_mode(0o500)).unwrap();

    let output = run(project.path(), home.path(), &["local", "init", "--json"]);

    std::fs::set_permissions(&runtime_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!runtime_dir.join(".gitignore").exists());
}

#[test]
fn init_rejects_a_runtime_gitignore_directory() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    std::fs::create_dir_all(project.path().join(".clickhouse/.gitignore")).unwrap();

    let output = run(project.path(), home.path(), &["local", "init", "--json"]);

    assert_eq!(output.status.code(), Some(1));
}
