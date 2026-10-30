//! Engine commands cover one engine; `local status` covers both (#1159).

use serde_json::{Value, json};
use std::path::Path;
use std::process::{Command, Output};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A project holding a stopped ClickHouse server `default` and a Postgres
/// instance `app` (major 18) backed by the container `pg-container`.
fn seeded_project() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    let servers = project.path().join(".clickhouse/servers");
    std::fs::create_dir_all(servers.join("default/data")).unwrap();
    std::fs::create_dir_all(servers.join("app-pg18/data")).unwrap();
    std::fs::write(
        servers.join("default.json"),
        serde_json::to_vec(&json!({
            "name": "default", "pid": 0, "version": "", "http_port": 0, "tcp_port": 0,
            "started_at": "1700000000", "cwd": project.path(), "engine": "clickhouse"
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        servers.join("app-pg18.json"),
        serde_json::to_vec(&json!({
            "name": "app-pg18", "pid": 0, "version": "postgres:18", "http_port": 0,
            "tcp_port": 5433, "started_at": "1700000000", "cwd": project.path(),
            "engine": "postgres", "container_id": "pg-container"
        }))
        .unwrap(),
    )
    .unwrap();
    project
}

fn run(project: &Path, docker_host: &str, args: &[&str]) -> Output {
    // A home outside the project, so `~/.clickhouse` never lands in it.
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_clickhousectl"))
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home.path())
        .env("PATH", "/usr/bin:/bin")
        .env("DOCKER_HOST", docker_host)
        .current_dir(project)
        .args(args)
        .output()
        .unwrap()
}

fn json_ok(output: &Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).expect("JSON on stdout")
}

fn names_and_engines(body: &Value) -> Vec<(String, String)> {
    body["servers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|server| {
            (
                server["name"].as_str().unwrap().to_string(),
                server["engine"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn unreachable_docker(project: &Path) -> String {
    format!("unix://{}/missing.sock", project.display())
}

/// A Docker daemon reporting `pg-container` as running.
async fn running_postgres_docker() -> MockServer {
    postgres_docker(true).await
}

/// A Docker daemon reporting `pg-container` as running or stopped.
async fn postgres_docker(running: bool) -> MockServer {
    let docker = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"/_ping$"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&docker)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"/containers/json$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&docker)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"/containers/pg-container/json$"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"State": {"Running": running}})),
        )
        .mount(&docker)
        .await;
    docker
}

#[test]
fn clickhouse_list_and_stop_all_cover_only_clickhouse_without_docker() {
    let project = seeded_project();
    // An unreachable Docker fails any command that checks the Postgres
    // instance, so success here also proves Postgres was never consulted.
    let docker = unreachable_docker(project.path());
    for spelling in ["clickhouse", "ch", "server"] {
        let body = json_ok(&run(
            project.path(),
            &docker,
            &["local", spelling, "list", "--json"],
        ));
        assert_eq!(
            names_and_engines(&body),
            [("default".to_string(), "clickhouse".to_string())],
            "{spelling}: {body}"
        );
        assert_eq!(body["total_servers"], 1);
        assert_eq!(body["total_running_servers"], 0);

        let body = json_ok(&run(
            project.path(),
            &docker,
            &["local", spelling, "stop-all", "--json"],
        ));
        assert_eq!(body["servers"], json!([]), "{spelling}: {body}");
    }

    let human = run(project.path(), &docker, &["local", "clickhouse", "list"]);
    assert!(human.status.success(), "{human:?}");
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains("default"), "{stdout}");
    assert!(!stdout.contains("app"), "{stdout}");
}

#[tokio::test(flavor = "multi_thread")]
async fn postgres_list_shows_only_postgres_and_status_shows_both() {
    let project = seeded_project();
    let docker = running_postgres_docker().await;
    let host = docker.uri().replace("http://", "tcp://");

    for spelling in ["postgres", "pg"] {
        let body = json_ok(&run(
            project.path(),
            &host,
            &["local", spelling, "list", "--json"],
        ));
        assert_eq!(
            names_and_engines(&body),
            [("app".to_string(), "postgres".to_string())],
            "{spelling}: {body}"
        );
        let server = &body["servers"][0];
        assert_eq!(server["running"], true);
        assert_eq!(server["container_id"], "pg-container");
        assert_eq!(server["version"], "postgres:18");
        assert_eq!(server["tcp_port"], 5433);
        assert_eq!(body["total_servers"], 1);
        assert_eq!(body["total_running_servers"], 1);
    }

    let body = json_ok(&run(project.path(), &host, &["local", "status", "--json"]));
    let mut listed = names_and_engines(&body);
    listed.sort();
    assert_eq!(
        listed,
        [
            ("app".to_string(), "postgres".to_string()),
            ("default".to_string(), "clickhouse".to_string()),
        ],
        "{body}"
    );
    assert_eq!(body["total_servers"], 2);
    assert_eq!(body["total_running_servers"], 1);
    assert_eq!(body["project_scope"]["kind"], "exact_current_project");
    let mut keys: Vec<_> = body.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "project_scope",
            "servers",
            "total_running_servers",
            "total_servers"
        ]
    );

    let human = run(project.path(), &host, &["local", "status"]);
    assert!(human.status.success(), "{human:?}");
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(
        stdout.contains("app") && stdout.contains("default"),
        "{stdout}"
    );

    // Listing is read-only: no request may change Docker state.
    assert!(
        docker
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|request| request.method == "GET")
    );
}

#[test]
fn empty_lists_point_at_their_own_command_and_create_no_state() {
    let project = tempfile::tempdir().unwrap();
    let docker = unreachable_docker(project.path());
    for (args, command, global) in [
        (
            &["local", "clickhouse", "list", "--json"][..],
            "clickhousectl local clickhouse list",
            true,
        ),
        (
            &["local", "postgres", "list", "--json"][..],
            "clickhousectl local postgres list",
            false,
        ),
        (
            &["local", "status", "--json"][..],
            "clickhousectl local status",
            true,
        ),
    ] {
        let body = json_ok(&run(project.path(), &docker, args));
        assert_eq!(body["servers"], json!([]), "{args:?}");
        assert_eq!(body["total_servers"], 0);
        let guidance = body["guidance"].as_array().unwrap();
        assert!(
            guidance
                .iter()
                .any(|hint| hint["action"] == "list_project_servers" && hint["command"] == command),
            "{args:?}: {body}"
        );
        assert_eq!(
            guidance
                .iter()
                .any(|hint| hint["action"] == "list_global_servers"),
            global,
            "{args:?}: {body}"
        );
        assert!(!project.path().join(".clickhouse").exists(), "{args:?}");
    }
}

/// A name-only error for a Postgres instance must point at a list that shows
/// it: `local clickhouse list` no longer does.
#[tokio::test(flavor = "multi_thread")]
async fn stopped_postgres_error_points_at_a_list_that_shows_it() {
    let project = seeded_project();
    let docker = postgres_docker(false).await;
    let output = run(
        project.path(),
        &docker.uri(),
        &["local", "postgres", "client", "app", "--json"],
    );
    assert!(!output.status.success(), "{output:?}");
    let body: Value = serde_json::from_slice(&output.stderr).expect("JSON error on stderr");
    assert_eq!(body["error"]["code"], "server_not_running", "{body}");
    let command = body["error"]["command"].as_str().unwrap();
    assert_eq!(command, "clickhousectl local status");

    // Following the hint shows the instance.
    let args: Vec<&str> = command
        .strip_prefix("clickhousectl ")
        .unwrap()
        .split(' ')
        .chain(["--json"])
        .collect();
    let listed = json_ok(&run(project.path(), &docker.uri(), &args));
    assert!(
        names_and_engines(&listed).contains(&("app".to_string(), "postgres".to_string())),
        "{listed}"
    );
}
