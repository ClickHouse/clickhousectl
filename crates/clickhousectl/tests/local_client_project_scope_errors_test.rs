//! Managed local-client project-scope diagnostics.

use serde_json::{Value, json};
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
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
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

/// The structured error a failed managed lookup prints on stderr, with
/// nothing on stdout.
fn error_json(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).expect("stderr is one JSON error")
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

const LIST: &str = "clickhousectl local clickhouse list";
const DIRECT: &str = "clickhousectl local client --host <host> --port <port>";

fn start_command(selection: &str) -> &'static str {
    if selection == "default" {
        "clickhousectl local clickhouse start"
    } else {
        "clickhousectl local clickhouse start <name>"
    }
}

/// The guidance commands for each code, in order; `None` is advice without a
/// command. Changing directory is advised only when no project was found.
fn expected_commands(code: &str, selection: &str, marked: bool) -> Vec<Option<&'static str>> {
    match code {
        "managed_client_server_not_found" if marked => {
            vec![Some(LIST), Some(start_command(selection)), Some(DIRECT)]
        }
        "managed_client_server_not_found" => {
            vec![
                Some(LIST),
                None,
                Some(start_command(selection)),
                Some(DIRECT),
            ]
        }
        "managed_client_server_not_running" => {
            vec![Some(LIST), Some(start_command(selection)), Some(DIRECT)]
        }
        "managed_client_binary_not_found" => vec![
            Some(LIST),
            Some("clickhousectl local install <version>"),
            Some(DIRECT),
        ],
        "managed_client_project_state_unavailable" => vec![None, Some(LIST), Some(DIRECT)],
        other => panic!("unexpected code: {other}"),
    }
}

/// Assert the managed-mode envelope by its fields: the code, the exact
/// project scope, the selected server, and the guidance commands.
fn assert_managed_error(
    error: &Value,
    project: &str,
    code: &str,
    selection: &str,
    name: &str,
    binary_version: Option<&str>,
) {
    let error = &error["error"];
    assert_eq!(error["code"], code, "{error}");
    assert!(error["message"].is_string(), "{error}");
    assert_eq!(error["project_scope"], json!({ "path": project }));
    let mut server = json!({ "selection": selection, "name": name });
    if let Some(version) = binary_version {
        server["binary_version"] = json!(version);
    }
    assert_eq!(error["server"], server);
    let guidance = error["guidance"].as_array().expect("guidance list");
    assert!(guidance.iter().all(|item| item["message"].is_string()));
    let commands: Vec<Option<&str>> = guidance
        .iter()
        .map(|item| item.get("command").map(|command| command.as_str().unwrap()))
        .collect();
    let marked = Path::new(project).join(".clickhouse").is_dir();
    assert_eq!(commands, expected_commands(code, selection, marked));
}

/// Human mode fails the same way and names the scope, the server and every
/// guidance command the JSON envelope carries.
fn assert_human_parity(output: &Output, project: &str, json: &Value) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("Error: "), "{stderr}");
    assert!(stderr.contains(&format!("'{project}'")), "{stderr}");
    let name = json["error"]["server"]["name"].as_str().unwrap();
    assert!(stderr.contains(&format!("'{name}'")), "{stderr}");
    for item in json["error"]["guidance"].as_array().unwrap() {
        if let Some(command) = item.get("command") {
            let command = command.as_str().unwrap();
            assert!(
                stderr.contains(&format!("`{command}`")),
                "{command}: {stderr}"
            );
        }
    }
}

fn assert_human_and_json(
    project: &Path,
    home: &Path,
    client_args: &[&str],
    (code, selection, name, binary_version): (&str, &str, &str, Option<&str>),
) {
    let project_path = canonical(project);
    let json = error_json(&run(project, home, true, client_args));
    assert_managed_error(&json, &project_path, code, selection, name, binary_version);
    assert_human_parity(
        &run(project, home, false, client_args),
        &project_path,
        &json,
    );
}

#[test]
fn empty_project_identifies_default_managed_mode_and_exact_scope() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    assert_human_and_json(
        project.path(),
        home.path(),
        &[],
        (
            "managed_client_server_not_found",
            "default",
            "default",
            None,
        ),
    );
}

/// A subdirectory reaches the enclosing project's server, and its errors name
/// that project, identically for an agent.
#[test]
fn child_of_a_project_reaches_its_server_and_agent_gets_json() {
    use std::os::unix::fs::PermissionsExt;
    let parent = tempfile::tempdir().expect("create parent project");
    let home = tempfile::tempdir().expect("create home");
    install_fake_clickhouse(home.path(), VERSION);
    let binary = home
        .path()
        .join(".clickhouse/versions")
        .join(VERSION)
        .join("clickhouse");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
        .expect("make fake ClickHouse executable");
    write_server(parent.path(), "default", std::process::id(), VERSION);
    let child = parent.path().join("child/deeper");
    std::fs::create_dir_all(&child).expect("create child directory");

    let reached = run(&child, home.path(), false, &[]);
    assert!(
        reached.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&reached.stderr)
    );

    let explicit = error_json(&run(&child, home.path(), true, &["--name", "missing"]));
    assert_managed_error(
        &explicit,
        &canonical(parent.path()),
        "managed_client_server_not_found",
        "named",
        "missing",
        None,
    );
    let agent = command(&child, home.path())
        .env("AGENT", "opencode")
        .args(["local", "client", "--name", "missing"])
        .output()
        .expect("run agent-mode client");
    assert_eq!(error_json(&agent), explicit);
    assert!(!child.join(".clickhouse").exists());
}

#[test]
fn explicit_wrong_name_preserves_named_selection_without_unsafe_command_interpolation() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    write_server(project.path(), "default", std::process::id(), VERSION);

    // The named start command keeps its `<name>` placeholder rather than
    // interpolating the unquoted name.
    assert_human_and_json(
        project.path(),
        home.path(),
        &["--name", "wrong name"],
        (
            "managed_client_server_not_found",
            "named",
            "wrong name",
            None,
        ),
    );
}

#[test]
fn stopped_metadata_suggests_starting_the_selected_named_server() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    write_server(project.path(), "dev", 0, "");

    assert_human_and_json(
        project.path(),
        home.path(),
        &["--name", "dev"],
        ("managed_client_server_not_running", "named", "dev", None),
    );
}

#[test]
fn selected_missing_binary_keeps_managed_scope_and_install_guidance() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    write_server(project.path(), "dev", std::process::id(), VERSION);

    assert_human_and_json(
        project.path(),
        home.path(),
        &["--name", "dev"],
        (
            "managed_client_binary_not_found",
            "named",
            "dev",
            Some(VERSION),
        ),
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

    assert_human_and_json(
        project.path(),
        home.path(),
        &[],
        (
            "managed_client_project_state_unavailable",
            "default",
            "default",
            None,
        ),
    );
}

#[test]
fn corrupt_metadata_keeps_managed_scope_and_redacts_source_from_json() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let servers = project.path().join(".clickhouse/servers");
    std::fs::create_dir_all(&servers).expect("create servers directory");
    std::fs::write(servers.join("default.json"), "{").expect("write corrupt metadata");

    assert_human_and_json(
        project.path(),
        home.path(),
        &[],
        (
            "managed_client_project_state_unavailable",
            "default",
            "default",
            None,
        ),
    );
    // The serde source reaches the human error only.
    let source = serde_json::from_str::<Value>("{").unwrap_err().to_string();
    let json_output = run(project.path(), home.path(), true, &[]);
    assert!(!String::from_utf8_lossy(&json_output.stderr).contains(&source));
    let human = run(project.path(), home.path(), false, &[]);
    assert!(String::from_utf8_lossy(&human.stderr).contains(&source));
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

    let output = run(&alias, home.path(), true, &[]);
    assert_managed_error(
        &error_json(&output),
        &canonical(&project),
        "managed_client_server_not_found",
        "default",
        "default",
        None,
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("project-alias"));
}
