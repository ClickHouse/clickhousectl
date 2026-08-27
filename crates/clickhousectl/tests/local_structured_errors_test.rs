//! Subprocess coverage for the stable local structured-error contract.

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

fn expected_error(code: &str, message: &str, recovery: Option<&str>) -> String {
    let mut error = serde_json::Map::new();
    error.insert("code".into(), code.into());
    error.insert("message".into(), message.into());
    if let Some(command) = recovery {
        error.insert("command".into(), command.into());
    }
    let value = serde_json::json!({ "error": error });
    format!("{}\n", serde_json::to_string_pretty(&value).unwrap())
}

fn assert_structured_failure(output: &Output, expected: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "runtime errors belong on stderr");
    assert_eq!(String::from_utf8_lossy(&output.stderr), expected);
    serde_json::from_slice::<serde_json::Value>(&output.stderr).expect("one JSON error object");
}

fn install_fake_clickhouse(home: &Path, script: &str) {
    let binary = home.join(format!(".clickhouse/versions/{VERSION}/clickhouse"));
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version directory");
    std::fs::write(&binary, script).expect("write fake ClickHouse");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
        .expect("make fake ClickHouse executable");
}

fn unused_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .expect("bind temporary port")
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn explicit_json_and_agent_mode_emit_the_same_exact_server_error() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let expected = expected_error(
        "server_not_found",
        "Server 'default' not found",
        Some("clickhousectl local server list"),
    );

    let explicit = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop"],
    );
    assert_structured_failure(&explicit, &expected);

    let agent = command(project.path(), home.path())
        .env("AGENT", "opencode")
        .args(["local", "server", "stop"])
        .output()
        .expect("run clickhousectl in agent mode");
    assert_structured_failure(&agent, &expected);
}

#[cfg(feature = "telemetry")]
#[test]
fn fresh_home_json_error_defers_telemetry_notice_to_human_mode() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let fresh_home_command = || {
        let mut command = Command::new(clickhousectl_binary());
        command
            .env_clear()
            .env("HOME", home.path())
            .current_dir(project.path());
        command
    };

    let output = fresh_home_command()
        .args(["local", "--json", "server", "stop"])
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
        .args(["local", "server", "stop"])
        .output()
        .expect("run human failure");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("human stderr is UTF-8");
    assert!(
        stderr.contains("Error: Server 'default' not found"),
        "{stderr}"
    );
    assert!(stderr.contains("anonymous usage data"), "{stderr}");
    assert!(home.path().join(".clickhouse/telemetry.json").exists());
}

#[cfg(feature = "telemetry")]
#[test]
fn telemetry_debug_does_not_append_to_a_structured_error() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let state_path = home.path().join(".clickhouse/telemetry.json");
    std::fs::create_dir_all(state_path.parent().unwrap()).expect("create telemetry directory");
    std::fs::write(state_path, r#"{"disabled":false}"#).expect("enable telemetry");

    let output = command(project.path(), home.path())
        .env_remove("DO_NOT_TRACK")
        .env("CHCTL_TELEMETRY_DEBUG", "1")
        .args(["local", "--json", "server", "stop"])
        .output()
        .expect("run structured failure with telemetry debug");

    assert_structured_failure(
        &output,
        &expected_error(
            "server_not_found",
            "Server 'default' not found",
            Some("clickhousectl local server list"),
        ),
    );
}

#[test]
fn version_port_and_startup_failures_have_typed_safe_shapes() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    let unavailable = run(
        project.path(),
        home.path(),
        &["local", "--json", "remove", "99.99.1.1"],
    );
    assert_structured_failure(
        &unavailable,
        &expected_error(
            "version_unavailable",
            "Requested version is unavailable",
            Some("clickhousectl local list --remote"),
        ),
    );

    install_fake_clickhouse(home.path(), "#!/bin/sh\nexit 7\n");
    let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("occupy HTTP port");
    let port = occupied.local_addr().unwrap().port();
    let port_arg = port.to_string();
    let port_error = run(
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
            &port_arg,
        ],
    );
    assert_structured_failure(
        &port_error,
        &expected_error(
            "port_in_use",
            &format!("HTTP port {port} is already in use"),
            Some("clickhousectl local server start --help"),
        ),
    );
    drop(occupied);

    let http_port = unused_port().to_string();
    let tcp_port = unused_port().to_string();
    let startup = run(
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
            "--no-wait",
        ],
    );
    assert_structured_failure(
        &startup,
        &expected_error(
            "startup_exit",
            "ClickHouse server 'default' exited before becoming ready",
            Some("clickhousectl local server list"),
        ),
    );
}

#[test]
fn io_and_fallback_errors_redact_paths_docker_details_and_secrets() {
    let root = tempfile::tempdir().expect("create root");
    let project = root.path().join("project-private-token");
    let home = root.path().join("home-private-token");
    std::fs::create_dir_all(project.join(".clickhouse/servers")).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        project.join(".clickhouse/servers/default.json"),
        b"{ private SQL and password=hunter2",
    )
    .unwrap();

    let io_error = run(&project, &home, &["local", "--json", "server", "list"]);
    assert_structured_failure(
        &io_error,
        &expected_error("io_error", "Local I/O operation failed", None),
    );
    std::fs::remove_file(project.join(".clickhouse/servers/default.json")).unwrap();

    let docker_secret = home.join("docker-secret-token.sock");
    let fallback = command(&project, &home)
        .env("DOCKER_HOST", format!("unix://{}", docker_secret.display()))
        .args(["local", "--json", "postgres", "start"])
        .output()
        .expect("run Docker fallback failure");
    assert_structured_failure(
        &fallback,
        &expected_error("local_error", "Local command failed", None),
    );

    for output in [&io_error, &fallback] {
        let stderr = String::from_utf8_lossy(&output.stderr);
        for sensitive in [
            "project-private-token",
            "home-private-token",
            "docker-secret-token",
            "hunter2",
            "private SQL",
            "DOCKER_HOST",
        ] {
            assert!(!stderr.contains(sensitive), "leaked {sensitive}: {stderr}");
        }
    }
}

#[test]
fn human_and_clap_errors_keep_their_existing_formats() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    let human = run(project.path(), home.path(), &["local", "server", "stop"]);
    assert_eq!(human.status.code(), Some(1));
    assert!(human.stdout.is_empty());
    assert_eq!(
        String::from_utf8_lossy(&human.stderr),
        "Error: Server 'default' not found\n"
    );

    let clap = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop", "--unknown"],
    );
    assert_eq!(clap.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&clap.stderr);
    assert!(stderr.starts_with("error: unexpected argument '--unknown'"));
    assert!(!stderr.contains("server_not_found"));
}

#[test]
fn foreground_child_exit_is_not_wrapped_as_a_local_error() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
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
            "--foreground",
        ],
    );

    assert_eq!(output.status.code(), Some(7));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Server 'default' running"), "{stderr}");
    assert!(!stderr.contains("\"error\""), "{stderr}");
    assert!(!stderr.contains("Error: child process exited"), "{stderr}");
}
