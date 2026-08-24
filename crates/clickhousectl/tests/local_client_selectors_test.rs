//! End-to-end coverage for local client selector validation (issue #466).

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VERSION: &str = "25.12.9.61";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn install_fake_clickhouse(home: &Path) {
    let binary = home
        .join(".clickhouse/versions")
        .join(VERSION)
        .join("clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version dir");
    std::fs::write(&binary, b"#!/bin/sh\nprintf '%s\\n' \"$@\"\n").expect("write fake ClickHouse");
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(binary, permissions).expect("make fake ClickHouse executable");
    std::fs::write(home.join(".clickhouse/default"), VERSION).expect("write default version");
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

#[test]
fn clickhouse_direct_client_defaults_missing_host_or_port() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path());

    let cases = [
        (
            &["local", "client", "--host", "db.example"][..],
            &["client", "--host", "db.example", "--port", "9000"][..],
        ),
        (
            &["local", "client", "--port", "1"][..],
            &["client", "--host", "localhost", "--port", "1"][..],
        ),
        (
            &["local", "client", "--port", "65535"][..],
            &["client", "--host", "localhost", "--port", "65535"][..],
        ),
    ];

    for (args, expected) in cases {
        let output = run(project.path(), home.path(), args);
        assert!(
            output.status.success(),
            "args: {args:?}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let forwarded: Vec<_> = String::from_utf8(output.stdout)
            .expect("fake ClickHouse output should be UTF-8")
            .lines()
            .map(str::to_string)
            .collect();
        assert_eq!(forwarded, expected, "args: {args:?}");
    }
}

#[test]
fn invalid_client_selectors_are_usage_errors_before_resolution() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let cases = [
        &["local", "client", "--name", "dev", "--host", "db.example"][..],
        &["local", "client", "--port", "9000", "--name", "dev"][..],
        &["local", "client", "--port", "0"][..],
        &[
            "local",
            "postgres",
            "client",
            "--host",
            "db.example",
            "--name",
            "dev",
        ][..],
        &[
            "local", "postgres", "client", "--name", "dev", "--port", "5432",
        ][..],
        &["local", "postgres", "client", "--port", "0"][..],
    ];

    for args in cases {
        let output = run(project.path(), home.path(), args);
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("error:"),
            "args: {args:?}\nstderr: {stderr}"
        );
        assert!(
            !stderr.contains("No default version configured")
                && !stderr.contains("Server 'dev' not found"),
            "args: {args:?}\nstderr: {stderr}"
        );
    }
}
