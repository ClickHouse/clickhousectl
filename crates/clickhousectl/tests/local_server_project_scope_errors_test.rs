//! Project-scope diagnostics for local clickhouse list, stop, and remove.

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
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
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

/// `project_scope` names `project`, found through its `.clickhouse/` marker.
fn assert_project_scope(value: &Value, project: &Path) {
    assert_eq!(value["kind"], "project_root");
    assert_eq!(value["path"], canonical(project));
}

/// `project_scope` names `dir`, used because no project was found.
fn assert_no_project_scope(value: &Value, dir: &Path) {
    assert_eq!(value["kind"], "current_directory");
    assert_eq!(value["path"], canonical(dir));
}

fn guidance_actions(guidance: &Value) -> Vec<&str> {
    guidance
        .as_array()
        .map(|hints| {
            hints
                .iter()
                .map(|hint| hint["action"].as_str().unwrap())
                .collect()
        })
        .unwrap_or_default()
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
    assert_no_project_scope(&stop["project_scope"], &stop_project);
    assert_eq!(stop["guidance"][0]["action"], "return_to_project_root");

    let remove = json_error(&run(
        &remove_project,
        home.path(),
        &["local", "--json", "server", "remove"],
    ));
    assert_eq!(remove["error"]["code"], "server_selection_required");
    assert!(remove["error"].get("command").is_none());
    assert!(remove["error"].get("server").is_none());
    assert_no_project_scope(&remove["error"]["project_scope"], &remove_project);
    assert_eq!(
        remove["error"]["guidance"][0]["action"],
        "return_to_project_root"
    );

    let human_project = root.path().join("human-project");
    std::fs::create_dir(&human_project).expect("create human project");
    let human = run(&human_project, home.path(), &["local", "server", "remove"]);
    assert_eq!(human.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human.stderr);
    assert!(stderr.starts_with("Error: "), "{stderr}");
    assert!(stderr.contains(&canonical(&human_project)), "{stderr}");
    assert!(
        stderr.contains("`clickhousectl local clickhouse list`"),
        "{stderr}"
    );
    assert!(stderr.contains("`clickhousectl local init`"), "{stderr}");
}

/// A subdirectory acts on the enclosing project: the same servers, the same
/// state, and errors that name the project rather than the subdirectory.
#[test]
fn a_subdirectory_acts_on_the_enclosing_project() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    write_server(project.path(), "running", std::process::id());
    write_server(project.path(), "stopped", 0);
    let child = project.path().join("child/deeper");
    std::fs::create_dir_all(&child).expect("create child directory");

    for dir in [project.path(), child.as_path()] {
        let list = json_stdout(&run(
            dir,
            home.path(),
            &["local", "--json", "server", "list"],
        ));
        assert_eq!(list["total_servers"], 2);
        assert_eq!(list["total_running_servers"], 1);
        assert_project_scope(&list["project_scope"], project.path());
        assert!(list.get("guidance").is_none());
        assert_eq!(list["servers"][0]["name"], "running");
        assert_eq!(list["servers"][0]["running"], true);
        assert_eq!(list["servers"][1]["name"], "stopped");
        assert_eq!(list["servers"][1]["running"], false);
    }

    let child_stop = json_stdout(&run(
        &child,
        home.path(),
        &["local", "--json", "server", "stop", "stopped"],
    ));
    assert_eq!(child_stop["already_stopped"], true);

    // Inside a project, changing directory is no remedy: guidance lists this
    // project and the others, and only `stop` offers the global recovery.
    let stop = json_error(&run(
        &child,
        home.path(),
        &["local", "--json", "server", "stop", "missing"],
    ));
    assert_eq!(stop["error"]["code"], "server_not_found");
    assert_eq!(stop["error"]["server"]["name"], "missing");
    assert_project_scope(&stop["error"]["project_scope"], project.path());
    assert_eq!(
        guidance_actions(&stop["error"]["guidance"]),
        [
            "list_project_servers",
            "list_global_servers",
            "stop_global_project_server"
        ]
    );
    let remove = json_error(&run(
        &child,
        home.path(),
        &["local", "--json", "server", "remove", "missing"],
    ));
    assert_eq!(remove["error"]["code"], "server_not_found");
    assert_project_scope(&remove["error"]["project_scope"], project.path());
    assert_eq!(
        guidance_actions(&remove["error"]["guidance"]),
        ["list_project_servers", "list_global_servers"]
    );

    let human_remove = run(
        &child,
        home.path(),
        &["local", "server", "remove", "missing"],
    );
    assert_eq!(human_remove.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&human_remove.stderr);
    assert!(stderr.contains(&canonical(project.path())), "{stderr}");
    assert!(!stderr.contains(&canonical(&child)), "{stderr}");
    assert!(
        stderr.contains("`clickhousectl local clickhouse list --global`"),
        "{stderr}"
    );
    // The global stop recovery belongs to `stop` only.
    assert!(!stderr.contains("stop <name> --global"), "{stderr}");

    let removed = json_stdout(&run(
        &child,
        home.path(),
        &["local", "--json", "server", "remove", "stopped"],
    ));
    assert_eq!(removed["name"], "stopped");
    assert!(!project.path().join(".clickhouse/servers/stopped").exists());
    assert!(
        project
            .path()
            .join(".clickhouse/servers/running.json")
            .exists()
    );
    assert!(!project.path().join("child/.clickhouse").exists());
    assert!(!child.join(".clickhouse").exists());
}

#[test]
fn nested_project_state_wins_over_parent_state() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let nested = project.path().join("nested");
    std::fs::create_dir(&nested).expect("create nested project");
    write_server(project.path(), "outer", 0);
    write_server(&nested, "inner", 0);
    let below_nested = nested.join("src");
    std::fs::create_dir(&below_nested).expect("create nested subdirectory");

    for dir in [nested.as_path(), below_nested.as_path()] {
        let list = json_stdout(&run(
            dir,
            home.path(),
            &["local", "--json", "server", "list"],
        ));
        assert_eq!(list["total_servers"], 1);
        assert_eq!(list["servers"][0]["name"], "inner");
        assert_project_scope(&list["project_scope"], &nested);
    }

    let outer = json_error(&run(
        &nested,
        home.path(),
        &["local", "--json", "server", "stop", "outer"],
    ));
    assert_project_scope(&outer["error"]["project_scope"], &nested);

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
    std::fs::create_dir(project.join("sub")).expect("create subdirectory");

    for dir in [alias.clone(), alias.join("sub")] {
        let list = json_stdout(&run(
            &dir,
            home.path(),
            &["local", "--json", "server", "list"],
        ));
        assert_eq!(list["servers"][0]["name"], "dev");
        assert_project_scope(&list["project_scope"], &project);
        assert!(!list.to_string().contains("project-alias"));
    }

    let missing = json_error(&run(
        &alias,
        home.path(),
        &["local", "--json", "server", "remove", "missing"],
    ));
    assert_project_scope(&missing["error"]["project_scope"], &project);
    assert!(!missing.to_string().contains("project-alias"));
}

/// `~/.clickhouse` holds global state, so it never makes the home directory a
/// project for the directories beneath it.
#[test]
fn the_home_directory_is_never_an_enclosing_project() {
    let home = tempfile::tempdir().expect("create home");
    write_server(home.path(), "global-looking", 0);
    let app = home.path().join("work/app");
    std::fs::create_dir_all(&app).expect("create app directory");

    let list = json_stdout(&run(
        &app,
        home.path(),
        &["local", "--json", "server", "list"],
    ));
    assert_eq!(list["total_servers"], 0);
    assert_no_project_scope(&list["project_scope"], &app);
    assert_eq!(list["guidance"][0]["action"], "return_to_project_root");
    assert!(!app.join(".clickhouse").exists());

    // From the home directory itself its `.clickhouse/` is the current
    // directory's own, as before.
    let at_home = json_stdout(&run(
        home.path(),
        home.path(),
        &["local", "--json", "server", "list"],
    ));
    assert_eq!(at_home["servers"][0]["name"], "global-looking");
    assert_project_scope(&at_home["project_scope"], home.path());
}

fn install_fake_clickhouse(home: &Path, version: &str) {
    use std::os::unix::fs::PermissionsExt;
    let binary = home
        .join(".clickhouse/versions")
        .join(version)
        .join("clickhouse");
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version dir");
    std::fs::write(&binary, b"#!/bin/sh\nexec sleep 30\n").expect("write fake ClickHouse");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
        .expect("make fake ClickHouse executable");
}

struct ProcessGuard(u32);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        unsafe {
            libc::kill(self.0 as i32, libc::SIGKILL);
        }
    }
}

/// A server started from a subdirectory belongs to the enclosing project: its
/// state lands in that project's `.clickhouse/`, and no new one is created.
#[test]
fn a_server_started_from_a_subdirectory_joins_the_enclosing_project() {
    const VERSION: &str = "25.12.9.61";
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    install_fake_clickhouse(home.path(), VERSION);
    std::fs::create_dir(project.path().join(".clickhouse")).expect("mark the project");
    let child = project.path().join("src/app");
    std::fs::create_dir_all(&child).expect("create child directory");

    let mut command = Command::new(clickhousectl_binary());
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("CHCTL_UPDATE_CHECK_TIMEOUT_MS", "0")
        .env("HOME", home.path())
        .current_dir(&child)
        .args(["local", "--json", "server", "start", "dev", "--no-wait"])
        .args(["--version", VERSION])
        .output()
        .expect("run clickhousectl");
    let body = json_stdout(&output);
    let _process = ProcessGuard(body["pid"].as_u64().expect("start PID") as u32);

    let metadata: Value = serde_json::from_slice(
        &std::fs::read(project.path().join(".clickhouse/servers/dev.json"))
            .expect("metadata in the enclosing project"),
    )
    .expect("parse metadata");
    assert_eq!(metadata["cwd"], canonical(project.path()));
    assert!(project.path().join(".clickhouse/servers/dev/data").is_dir());
    assert!(!child.join(".clickhouse").exists());
    assert!(!project.path().join("src/.clickhouse").exists());
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
