//! Regression coverage for server start argument parsing (issue #357).

use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

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

fn run_start(project: &Path, home: &Path, args_file: &Path, selector: &[&str]) -> Output {
    Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .env("FAKE_CLICKHOUSE_ARGS_FILE", args_file)
        .current_dir(project)
        .args(["local", "--json", "server", "start", "--no-wait"])
        .args(selector)
        .args(["--version", REQUESTED_VERSION, "--", "--logger.level=trace"])
        .output()
        .expect("run clickhousectl")
}

fn read_file_eventually(path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match std::fs::read_to_string(path) {
            Ok(contents) if !contents.is_empty() => return contents,
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("read fake ClickHouse arguments: {error}"),
        }
        assert!(Instant::now() < deadline, "fake ClickHouse did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
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
fn both_name_forms_keep_following_version_and_passthrough_separate() {
    for selector in [&["existing"][..], &["--name", "existing"][..]] {
        let project = tempfile::tempdir().expect("create project tempdir");
        let home = tempfile::tempdir().expect("create home tempdir");
        install_fake_clickhouse(home.path(), DEFAULT_VERSION);
        install_fake_clickhouse(home.path(), REQUESTED_VERSION);
        std::fs::write(home.path().join(".clickhouse/default"), DEFAULT_VERSION)
            .expect("write default version");
        let args_file = home.path().join("clickhouse-args.txt");

        let output = run_start(project.path(), home.path(), &args_file, selector);
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
        assert_eq!(
            std::fs::read_to_string(project.path().join(".clickhouse/.gitignore")).unwrap(),
            "*\n"
        );
        assert!(
            !project
                .path()
                .join(".clickhouse/servers/default.json")
                .exists()
        );

        let child_args = read_file_eventually(&args_file);
        assert_eq!(child_args.lines().last(), Some("--logger.level=trace"));
        assert!(!child_args.lines().any(|arg| arg == "--version"));
    }
}

/// One port for both listeners is a usage error found before anything else
/// in `start` runs: no project state, version lookup or download (#1452).
#[test]
fn identical_http_and_tcp_ports_fail_before_any_side_effect() {
    for json in [false, true] {
        let project = tempfile::tempdir().expect("create project tempdir");
        let home = tempfile::tempdir().expect("create home tempdir");
        let mut command = Command::new(clickhousectl_binary());
        command
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
            .env("HOME", home.path())
            .env("PATH", "/usr/bin:/bin")
            .current_dir(project.path())
            .args(["local", "clickhouse", "start"])
            .args(["--http-port", "18130", "--tcp-port", "18130"]);
        if json {
            command.arg("--json");
        }
        let output = command.output().expect("run clickhousectl");

        assert_eq!(output.status.code(), Some(2), "json={json}: {output:?}");
        assert!(output.stdout.is_empty(), "json={json}: {output:?}");
        assert!(!project.path().join(".clickhouse").exists(), "json={json}");
        assert!(!home.path().join(".clickhouse").exists(), "json={json}");
    }
}
