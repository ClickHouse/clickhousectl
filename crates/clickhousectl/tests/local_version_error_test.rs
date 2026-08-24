//! Regression coverage for local version-spec parsing.

use std::path::PathBuf;
use std::process::Command;

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

#[test]
fn invalid_local_versions_fail_as_clap_usage_errors() {
    for (args, expected) in [
        (
            &["local", "use", "not.a.version"][..],
            "all parts must be numeric",
        ),
        (
            &["local", "install", "25.12.9"][..],
            "3-part version '25.12.9' is not supported",
        ),
        (
            &["local", "server", "start", "--version", "not.a.version"][..],
            "all parts must be numeric",
        ),
    ] {
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let output = Command::new(clickhousectl_binary())
            .env("DO_NOT_TRACK", "1")
            .env("HOME", tempdir.path())
            .args(args)
            .output()
            .expect("run clickhousectl");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "stderr: {stderr}");
        assert!(stderr.contains("error: invalid value"), "stderr: {stderr}");
        assert!(stderr.contains(expected), "stderr: {stderr}");
        assert!(
            !stderr.contains("Error:"),
            "version reached runtime dispatch: {stderr}"
        );
    }
}
