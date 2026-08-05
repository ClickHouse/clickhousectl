//! Regression coverage for local version-spec error reporting.

use std::path::PathBuf;
use std::process::Command;

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

#[test]
fn local_use_reports_an_invalid_version_without_a_lookup_wrapper() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let output = Command::new(clickhousectl_binary())
        .env("DO_NOT_TRACK", "1")
        .env("HOME", tempdir.path())
        .args(["local", "use", "not.a.version"])
        .output()
        .expect("run clickhousectl");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("Error: invalid version 'not.a.version': all parts must be numeric"),
        "unexpected stderr: {stderr}"
    );
    assert!(
        !stderr.contains("No matching version found"),
        "parse error was wrapped as a lookup miss: {stderr}"
    );
}
