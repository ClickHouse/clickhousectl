//! Regression tests for local-first installs. Issue #217 requires
//! `local install <concrete-spec>` to satisfy the request from already-installed
//! versions without any remote call, while issue #339 requires exact and partial
//! spellings of an installed version to both be successful no-ops.
//!
//! Strategy: spawn the binary with `HOME=<tempdir>`, pre-seed a fake installed
//! version, run both partial and exact `local install` commands in human-output
//! mode, and assert that:
//!   - the command exits 0,
//!   - stderr says "already installed",
//!   - stdout has no generic "Installed version" confirmation,
//!   - stderr does NOT say "Resolving" (which only prints on the remote path).
//!
//! Network calls aren't mocked because the version-manager URLs aren't currently
//! overridable; the timing + stderr-content assertions are sufficient to detect
//! a regression where the remote path runs when it shouldn't.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn install_with_existing_version(installed_version: &str, requested_spec: &str) -> Output {
    let tempdir = tempfile::tempdir().expect("create tempdir");

    let version_dir = tempdir
        .path()
        .join(".clickhouse/versions")
        .join(installed_version);
    std::fs::create_dir_all(&version_dir).expect("create version dir");

    let binary = version_dir.join("clickhouse");
    std::fs::write(&binary, b"#!/bin/sh\necho stub\n").expect("write fake binary");
    let mut perms = std::fs::metadata(&binary).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&binary, perms).unwrap();

    let mut command = Command::new(clickhousectl_binary());
    command
        // Prevent coding-agent detection from switching the subprocess to JSON
        // so this exercises the human output reported in issue #328.
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", tempdir.path())
        .args(["local", "install", requested_spec])
        .output()
        .expect("run clickhousectl")
}

#[test]
fn local_install_minor_with_existing_match_does_not_hit_network() {
    let output = install_with_existing_version("25.12.9.61", "25.12");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "expected success, got status {:?}\nstderr: {}",
        output.status,
        stderr
    );
    assert!(
        stderr.contains("already installed"),
        "expected 'already installed' in stderr, got: {}",
        stderr
    );
    assert!(
        output.stdout.is_empty(),
        "expected no generic install confirmation, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !stderr.contains("Resolving"),
        "expected no remote-resolve message, got: {}",
        stderr
    );
}

#[test]
fn local_install_exact_with_existing_match_is_a_successful_no_op() {
    let version = "25.12.9.61";
    let output = install_with_existing_version(version, version);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "expected success, got status {:?}\nstderr: {}",
        output.status,
        stderr
    );
    assert!(
        stderr.contains(&format!(
            "ClickHouse {version} is already installed as {version}"
        )),
        "expected exact-version no-op message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("Use --force to re-download the latest build"),
        "expected --force hint, got: {}",
        stderr
    );
    assert!(
        output.stdout.is_empty(),
        "expected no generic install confirmation, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !stderr.contains("Resolving"),
        "expected no remote-resolve message, got: {}",
        stderr
    );
}
