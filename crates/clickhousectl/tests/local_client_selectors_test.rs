//! Subprocess coverage for local client selector validation and direct-mode defaults.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VERSION: &str = "25.12.9.61";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn write_arg_printer(path: &Path) {
    std::fs::create_dir_all(path.parent().expect("fake child parent"))
        .expect("create fake child directory");
    std::fs::write(path, b"#!/bin/sh\nprintf '%s\\n' \"$@\"\n").expect("write fake child");
    let mut permissions = std::fs::metadata(path)
        .expect("read fake child metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("make fake child executable");
}

fn run(project: &Path, home: &Path, path: Option<&Path>, args: &[&str]) -> Output {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project)
        .args(args);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.output().expect("run clickhousectl")
}

fn assert_child_args(output: Output, expected: &[&str]) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    let args: Vec<&str> = std::str::from_utf8(&output.stdout)
        .expect("child output is UTF-8")
        .lines()
        .collect();
    assert_eq!(args, expected);
}

fn assert_usage_before_resolution(args: &[&str], expected: &[&str]) {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let output = run(project.path(), home.path(), None, args);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "stderr: {stderr}");
    for text in expected {
        assert!(stderr.contains(text), "missing {text:?} in: {stderr}");
    }
    assert!(!stderr.contains("No default version"), "{stderr}");
    assert!(!stderr.contains("Failed to execute"), "{stderr}");
    assert!(
        !home.path().join(".clickhouse").exists(),
        "parser error resolved a ClickHouse binary"
    );
    assert!(
        !project.path().join(".clickhouse").exists(),
        "parser error resolved project state"
    );
}

#[test]
fn invalid_clickhouse_selectors_fail_before_binary_or_project_resolution() {
    for args in [
        &["local", "client", "--name", "dev", "--host", "remote"][..],
        &["local", "client", "--host", "remote", "--name", "dev"],
        &["local", "client", "--name", "dev", "--port", "9000"],
        &["local", "client", "--port", "9000", "--name", "dev"],
    ] {
        assert_usage_before_resolution(args, &["--name", "cannot be used"]);
    }
    assert_usage_before_resolution(
        &["local", "client", "--port", "0"],
        &["invalid value", "--port"],
    );
    assert_usage_before_resolution(
        &["local", "client", "--port", "not-a-port"],
        &["invalid value", "--port"],
    );
}

#[test]
fn clickhouse_direct_selectors_reach_fake_child_with_documented_defaults() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let binary = home
        .path()
        .join(".clickhouse/versions")
        .join(VERSION)
        .join("clickhouse");
    write_arg_printer(&binary);
    std::fs::write(home.path().join(".clickhouse/default"), VERSION)
        .expect("write default version");

    assert_child_args(
        run(
            project.path(),
            home.path(),
            None,
            &["local", "client", "--host", "remote", "--query", "SELECT 1"],
        ),
        &[
            "client", "--host", "remote", "--port", "9000", "--query", "SELECT 1",
        ],
    );
    assert_child_args(
        run(
            project.path(),
            home.path(),
            None,
            &["local", "client", "--port", "65535"],
        ),
        &["client", "--host", "localhost", "--port", "65535"],
    );
}

#[test]
fn postgres_client_uses_the_same_validation_and_direct_mode_defaults() {
    assert_usage_before_resolution(
        &[
            "local", "postgres", "client", "--name", "dev", "--host", "remote",
        ],
        &["--name", "cannot be used"],
    );
    assert_usage_before_resolution(
        &[
            "local",
            "postgres",
            "client",
            "--version",
            "18",
            "--port",
            "5432",
        ],
        &["--version", "cannot be used"],
    );
    assert_usage_before_resolution(
        &["local", "postgres", "client", "--port", "0"],
        &["invalid value", "--port"],
    );

    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let bin = home.path().join("bin");
    write_arg_printer(&bin.join("psql"));

    assert_child_args(
        run(
            project.path(),
            home.path(),
            Some(&bin),
            &[
                "local", "postgres", "client", "--host", "remote", "--query", "SELECT 1",
            ],
        ),
        &[
            "-h", "remote", "-p", "5432", "-U", "postgres", "-d", "postgres", "-c", "SELECT 1",
        ],
    );
    assert_child_args(
        run(
            project.path(),
            home.path(),
            Some(&bin),
            &["local", "postgres", "client", "--port", "65535"],
        ),
        &[
            "-h",
            "127.0.0.1",
            "-p",
            "65535",
            "-U",
            "postgres",
            "-d",
            "postgres",
        ],
    );
}
