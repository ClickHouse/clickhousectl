//! Subprocess coverage for ClickHouse server name compatibility (issues #474 and #889).

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
    assert_eq!(
        body["selection"],
        if name_args.is_empty() {
            "implicit"
        } else {
            "explicit"
        }
    );
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
    assert_eq!(
        body["selection"],
        if name_args.is_empty() {
            "implicit"
        } else {
            "explicit"
        }
    );
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
fn omitted_names_select_an_existing_default_implicitly() {
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

#[test]
fn dotenv_positional_and_compatibility_names_select_the_same_running_server() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let servers = project.path().join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).unwrap();
    for (name, port) in [("default", 9000), ("dev", 19000)] {
        std::fs::write(
            servers.join(format!("{name}.json")),
            serde_json::json!({
                "name": name, "pid": std::process::id(), "version": "26.8.1.1760",
                "http_port": 8123, "tcp_port": port, "started_at": "test",
                "cwd": project.path(), "engine": "clickhouse"
            })
            .to_string(),
        )
        .unwrap();
    }
    for (selector, port) in [
        (&[][..], 9000),
        (&["default"][..], 9000),
        (&["--name", "default"][..], 9000),
        (&["dev"][..], 19000),
        (&["--name", "dev"][..], 19000),
    ] {
        let args: Vec<_> = ["local", "server", "dotenv"]
            .into_iter()
            .chain(selector.iter().copied())
            .chain(["--json"])
            .collect();
        assert_success(&run(project.path(), home.path(), &args));
        let contents = std::fs::read_to_string(project.path().join(".env")).unwrap();
        assert!(
            contents
                .lines()
                .any(|line| line == format!("CLICKHOUSE_PORT={port}")),
            "{contents}"
        );
    }
}
