//! Subprocess coverage for the `local remove` default-version guard (issue #599).
//!
//! Strategy: spawn the binary with `HOME=<tempdir>`, pre-seed two fake installed
//! versions plus the `~/.clickhouse/default` marker and the global
//! `~/.local/bin/clickhouse` symlink, then assert that removing the default is
//! refused (exit 1, nothing deleted) unless `--force` is passed, and that both
//! human and `--json` output carry the same facts.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_VERSION: &str = "26.9.1.217";
const OTHER_VERSION: &str = "25.12.9.61";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

struct Home {
    home: tempfile::TempDir,
    project: tempfile::TempDir,
}

impl Home {
    /// Two installed versions, `DEFAULT_VERSION` marked default and linked from
    /// `~/.local/bin/clickhouse`.
    fn with_default_version() -> Self {
        let home = tempfile::tempdir().expect("create home");
        let project = tempfile::tempdir().expect("create project");
        let this = Self { home, project };

        for version in [DEFAULT_VERSION, OTHER_VERSION] {
            let binary = this.binary(version);
            std::fs::create_dir_all(binary.parent().unwrap()).expect("create version dir");
            std::fs::write(&binary, b"#!/bin/sh\necho stub\n").expect("write fake binary");
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
                .expect("make fake binary executable");
        }

        std::fs::write(this.default_marker(), DEFAULT_VERSION).expect("write default marker");
        let link = this.global_symlink();
        std::fs::create_dir_all(link.parent().unwrap()).expect("create ~/.local/bin");
        std::os::unix::fs::symlink(this.binary(DEFAULT_VERSION), &link).expect("create symlink");

        this
    }

    fn binary(&self, version: &str) -> PathBuf {
        self.home
            .path()
            .join(".clickhouse/versions")
            .join(version)
            .join("clickhouse")
    }

    fn version_dir(&self, version: &str) -> PathBuf {
        self.home.path().join(".clickhouse/versions").join(version)
    }

    fn default_marker(&self) -> PathBuf {
        self.home.path().join(".clickhouse/default")
    }

    fn global_symlink(&self) -> PathBuf {
        self.home.path().join(".local/bin/clickhouse")
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(clickhousectl_binary());
        command
            // `env_clear` keeps coding-agent detection from switching the
            // subprocess to JSON, so human-output assertions stay meaningful.
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", self.home.path())
            .current_dir(self.project.path())
            .args(args);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("run clickhousectl")
    }

    fn spawn(&self, args: &[&str]) -> Child {
        self.command(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn clickhousectl")
    }

    /// Takes the same `flock` the binary uses to serialize install commits,
    /// default-marker writes and removals, so a spawned command blocks at that
    /// point until the returned handle is dropped.
    fn hold_commit_lock(&self) -> std::fs::File {
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.versions_dir().join(".install-commit.lock"))
            .expect("open commit lock");
        lock.lock().expect("hold commit lock");
        lock
    }

    fn versions_dir(&self) -> PathBuf {
        self.home.path().join(".clickhouse/versions")
    }

    /// `local remove` creates its staging directory immediately before it
    /// blocks on the commit lock, so its appearance proves the pre-lock guard
    /// has already been passed.
    fn wait_for_staging_dir(&self) {
        let staging = self.versions_dir().join(".staging");
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Ok(entries) = std::fs::read_dir(&staging)
                && entries
                    .flatten()
                    .any(|e| e.file_name().to_string_lossy().starts_with("install-"))
            {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "remove never reached the commit lock"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn assert_default_state_intact(&self) {
        assert!(
            self.version_dir(DEFAULT_VERSION).exists(),
            "the default version directory must survive a refused removal"
        );
        assert_eq!(
            std::fs::read_to_string(self.default_marker()).expect("default marker"),
            DEFAULT_VERSION,
            "the default marker must survive a refused removal"
        );
        assert_eq!(
            std::fs::read_link(self.global_symlink()).expect("global symlink"),
            self.binary(DEFAULT_VERSION),
            "the global symlink must survive a refused removal"
        );
    }
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn removing_the_default_version_is_refused_and_changes_nothing() {
    let home = Home::with_default_version();

    let output = home.run(&["local", "remove", DEFAULT_VERSION]);

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    let named_default = format!("{DEFAULT_VERSION} is the current default");
    for required in [
        named_default.as_str(),
        "~/.clickhouse/default",
        "~/.local/bin/clickhouse",
        "clickhousectl local use <other-version>",
        "--force",
    ] {
        assert!(stderr.contains(required), "missing {required:?}: {stderr}");
    }
    home.assert_default_state_intact();
}

#[test]
fn refusing_to_remove_the_default_version_emits_the_structured_error() {
    let home = Home::with_default_version();

    let output = home.run(&["local", "--json", "remove", DEFAULT_VERSION]);

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        output.stdout.is_empty(),
        "runtime errors belong on stderr, got: {}",
        stdout_of(&output)
    );
    let error: serde_json::Value =
        serde_json::from_str(&stderr).expect("one JSON error object on stderr");
    assert_eq!(error["error"]["code"], "version_is_default");
    let message = error["error"]["message"].as_str().expect("message");
    for required in [
        "current default",
        "~/.clickhouse/default",
        "~/.local/bin/clickhouse",
        "--force",
    ] {
        assert!(
            message.contains(required),
            "missing {required:?}: {message}"
        );
    }
    assert_eq!(error["error"]["command"], "clickhousectl local use latest");
    home.assert_default_state_intact();
}

#[test]
fn force_removes_the_default_version_and_warns_about_the_marker_and_symlink() {
    let home = Home::with_default_version();

    let output = home.run(&["local", "remove", DEFAULT_VERSION, "--force"]);

    let stderr = stderr_of(&output);
    let stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstderr: {stderr}",
        output.status
    );
    assert!(
        stderr.contains("Warning:")
            && stderr.contains("is the default version")
            && stderr.contains("~/.clickhouse/default")
            && stderr.contains("~/.local/bin/clickhouse"),
        "missing prominent warning: {stderr}"
    );
    assert!(
        stdout.contains(&format!("Removed version {DEFAULT_VERSION}"))
            && stdout.contains("clickhousectl local use latest"),
        "missing removal confirmation and recovery hint: {stdout}"
    );

    assert!(
        !home.version_dir(DEFAULT_VERSION).exists(),
        "--force must remove the version directory"
    );
    assert!(
        !home.default_marker().exists(),
        "--force must clear the default marker"
    );
    assert!(
        std::fs::symlink_metadata(home.global_symlink()).is_err(),
        "--force must remove the global symlink pointing into the removed version"
    );
    assert!(
        home.version_dir(OTHER_VERSION).exists(),
        "other installed versions must be untouched"
    );
}

#[test]
fn force_json_output_reports_that_the_default_was_cleared() {
    let home = Home::with_default_version();

    let output = home.run(&["local", "--json", "remove", DEFAULT_VERSION, "--force"]);

    let stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstderr: {}",
        output.status,
        stderr_of(&output)
    );
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("one JSON object");
    assert_eq!(value["version"], DEFAULT_VERSION);
    assert_eq!(value["was_default"], true);
    assert!(
        !home.default_marker().exists(),
        "--force must clear the default marker"
    );
}

#[test]
fn removing_a_non_default_version_still_needs_no_force_and_keeps_the_default() {
    let home = Home::with_default_version();

    let output = home.run(&["local", "--json", "remove", OTHER_VERSION]);

    let stdout = stdout_of(&output);
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstderr: {}",
        output.status,
        stderr_of(&output)
    );
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("one JSON object");
    assert_eq!(value["version"], OTHER_VERSION);
    assert_eq!(value["was_default"], false);
    assert!(!home.version_dir(OTHER_VERSION).exists());
    home.assert_default_state_intact();
}

#[test]
fn a_stale_default_marker_still_guards_and_is_cleared_by_force() {
    let home = Home::with_default_version();
    // The marker names a version whose binary is gone: `local which` already
    // fails here, and removal must still refuse, then clean the marker up under
    // `--force` rather than leaving it dangling.
    std::fs::remove_file(home.binary(DEFAULT_VERSION)).expect("delete fake binary");

    let refused = home.run(&["local", "remove", DEFAULT_VERSION]);
    assert_eq!(
        refused.status.code(),
        Some(1),
        "stderr: {}",
        stderr_of(&refused)
    );
    assert!(
        stderr_of(&refused).contains("is the current default"),
        "stale marker must still guard: {}",
        stderr_of(&refused)
    );
    assert!(home.version_dir(DEFAULT_VERSION).exists());
    assert!(home.default_marker().exists());

    let forced = home.run(&["local", "remove", DEFAULT_VERSION, "--force"]);
    assert!(
        forced.status.success(),
        "expected success, got {:?}\nstderr: {}",
        forced.status,
        stderr_of(&forced)
    );
    assert!(!home.version_dir(DEFAULT_VERSION).exists());
    assert!(
        !home.default_marker().exists(),
        "--force must clear a stale default marker too"
    );
}

#[test]
fn a_version_made_default_while_remove_waits_for_the_lock_is_still_refused() {
    let home = Home::with_default_version();
    // The other version is not the default when `remove` takes its pre-lock
    // snapshot, so the early guard passes. Hold the commit lock so the removal
    // blocks, then switch the default to it — what a concurrent `local use`
    // would do — before letting the removal continue.
    let lock = home.hold_commit_lock();
    let child = home.spawn(&["local", "--json", "remove", OTHER_VERSION]);
    home.wait_for_staging_dir();
    std::fs::write(home.default_marker(), OTHER_VERSION).expect("switch default");
    drop(lock);

    let output = child.wait_with_output().expect("wait for remove");

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    let error: serde_json::Value =
        serde_json::from_str(&stderr).expect("one JSON error object on stderr");
    assert_eq!(error["error"]["code"], "version_is_default");
    assert!(
        home.version_dir(OTHER_VERSION).exists(),
        "the newly default version must survive the racing removal"
    );
    assert_eq!(
        std::fs::read_to_string(home.default_marker()).expect("default marker"),
        OTHER_VERSION,
        "the marker written during the lock wait must be kept"
    );
    assert!(
        std::fs::read_dir(home.versions_dir().join(".staging"))
            .map(|entries| entries
                .flatten()
                .all(|e| !e.file_name().to_string_lossy().starts_with("install-")))
            .unwrap_or(true),
        "a refused removal must clean up its staging directory"
    );
}

#[test]
fn local_use_writes_the_default_marker_under_the_commit_lock() {
    let home = Home::with_default_version();
    let lock = home.hold_commit_lock();
    let mut child = home.spawn(&["local", "--json", "use", OTHER_VERSION]);

    // While the lock is held the marker cannot move, and `use` cannot finish.
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(
        std::fs::read_to_string(home.default_marker()).expect("default marker"),
        DEFAULT_VERSION,
        "`local use` must not write the marker while the commit lock is held"
    );
    assert!(
        child.try_wait().expect("poll use").is_none(),
        "`local use` must block on the commit lock rather than finish"
    );

    drop(lock);
    let output = child.wait_with_output().expect("wait for use");
    assert!(
        output.status.success(),
        "expected success once the lock is released, got {:?}\nstderr: {}",
        output.status,
        stderr_of(&output)
    );
    assert_eq!(
        std::fs::read_to_string(home.default_marker()).expect("default marker"),
        OTHER_VERSION
    );
}
