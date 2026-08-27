//! Managed local-client project-scope diagnostics.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VERSION: &str = "27.1.2.3";

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

fn run(project: &Path, home: &Path, json: bool, client_args: &[&str]) -> Output {
    let mut args = vec!["local"];
    if json {
        args.push("--json");
    }
    args.push("client");
    args.extend_from_slice(client_args);
    command(project, home)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn assert_failure(output: &Output, expected: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert_eq!(String::from_utf8_lossy(&output.stderr), expected);
}

fn canonical(project: &Path) -> String {
    project
        .canonicalize()
        .expect("canonical project path")
        .display()
        .to_string()
}

fn write_server(project: &Path, name: &str, pid: u32, version: &str) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).expect("create server metadata directory");
    std::fs::write(
        servers.join(format!("{name}.json")),
        serde_json::to_vec(&serde_json::json!({
            "name": name,
            "pid": pid,
            "version": version,
            "http_port": if pid == 0 { 0 } else { 8123 },
            "tcp_port": if pid == 0 { 0 } else { 9000 },
            "started_at": "test",
            "cwd": canonical(project),
            "engine": "clickhouse"
        }))
        .expect("serialize server metadata"),
    )
    .expect("write server metadata");
}

fn install_fake_clickhouse(home: &Path, version: &str) {
    let binary = home
        .join(".clickhouse/versions")
        .join(version)
        .join("clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create version directory");
    std::fs::write(binary, "#!/bin/sh\nexit 0\n").expect("write fake ClickHouse");
}

#[derive(Serialize)]
struct ExpectedEnvelope<'a> {
    error: ExpectedError<'a>,
}

#[derive(Serialize)]
struct ExpectedError<'a> {
    code: &'a str,
    message: &'a str,
    project_scope: ExpectedScope<'a>,
    server: ExpectedServer<'a>,
    guidance: Vec<ExpectedGuidance>,
}

#[derive(Serialize)]
struct ExpectedScope<'a> {
    path: &'a str,
}

#[derive(Serialize)]
struct ExpectedServer<'a> {
    selection: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary_version: Option<&'a str>,
}

#[derive(Serialize)]
struct ExpectedGuidance {
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<&'static str>,
}

fn list_guidance() -> ExpectedGuidance {
    ExpectedGuidance {
        message: "List managed servers in this exact project",
        command: Some("clickhousectl local server list"),
    }
}

fn direct_guidance() -> ExpectedGuidance {
    ExpectedGuidance {
        message: "Bypass managed project lookup and connect directly",
        command: Some("clickhousectl local client --host <host> --port <port>"),
    }
}

fn start_guidance(selection: &str) -> ExpectedGuidance {
    if selection == "default" {
        ExpectedGuidance {
            message: "Start the default managed server in this project",
            command: Some("clickhousectl local server start"),
        }
    } else {
        ExpectedGuidance {
            message: "Start the selected named managed server in this project",
            command: Some("clickhousectl local server start <name>"),
        }
    }
}

fn expected_json(
    project: &str,
    code: &str,
    message: &str,
    selection: &str,
    name: &str,
    binary_version: Option<&str>,
) -> String {
    let mut guidance = vec![list_guidance()];
    match code {
        "managed_client_server_not_found" => {
            guidance.push(ExpectedGuidance {
                message: "Return to the project directory that owns the managed server",
                command: None,
            });
            guidance.push(start_guidance(selection));
        }
        "managed_client_server_not_running" => guidance.push(start_guidance(selection)),
        "managed_client_binary_not_found" => guidance.push(ExpectedGuidance {
            message: "Install the version selected by the managed server metadata",
            command: Some("clickhousectl local install <version>"),
        }),
        "managed_client_project_state_unavailable" => guidance.insert(
            0,
            ExpectedGuidance {
                message: "Repair the reported project state error before retrying",
                command: None,
            },
        ),
        other => panic!("unexpected code: {other}"),
    }
    guidance.push(direct_guidance());

    let expected = ExpectedEnvelope {
        error: ExpectedError {
            code,
            message,
            project_scope: ExpectedScope { path: project },
            server: ExpectedServer {
                selection,
                name,
                binary_version,
            },
            guidance,
        },
    };
    format!("{}\n", serde_json::to_string_pretty(&expected).unwrap())
}

fn assert_human_and_json(
    project: &Path,
    home: &Path,
    client_args: &[&str],
    human: &str,
    json: &str,
) {
    assert_failure(&run(project, home, false, client_args), human);
    assert_failure(&run(project, home, true, client_args), json);
}

#[test]
fn empty_project_identifies_default_managed_mode_and_exact_scope() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let project_path = canonical(project.path());
    let human = format!(
        "Error: Managed client mode: server 'default' was not found in current project '{project_path}'; parent projects are not searched.\n\
         Run `clickhousectl local server list`; return to the project root if needed; start it with `clickhousectl local server start`, or use direct mode with `clickhousectl local client --host <host> --port <port>`.\n"
    );
    let json = expected_json(
        &project_path,
        "managed_client_server_not_found",
        "Managed client server was not found in the current project",
        "default",
        "default",
        None,
    );

    assert_human_and_json(project.path(), home.path(), &[], &human, &json);
}

#[test]
fn child_of_valid_parent_does_not_search_parent_and_agent_gets_exact_json() {
    let parent = tempfile::tempdir().expect("create parent project");
    let home = tempfile::tempdir().expect("create home");
    install_fake_clickhouse(home.path(), VERSION);
    write_server(parent.path(), "default", std::process::id(), VERSION);
    let child = parent.path().join("child");
    std::fs::create_dir(&child).expect("create child directory");
    let child_path = canonical(&child);
    let expected = expected_json(
        &child_path,
        "managed_client_server_not_found",
        "Managed client server was not found in the current project",
        "default",
        "default",
        None,
    );

    assert_failure(&run(&child, home.path(), true, &[]), &expected);
    let agent = command(&child, home.path())
        .env("AGENT", "opencode")
        .args(["local", "client"])
        .output()
        .expect("run agent-mode client");
    assert_failure(&agent, &expected);
    assert!(
        parent
            .path()
            .join(".clickhouse/servers/default.json")
            .exists()
    );
}

#[test]
fn explicit_wrong_name_preserves_named_selection_without_unsafe_command_interpolation() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    write_server(project.path(), "default", std::process::id(), VERSION);
    let project_path = canonical(project.path());
    let human = format!(
        "Error: Managed client mode: server 'wrong name' was not found in current project '{project_path}'; parent projects are not searched.\n\
         Run `clickhousectl local server list`; return to the project root if needed; start it with `clickhousectl local server start <name>`, or use direct mode with `clickhousectl local client --host <host> --port <port>`.\n"
    );
    let json = expected_json(
        &project_path,
        "managed_client_server_not_found",
        "Managed client server was not found in the current project",
        "named",
        "wrong name",
        None,
    );

    assert_human_and_json(
        project.path(),
        home.path(),
        &["--name", "wrong name"],
        &human,
        &json,
    );
    assert!(!json.contains("server start wrong name"));
}

#[test]
fn stopped_metadata_suggests_starting_the_selected_named_server() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    write_server(project.path(), "dev", 0, "");
    let project_path = canonical(project.path());
    let human = format!(
        "Error: Managed client mode: server 'dev' is not running in current project '{project_path}'.\n\
         Run `clickhousectl local server list`, then `clickhousectl local server start <name>`; or use direct mode with `clickhousectl local client --host <host> --port <port>`.\n"
    );
    let json = expected_json(
        &project_path,
        "managed_client_server_not_running",
        "Managed client server is not running in the current project",
        "named",
        "dev",
        None,
    );

    assert_human_and_json(
        project.path(),
        home.path(),
        &["--name", "dev"],
        &human,
        &json,
    );
}

#[test]
fn selected_missing_binary_keeps_managed_scope_and_install_guidance() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    write_server(project.path(), "dev", std::process::id(), VERSION);
    let project_path = canonical(project.path());
    let human = format!(
        "Error: Managed client mode: server 'dev' in current project '{project_path}' selected ClickHouse version '{VERSION}', but its client binary is missing.\n\
         Run `clickhousectl local server list` and install the selected version with `clickhousectl local install <version>`, or use direct mode with `clickhousectl local client --host <host> --port <port>`.\n"
    );
    let json = expected_json(
        &project_path,
        "managed_client_binary_not_found",
        "Managed client binary selected by server metadata is not installed",
        "named",
        "dev",
        Some(VERSION),
    );

    assert_human_and_json(
        project.path(),
        home.path(),
        &["--name", "dev"],
        &human,
        &json,
    );
}

#[test]
fn invalid_metadata_lock_path_keeps_managed_scope_in_json_and_human_errors() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let state_dir = project.path().join(".clickhouse");
    std::fs::create_dir(&state_dir).expect("create state directory");
    std::fs::write(state_dir.join("servers"), "not a directory")
        .expect("create invalid servers path");
    let project_path = canonical(project.path());
    let json = expected_json(
        &project_path,
        "managed_client_project_state_unavailable",
        "Managed client project state is unavailable",
        "default",
        "default",
        None,
    );

    assert_failure(&run(project.path(), home.path(), true, &[]), &json);

    let human = run(project.path(), home.path(), false, &[]);
    assert_eq!(human.status.code(), Some(1));
    assert!(human.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(
        stderr.starts_with("Error: Managed client mode: server 'default' could not be resolved")
    );
    assert!(stderr.contains(&project_path));
    assert!(stderr.contains("create the server metadata lock directory"));
    assert!(stderr.contains("`clickhousectl local server list`"));
    assert!(stderr.contains("`clickhousectl local client --host <host> --port <port>`"));
}

#[test]
fn corrupt_metadata_keeps_managed_scope_and_redacts_source_from_json() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let servers = project.path().join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).expect("create servers directory");
    std::fs::write(servers.join("default.json"), "{").expect("write corrupt metadata");
    let project_path = canonical(project.path());
    let json = expected_json(
        &project_path,
        "managed_client_project_state_unavailable",
        "Managed client project state is unavailable",
        "default",
        "default",
        None,
    );

    let json_output = run(project.path(), home.path(), true, &[]);
    assert_failure(&json_output, &json);
    assert!(!String::from_utf8_lossy(&json_output.stderr).contains("not valid JSON"));

    let human = run(project.path(), home.path(), false, &[]);
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(stderr.contains(&project_path));
    assert!(stderr.contains("not valid JSON"));
}

#[cfg(unix)]
#[test]
fn symlinked_working_directory_reports_the_canonical_project_scope() {
    let root = tempfile::tempdir().expect("create root");
    let project = root.path().join("real-project");
    let alias = root.path().join("project-alias");
    let home = tempfile::tempdir().expect("create home");
    std::fs::create_dir(&project).expect("create real project");
    std::os::unix::fs::symlink(&project, &alias).expect("create project symlink");
    let project_path = canonical(&project);
    let expected = expected_json(
        &project_path,
        "managed_client_server_not_found",
        "Managed client server was not found in the current project",
        "default",
        "default",
        None,
    );

    let output = run(&alias, home.path(), true, &[]);
    assert_failure(&output, &expected);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("project-alias"));
}
