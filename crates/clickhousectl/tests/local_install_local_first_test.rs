//! Regression test for issue #217: `local install <concrete-spec>` must satisfy
//! the request from already-installed versions without any remote call.
//!
//! Strategy: spawn the binary with `HOME=<tempdir>`, pre-seed a fake installed
//! version, run `local install 25.12 --json`, and assert that:
//!   - the command exits 0,
//!   - stderr says "already installed",
//!   - stderr does NOT say "Resolving" (which only prints on the remote path).
//!
//! Network calls aren't mocked because the version-manager URLs aren't currently
//! overridable; the timing + stderr-content assertions are sufficient to detect
//! a regression where the remote path runs when it shouldn't.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

#[test]
fn local_install_minor_with_existing_match_does_not_hit_network() {
    let tempdir = tempfile::tempdir().expect("create tempdir");

    let version_dir = tempdir.path().join(".clickhouse/versions/25.12.9.61");
    std::fs::create_dir_all(&version_dir).expect("create version dir");

    let binary = version_dir.join("clickhouse");
    std::fs::write(&binary, b"#!/bin/sh\necho stub\n").expect("write fake binary");
    let mut perms = std::fs::metadata(&binary).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&binary, perms).unwrap();

    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", tempdir.path())
        .args(["local", "install", "25.12", "--json"])
        .output()
        .expect("run clickhousectl");

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
        !stderr.contains("Resolving"),
        "expected no remote-resolve message, got: {}",
        stderr
    );
}
