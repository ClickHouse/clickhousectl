//! Subprocess coverage for the stable local structured-error contract.

use serde_json::{Value, json};
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
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
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

/// Expected envelopes compare by field. Every `message` only has to be a
/// string here (wording is unit-tested beside `LocalErrorOutput`); callers
/// check the values a message must carry.
const ANY_MESSAGE: &str = "<message>";

fn without_messages(mut value: Value) -> Value {
    fn walk(value: &mut Value) {
        match value {
            Value::Object(map) => {
                for (key, child) in map.iter_mut() {
                    if key == "message" && child.is_string() {
                        *child = ANY_MESSAGE.into();
                    } else {
                        walk(child);
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut value);
    value
}

fn expected_error(code: &str, recovery: Option<&str>) -> Value {
    let mut error = serde_json::Map::new();
    error.insert("code".into(), code.into());
    error.insert("message".into(), ANY_MESSAGE.into());
    if let Some(command) = recovery {
        error.insert("command".into(), command.into());
    }
    json!({ "error": error })
}

fn expected_project_server_error(project: &Path, name: &str) -> Value {
    let guidance = |action: &str, command: &str| json!({ "action": action, "message": ANY_MESSAGE, "command": command });
    json!({
        "error": {
            "code": "server_not_found",
            "message": ANY_MESSAGE,
            "project_scope": {
                "kind": "exact_current_project",
                "path": project.canonicalize().unwrap(),
                "parent_projects_searched": false
            },
            "server": { "name": name },
            "guidance": [
                guidance("return_to_project_root", "cd <project-root>"),
                guidance("list_project_servers", "clickhousectl local clickhouse list"),
                guidance("list_global_servers", "clickhousectl local clickhouse list --global"),
                guidance(
                    "stop_global_project_server",
                    "clickhousectl local clickhouse stop <name> --global --project <project-root>",
                ),
            ]
        }
    })
}

/// Exit 1, nothing on stdout, and stderr is exactly one pretty-printed JSON
/// error matching `expected`. Returns the actual message.
fn assert_structured_failure(output: &Output, expected: &Value) -> String {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "runtime errors belong on stderr");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let actual: Value = serde_json::from_str(&stderr).expect("one JSON error object");
    assert_eq!(
        stderr,
        format!("{}\n", serde_json::to_string_pretty(&actual).unwrap()),
        "pretty-printed with a trailing newline"
    );
    assert_eq!(without_messages(actual.clone()), *expected, "{stderr}");
    actual["error"]["message"].as_str().unwrap().to_owned()
}

fn install_fake_clickhouse(home: &Path, script: &str) {
    let binary = home.join(format!(".clickhouse/versions/{VERSION}/clickhouse"));
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version directory");
    std::fs::write(&binary, script).expect("write fake ClickHouse");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
        .expect("make fake ClickHouse executable");
}

#[test]
fn explicit_json_and_agent_mode_emit_the_same_exact_server_error() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let expected = expected_project_server_error(project.path(), "missing");

    let explicit = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop", "missing"],
    );
    assert_structured_failure(&explicit, &expected);

    let agent = command(project.path(), home.path())
        .env("AGENT", "opencode")
        .args(["local", "server", "stop", "missing"])
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
            .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
            .current_dir(project.path());
        command
    };

    let output = fresh_home_command()
        .args(["local", "--json", "server", "stop", "missing"])
        .output()
        .expect("run structured failure");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let parsed: Value =
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
    assert!(stderr.starts_with("Error: "), "{stderr}");
    assert!(stderr.contains("'missing'"), "{stderr}");
    // The first-run notice follows, naming its opt-outs.
    assert!(
        stderr.contains("`clickhousectl telemetry disable`"),
        "{stderr}"
    );
    assert!(stderr.contains("DO_NOT_TRACK=1"), "{stderr}");
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
        .args(["local", "--json", "server", "stop", "missing"])
        .output()
        .expect("run structured failure with telemetry debug");

    assert_structured_failure(
        &output,
        &expected_project_server_error(project.path(), "missing"),
    );
}

#[test]
fn version_port_and_startup_failures_have_typed_safe_shapes() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    // A version that is not installed is a local miss, not an unresolvable
    // download: the code and the hint both stay local.
    let not_installed = run(
        project.path(),
        home.path(),
        &["local", "--json", "remove", "99.99.1.1"],
    );
    let message = assert_structured_failure(
        &not_installed,
        &expected_error("version_not_installed", Some("clickhousectl local list")),
    );
    assert!(message.contains("99.99.1.1"), "{message}");

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
    let message = assert_structured_failure(
        &port_error,
        &expected_error(
            "port_in_use",
            Some("clickhousectl local clickhouse start --help"),
        ),
    );
    assert!(message.contains(&port_arg), "{message}");
    // Keep the occupied port reserved through its assertion. For the fake
    // child below, let the CLI choose ports: it never binds a socket, so
    // handing it bind-then-drop ephemeral ports only introduces a race.
    drop(occupied);
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
            "--no-wait",
        ],
    );
    let message = assert_structured_failure(
        &startup,
        &expected_error("startup_exit", Some("clickhousectl local clickhouse list")),
    );
    assert!(message.contains("'default'"), "{message}");
}

#[test]
fn io_and_docker_errors_redact_paths_daemon_details_and_secrets() {
    let root = tempfile::tempdir().expect("create root");
    let project = root.path().join("project-private-token");
    let home = root.path().join("home-private-token");
    std::fs::create_dir_all(project.join(".clickhouse")).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    let invalid_servers_dir = project.join(".clickhouse/servers");
    std::fs::write(&invalid_servers_dir, b"private SQL and password=hunter2").unwrap();

    let io_error = run(&project, &home, &["local", "--json", "server", "list"]);
    assert_structured_failure(&io_error, &expected_error("io_error", None));
    std::fs::remove_file(invalid_servers_dir).unwrap();

    // Docker unavailability is classified and described by clickhousectl
    // itself, so JSON carries the full human diagnostic — but never the
    // daemon's own text or the socket path it was pointed at.
    let docker_secret = home.join("docker-secret-token.sock");
    let unavailable = command(&project, &home)
        .env("DOCKER_HOST", format!("unix://{}", docker_secret.display()))
        .args(["local", "--json", "postgres", "start"])
        .output()
        .expect("run Docker unavailable failure");
    assert_eq!(unavailable.status.code(), Some(1));
    assert!(unavailable.stdout.is_empty());
    let error: Value = serde_json::from_slice(&unavailable.stderr).expect("one JSON error object");
    assert_eq!(error["error"]["code"], "docker_unavailable");
    let message = error["error"]["message"].as_str().expect("message");
    let human = command(&project, &home)
        .env("DOCKER_HOST", format!("unix://{}", docker_secret.display()))
        .args(["local", "postgres", "start"])
        .output()
        .expect("run human Docker unavailable failure");
    assert_eq!(
        String::from_utf8_lossy(&human.stderr).trim_end(),
        format!("Error: {message}"),
        "JSON must carry the full human diagnostic"
    );

    for output in [&io_error, &unavailable] {
        let stderr = String::from_utf8_lossy(&output.stderr);
        for sensitive in [
            "project-private-token",
            "home-private-token",
            "docker-secret-token",
            "hunter2",
            "private SQL",
        ] {
            assert!(!stderr.contains(sensitive), "leaked {sensitive}: {stderr}");
        }
    }
}

/// Issue #608: the message an agent gets in JSON must not be thinner than the
/// human `Error: ...` line for a self-composed local failure.
#[test]
fn config_and_argument_failures_carry_their_full_human_detail_in_json() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    install_fake_clickhouse(home.path(), "#!/bin/sh\nexit 7\n");
    let configs = home.path().join(".clickhouse/configs");
    std::fs::create_dir_all(&configs).expect("create configs dir");
    std::fs::write(configs.join("analytics.xml"), b"<clickhouse/>").expect("write config");

    let missing_config = run(
        project.path(),
        home.path(),
        &[
            "local",
            "--json",
            "server",
            "start",
            "--version",
            VERSION,
            "--config",
            "does-not-exist",
        ],
    );
    let message = assert_structured_failure(
        &missing_config,
        &expected_error(
            "config_not_found",
            Some("clickhousectl local clickhouse configs"),
        ),
    );
    let configs_dir = configs.display().to_string();
    for value in ["'does-not-exist'", configs_dir.as_str(), "analytics.xml"] {
        assert!(message.contains(value), "{value}: {message}");
    }

    let human = run(
        project.path(),
        home.path(),
        &[
            "local",
            "server",
            "start",
            "--version",
            VERSION,
            "--config",
            "does-not-exist",
        ],
    );
    assert_eq!(
        String::from_utf8_lossy(&human.stderr),
        format!("Error: {message}\n"),
        "JSON and human mode must carry the same detail"
    );

    // Port 0 would hand the choice to the OS, which managed metadata cannot
    // track; the refusal is self-composed guidance, not a redacted fallback.
    let zero_port = run(
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
            "0",
        ],
    );
    let message = assert_structured_failure(
        &zero_port,
        &expected_error(
            "unsupported_argument",
            Some("clickhousectl local clickhouse start --help"),
        ),
    );
    assert!(message.contains("--http-port"), "{message}");
}

#[test]
fn human_and_clap_errors_keep_their_existing_formats() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    let human = run(
        project.path(),
        home.path(),
        &["local", "server", "stop", "missing"],
    );
    assert_eq!(human.status.code(), Some(1));
    assert!(human.stdout.is_empty());
    // The human form names the scope and every recovery command the JSON
    // guidance carries.
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(stderr.starts_with("Error: "), "{stderr}");
    let project_path = format!("'{}'", project.path().canonicalize().unwrap().display());
    assert!(stderr.contains(&project_path), "{stderr}");
    let guidance = &expected_project_server_error(project.path(), "missing")["error"]["guidance"];
    for item in guidance.as_array().unwrap() {
        let command = item["command"].as_str().unwrap();
        if command.starts_with("clickhousectl") {
            assert!(
                stderr.contains(&format!("`{command}`")),
                "{command}: {stderr}"
            );
        }
    }

    let clap = run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop", "--unknown"],
    );
    assert_eq!(clap.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&clap.stderr);
    assert!(stderr.starts_with("error: "), "{stderr}");
    assert!(stderr.contains("'--unknown'"), "{stderr}");
    assert!(!stderr.contains("server_not_found"));
}

#[test]
fn foreground_child_exit_is_not_wrapped_as_a_local_error() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    install_fake_clickhouse(home.path(), "#!/bin/sh\nexit 7\n");

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
            "--foreground",
        ],
    );

    assert_eq!(output.status.code(), Some(7));
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Only the startup line naming the server; no error rendering of any kind.
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.contains("'default'"), "{stderr}");
    assert!(!stderr.contains("\"error\""), "{stderr}");
    assert!(!stderr.contains("Error: "), "{stderr}");
}
