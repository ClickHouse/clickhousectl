//! Project-scope diagnostics for local server list, stop, and remove.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

fn canonical(path: &Path) -> String {
    path.canonicalize()
        .expect("canonical project path")
        .display()
        .to_string()
}

fn run(project: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .current_dir(project)
        .args(args)
        .output()
        .expect("run clickhousectl")
}

fn write_server(project: &Path, name: &str, pid: u32) {
    let servers = project.join(".clickhouse/servers");
    std::fs::create_dir_all(servers.join(name).join("data")).expect("create server data directory");
    std::fs::write(
        servers.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&json!({
            "name": name,
            "pid": pid,
            "version": if pid == 0 { "" } else { "25.12.9.61" },
            "http_port": if pid == 0 { 0 } else { 8123 },
            "tcp_port": if pid == 0 { 0 } else { 9000 },
            "started_at": "1700000000",
            "cwd": canonical(project),
            "engine": "clickhouse"
        }))
        .unwrap(),
    )
    .expect("write server metadata");
}

fn json_stdout(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("parse stdout JSON")
}

fn json_error(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).expect("parse structured error")
}

fn assert_exact_scope(value: &Value, project: &Path) {
    assert_eq!(value["kind"], "exact_current_project");
    assert_eq!(value["path"], canonical(project));
    assert_eq!(value["parent_projects_searched"], false);
}

#[test]
fn omitted_commands_explain_absent_project_state() {
    let root = tempfile::tempdir().expect("create root");
    let home = tempfile::tempdir().expect("create home");
    let stop_project = root.path().join("stop-project");
    let remove_project = root.path().join("remove-project");
    std::fs::create_dir(&stop_project).expect("create stop project");
    std::fs::create_dir(&remove_project).expect("create remove project");

    let stop = json_stdout(&run(
        &stop_project,
        home.path(),
        &["local", "--json", "server", "stop"],
    ));
    assert_eq!(stop["stopped"], false);
    assert_exact_scope(&stop["project_scope"], &stop_project);
    assert_eq!(
        stop["guidance"][0]["message"],
        "Change to the local project root where the server was started"
    );

    let remove = json_error(&run(
        &remove_project,
        home.path(),
        &["local", "--json", "server", "remove"],
    ));
    assert_eq!(remove["error"]["code"], "server_selection_required");
    assert!(remove["error"].get("command").is_none());
    assert!(remove["error"].get("server").is_none());
    assert_exact_scope(&remove["error"]["project_scope"], &remove_project);
    assert_eq!(
        remove["error"]["guidance"][0]["message"],
        "Change to the local project root where the server was started"
    );

    let human_project = root.path().join("human-project");
    std::fs::create_dir(&human_project).expect("create human project");
    let human = run(&human_project, home.path(), &["local", "server", "remove"]);
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(stderr.contains("No `.clickhouse` project state was found"));
    assert!(stderr.contains("parent `.clickhouse` directories are not searched"));
    assert!(stderr.contains("where the server was started"));
}

#[test]
fn root_and_child_scope_running_and_stopped_metadata_independently() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    write_server(project.path(), "running", std::process::id());
    write_server(project.path(), "stopped", 0);

    let root_list = json_stdout(&run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "list"],
    ));
    assert_eq!(root_list["total_servers"], 2);
    assert_eq!(root_list["total_running_servers"], 1);
    assert_exact_scope(&root_list["project_scope"], project.path());
    assert!(root_list.get("guidance").is_none());
    assert_eq!(root_list["servers"][0]["name"], "running");
    assert_eq!(root_list["servers"][0]["running"], true);
    assert_eq!(root_list["servers"][1]["name"], "stopped");
    assert_eq!(root_list["servers"][1]["running"], false);

    let root_stop = json_stdout(&run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "stop", "stopped"],
    ));
    assert_eq!(root_stop["already_stopped"], true);

    let child = project.path().join("child");
    std::fs::create_dir(&child).expect("create child directory");
    let stop = json_error(&run(
        &child,
        home.path(),
        &["local", "--json", "server", "stop", "running"],
    ));
    assert_eq!(stop["error"]["code"], "server_not_found");
    assert_eq!(stop["error"]["server"]["name"], "running");
    assert_exact_scope(&stop["error"]["project_scope"], &child);
    assert_eq!(
        stop["error"]["guidance"][3]["command"],
        "clickhousectl local server stop <name> --global --project <project-root>"
    );

    let remove = json_error(&run(
        &child,
        home.path(),
        &["local", "--json", "server", "remove", "stopped"],
    ));
    assert_eq!(remove["error"]["code"], "server_not_found");
    assert_eq!(remove["error"]["server"]["name"], "stopped");
    assert_exact_scope(&remove["error"]["project_scope"], &child);
    assert_eq!(remove["error"]["guidance"].as_array().unwrap().len(), 3);

    let human_remove = run(
        &child,
        home.path(),
        &["local", "server", "remove", "stopped"],
    );
    assert_eq!(human_remove.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human_remove.stderr);
    assert!(stderr.contains(&canonical(&child)), "stderr: {stderr}");
    assert!(
        stderr.contains("Project-local server remove uses the exact current working directory")
    );
    assert!(stderr.contains("parent `.clickhouse` directories are not searched"));
    assert!(!stderr.contains("server stop <name> --global"));

    for error in [&stop, &remove] {
        for guidance in error["error"]["guidance"].as_array().unwrap() {
            if let Some(command) = guidance.get("command").and_then(Value::as_str) {
                assert!(
                    !command.contains(&canonical(&child)),
                    "recovery command interpolated a raw path: {command}"
                );
            }
        }
    }

    let child_list = json_stdout(&run(
        &child,
        home.path(),
        &["local", "--json", "server", "list"],
    ));
    assert_eq!(child_list["total_servers"], 0);
    assert_exact_scope(&child_list["project_scope"], &child);
    assert_eq!(
        child_list["guidance"][2]["command"],
        "clickhousectl local server list --global"
    );

    let human = run(&child, home.path(), &["local", "server", "list"]);
    assert!(human.status.success());
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains(&canonical(&child)), "stdout: {stdout}");
    assert!(stdout.contains("exact current working directory"));
    assert!(stdout.contains("parent `.clickhouse` directories are not searched"));
    assert!(stdout.contains("clickhousectl local server list --global"));

    assert!(
        project
            .path()
            .join(".clickhouse/servers/running.json")
            .exists()
    );
    assert!(
        project
            .path()
            .join(".clickhouse/servers/stopped/data")
            .is_dir()
    );
    let removed = json_stdout(&run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "remove", "stopped"],
    ));
    assert_eq!(removed["name"], "stopped");
}

#[test]
fn nested_project_state_wins_over_parent_state() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let nested = project.path().join("nested");
    std::fs::create_dir(&nested).expect("create nested project");
    write_server(project.path(), "outer", 0);
    write_server(&nested, "inner", 0);

    let list = json_stdout(&run(
        &nested,
        home.path(),
        &["local", "--json", "server", "list"],
    ));
    assert_eq!(list["total_servers"], 1);
    assert_eq!(list["servers"][0]["name"], "inner");
    assert_exact_scope(&list["project_scope"], &nested);

    let outer = json_error(&run(
        &nested,
        home.path(),
        &["local", "--json", "server", "stop", "outer"],
    ));
    assert_exact_scope(&outer["error"]["project_scope"], &nested);

    let inner = json_stdout(&run(
        &nested,
        home.path(),
        &["local", "--json", "server", "remove", "inner"],
    ));
    assert_eq!(inner["name"], "inner");
    assert!(
        project
            .path()
            .join(".clickhouse/servers/outer/data")
            .is_dir()
    );
}

#[cfg(unix)]
#[test]
fn symlinked_cwd_reports_and_uses_the_canonical_project() {
    let root = tempfile::tempdir().expect("create root");
    let home = tempfile::tempdir().expect("create home");
    let project = root.path().join("real-project");
    let alias = root.path().join("project-alias");
    std::fs::create_dir(&project).expect("create project");
    std::os::unix::fs::symlink(&project, &alias).expect("create project symlink");
    write_server(&project, "dev", 0);

    let list = json_stdout(&run(
        &alias,
        home.path(),
        &["local", "--json", "server", "list"],
    ));
    assert_eq!(list["servers"][0]["name"], "dev");
    assert_exact_scope(&list["project_scope"], &project);
    assert!(!list.to_string().contains("project-alias"));

    let missing = json_error(&run(
        &alias,
        home.path(),
        &["local", "--json", "server", "remove", "missing"],
    ));
    assert_exact_scope(&missing["error"]["project_scope"], &project);
    assert!(!missing.to_string().contains("project-alias"));
}

#[test]
fn global_list_omits_project_local_scope_and_recovery() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");

    let local = json_stdout(&run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "list"],
    ));
    assert!(local.get("project_scope").is_some());
    assert!(local.get("guidance").is_some());

    let global = json_stdout(&run(
        project.path(),
        home.path(),
        &["local", "--json", "server", "list", "--global"],
    ));
    assert!(global.get("project_scope").is_none());
    assert!(global.get("guidance").is_none());
}
