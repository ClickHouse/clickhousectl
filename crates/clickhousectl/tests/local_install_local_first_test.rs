//! Regression tests for local-first installs. Issue #217 requires
//! `local install <concrete-spec>` to satisfy the request from already-installed
//! versions without any remote call, while issue #339 requires exact and partial
//! spellings of an installed version to both be successful no-ops.
//!
//! Strategy: spawn the binary with `HOME=<tempdir>`, pre-seed a fake installed
//! version, run both partial and exact `local install` commands in human-output
//! mode with every proxy variable pointed at a closed port, and assert that:
//!   - the command exits 0, which it cannot if any remote request is made,
//!   - stderr names the requested spec, the installed version and `--force`,
//!   - stdout has no generic "Installed version" confirmation.
//!
//! Issue #1302: the same no-op, with stderr a pipe whose reader has gone, must
//! still exit 0 rather than panic (exit 101) on its stderr notice.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn install_with_existing_version(installed_version: &str, requested_spec: &str) -> Output {
    install_with_existing_version_to(installed_version, requested_spec, Stdio::piped())
}

fn install_with_existing_version_to(
    installed_version: &str,
    requested_spec: &str,
    stderr: Stdio,
) -> Output {
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

    let output = Command::new(clickhousectl_binary())
        // Prevent coding-agent detection from switching the subprocess to JSON
        // so this exercises the human output reported in issue #328.
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("HOME", tempdir.path())
        .current_dir(tempdir.path())
        // Any remote resolve fails fast through this closed port.
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env("HTTP_PROXY", "http://127.0.0.1:9")
        .env("ALL_PROXY", "http://127.0.0.1:9")
        .args(["local", "install", requested_spec])
        .stderr(stderr)
        .output()
        .expect("run clickhousectl");
    // A zero timeout skips the background update request, so nothing writes
    // the update cache. Any attempt, even a failed one, would leave it behind.
    assert!(
        !tempdir
            .path()
            .join(".clickhouse/last_update_check")
            .exists(),
        "a zero update-check timeout must skip the background request"
    );
    output
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
    assert!(stderr.contains("25.12.9.61"), "{stderr}");
    assert!(stderr.contains("--force"), "{stderr}");
    assert!(
        output.stdout.is_empty(),
        "expected no generic install confirmation, got: {}",
        String::from_utf8_lossy(&output.stdout)
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
    assert!(stderr.contains(version), "{stderr}");
    assert!(stderr.contains("--force"), "{stderr}");
    assert!(
        output.stdout.is_empty(),
        "expected no generic install confirmation, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn local_install_no_op_survives_a_closed_stderr() {
    // EPIPE, not EBADF: a closed descriptor (`2>&-`) is treated as success by
    // std, so only a pipe whose reader is gone reaches the failing write.
    let (reader, writer) = std::io::pipe().expect("create pipe");
    drop(reader);
    let version = "25.12.9.61";
    let output = install_with_existing_version_to(version, version, writer.into());
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}
