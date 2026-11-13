//! `cloud saved-query` (src/cloud/saved_queries.rs).

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

// ── Saved queries (#1029) ───────────────────────────────────────────────────

const SAVED_QUERIES_PATH: &str = "/v1/organizations/org-1/services/svc-1/saved-queries";
const SAVED_QUERY_ID: &str = "22222222-3333-4444-8555-666666666666";
const SAVED_QUERY_OTHER_ID: &str = "33333333-4444-4555-8666-777777777777";

fn saved_query_body() -> Value {
    serde_json::json!({
        "name": "orders",
        "sql": "SELECT * FROM orders WHERE id = {id:String}",
        "database": "analytics",
        "parameters": {"id": "default-id", "region": "eu=west"}
    })
}

fn saved_query_response() -> Value {
    let mut response = saved_query_body();
    response["id"] = serde_json::json!(SAVED_QUERY_ID);
    response
}

fn saved_query_create_args(extra: &[&'static str]) -> Vec<&'static str> {
    let mut args = vec![
        "create",
        "svc-1",
        "--name",
        "orders",
        "--sql",
        "SELECT * FROM orders WHERE id = {id:String}",
        "--database",
        "analytics",
        "--param",
        "id=default-id",
        "--param",
        "region=eu=west",
        "--org-id",
        "org-1",
    ];
    args.extend_from_slice(extra);
    args
}

fn invoke_saved_query(
    server: &MockServer,
    project: &Path,
    oauth: bool,
    json: bool,
    args: &[&str],
    stdin: Option<&str>,
) -> std::process::Output {
    let home = project.join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    if oauth {
        write_oauth_tokens(&cloud_dir, &server.uri());
    }
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .current_dir(project)
        .args(["cloud", "--url", &server.uri()]);
    if !oauth {
        command.args([
            "--api-key",
            "saved-query-key",
            "--api-secret",
            "saved-query-secret",
        ]);
    }
    if json {
        command.arg("--json");
    }
    command.arg("saved-query").args(args);
    if let Some(input) = stdin {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    } else {
        command.stdin(Stdio::null()).output().unwrap()
    }
}

fn saved_query_envelope(status: u16, result: Value) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(serde_json::json!({
        "status": status,
        "requestId": "saved-query-request",
        "result": result
    }))
}

fn saved_query_list_envelope(
    result: Value,
    total: i64,
    next_cursor: Option<&str>,
) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "saved-query-list",
        "result": result,
        "limit": 100,
        "totalCount": total,
        "nextCursor": next_cursor
    }))
}

#[tokio::test]
async fn saved_query_all_verbs_use_exact_routes_auth_and_bodies() {
    let server = MockServer::start().await;
    let item_route = format!("{SAVED_QUERIES_PATH}/{SAVED_QUERY_ID}");
    Mock::given(method("POST"))
        .and(path(SAVED_QUERIES_PATH))
        .and(wiremock::matchers::basic_auth(
            "saved-query-key",
            "saved-query-secret",
        ))
        .and(body_json(saved_query_body()))
        .respond_with(saved_query_envelope(201, saved_query_response()))
        .expect(1)
        .mount(&server)
        .await;
    let update_body = serde_json::json!({
        "name": "orders v2",
        "sql": "SELECT count() FROM orders\n",
        "database": "default"
    });
    Mock::given(method("PUT"))
        .and(path(item_route.clone()))
        .and(body_json(update_body.clone()))
        .respond_with(saved_query_envelope(200, saved_query_response()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(item_route.clone()))
        .respond_with(saved_query_envelope(200, saved_query_response()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(item_route.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200, "requestId": "saved-query-delete"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let list_item = serde_json::json!([
        {"id": SAVED_QUERY_ID, "name": "orders", "database": "analytics"}
    ]);
    Mock::given(method("GET"))
        .and(path(SAVED_QUERIES_PATH))
        .and(query_param("cursor", "page /+?"))
        .and(query_param("limit", "2"))
        .respond_with(saved_query_list_envelope(
            list_item.clone(),
            3,
            Some("next /+?"),
        ))
        .expect(1)
        .mount(&server)
        .await;

    let project = tempfile::tempdir().unwrap();
    let create = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &saved_query_create_args(&[]),
        None,
    );
    assert_success(&create);
    assert_eq!(
        serde_json::from_slice::<Value>(&create.stdout).unwrap(),
        saved_query_response()
    );

    let update = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &[
            "update",
            "svc-1",
            SAVED_QUERY_ID,
            "--new-name",
            "orders v2",
            "--sql-file",
            "-",
            "--database",
            "default",
            "--org-id",
            "org-1",
        ],
        Some("SELECT count() FROM orders\n"),
    );
    assert_success(&update);
    assert_eq!(
        serde_json::from_slice::<Value>(&update.stdout).unwrap(),
        saved_query_response()
    );

    let list = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &[
            "list", "svc-1", "--cursor", "page /+?", "--limit", "2", "--org-id", "org-1",
        ],
        None,
    );
    assert_success(&list);
    assert_eq!(
        serde_json::from_slice::<Value>(&list.stdout).unwrap(),
        serde_json::json!({
            "result": list_item,
            "limit": 100,
            "totalCount": 3,
            "nextCursor": "next /+?"
        })
    );

    let get = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &["get", "svc-1", SAVED_QUERY_ID, "--org-id", "org-1"],
        None,
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        saved_query_response()
    );

    let delete = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &["delete", "svc-1", SAVED_QUERY_ID, "--org-id", "org-1"],
        None,
    );
    assert_success(&delete);
    assert_eq!(
        serde_json::from_slice::<Value>(&delete.stdout).unwrap(),
        serde_json::json!({"status": 200, "requestId": "saved-query-delete"})
    );
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        5,
        "an ID target must not list or prefetch"
    );
}

#[tokio::test]
async fn saved_query_sql_file_path_is_sent_verbatim() {
    let server = MockServer::start().await;
    let sql = "SELECT *\nFROM orders -- é\n";
    Mock::given(method("POST"))
        .and(path(SAVED_QUERIES_PATH))
        .and(body_json(serde_json::json!({
            "name": "orders",
            "sql": sql,
            "database": "default"
        })))
        .respond_with(saved_query_envelope(201, saved_query_response()))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let file = project.path().join("orders.sql");
    std::fs::write(&file, sql).unwrap();
    let output = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &[
            "create",
            "svc-1",
            "--name",
            "orders",
            "--sql-file",
            file.to_str().unwrap(),
            "--database",
            "default",
            "--org-id",
            "org-1",
        ],
        None,
    );
    assert_success(&output);
}

#[tokio::test]
async fn saved_query_name_selector_pages_through_list_before_mutating() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SAVED_QUERIES_PATH))
        .and(query_param("limit", "100"))
        .and(query_param("cursor", "page-2"))
        .respond_with(saved_query_list_envelope(
            serde_json::json!([{"id": SAVED_QUERY_ID, "name": "orders", "database": "default"}]),
            2,
            None,
        ))
        .with_priority(1)
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(SAVED_QUERIES_PATH))
        .and(query_param("limit", "100"))
        .respond_with(saved_query_list_envelope(
            serde_json::json!([{"id": SAVED_QUERY_OTHER_ID, "name": "other", "database": "default"}]),
            2,
            Some("page-2"),
        ))
        .with_priority(10)
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{SAVED_QUERIES_PATH}/{SAVED_QUERY_ID}")))
        .respond_with(saved_query_envelope(200, saved_query_response()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("{SAVED_QUERIES_PATH}/{SAVED_QUERY_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200, "requestId": "saved-query-delete"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let get = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &["get", "svc-1", "--name", "orders", "--org-id", "org-1"],
        None,
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        saved_query_response()
    );
    let delete = invoke_saved_query(
        &server,
        project.path(),
        false,
        false,
        &["delete", "svc-1", "--name", "orders", "--org-id", "org-1"],
        None,
    );
    assert_success(&delete);
    let stdout = String::from_utf8_lossy(&delete.stdout);
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(stdout.contains(SAVED_QUERY_ID), "{stdout}");

    // An unknown name fails before any mutation.
    let missing = invoke_saved_query(
        &server,
        project.path(),
        false,
        true,
        &["delete", "svc-1", "--name", "absent", "--org-id", "org-1"],
        None,
    );
    assert_eq!(missing.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("absent"),
        "{}",
        String::from_utf8_lossy(&missing.stderr)
    );
    let deletes = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|request| request.method == wiremock::http::Method::DELETE)
        .count();
    assert_eq!(deletes, 1);
}

#[tokio::test]
async fn saved_query_human_list_renders_table_and_next_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SAVED_QUERIES_PATH))
        .respond_with(saved_query_list_envelope(
            serde_json::json!([
                {},
                {"id": SAVED_QUERY_ID, "name": "orders", "database": "analytics"}
            ]),
            5,
            Some("next-page"),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let output = invoke_saved_query(
        &server,
        project.path(),
        false,
        false,
        &["list", "svc-1", "--org-id", "org-1"],
        None,
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    for expected in [
        SAVED_QUERY_ID,
        "orders",
        "analytics",
        "Next cursor: next-page",
    ] {
        assert!(stdout.contains(expected), "missing {expected}: {stdout}");
    }
    let request = &server.received_requests().await.unwrap()[0];
    assert!(request.url.query().is_none(), "{:?}", request.url.query());
}
