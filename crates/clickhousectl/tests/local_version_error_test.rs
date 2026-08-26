//! Subprocess coverage for clap-time local version validation.

use std::path::PathBuf;
use std::process::Command;

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn assert_rejected_before_side_effects(args: &[&str], expected_error: &str) {
    let home = tempfile::tempdir().expect("create home");
    let project = tempfile::tempdir().expect("create project");
    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home.path())
        .current_dir(project.path())
        .args(args)
        .output()
        .expect("run clickhousectl");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "stderr: {stderr}");
    assert!(
        stderr.contains("error: invalid value"),
        "unexpected stderr: {stderr}"
    );
    assert!(
        stderr.contains(expected_error),
        "missing actionable parser error: {stderr}"
    );
    assert!(
        stderr.contains("For more information, try '--help'."),
        "missing clap help guidance: {stderr}"
    );
    assert!(
        !home.path().join(".clickhouse").exists(),
        "invalid version created home state"
    );
    assert!(
        !project.path().join(".clickhouse").exists(),
        "invalid version created project state"
    );
}

#[test]
fn invalid_local_versions_fail_before_dispatch_or_filesystem_setup() {
    assert_rejected_before_side_effects(
        &["local", "install", "not.a.version"],
        "all parts must be numeric",
    );
    assert_rejected_before_side_effects(
        &["local", "use", "25.12.9"],
        "3-part version '25.12.9' is not supported",
    );
    assert_rejected_before_side_effects(
        &["local", "server", "start", "--version", "25.12.9.61.2"],
        "expected 1-2 or 4 parts",
    );
}

#[test]
fn postgres_install_selector_is_rejected_by_clickhouse_only_commands() {
    assert_rejected_before_side_effects(
        &["local", "use", "postgres@18"],
        "only supported by `local install`; `local use` requires a ClickHouse version",
    );
    assert_rejected_before_side_effects(
        &["local", "server", "start", "--version", "postgres@18"],
        "only supported by `local install`; `local server start --version` requires a ClickHouse version",
    );
}
