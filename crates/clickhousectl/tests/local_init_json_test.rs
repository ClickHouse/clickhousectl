//! Subprocess coverage for `local init`: the `--json` payload must report the
//! full set of paths the command created, and the human output must report
//! each created path exactly once (issue #609).

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

#[test]
fn init_human_output_reports_each_created_path_exactly_once() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    let output = run(project.path(), home.path(), &["local", "init"]);
    assert!(output.status.success());

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for path in [".clickhouse/", "clickhouse/", "postgres/"] {
        // `.clickhouse/` is a substring match of itself only; `clickhouse/`
        // also matches inside `.clickhouse/`, so count line-anchored mentions.
        let mentions = combined
            .lines()
            .filter(|line| line.ends_with(&format!(" {path}")))
            .count();
        assert_eq!(
            mentions, 1,
            "expected one line mentioning {path}: {combined}"
        );
    }
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Initialized ClickHouse project in .clickhouse/\n\
         Created project scaffold in clickhouse/\n\
         Created project scaffold in postgres/\n"
    );
}

#[test]
fn init_human_output_second_run_reports_already_initialized() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    run(project.path(), home.path(), &["local", "init"]);
    let output = run(project.path(), home.path(), &["local", "init"]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Already initialized at .clickhouse/\n"
    );
}

#[test]
fn init_human_output_reports_a_scaffold_only_repair() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    run(project.path(), home.path(), &["local", "init"]);
    std::fs::remove_dir_all(project.path().join("postgres/views"))
        .expect("remove one scaffold directory");
    let output = run(project.path(), home.path(), &["local", "init"]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Already initialized at .clickhouse/\nCreated project scaffold in postgres/\n"
    );
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
    assert!(
        project
            .path()
            .join(".clickhouse/servers/.metadata.lock")
            .is_file()
    );

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
