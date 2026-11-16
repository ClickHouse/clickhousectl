//! Subprocess coverage for `local install --binary` (issue #129): a local
//! ClickHouse build is copied in under the version its `--version` reports,
//! then listed and removed like any other install.
//!
//! The build is the compiled stand-in from `support/fake_clickhouse.rs`, a
//! real executable (not a script) whose `--version` output the test chooses.

#[path = "support/fake_clickhouse.rs"]
#[allow(dead_code)] // Only `install` is used here.
mod fake_clickhouse;

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// What the stand-in reports by default.
const VERSION: &str = "26.9.1.4242";

struct Fixture {
    home: tempfile::TempDir,
    build: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().expect("create HOME");
        let build = home.path().join("ClickHouse/build/programs/clickhouse");
        fake_clickhouse::install(&build);
        Self { home, build }
    }

    fn versions(&self) -> PathBuf {
        self.home.path().join(".clickhouse/versions")
    }

    fn run(&self, args: &[&str], version_output: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_clickhousectl"));
        command
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
            .env("HOME", self.home.path())
            .current_dir(self.home.path())
            .args(args);
        if let Some(output) = version_output {
            command.env("FAKE_CLICKHOUSE_VERSION_OUTPUT", output);
        }
        command.output().expect("run clickhousectl")
    }

    fn install(&self, extra: &[&str], version_output: Option<&str>) -> Output {
        let build = self.build.to_str().unwrap();
        let mut args = vec!["local", "--json", "install", "--binary", build];
        args.extend_from_slice(extra);
        self.run(&args, version_output)
    }

    /// Installed version directories, without the version manager's own
    /// dot-prefixed staging and lock entries.
    fn installed(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(self.versions()) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| !name.starts_with('.'))
            .collect();
        names.sort();
        names
    }
}

fn stdout_json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "status {:?}, stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON on stdout")
}

fn error_code(output: &Output) -> String {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error on stderr");
    error["error"]["code"].as_str().unwrap().to_string()
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    std::fs::read(a).unwrap() == std::fs::read(b).unwrap()
}

#[test]
fn a_local_build_installs_under_its_reported_version_and_lists_and_removes() {
    let fixture = Fixture::new();

    let installed = stdout_json(&fixture.install(&[], None));
    assert_eq!(installed["version"], VERSION);
    assert_eq!(installed["set_as_default"], true);
    assert_eq!(fixture.installed(), [VERSION]);
    let binary = fixture.versions().join(VERSION).join("clickhouse");
    assert!(same_bytes(&binary, &fixture.build), "a copy, not a link");
    assert!(!binary.is_symlink());

    let listed = stdout_json(&fixture.run(&["local", "--json", "list"], None));
    assert_eq!(
        listed["versions"],
        serde_json::json!([{ "version": VERSION, "default": true }])
    );

    // The same version again needs --force, which replaces it.
    assert_eq!(
        error_code(&fixture.install(&[], None)),
        "version_already_installed"
    );
    let replaced = stdout_json(&fixture.install(&["--force"], None));
    assert_eq!(replaced["version"], VERSION);
    assert_eq!(replaced["set_as_default"], false);

    // It is the default, so removing it takes --force like any default.
    stdout_json(&fixture.run(&["local", "--json", "remove", VERSION, "--force"], None));
    assert!(fixture.installed().is_empty());
}

#[test]
fn a_binary_that_does_not_report_a_clickhouse_version_installs_nothing() {
    let fixture = Fixture::new();
    for output in ["Python 3.12.1", "tool 1.2.3.4"] {
        assert_eq!(
            error_code(&fixture.install(&[], Some(output))),
            "not_a_clickhouse_binary",
            "{output}"
        );
    }
    assert!(fixture.installed().is_empty());
    assert!(!fixture.home.path().join(".clickhouse/default").exists());
    // Its staged copy is gone too.
    let staging = fixture.versions().join(".staging");
    let leftovers: Vec<_> = std::fs::read_dir(&staging)
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().file_name())
                .filter(|name| name.to_string_lossy().starts_with("install-"))
                .collect()
        })
        .unwrap_or_default();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}
