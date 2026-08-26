//! Subprocess coverage for ClickHouse server name compatibility (issue #474).

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
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

fn create_stopped_server(project: &Path, name: &str) -> PathBuf {
    let directory = project.join(".clickhouse/servers").join(name);
    std::fs::create_dir_all(directory.join("data")).expect("create stopped server data");
    directory
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_stop_dispatch(name_args: &[&str], expected: &str) {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    create_stopped_server(project.path(), expected);
    let decoy = create_stopped_server(project.path(), "decoy");
    let mut args = vec!["local", "server", "stop"];
    args.extend_from_slice(name_args);
    args.push("--json");

    let output = run(project.path(), home.path(), &args);

    assert_success(&output);
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse stop JSON");
    assert_eq!(body["name"], expected);
    assert_eq!(body["already_stopped"], true);
    assert!(decoy.exists());
}

fn assert_remove_dispatch(name_args: &[&str], expected: &str) {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let selected = create_stopped_server(project.path(), expected);
    let decoy = create_stopped_server(project.path(), "decoy");
    let mut args = vec!["local", "server", "remove"];
    args.extend_from_slice(name_args);
    args.push("--json");

    let output = run(project.path(), home.path(), &args);

    assert_success(&output);
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse remove JSON");
    assert_eq!(body["name"], expected);
    assert!(!selected.exists());
    assert!(decoy.exists());
}

#[test]
fn stop_dispatches_positional_and_compatibility_names_exactly() {
    assert_stop_dispatch(&["positional-stop"], "positional-stop");
    assert_stop_dispatch(&["--name", "flag-stop"], "flag-stop");
}

#[test]
fn remove_dispatches_positional_and_compatibility_names_exactly() {
    assert_remove_dispatch(&["positional-remove"], "positional-remove");
    assert_remove_dispatch(&["--name", "flag-remove"], "flag-remove");
}

#[test]
fn omitted_names_keep_the_current_default_runtime_behavior() {
    assert_stop_dispatch(&[], "default");
    assert_remove_dispatch(&[], "default");
}

#[test]
fn conflicting_name_forms_fail_before_dispatch() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");

    for command in ["stop", "remove"] {
        let output = run(
            project.path(),
            home.path(),
            &[
                "local",
                "server",
                command,
                "positional",
                "--name",
                "flagged",
            ],
        );

        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("cannot be used with"), "stderr: {stderr}");
        assert!(!project.path().join(".clickhouse").exists());
    }
}
