//! Regression coverage for server start argument parsing (issue #357).

use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DEFAULT_VERSION: &str = "25.11.1.1";
const REQUESTED_VERSION: &str = "25.12.9.61";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn install_fake_clickhouse(home: &Path, version: &str) {
    let binary = home
        .join(".clickhouse/versions")
        .join(version)
        .join("clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version dir");
    std::fs::write(
        &binary,
        b"#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$FAKE_CLICKHOUSE_ARGS_FILE\"\nexec sleep 30\n",
    )
    .expect("write fake ClickHouse");
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(binary, permissions).expect("make fake ClickHouse executable");
}

fn run_start(project: &Path, home: &Path, args_file: &Path) -> Output {
    Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("FAKE_CLICKHOUSE_ARGS_FILE", args_file)
        .current_dir(project)
        .args([
            "local",
            "--json",
            "server",
            "start",
            "--no-wait",
            "existing",
            "--version",
            REQUESTED_VERSION,
            "--",
            "--logger.level=trace",
        ])
        .output()
        .expect("run clickhousectl")
}

struct ProcessGuard(u32);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        unsafe {
            libc::kill(self.0 as i32, libc::SIGKILL);
        }
    }
}

#[test]
fn positional_name_keeps_following_version_and_passthrough_separate() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), DEFAULT_VERSION);
    install_fake_clickhouse(home.path(), REQUESTED_VERSION);
    std::fs::write(home.path().join(".clickhouse/default"), DEFAULT_VERSION)
        .expect("write default version");
    let args_file = home.path().join("clickhouse-args.txt");

    let output = run_start(project.path(), home.path(), &args_file);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: Value = serde_json::from_slice(&output.stdout).expect("parse start JSON");
    let _process = ProcessGuard(body["pid"].as_u64().expect("start PID") as u32);

    assert_eq!(body["name"], "existing");
    assert_eq!(body["version"], REQUESTED_VERSION);
    assert!(
        project
            .path()
            .join(".clickhouse/servers/existing.json")
            .exists()
    );
    assert!(
        !project
            .path()
            .join(".clickhouse/servers/default.json")
            .exists()
    );

    let child_args = std::fs::read_to_string(args_file).expect("read fake ClickHouse arguments");
    assert_eq!(child_args.lines().last(), Some("--logger.level=trace"));
    assert!(!child_args.lines().any(|arg| arg == "--version"));
}
