//! `cloud query-endpoint` (src/cloud/query_api_endpoints.rs).

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

// ── Organization-scoped error context (issue #334) ─────────────────────────

const MANUAL_QUERY_ENDPOINT_PATH: &str =
    "/v1/organizations/org-1/services/svc-1/serviceQueryEndpoint";

async fn manual_query_endpoint_mock(get_response: ResponseTemplate) -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(MANUAL_QUERY_ENDPOINT_PATH))
        .respond_with(get_response)
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(MANUAL_QUERY_ENDPOINT_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "id": "endpoint-1" },
            "status": 200,
            "requestId": "stub-query-endpoint-create"
        })))
        .mount(&mock)
        .await;
    mock
}

fn manual_query_endpoint_args<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "service",
        "query-endpoint",
        "create",
        "svc-1",
        "--org-id",
        "org-1",
        "--role",
        "sql_console_read_only",
    ];
    args.extend_from_slice(extra);
    args
}

#[tokio::test]
async fn query_endpoint_create_sends_typed_roles_and_explicit_first_origins() {
    // How explicit origins are carried is the builder's rule; one value is
    // enough to show the 404-then-create path and its permissions.
    let origins = "https://app.example.com";
    let mock = manual_query_endpoint_mock(ResponseTemplate::new(404)).await;
    let help = permission_help(&mock, &["service", "query-endpoint", "create"], "--help");
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &manual_query_endpoint_args(&[
            "--role",
            "sql_console_admin",
            "--open-api-key",
            "key-1",
            "--allowed-origins",
            origins,
        ]),
    );
    assert_success(&output);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result, serde_json::json!({"id": "endpoint-1"}));
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, wiremock::http::Method::GET);
    assert_eq!(requests[1].method, wiremock::http::Method::POST);
    assert_observed_permissions_advertised(&mock, &help).await;
    assert_eq!(
        requests[1].body_json::<Value>().unwrap(),
        serde_json::json!({
            "roles": ["sql_console_read_only", "sql_console_admin"],
            "openApiKeys": ["key-1"], "allowedOrigins": origins,
        })
    );
    for request in &requests {
        assert!(
            request
                .headers
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("Basic ")
        );
    }
}

#[tokio::test]
async fn query_endpoint_create_preserves_existing_keys_and_default_origins() {
    let mock = manual_query_endpoint_mock(manual_query_endpoint_response(serde_json::json!({
        "id": "endpoint-1", "roles": ["sql_console_admin"],
        "openApiKeys": ["existing-1", "existing-2", "existing-1"],
        "allowedOrigins": "https://before.example.com",
    })))
    .await;
    let body = invoke_cli_capture_body(
        &mock,
        &manual_query_endpoint_args(&[
            "--open-api-key",
            "existing-2",
            "--open-api-key",
            "new-key",
            "--open-api-key",
            "new-key",
        ]),
    )
    .await;
    assert_eq!(
        body,
        serde_json::json!({
            "roles": ["sql_console_read_only"],
            "openApiKeys": ["existing-1", "existing-2", "new-key"],
            "allowedOrigins": "https://before.example.com",
        })
    );
}

/// The read-modify-write keeps existing keys unless the flag says otherwise;
/// this is the only run where `--replace-open-api-keys` reaches the handler
/// with an existing endpoint.
#[tokio::test]
async fn query_endpoint_create_key_replacement_is_explicit() {
    let mock = manual_query_endpoint_mock(manual_query_endpoint_response(serde_json::json!({
        "openApiKeys": ["existing-1", "existing-2"],
        "allowedOrigins": "https://before.example.com",
    })))
    .await;
    let body = invoke_cli_capture_body(
        &mock,
        &manual_query_endpoint_args(&["--open-api-key", "new-key", "--replace-open-api-keys"]),
    )
    .await;
    assert_eq!(
        body,
        serde_json::json!({
            "roles": ["sql_console_read_only"], "openApiKeys": ["new-key"],
            "allowedOrigins": "https://before.example.com",
        })
    );
}

#[tokio::test]
async fn query_endpoint_create_refuses_failed_or_incomplete_get_without_writing() {
    let cases = [
        (
            ResponseTemplate::new(401),
            vec!["--allowed-origins", "*"],
            4,
        ),
        (
            ResponseTemplate::new(503),
            vec![
                "--allowed-origins",
                "*",
                "--replace-open-api-keys",
                "--open-api-key",
                "new-key",
            ],
            1,
        ),
        (
            ResponseTemplate::new(200).set_body_string("invalid JSON"),
            vec![],
            1,
        ),
        (ResponseTemplate::new(404), vec![], 1),
        (
            manual_query_endpoint_response(
                serde_json::json!({"allowedOrigins": "https://before.example.com"}),
            ),
            vec!["--open-api-key", "new-key"],
            1,
        ),
        (
            manual_query_endpoint_response(serde_json::json!(null)),
            vec!["--allowed-origins", "*"],
            1,
        ),
    ];
    for (response, flags, exit_code) in cases {
        let mock = manual_query_endpoint_mock(response).await;
        let output = invoke_cli_with_cloud_credentials(&mock, &manual_query_endpoint_args(&flags));
        assert_eq!(
            output.status.code(),
            Some(exit_code),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, wiremock::http::Method::GET);
    }
}

// Query API endpoint management (#766).

const QUERY_API_ENDPOINTS_PATH: &str = "/v1/organizations/org-1/services/svc-1/query-api-endpoints";
const QUERY_API_ENDPOINT_ID: &str = "11111111-2222-3333-8444-555555555555";
const QUERY_API_KEY_ID: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";

fn query_api_endpoint_config() -> Value {
    serde_json::json!({
        "name": "orders",
        "sql": "SELECT * FROM orders WHERE id = {id:String}",
        "database": "analytics",
        "parameters": {"id": "default-id", "region": "eu-west"},
        "apiKeyIds": [QUERY_API_KEY_ID],
        "roles": ["query_api", "reader"],
        "allowedOrigins": []
    })
}

fn query_api_endpoint_response() -> Value {
    let mut response = query_api_endpoint_config();
    response.as_object_mut().unwrap().extend(
        serde_json::json!({
            "id": QUERY_API_ENDPOINT_ID,
            "url": format!("https://queries.clickhouse.cloud/run/{QUERY_API_ENDPOINT_ID}"),
            "ownerType": "queryApiEndpoint"
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    response
}

fn invoke_query_api_endpoint(
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
            "query-endpoint-key",
            "--api-secret",
            "query-endpoint-secret",
        ]);
    }
    if json {
        command.arg("--json");
    }
    command.arg("query-api-endpoint").args(args);
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

fn query_api_endpoint_envelope(status: u16, result: Value) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(serde_json::json!({
        "status": status,
        "requestId": "query-api-endpoint-request",
        "result": result
    }))
}

#[tokio::test]
async fn query_api_endpoint_all_verbs_use_exact_routes_auth_and_complete_bodies() {
    // The --config-file alias is clap's; see `primary_json_file_argument_contract`
    // in `cloud::query_api_endpoints`.
    query_api_endpoint_all_verbs_use_exact_routes_auth_and_complete_bodies_with_file_flag("--file")
        .await;
}

async fn query_api_endpoint_all_verbs_use_exact_routes_auth_and_complete_bodies_with_file_flag(
    file_flag: &str,
) {
    let server = MockServer::start().await;
    let endpoint = query_api_endpoint_response();
    let list = serde_json::json!({
        "items": [{
            "id": QUERY_API_ENDPOINT_ID,
            "name": "orders",
            "database": "analytics",
            "apiKeyIds": [QUERY_API_KEY_ID],
            "roles": ["query_api", "reader"],
            "allowedOrigins": [],
            "url": format!("https://queries.clickhouse.cloud/run/{QUERY_API_ENDPOINT_ID}"),
            "ownerType": "queryApiEndpoint"
        }],
        "pagination": {"limit": 100, "totalRecords": 1}
    });
    for (verb, suffix, body, status, result) in [
        (
            "POST",
            "",
            Some(query_api_endpoint_config()),
            201,
            endpoint.clone(),
        ),
        (
            "PUT",
            QUERY_API_ENDPOINT_ID,
            Some(query_api_endpoint_config()),
            200,
            endpoint.clone(),
        ),
        ("GET", "", None, 200, list.clone()),
        ("GET", QUERY_API_ENDPOINT_ID, None, 200, endpoint.clone()),
    ] {
        let route = if suffix.is_empty() {
            QUERY_API_ENDPOINTS_PATH.to_string()
        } else {
            format!("{QUERY_API_ENDPOINTS_PATH}/{suffix}")
        };
        let mut mock =
            Mock::given(method(verb))
                .and(path(route))
                .and(wiremock::matchers::basic_auth(
                    "query-endpoint-key",
                    "query-endpoint-secret",
                ));
        if let Some(body) = body {
            mock = mock.and(body_json(body));
        }
        let response = if verb == "GET" && suffix.is_empty() {
            query_api_endpoint_list_page(result["items"].clone(), None, Some(1), Some(100))
        } else {
            query_api_endpoint_envelope(status, result)
        };
        mock.respond_with(response).expect(1).mount(&server).await;
    }
    Mock::given(method("DELETE"))
        .and(path(format!(
            "{QUERY_API_ENDPOINTS_PATH}/{QUERY_API_ENDPOINT_ID}"
        )))
        .and(wiremock::matchers::basic_auth(
            "query-endpoint-key",
            "query-endpoint-secret",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200, "requestId": "query-api-endpoint-delete"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let project = tempfile::tempdir().unwrap();
    let config_path = project.path().join("endpoint.json");
    std::fs::write(&config_path, query_api_endpoint_config().to_string()).unwrap();
    let create = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        true,
        &[
            "--org-id",
            "org-1",
            "create",
            "svc-1",
            file_flag,
            config_path.to_str().unwrap(),
        ],
        None,
    );
    assert_success(&create);
    assert_eq!(
        serde_json::from_slice::<Value>(&create.stdout).unwrap(),
        endpoint
    );

    let request_stdin = query_api_endpoint_config().to_string();
    let update = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        true,
        &[
            "update",
            "svc-1",
            QUERY_API_ENDPOINT_ID,
            file_flag,
            "-",
            "--org-id",
            "org-1",
        ],
        Some(&request_stdin),
    );
    assert_success(&update);
    assert_eq!(
        serde_json::from_slice::<Value>(&update.stdout).unwrap(),
        endpoint
    );

    let list_output = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        true,
        &["list", "svc-1", "--org-id", "org-1"],
        None,
    );
    assert_success(&list_output);
    assert_eq!(
        serde_json::from_slice::<Value>(&list_output.stdout).unwrap(),
        list
    );
    let get = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        true,
        &["get", "svc-1", QUERY_API_ENDPOINT_ID, "--org-id", "org-1"],
        None,
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        endpoint
    );
    let delete = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        true,
        &[
            "delete",
            "svc-1",
            QUERY_API_ENDPOINT_ID,
            "--org-id",
            "org-1",
        ],
        None,
    );
    assert_success(&delete);
    assert_eq!(
        serde_json::from_slice::<Value>(&delete.stdout).unwrap(),
        serde_json::json!({"status": 200, "requestId": "query-api-endpoint-delete"})
    );

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 5);
    let list_request = requests
        .iter()
        .find(|request| {
            request.method == wiremock::http::Method::GET
                && request.url.path() == QUERY_API_ENDPOINTS_PATH
        })
        .unwrap();
    assert!(list_request.url.query().is_none());
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == wiremock::http::Method::GET)
            .count(),
        2,
        "update must not prefetch the endpoint"
    );
}

#[tokio::test]
async fn query_api_endpoint_reads_support_oauth_and_preserve_reserved_list_cursor() {
    let server = MockServer::start().await;
    let result = serde_json::json!({
        "items": [{}],
        "pagination": {
            "currentCursor": "page /+?",
            "nextCursor": null,
            "limit": 2,
            "totalRecords": 3
        }
    });
    Mock::given(method("GET"))
        .and(path(QUERY_API_ENDPOINTS_PATH))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(query_api_endpoint_list_page(
            result["items"].clone(),
            None,
            Some(3),
            Some(2),
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!(
            "{QUERY_API_ENDPOINTS_PATH}/{QUERY_API_ENDPOINT_ID}"
        )))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(query_api_endpoint_envelope(200, serde_json::json!({})))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let list = invoke_query_api_endpoint(
        &server,
        project.path(),
        true,
        true,
        &[
            "list", "svc-1", "--cursor", "page /+?", "--limit", "2", "--org-id", "org-1",
        ],
        None,
    );
    assert_success(&list);
    let mut expected = result;
    expected["pagination"]
        .as_object_mut()
        .unwrap()
        .remove("nextCursor");
    assert_eq!(
        serde_json::from_slice::<Value>(&list.stdout).unwrap(),
        expected
    );
    let get = invoke_query_api_endpoint(
        &server,
        project.path(),
        true,
        true,
        &["get", "svc-1", QUERY_API_ENDPOINT_ID, "--org-id", "org-1"],
        None,
    );
    assert_success(&get);
    assert_eq!(
        serde_json::from_slice::<Value>(&get.stdout).unwrap(),
        serde_json::json!({})
    );

    let request = server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .find(|request| request.url.path() == QUERY_API_ENDPOINTS_PATH)
        .unwrap();
    let query: std::collections::HashMap<_, _> = request.url.query_pairs().into_owned().collect();
    assert_eq!(query.get("cursor").map(String::as_str), Some("page /+?"));
    assert_eq!(query.get("limit").map(String::as_str), Some("2"));
}

#[tokio::test]
async fn query_api_endpoint_human_reads_tolerate_sparse_fields_and_show_next_cursor() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!(
            "{QUERY_API_ENDPOINTS_PATH}/{QUERY_API_ENDPOINT_ID}"
        )))
        .respond_with(query_api_endpoint_envelope(200, serde_json::json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(QUERY_API_ENDPOINTS_PATH))
        .respond_with(query_api_endpoint_list_page(
            serde_json::json!([
                {},
                {
                    "id": QUERY_API_ENDPOINT_ID,
                    "name": "orders",
                    "database": "analytics",
                    "ownerType": "futureOwner",
                    "url": "https://queries.clickhouse.cloud/run/example"
                }
            ]),
            Some("next /+?"),
            None,
            None,
        ))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    let get = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        false,
        &["get", "svc-1", QUERY_API_ENDPOINT_ID, "--org-id", "org-1"],
        None,
    );
    assert_success(&get);
    assert_eq!(String::from_utf8_lossy(&get.stdout), "{}\n");
    let list = invoke_query_api_endpoint(
        &server,
        project.path(),
        false,
        false,
        &["list", "svc-1", "--org-id", "org-1"],
        None,
    );
    assert_success(&list);
    let stdout = String::from_utf8_lossy(&list.stdout);
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .filter(|line| line.starts_with('│'))
        .map(|line| {
            line.split('│')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect()
        })
        .collect();
    assert!(
        rows.contains(&vec!["-", "-", "-", "-", "-"]),
        "the empty item renders every column as absent:\n{stdout}"
    );
    assert!(
        rows.contains(&vec![
            QUERY_API_ENDPOINT_ID,
            "orders",
            "analytics",
            "futureOwner",
            "https://queries.clickhouse.cloud/run/example"
        ]),
        "{stdout}"
    );
    assert!(stdout.contains("Next cursor: next /+?"), "{stdout}");

    let empty_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(QUERY_API_ENDPOINTS_PATH))
        .respond_with(query_api_endpoint_list_page(
            serde_json::json!([]),
            Some("after-empty-page"),
            None,
            None,
        ))
        .expect(1)
        .mount(&empty_server)
        .await;
    let empty_project = tempfile::tempdir().unwrap();
    let empty = invoke_query_api_endpoint(
        &empty_server,
        empty_project.path(),
        false,
        false,
        &["list", "svc-1", "--org-id", "org-1"],
        None,
    );
    // No table: one empty-list line, then the cursor.
    assert_success(&empty);
    let stdout = String::from_utf8_lossy(&empty.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(!lines[0].contains('|'), "{stdout}");
    assert!(lines[1].ends_with("after-empty-page"), "{stdout}");
}
