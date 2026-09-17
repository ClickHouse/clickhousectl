//! Docker status failures must never become a stopped or missing instance.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::{Command, Output};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct Project {
    directory: tempfile::TempDir,
    metadata: PathBuf,
    original_metadata: Vec<u8>,
    data: PathBuf,
}

impl Project {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let servers = directory.path().join(".clickhouse/servers");
        let data = servers.join("default-pg18/data/sentinel");
        std::fs::create_dir_all(data.parent().unwrap()).unwrap();
        std::fs::write(&data, "keep existing data").unwrap();
        let metadata = servers.join("default-pg18.json");
        let original_metadata = serde_json::to_vec(&json!({
            "name": "default-pg18", "pid": 0, "version": "postgres:18",
            "http_port": 0, "tcp_port": 5432, "started_at": "1700000000",
            "cwd": directory.path(), "engine": "postgres", "container_id": "test-container"
        }))
        .unwrap();
        std::fs::write(&metadata, &original_metadata).unwrap();
        Self {
            directory,
            metadata,
            original_metadata,
            data,
        }
    }

    fn run(&self, docker_host: &str, args: &[&str], json: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_clickhousectl"));
        command
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", self.directory.path())
            .env("PATH", "/usr/bin:/bin")
            .env("DOCKER_HOST", docker_host)
            .current_dir(self.directory.path());
        command.args(args);
        if json {
            command.arg("--json");
        }
        command.output().unwrap()
    }

    fn assert_preserved(&self) {
        assert_eq!(
            std::fs::read(&self.metadata).unwrap(),
            self.original_metadata
        );
        assert_eq!(
            std::fs::read_to_string(&self.data).unwrap(),
            "keep existing data"
        );
    }
}

const STATUS_COMMANDS: &[&[&str]] = &[
    &["local", "server", "list"],
    &["local", "server", "stop-all"],
    &["local", "postgres", "client", "-q", "SELECT 1"],
    &["local", "postgres", "stop", "default"],
    &["local", "postgres", "stop-all"],
    &["local", "postgres", "remove", "default"],
    &["local", "postgres", "start", "--name", "default"],
    &["local", "postgres", "start"],
];

fn assert_error(output: &Output, code: &str) {
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let body: Value = serde_json::from_slice(&output.stderr).expect("structured error");
    assert_eq!(body["error"]["code"], code, "{body}");
}

#[test]
fn unreachable_docker_propagates_across_status_and_lifecycle() {
    let project = Project::new();
    let host = format!("unix://{}/missing.sock", project.directory.path().display());
    for args in STATUS_COMMANDS {
        assert_error(&project.run(&host, args, true), "docker_unavailable");
        project.assert_preserved();
    }
    let human = project.run(&host, &["local", "server", "list"], false);
    assert_eq!(human.status.code(), Some(1));
    assert!(human.stdout.is_empty());
    assert!(String::from_utf8_lossy(&human.stderr).contains("Docker is not available"));
}

async fn docker_mock(inspect_status: u16, body: Value) -> MockServer {
    let docker = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"/_ping$"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&docker)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"/containers/test-container/json$"))
        .respond_with(ResponseTemplate::new(inspect_status).set_body_json(body))
        .mount(&docker)
        .await;
    docker
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_or_incomplete_inspection_never_reports_stopped_or_mutates_data() {
    for (status, body) in [
        (500, json!({"message": "daemon inspect failure"})),
        (403, json!({"message": "inspection denied"})),
        (200, json!({"Id": "test-container"})),
        (200, json!({"State": {}})),
    ] {
        let project = Project::new();
        let docker = docker_mock(status, body).await;
        let host = docker.uri().replace("http://", "tcp://");
        for args in STATUS_COMMANDS {
            assert_error(&project.run(&host, args, true), "docker_error");
            project.assert_preserved();
        }
        assert!(
            docker
                .received_requests()
                .await
                .unwrap()
                .iter()
                .all(|request| request.method == "GET")
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn successful_inspection_distinguishes_running_stopped_and_missing() {
    for (status, body, running) in [
        (200, json!({"State": {"Running": true}}), true),
        (200, json!({"State": {"Running": false}}), false),
        (404, json!({"message": "No such container"}), false),
    ] {
        let project = Project::new();
        let docker = docker_mock(status, body).await;
        let host = docker.uri().replace("http://", "tcp://");
        let output = project.run(&host, &["local", "server", "list"], true);
        assert!(output.status.success(), "{output:?}");
        let body: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(body["total_servers"], 1);
        assert_eq!(body["total_running_servers"], usize::from(running));
        assert_eq!(body["servers"][0]["running"], running);
        if running {
            assert_error(
                &project.run(
                    &host,
                    &["local", "postgres", "start", "--name", "default"],
                    true,
                ),
                "server_running",
            );
            assert_error(
                &project.run(&host, &["local", "postgres", "remove", "default"], true),
                "server_running",
            );
        } else {
            assert_error(
                &project.run(
                    &host,
                    &["local", "postgres", "client", "-q", "SELECT 1"],
                    true,
                ),
                "server_not_running",
            );
        }
        if status == 404 {
            let output = project.run(
                &host,
                &["local", "postgres", "start", "--name", "default"],
                true,
            );
            assert_error(&output, "postgres_error");
            let body: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert!(
                body["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("container is gone")
            );
        }
        project.assert_preserved();
    }
}
