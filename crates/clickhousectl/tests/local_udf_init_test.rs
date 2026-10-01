//! Subprocess coverage for `local udf init` and the `udfs/` entry of the
//! `local init` scaffold.

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

fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o111 == 0o111
}

#[test]
fn udf_init_scaffolds_a_python_definition_and_entrypoint() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let output = run(
        project.path(),
        home.path(),
        &["local", "udf", "init", "my_fn", "--json"],
    );
    let json = stdout_json(&output);
    assert_eq!(
        json,
        serde_json::json!({
            "name": "my_fn",
            "dir": "clickhouse/udfs/my_fn",
            "created": ["udf.json", "main.py"]
        })
    );

    let dir = project.path().join("clickhouse/udfs/my_fn");
    let definition: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("udf.json")).unwrap()).unwrap();
    assert_eq!(definition["functionName"], "my_fn");
    assert_eq!(definition["type"], "executable");
    assert_eq!(definition["runtime"], "python3.11");
    assert_eq!(definition["returnType"], "String");
    assert_eq!(definition["arguments"][0]["type"], "String");
    assert!(definition.get("uploadId").is_none());

    let entrypoint = std::fs::read_to_string(dir.join("main.py")).unwrap();
    assert!(entrypoint.starts_with("#!/usr/bin/env python3\n"));
    assert!(is_executable(&dir.join("main.py")));
    assert!(!is_executable(&dir.join("udf.json")));
}

#[test]
fn udf_init_is_idempotent_and_keeps_edited_files() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let args = ["local", "udf", "init", "my_fn", "--json"];

    stdout_json(&run(project.path(), home.path(), &args));
    let entrypoint = project.path().join("clickhouse/udfs/my_fn/main.py");
    std::fs::write(&entrypoint, "print('edited')\n").unwrap();
    std::fs::remove_file(project.path().join("clickhouse/udfs/my_fn/udf.json")).unwrap();

    let json = stdout_json(&run(project.path(), home.path(), &args));
    assert_eq!(json["created"], serde_json::json!(["udf.json"]));
    assert_eq!(
        std::fs::read_to_string(&entrypoint).unwrap(),
        "print('edited')\n"
    );

    let human = run(
        project.path(),
        home.path(),
        &["local", "udf", "init", "my_fn"],
    );
    assert!(human.status.success());
    assert_eq!(
        String::from_utf8_lossy(&human.stdout),
        "UDF my_fn already exists in clickhouse/udfs/my_fn\n"
    );
}

#[test]
fn udf_init_native_pool_writes_a_shell_entrypoint_into_a_custom_dir() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let output = run(
        project.path(),
        home.path(),
        &[
            "local",
            "udf",
            "init",
            "native_fn",
            "--runtime",
            "native",
            "--type",
            "executable_pool",
            "--dir",
            "funcs",
            "--json",
        ],
    );
    let json = stdout_json(&output);
    assert_eq!(json["dir"], "funcs/native_fn");
    assert_eq!(json["created"], serde_json::json!(["udf.json", "main"]));

    let dir = project.path().join("funcs/native_fn");
    let definition: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("udf.json")).unwrap()).unwrap();
    assert_eq!(definition["type"], "executable_pool");
    assert_eq!(definition["runtime"], "native");
    assert!(
        std::fs::read_to_string(dir.join("main"))
            .unwrap()
            .starts_with("#!/bin/sh\n")
    );
    assert!(is_executable(&dir.join("main")));
    assert!(!dir.join("main.py").exists());
}

#[test]
fn udf_init_human_output_lists_each_created_file() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let output = run(
        project.path(),
        home.path(),
        &["local", "udf", "init", "my_fn"],
    );
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Scaffolded UDF my_fn in clickhouse/udfs/my_fn\n\
         Created clickhouse/udfs/my_fn/udf.json\n\
         Created clickhouse/udfs/my_fn/main.py\n"
    );
}

#[test]
fn udf_init_rejects_an_invalid_name_as_a_usage_error() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let output = run(
        project.path(),
        home.path(),
        &["local", "udf", "init", "1abc"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(!project.path().join("clickhouse").exists());
}

#[test]
fn local_init_scaffold_includes_the_udfs_directory() {
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let output = run(project.path(), home.path(), &["local", "init", "--json"]);
    let json = stdout_json(&output);
    assert_eq!(
        json["paths"],
        serde_json::json!([".clickhouse/", "clickhouse/", "postgres/"])
    );
    assert!(project.path().join("clickhouse/udfs/.gitkeep").is_file());
}
