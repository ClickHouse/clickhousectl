//! Subprocess coverage for local structured runtime errors (issue #475).

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VERSION: &str = "25.12.9.61";

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn command(project: &Path, home: &Path) -> Command {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project);
    command
}

fn run(project: &Path, home: &Path, args: &[&str]) -> Output {
    command(project, home)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn expected_error(code: &str, message: &str, command: &str) -> String {
    format!(
        "{{\n  \"error\": {{\n    \"code\": \"{code}\",\n    \"message\": \"{message}\",\n    \"command\": \"{command}\"\n  }}\n}}\n"
    )
}

fn assert_structured_error(output: &Output, expected: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), expected);
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("stderr is one JSON object");
    assert!(parsed.get("error").is_some());
}

fn install_fake_clickhouse(home: &Path, script: &str) {
    let binary = home
        .join(".clickhouse/versions")
        .join(VERSION)
        .join("clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version dir");
    std::fs::write(&binary, script).expect("write fake ClickHouse");
    let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(binary, permissions).expect("make fake ClickHouse executable");
}

fn unused_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .expect("bind temporary port")
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn explicit_json_writes_exact_server_not_found_error_to_stderr() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let output = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop", "missing"],
    );

    assert_structured_error(
        &output,
        &expected_error(
            "server_not_found",
            "Server 'missing' not found",
            "clickhousectl local server list",
        ),
    );
}

#[test]
fn fresh_home_json_error_defers_telemetry_notice_to_human_mode() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let fresh_home_command = || {
        let mut command = Command::new(clickhousectl_binary());
        command
            .env_clear()
            .env("HOME", home.path())
            .current_dir(project.path());
        command
    };

    let output = fresh_home_command()
        .args(["local", "--json", "server", "stop", "missing"])
        .output()
        .expect("run structured failure");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("all stderr is exactly one JSON value");
    assert_eq!(parsed["error"]["code"], "server_not_found");
    assert!(
        !home.path().join(".clickhouse/telemetry.json").exists(),
        "structured output must leave first-run consent pending"
    );

    let output = fresh_home_command()
        .args(["local", "server", "stop", "missing"])
        .output()
        .expect("run human failure");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("human stderr is UTF-8");
    assert!(
        stderr.contains("Error: Server 'missing' not found"),
        "{stderr}"
    );
    assert!(stderr.contains("anonymous usage data"), "{stderr}");
    assert!(home.path().join(".clickhouse/telemetry.json").exists());
}

#[test]
fn telemetry_debug_does_not_append_to_a_structured_error() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let state_path = home.path().join(".clickhouse/telemetry.json");
    std::fs::create_dir_all(state_path.parent().unwrap()).expect("create telemetry directory");
    std::fs::write(state_path, r#"{"disabled":false}"#).expect("enable telemetry");

    let output = command(project.path(), home.path())
        .env_remove("DO_NOT_TRACK")
        .env("CHCTL_TELEMETRY_DEBUG", "1")
        .args(["local", "--json", "server", "stop", "missing"])
        .output()
        .expect("run structured failure with telemetry debug");

    assert_structured_error(
        &output,
        &expected_error(
            "server_not_found",
            "Server 'missing' not found",
            "clickhousectl local server list",
        ),
    );
}

#[test]
fn agent_mode_writes_the_same_structured_error_without_json_flag() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let output = command(project.path(), home.path())
        .env("AGENT", "opencode")
        .args(["local", "server", "stop", "missing"])
        .output()
        .expect("run clickhousectl");

    assert_structured_error(
        &output,
        &expected_error(
            "server_not_found",
            "Server 'missing' not found",
            "clickhousectl local server list",
        ),
    );
}

#[test]
fn human_mode_keeps_concise_error_text() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let output = run(
        project.path(),
        home.path(),
        &["local", "server", "stop", "missing"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"Error: Server 'missing' not found\n");
}

#[test]
fn malformed_versions_remain_clap_errors_and_unavailable_versions_are_structured() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");

    let invalid = run(
        project.path(),
        home.path(),
        &["local", "--json", "use", "not.a.version"],
    );
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
    let invalid_stderr = String::from_utf8_lossy(&invalid.stderr);
    assert!(
        invalid_stderr.contains("error: invalid value"),
        "{invalid_stderr}"
    );
    assert!(
        invalid_stderr.contains("all parts must be numeric"),
        "{invalid_stderr}"
    );
    assert!(!invalid_stderr.contains("\"error\""), "{invalid_stderr}");

    let unavailable = run(
        project.path(),
        home.path(),
        &["local", "--json", "remove", "99.99.1.1"],
    );
    assert_structured_error(
        &unavailable,
        &expected_error(
            "version_unavailable",
            "Version 99.99.1.1 not found",
            "clickhousectl local list --remote",
        ),
    );
}

#[test]
fn occupied_port_has_an_exact_structured_error() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), "#!/bin/sh\nexit 0\n");
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("occupy port");
    let port = listener.local_addr().unwrap().port().to_string();
    let output = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "server",
            "start",
            "--version",
            VERSION,
            "--http-port",
            &port,
        ],
    );

    assert_structured_error(
        &output,
        &expected_error(
            "port_in_use",
            &format!("HTTP port {port} is already in use"),
            "clickhousectl local server start --help",
        ),
    );
}

#[test]
fn occupied_explicit_postgres_port_has_an_exact_structured_error() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("occupy port");
    let port = listener.local_addr().unwrap().port().to_string();
    let output = run(
        project.path(),
        home.path(),
        &["local", "--json", "postgres", "start", "--port", &port],
    );

    assert_structured_error(
        &output,
        &expected_error(
            "port_in_use",
            &format!(
                "explicit Postgres port {port} is already in use; choose a free --port or omit the flag to auto-select"
            ),
            "clickhousectl local server start --help",
        ),
    );
}

#[test]
fn non_port_postgres_validation_remains_a_generic_structured_error() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    let output = run(
        project.path(),
        home.path(),
        &["local", "--json", "postgres", "start", "--version", "16"],
    );

    assert_structured_error(
        &output,
        &expected_error(
            "local_error",
            "Postgres validation failed",
            "clickhousectl local postgres start --help",
        ),
    );
}

#[test]
fn startup_exit_is_structured_without_exposing_the_log_path() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(home.path(), "#!/bin/sh\nexit 7\n");
    let http_port = unused_port().to_string();
    let tcp_port = unused_port().to_string();
    let output = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "server",
            "start",
            "--version",
            VERSION,
            "--http-port",
            &http_port,
            "--tcp-port",
            &tcp_port,
        ],
    );

    assert_structured_error(
        &output,
        &expected_error(
            "startup_exit",
            "Server exited before startup completed",
            "clickhousectl local server list",
        ),
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains(project.path().to_str().unwrap()));
}

#[test]
fn io_and_fallback_errors_hide_raw_diagnostics() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    std::fs::create_dir_all(home.path().join(".clickhouse/default"))
        .expect("create directory at default file path");

    let io = run(project.path(), home.path(), &["local", "--json", "which"]);
    assert_structured_error(
        &io,
        &expected_error(
            "io_error",
            "Local filesystem operation failed",
            "clickhousectl local --help",
        ),
    );

    let fallback = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop", "../secret-name"],
    );
    assert_structured_error(
        &fallback,
        &expected_error(
            "local_error",
            "Local command failed",
            "clickhousectl local --help",
        ),
    );
    assert!(!String::from_utf8_lossy(&fallback.stderr).contains("secret-name"));
}

#[test]
fn native_client_keeps_its_streams_and_exit_status_in_json_mode() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(
        home.path(),
        "#!/bin/sh\nprintf 'native stdout\\n'\nprintf 'native stderr\\n' >&2\nexit 23\n",
    );
    let output = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "client",
            "--host",
            "localhost",
            "--version",
            VERSION,
        ],
    );

    assert_eq!(output.status.code(), Some(23));
    assert_eq!(output.stdout, b"native stdout\n");
    assert_eq!(output.stderr, b"native stderr\n");
}

#[test]
fn foreground_child_exit_is_not_wrapped() {
    let project = tempfile::tempdir().expect("create project tempdir");
    let home = tempfile::tempdir().expect("create home tempdir");
    install_fake_clickhouse(
        home.path(),
        "#!/bin/sh\nprintf 'server stdout\\n'\nprintf 'server stderr\\n' >&2\nexit 23\n",
    );
    let http_port = unused_port().to_string();
    let tcp_port = unused_port().to_string();
    let output = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "server",
            "start",
            "--foreground",
            "--version",
            VERSION,
            "--http-port",
            &http_port,
            "--tcp-port",
            &tcp_port,
        ],
    );

    assert_eq!(output.status.code(), Some(23));
    assert_eq!(output.stdout, b"server stdout\n");
    let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
    assert!(stderr.contains("Server 'default' running"), "{stderr}");
    assert!(stderr.contains("server stderr\n"), "{stderr}");
    assert!(!stderr.contains("\"error\""), "{stderr}");
    assert!(!stderr.contains("Error:"), "{stderr}");
}
