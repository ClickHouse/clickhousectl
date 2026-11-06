//! Read-only and lookup local commands leave no project state behind (#1077).
//!
//! Each command runs in an empty directory, or one with only `local init`
//! state, and must report its usual empty or not-found result without creating
//! `.clickhouse/` or `.clickhouse/servers/`. Deleting that state does not stop
//! a running instance, though, so one this project still owns is recovered
//! exactly as when the state is present.

use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

/// Run with Docker unreachable: discovery must tolerate it and find nothing.
fn run(project: &Path, home: &Path, args: &[&str]) -> Output {
    let docker_host = format!("unix://{}/missing-docker.sock", home.display());
    run_with_docker(project, home, &docker_host, args)
}

fn run_with_docker(project: &Path, home: &Path, docker_host: &str, args: &[&str]) -> Output {
    Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .env("DOCKER_HOST", docker_host)
        .current_dir(project)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run clickhousectl")
}

fn created_entries(project: &Path) -> Vec<String> {
    let mut entries: Vec<_> = std::fs::read_dir(project)
        .expect("read project directory")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    entries
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "parse stdout JSON: {error}; stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn stderr_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
        panic!(
            "parse stderr JSON: {error}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Run `args` in a fresh empty directory and assert it stays empty.
fn run_in_empty_project(args: &[&str]) -> Output {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let output = run(project.path(), home.path(), args);
    assert_eq!(
        created_entries(project.path()),
        Vec::<String>::new(),
        "{args:?} created project state; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn assert_exit(output: &Output, code: i32, args: &[&str]) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{args:?}; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn server_list_is_empty_without_creating_project_state() {
    let args = ["local", "--json", "server", "list"];
    let output = run_in_empty_project(&args);
    assert_exit(&output, 0, &args);
    let body = stdout_json(&output);
    assert_eq!(body["servers"], serde_json::json!([]));
    assert_eq!(body["total_servers"], 0);
    assert_eq!(body["project_scope"]["kind"], "exact_current_project");

    let args = ["local", "server", "list"];
    let output = run_in_empty_project(&args);
    assert_exit(&output, 0, &args);
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("No servers found in project"));
}

#[test]
fn server_stop_reports_nothing_to_stop_without_creating_project_state() {
    let args = ["local", "--json", "server", "stop"];
    let output = run_in_empty_project(&args);
    assert_exit(&output, 0, &args);
    let body = stdout_json(&output);
    assert_eq!(body["stopped"], false);
    assert_eq!(body["reason"], "no_clickhouse_servers");
    assert_eq!(body["project_scope"]["kind"], "exact_current_project");

    for args in [
        ["local", "--json", "server", "stop", "x"],
        ["local", "--json", "server", "remove", "x"],
    ] {
        let output = run_in_empty_project(&args);
        assert_exit(&output, 1, &args);
        let error = &stderr_json(&output)["error"];
        assert_eq!(error["code"], "server_not_found", "{args:?}");
        assert_eq!(error["server"]["name"], "x", "{args:?}");
    }

    let args = ["local", "--json", "server", "remove"];
    let output = run_in_empty_project(&args);
    assert_exit(&output, 1, &args);
    assert_eq!(
        stderr_json(&output)["error"]["code"],
        "server_selection_required"
    );
}

#[test]
fn stop_all_reports_no_servers_without_creating_project_state() {
    for args in [
        ["local", "--json", "server", "stop-all"],
        ["local", "--json", "postgres", "stop-all"],
    ] {
        let output = run_in_empty_project(&args);
        assert_exit(&output, 0, &args);
        assert_eq!(
            stdout_json(&output),
            serde_json::json!({ "servers": [] }),
            "{args:?}"
        );
    }

    let args = ["local", "server", "stop-all"];
    let output = run_in_empty_project(&args);
    assert_exit(&output, 0, &args);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "No running ClickHouse servers\n"
    );

    let args = ["local", "postgres", "stop-all"];
    let output = run_in_empty_project(&args);
    assert_exit(&output, 0, &args);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "No running Postgres servers\n"
    );
}

#[test]
fn managed_client_and_dotenv_report_not_found_without_creating_project_state() {
    for (args, code) in [
        (
            &["local", "--json", "client"][..],
            "managed_client_server_not_found",
        ),
        (
            &["local", "--json", "client", "--name", "x"][..],
            "managed_client_server_not_found",
        ),
        (
            &["local", "--json", "server", "dotenv"][..],
            "server_not_found",
        ),
    ] {
        let output = run_in_empty_project(args);
        assert_exit(&output, 1, args);
        assert_eq!(stderr_json(&output)["error"]["code"], code, "{args:?}");
    }
}

#[test]
fn udf_commands_report_server_not_found_without_creating_project_state() {
    for args in [
        &["local", "--json", "udf", "list"][..],
        &["local", "--json", "udf", "reload"][..],
        &["local", "--json", "udf", "remove", "f"][..],
    ] {
        let output = run_in_empty_project(args);
        assert_exit(&output, 1, args);
        let error = &stderr_json(&output)["error"];
        assert_eq!(error["code"], "server_not_found", "{args:?}");
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("no clickhouse/ directory or servers"),
            "{args:?}: {error}"
        );
    }
}

#[test]
fn postgres_lookups_report_not_found_without_creating_project_state() {
    for (args, message) in [
        (
            &["local", "--json", "postgres", "stop", "x"][..],
            "Server 'x' not found",
        ),
        (
            &[
                "local",
                "--json",
                "postgres",
                "stop",
                "x",
                "--version",
                "17",
            ][..],
            "Server 'x (postgres:17)' not found",
        ),
        (
            &["local", "--json", "postgres", "remove", "x"][..],
            "Server 'x' not found",
        ),
        (
            &["local", "--json", "postgres", "client"][..],
            "Server 'default' not found",
        ),
        (
            &["local", "--json", "postgres", "dotenv"][..],
            "Server 'default' not found",
        ),
    ] {
        let output = run_in_empty_project(args);
        assert_exit(&output, 1, args);
        let error = &stderr_json(&output)["error"];
        assert_eq!(error["code"], "server_not_found", "{args:?}");
        assert_eq!(error["message"], message, "{args:?}");
    }
}

#[test]
fn postgres_lookup_still_validates_the_version_without_project_state() {
    let args = ["local", "--json", "postgres", "stop", "x", "--version", "9"];
    let output = run_in_empty_project(&args);
    assert_exit(&output, 1, &args);
    assert_ne!(stderr_json(&output)["error"]["code"], "server_not_found");
}

#[test]
fn removing_an_installed_version_creates_no_project_state() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let version = "25.12.9.61";
    let binary = home
        .path()
        .join(format!(".clickhouse/versions/{version}/clickhouse"));
    std::fs::create_dir_all(binary.parent().unwrap()).expect("create fake version directory");
    std::fs::write(&binary, b"#!/bin/sh\nexit 0\n").expect("write fake ClickHouse");
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
        .expect("make fake ClickHouse executable");

    let args = ["local", "--json", "remove", version];
    let output = run(project.path(), home.path(), &args);
    assert_exit(&output, 0, &args);
    assert!(!binary.exists());
    assert_eq!(created_entries(project.path()), Vec::<String>::new());
}

#[test]
fn initialized_project_without_servers_gains_no_servers_directory() {
    let project = tempfile::tempdir().expect("create project");
    let home = tempfile::tempdir().expect("create home");
    let init = run(project.path(), home.path(), &["local", "init"]);
    assert_exit(&init, 0, &["local", "init"]);
    let servers = project.path().join(".clickhouse/servers");
    assert!(!servers.exists());

    for (args, code) in [
        (&["local", "--json", "server", "list"][..], 0),
        (&["local", "--json", "server", "stop", "x"][..], 1),
        (&["local", "--json", "server", "remove", "x"][..], 1),
        (&["local", "--json", "client"][..], 1),
        (&["local", "--json", "udf", "list"][..], 1),
        (&["local", "--json", "postgres", "dotenv"][..], 1),
    ] {
        let output = run(project.path(), home.path(), args);
        assert_exit(&output, code, args);
        assert!(!servers.exists(), "{args:?} created {}", servers.display());
    }

    // `local init` state is not server state, so omitted remove keeps asking
    // for a selection rather than reporting missing `.clickhouse/` state.
    let args = ["local", "--json", "server", "remove"];
    let output = run(project.path(), home.path(), &args);
    assert_exit(&output, 1, &args);
    assert_eq!(
        stderr_json(&output)["error"]["code"],
        "server_selection_required"
    );
    assert!(!servers.exists());
}

/// A Docker daemon that still runs this project's labelled Postgres container,
/// although the project's `.clickhouse/` state is gone.
async fn docker_with_orphan_postgres(project: &Path) -> MockServer {
    let project = project.canonicalize().expect("canonicalize project");
    let docker = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"/_ping$"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&docker)
        .await;
    // Docker filters on the project label, so only that project's list
    // carries the container; every other project lists nothing.
    let project_label = format!("clickhousectl.project={}", project.display());
    Mock::given(method("GET"))
        .and(path_regex(r"/containers/json$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .with_priority(10)
        .mount(&docker)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"/containers/json$"))
        .and(move |request: &wiremock::Request| {
            request
                .url
                .query_pairs()
                .any(|(key, value)| key == "filters" && value.contains(&project_label))
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "Id": "orphan-container",
            "Image": "postgres:18",
            "Labels": {
                "clickhousectl.engine": "postgres",
                "clickhousectl.name": "default",
                "clickhousectl.major": "18",
                "clickhousectl.project": project.display().to_string(),
            },
            "Ports": [{ "PrivatePort": 5432, "PublicPort": 55432, "Type": "tcp" }],
        }])))
        .mount(&docker)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"/containers/orphan-container/json$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Id": "orphan-container",
            "State": { "Running": true, "Status": "running" },
        })))
        .mount(&docker)
        .await;
    Mock::given(method("POST"))
        .and(path_regex(r"/containers/orphan-container/stop$"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&docker)
        .await;
    docker
}

async fn stop_requests(docker: &MockServer) -> usize {
    docker
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.method.as_str() == "POST")
        .filter(|request| {
            request
                .url
                .path()
                .ends_with("/containers/orphan-container/stop")
        })
        .count()
}

#[tokio::test(flavor = "multi_thread")]
async fn running_postgres_container_is_recovered_when_project_state_is_missing() {
    for args in [
        &["local", "--json", "postgres", "stop-all"][..],
        &["local", "--json", "postgres", "stop", "default"][..],
    ] {
        let project = tempfile::tempdir().expect("create project");
        let home = tempfile::tempdir().expect("create home");
        let docker = docker_with_orphan_postgres(project.path()).await;
        let host = docker.uri().replace("http://", "tcp://");

        let output = run_with_docker(project.path(), home.path(), &host, args);

        assert_exit(&output, 0, args);
        assert_eq!(stop_requests(&docker).await, 1, "{args:?}");
        // Recovery records the instance, as it does when the state is present.
        assert!(
            project
                .path()
                .join(".clickhouse/servers/default-pg18.json")
                .is_file(),
            "{args:?}"
        );
    }

    for args in [
        &["local", "--json", "status"][..],
        &["local", "--json", "postgres", "list"][..],
    ] {
        let project = tempfile::tempdir().expect("create project");
        let home = tempfile::tempdir().expect("create home");
        let docker = docker_with_orphan_postgres(project.path()).await;
        let host = docker.uri().replace("http://", "tcp://");
        let output = run_with_docker(project.path(), home.path(), &host, args);
        assert_exit(&output, 0, args);
        let servers = stdout_json(&output)["servers"].clone();
        assert_eq!(
            servers.as_array().map(Vec::len),
            Some(1),
            "{args:?}: {servers}"
        );
        assert_eq!(servers[0]["name"], "default", "{args:?}: {servers}");
        assert_eq!(servers[0]["engine"], "postgres", "{args:?}: {servers}");
        assert_eq!(servers[0]["running"], true, "{args:?}: {servers}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn other_projects_postgres_containers_create_no_project_state() {
    let other = tempfile::tempdir().expect("create other project");
    let docker = docker_with_orphan_postgres(other.path()).await;
    let host = docker.uri().replace("http://", "tcp://");
    for (args, code) in [
        (&["local", "--json", "status"][..], 0),
        (&["local", "--json", "postgres", "list"][..], 0),
        (&["local", "--json", "postgres", "stop-all"][..], 0),
        (&["local", "--json", "postgres", "stop", "default"][..], 1),
    ] {
        let project = tempfile::tempdir().expect("create project");
        let home = tempfile::tempdir().expect("create home");
        let output = run_with_docker(project.path(), home.path(), &host, args);
        assert_exit(&output, code, args);
        assert_eq!(
            created_entries(project.path()),
            Vec::<String>::new(),
            "{args:?}"
        );
    }
    assert_eq!(stop_requests(&docker).await, 0);
}
