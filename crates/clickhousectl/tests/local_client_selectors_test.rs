//! Subprocess coverage for local client selector and native binary selection.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VERSION_A: &str = "25.12.9.61";
const VERSION_B: &str = "26.8.1.1760";
const MISSING_VERSION: &str = "27.1.2.3";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn write_executable(path: &Path, script: &str) {
    std::fs::create_dir_all(path.parent().expect("fake child parent"))
        .expect("create fake child directory");
    std::fs::write(path, script).expect("write fake child");
    let mut permissions = std::fs::metadata(path)
        .expect("read fake child metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("make fake child executable");
}

fn write_arg_printer(path: &Path) {
    write_executable(path, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n");
}

fn install_fake_clickhouse(home: &Path, version: &str) {
    let binary = home
        .join(".clickhouse/versions")
        .join(version)
        .join("clickhouse");
    write_executable(
        &binary,
        &format!("#!/bin/sh\nprintf '%s\\n' 'binary:{version}'\nprintf '%s\\n' \"$@\"\n"),
    );
}

fn write_default(home: &Path, version: &str) {
    let path = home.join(".clickhouse/default");
    std::fs::create_dir_all(path.parent().expect("default parent")).expect("create default parent");
    std::fs::write(path, version).expect("write default version");
}

fn run(project: &Path, home: &Path, path: Option<&Path>, args: &[&str]) -> Output {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project)
        .args(args);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.output().expect("run clickhousectl")
}

fn assert_child_args(output: Output, expected: &[&str]) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    let args: Vec<&str> = std::str::from_utf8(&output.stdout)
        .expect("child output is UTF-8")
        .lines()
        .collect();
    assert_eq!(args, expected);
}

fn assert_clickhouse_child(output: Output, version: &str, expected: &[&str]) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    let lines: Vec<&str> = std::str::from_utf8(&output.stdout)
        .expect("child output is UTF-8")
        .lines()
        .collect();
    let marker = format!("binary:{version}");
    assert_eq!(lines.first().copied(), Some(marker.as_str()));
    assert_eq!(&lines[1..], expected);
}

fn assert_runtime_error(output: Output, expected: &[&str]) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    for text in expected {
        assert!(stderr.contains(text), "missing {text:?} in: {stderr}");
    }
}

fn assert_default(home: &Path, expected: Option<&str>) {
    let path = home.join(".clickhouse/default");
    match expected {
        Some(version) => assert_eq!(
            std::fs::read_to_string(path).expect("read default version"),
            version
        ),
        None => assert!(!path.exists(), "client unexpectedly created a default"),
    }
}

fn assert_usage_before_resolution(args: &[&str], expected: &[&str]) {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let output = run(project.path(), home.path(), None, args);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "stderr: {stderr}");
    for text in expected {
        assert!(stderr.contains(text), "missing {text:?} in: {stderr}");
    }
    assert!(!stderr.contains("No default version"), "{stderr}");
    assert!(!stderr.contains("Failed to execute"), "{stderr}");
    assert!(
        !home.path().join(".clickhouse").exists(),
        "parser error resolved a ClickHouse binary"
    );
    assert!(
        !project.path().join(".clickhouse").exists(),
        "parser error resolved project state"
    );
}

#[test]
fn invalid_clickhouse_selectors_fail_before_binary_or_project_resolution() {
    for args in [
        &["local", "client", "--name", "dev", "--host", "remote"][..],
        &["local", "client", "--host", "remote", "--name", "dev"],
        &["local", "client", "--name", "dev", "--port", "9000"],
        &["local", "client", "--port", "9000", "--name", "dev"],
        &["local", "client", "--name", "dev", "--version", VERSION_A],
    ] {
        assert_usage_before_resolution(args, &["--name", "cannot be used"]);
    }
    assert_usage_before_resolution(
        &["local", "client", "--version", VERSION_A],
        &["required arguments", "--host", "--port"],
    );
    assert_usage_before_resolution(
        &["local", "client", "--port", "0"],
        &["invalid value", "--port"],
    );
    assert_usage_before_resolution(
        &["local", "client", "--port", "not-a-port"],
        &["invalid value", "--port"],
    );
}

#[test]
fn clickhouse_direct_default_and_single_installed_selection_reach_fake_child() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_A);
    install_fake_clickhouse(home.path(), VERSION_B);
    write_default(home.path(), VERSION_A);

    assert_clickhouse_child(
        run(
            project.path(),
            home.path(),
            None,
            &["local", "client", "--host", "remote", "--query", "SELECT 1"],
        ),
        VERSION_A,
        &[
            "client", "--host", "remote", "--port", "9000", "--query", "SELECT 1",
        ],
    );
    assert_default(home.path(), Some(VERSION_A));

    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_B);
    assert_clickhouse_child(
        run(
            project.path(),
            home.path(),
            None,
            &["local", "client", "--port", "65535"],
        ),
        VERSION_B,
        &["client", "--host", "localhost", "--port", "65535"],
    );
    assert_default(home.path(), None);
}

#[test]
fn clickhouse_direct_without_version_reports_zero_and_multiple_installs() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    assert_runtime_error(
        run(
            project.path(),
            home.path(),
            None,
            &["local", "client", "--host", "remote"],
        ),
        &[
            "No ClickHouse client versions are installed",
            "clickhousectl local install <version>",
        ],
    );
    assert_default(home.path(), None);

    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_A);
    install_fake_clickhouse(home.path(), VERSION_B);
    assert_runtime_error(
        run(
            project.path(),
            home.path(),
            None,
            &["local", "client", "--host", "remote"],
        ),
        &[
            "Multiple ClickHouse client versions are installed",
            "--version <version>",
            "clickhousectl local use <version>",
        ],
    );
    assert_default(home.path(), None);
}

#[test]
fn clickhouse_direct_explicit_version_overrides_but_never_changes_default() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_B);
    assert_clickhouse_child(
        run(
            project.path(),
            home.path(),
            None,
            &[
                "local",
                "client",
                "--host",
                "remote",
                "--version",
                VERSION_B,
            ],
        ),
        VERSION_B,
        &["client", "--host", "remote", "--port", "9000"],
    );
    assert_default(home.path(), None);

    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_A);
    install_fake_clickhouse(home.path(), VERSION_B);
    write_default(home.path(), VERSION_A);

    assert_clickhouse_child(
        run(
            project.path(),
            home.path(),
            None,
            &["local", "client", "--host", "remote", "--version", "26.8"],
        ),
        VERSION_B,
        &["client", "--host", "remote", "--port", "9000"],
    );
    assert_default(home.path(), Some(VERSION_A));
}

#[test]
fn clickhouse_direct_reports_stale_default_and_explicit_missing_version() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_B);
    write_default(home.path(), VERSION_A);

    let stale_message = format!("Default ClickHouse version '{VERSION_A}' is not installed");
    assert_runtime_error(
        run(
            project.path(),
            home.path(),
            None,
            &["local", "client", "--host", "remote"],
        ),
        &[
            &stale_message,
            "clickhousectl local use <version>",
            "--version <installed-version>",
        ],
    );
    assert_default(home.path(), Some(VERSION_A));

    assert_clickhouse_child(
        run(
            project.path(),
            home.path(),
            None,
            &[
                "local",
                "client",
                "--host",
                "remote",
                "--version",
                VERSION_B,
            ],
        ),
        VERSION_B,
        &["client", "--host", "remote", "--port", "9000"],
    );
    assert_default(home.path(), Some(VERSION_A));

    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_A);
    write_default(home.path(), VERSION_A);
    let missing_message = format!("ClickHouse client version '{MISSING_VERSION}' is not installed");
    let install_message = format!("clickhousectl local install {MISSING_VERSION}");
    assert_runtime_error(
        run(
            project.path(),
            home.path(),
            None,
            &[
                "local",
                "client",
                "--host",
                "remote",
                "--version",
                MISSING_VERSION,
            ],
        ),
        &[
            &missing_message,
            &install_message,
            "clickhousectl local list",
        ],
    );
    assert_default(home.path(), Some(VERSION_A));
}

#[test]
fn clickhouse_named_mode_uses_server_version_without_a_default() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), VERSION_B);

    let servers = project.path().join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).expect("create server metadata directory");
    std::fs::write(
        servers.join("dev.json"),
        serde_json::to_vec(&serde_json::json!({
            "name": "dev",
            "pid": std::process::id(),
            "version": VERSION_B,
            "http_port": 8123,
            "tcp_port": 19000,
            "started_at": "test",
            "cwd": project.path(),
            "engine": "clickhouse"
        }))
        .expect("serialize server metadata"),
    )
    .expect("write server metadata");

    assert_clickhouse_child(
        run(
            project.path(),
            home.path(),
            None,
            &["local", "client", "--name", "dev"],
        ),
        VERSION_B,
        &["client", "--host", "localhost", "--port", "19000"],
    );
    assert_default(home.path(), None);
}

#[test]
fn postgres_client_uses_the_same_validation_and_direct_mode_defaults() {
    assert_usage_before_resolution(
        &[
            "local", "postgres", "client", "--name", "dev", "--host", "remote",
        ],
        &["--name", "cannot be used"],
    );
    assert_usage_before_resolution(
        &[
            "local",
            "postgres",
            "client",
            "--version",
            "18",
            "--port",
            "5432",
        ],
        &["--version", "cannot be used"],
    );
    assert_usage_before_resolution(
        &["local", "postgres", "client", "--port", "0"],
        &["invalid value", "--port"],
    );

    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let bin = home.path().join("bin");
    write_arg_printer(&bin.join("psql"));

    assert_child_args(
        run(
            project.path(),
            home.path(),
            Some(&bin),
            &[
                "local", "postgres", "client", "--host", "remote", "--query", "SELECT 1",
            ],
        ),
        &[
            "-h", "remote", "-p", "5432", "-U", "postgres", "-d", "postgres", "-c", "SELECT 1",
        ],
    );
    assert_child_args(
        run(
            project.path(),
            home.path(),
            Some(&bin),
            &["local", "postgres", "client", "--port", "65535"],
        ),
        &[
            "-h",
            "127.0.0.1",
            "-p",
            "65535",
            "-U",
            "postgres",
            "-d",
            "postgres",
        ],
    );
}
