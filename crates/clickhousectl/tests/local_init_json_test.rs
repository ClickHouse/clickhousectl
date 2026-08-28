//! Subprocess coverage for `local init --json`: the JSON payload must report
//! the full set of paths the command created, matching what the
//! human-readable output says (issue #609).

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
fn init_json_on_second_run_reports_only_the_clickhouse_dir() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    run(project.path(), home.path(), &["local", "init", "--json"]);
    let output = run(project.path(), home.path(), &["local", "init", "--json"]);
    let json = stdout_json(&output);

    assert_eq!(json["paths"], serde_json::json!([".clickhouse/"]));
}
