//! `cloud udf` (src/cloud/udfs.rs).

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

// UDF operations and artifact uploads run against separate hosts so auth and
// presigned-query isolation are exercised by the real executable.
fn udf_test_command(
    server: &MockServer,
    project: &Path,
    oauth: bool,
    json: bool,
    args: &[&str],
) -> Command {
    let home = project.join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    if oauth {
        write_oauth_tokens(&cloud_dir, &server.uri());
    }
    let mut cmd = Command::new(clickhousectl_binary());
    cmd.isolated_env();
    cmd.env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        // Waits poll the service; skip the shipped 5 s gap between polls.
        .env("CHCTL_UDF_POLL_INTERVAL_MS", "100")
        .current_dir(project)
        .args(["cloud", "--url", &server.uri()]);
    if !oauth {
        cmd.args(["--api-key", "udf-key", "--api-secret", "udf-secret"]);
    }
    if json {
        cmd.arg("--json");
    }
    cmd.args(["udf", "--org-id", "org-1"])
        .args(args)
        .stdin(Stdio::null());
    cmd
}

fn udf_definition(kind: &str, create: bool) -> Value {
    let mut value = serde_json::json!({"type": kind, "runtime": "native", "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64"});
    if create {
        value["functionName"] = serde_json::json!("my_udf");
    }
    value
}

#[tokio::test]
async fn udf_all_reads_preserve_pagination_and_sparse_responses_with_oauth() {
    for (args, suffix, list) in [
        (vec!["list"], "", true),
        (vec!["get", "my_udf"], "/my_udf", false),
        (
            vec!["attachment", "list", "my_udf"],
            "/my_udf/attachments",
            true,
        ),
        (
            vec!["attachment", "get", "my_udf", "svc-1"],
            "/my_udf/attachments/svc-1",
            false,
        ),
        (vec!["version", "list", "my_udf"], "/my_udf/versions", true),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        let item = serde_json::json!({"functionName":"my_udf", "serviceId":"svc-1", "version":2, "status":"future", "deterministic":false});
        let result = if list {
            serde_json::json!([item])
        } else {
            item
        };
        Mock::given(method("GET"))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .and(header("authorization", "Bearer test-bearer-token"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(if list { serde_json::json!({"result":result,"nextCursor":"next page","limit":2,"totalCount":3}) } else { serde_json::json!({"result":result}) }),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut args = args.clone();
        if list {
            args.extend(["--cursor", "page /+?", "--limit", "2"]);
        }
        let output = udf_test_command(&server, project.path(), true, true, &args)
            .output()
            .unwrap();
        assert_success(&output);
        let output: Value = serde_json::from_slice(&output.stdout).unwrap();
        if list {
            assert_eq!(output["nextCursor"], "next page");
            assert_eq!(output["limit"], 2);
            assert_eq!(output["totalCount"], 3);
            assert_eq!(output["result"][0]["status"], result[0]["status"]);
            assert!(output.get("items").is_none());
            assert!(output.get("pagination").is_none());
            let req = server.received_requests().await.unwrap().pop().unwrap();
            let query: std::collections::HashMap<_, _> =
                req.url.query_pairs().into_owned().collect();
            assert_eq!(query.get("cursor").map(String::as_str), Some("page /+?"));
            assert_eq!(query.get("limit").map(String::as_str), Some("2"));
        } else {
            assert_eq!(output["status"], result["status"]);
        }
    }
}

#[tokio::test]
async fn udf_delete_attach_detach_methods_and_auth() {
    for (args, suffix, verb, expected_body) in [
        (vec!["delete", "my_udf"], "/my_udf", "DELETE", None),
        (
            vec!["version", "delete", "my_udf", "2"],
            "/my_udf/versions/2",
            "DELETE",
            None,
        ),
        (
            vec!["detach", "my_udf", "svc-1"],
            "/my_udf/attachments/svc-1",
            "DELETE",
            None,
        ),
        (
            vec!["attach", "my_udf", "svc-1"],
            "/my_udf/attachments/svc-1",
            "PUT",
            Some(serde_json::json!({})),
        ),
        (
            vec!["attach", "my_udf", "svc-1", "--version", "2"],
            "/my_udf/attachments/svc-1",
            "PUT",
            Some(serde_json::json!({"version":2})),
        ),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        Mock::given(method(verb))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .and(header("authorization", "Basic dWRmLWtleTp1ZGYtc2VjcmV0"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(if verb == "DELETE" {
                    serde_json::json!({"status":200,"requestId":"delete-request"})
                } else {
                    serde_json::json!({"result":{}})
                }),
            )
            .expect(1)
            .mount(&server)
            .await;
        let output = udf_test_command(&server, project.path(), false, true, &args)
            .output()
            .unwrap();
        assert_success(&output);
        if verb == "DELETE" {
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                serde_json::json!({"status":200,"requestId":"delete-request"})
            );
        }
        let request = server.received_requests().await.unwrap().pop().unwrap();
        match expected_body {
            Some(body) => assert_eq!(
                serde_json::from_slice::<Value>(&request.body).unwrap(),
                body
            ),
            None => assert!(request.body.is_empty()),
        }
    }
}

#[tokio::test]
async fn udf_delete_commands_print_human_confirmations() {
    // One confirmation line naming what was deleted.
    for (args, suffix, expected) in [
        (vec!["delete", "my_udf"], "/my_udf", &["my_udf"][..]),
        (
            vec!["version", "delete", "my_udf", "2"],
            "/my_udf/versions/2",
            &["my_udf", "2"][..],
        ),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        Mock::given(method("DELETE"))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .and(header("authorization", "Basic dWRmLWtleTp1ZGYtc2VjcmV0"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"status":200,"requestId":"delete-request"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let output = udf_test_command(&server, project.path(), false, false, &args)
            .output()
            .unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout.lines().count(), 1, "{stdout}");
        for identifier in expected {
            assert!(stdout.contains(identifier), "{identifier}: {stdout}");
        }
    }
}

#[tokio::test]
async fn udf_empty_lists_have_named_empty_states_and_readable_pagination() {
    // Each list has its own empty-state line (no table), then the page data.
    let mut empty_states = std::collections::BTreeSet::new();
    for (args, suffix) in [
        (vec!["list"], ""),
        (vec!["attachment", "list", "my_udf"], "/my_udf/attachments"),
        (vec!["version", "list", "my_udf"], "/my_udf/versions"),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        Mock::given(method("GET"))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": [], "limit":100, "nextCursor":"next page", "totalCount":0
            })))
            .expect(1)
            .mount(&server)
            .await;

        let output = udf_test_command(&server, project.path(), false, false, &args)
            .output()
            .unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(lines.len(), 2, "{stdout}");
        assert!(!lines[0].contains('|'), "{stdout}");
        assert!(empty_states.insert(lines[0].to_owned()), "{stdout}");
        assert!(
            lines[1].contains("100") && lines[1].ends_with("next page"),
            "{stdout}"
        );
    }
}

#[tokio::test]
async fn udf_create_and_version_upload_full_definitions_without_auth_leakage() {
    // The `--file` aliases are `primary_json_file_argument_contract`, and the
    // per-kind body is the builder's; one run each of create and version create.
    for (create, kind) in [(true, "executable_pool"), (false, "executable")] {
        let server = MockServer::start().await;
        let storage = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        let archive = b"PK\x03\x04test archive";
        std::fs::write(project.path().join("code.zip"), archive).unwrap();
        let mut definition = udf_definition(kind, create);
        definition.as_object_mut().unwrap().extend(serde_json::json!({
            "commandReadTimeout": 5000, "commandWriteTimeout": 6000, "memoryLimitMib": 128,
            "deterministic": false, "sendChunkHeader": false, "returnName":"result", "format":"JSONEachRow",
            "sandboxType":"netenable", "sandboxVersion":"v3", "maxCommandExecutionTime":20,
            "poolSize": if kind == "executable_pool" { serde_json::json!(4) } else { Value::Null }
        }).as_object().unwrap().clone());
        std::fs::write(
            project.path().join("definition.json"),
            definition.to_string(),
        )
        .unwrap();
        Mock::given(method("POST")).and(path("/v1/organizations/org-1/udfUploads/url"))
            .and(header("authorization", "Basic dWRmLWtleTp1ZGYtc2VjcmV0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":{
                "uploadId":"fresh-upload", "uploadUrl":format!("{}/artifact?signature=upload-secret", storage.uri())
            }}))).expect(1).mount(&server).await;
        Mock::given(method("PUT"))
            .and(path("/artifact"))
            .and(header("content-type", "application/zip"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&storage)
            .await;
        let mut expected = definition;
        expected["uploadId"] = serde_json::json!("fresh-upload");
        // Cloud no longer accepts sandboxVersion: it is dropped, with a warning.
        expected.as_object_mut().unwrap().remove("sandboxVersion");
        if kind == "executable" {
            expected.as_object_mut().unwrap().remove("poolSize");
        }
        let suffix = if create { "" } else { "/my_udf/versions" };
        Mock::given(method("POST")).and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .and(header("authorization", "Basic dWRmLWtleTp1ZGYtc2VjcmV0"))
            .and(body_json(expected))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"result":{"functionName":"my_udf","version":2,"status":"building","deterministic":false}})))
            .expect(1).mount(&server).await;
        let mut args = if create {
            vec!["create"]
        } else {
            vec!["version", "create", "my_udf"]
        };
        args.extend([
            "--file",
            "definition.json",
            "--artifact",
            "code.zip",
            "--debug",
        ]);
        let output = udf_test_command(&server, project.path(), false, true, &args)
            .output()
            .unwrap();
        assert_success(&output);
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap()["deterministic"],
            false
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr
                .lines()
                .any(|line| line.starts_with("Warning:") && line.contains("sandboxVersion")),
            "{stderr}"
        );
        let upload = storage.received_requests().await.unwrap().pop().unwrap();
        assert_eq!(upload.body, archive);
        assert_eq!(upload.url.query(), Some("signature=upload-secret"));
        assert!(!upload.headers.contains_key("authorization"));
        assert!(!upload.headers.contains_key("cookie"));
        let logged = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        for secret in ["upload-secret", "fresh-upload", "udf-key", "udf-secret"] {
            assert!(!logged.contains(secret), "leaked {secret}");
        }
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].body.is_empty());
    }
}

#[tokio::test]
async fn udf_upload_failures_never_create_or_leak_presigned_url() {
    for scenario in [
        "missing_id",
        "missing_url",
        "bad_url",
        "credentials",
        "redirect",
        "storage_error",
        "connection_error",
        "session_error",
    ] {
        let server = MockServer::start().await;
        let storage = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("code.zip"), b"PK\x03\x04archive").unwrap();
        std::fs::write(
            project.path().join("definition.json"),
            udf_definition("executable", true).to_string(),
        )
        .unwrap();
        let url = match scenario {
            "bad_url" => "not-a-url?signature=upload-secret".to_string(),
            "credentials" => "https://user:upload-secret@example.com/code".to_string(),
            "connection_error" => "http://127.0.0.1:1/code?signature=upload-secret".to_string(),
            _ => format!("{}/code?signature=upload-secret", storage.uri()),
        };
        let mut session = serde_json::json!({"uploadId":"upload-1","uploadUrl":url});
        if scenario == "missing_id" {
            session.as_object_mut().unwrap().remove("uploadId");
        }
        if scenario == "missing_url" {
            session.as_object_mut().unwrap().remove("uploadUrl");
        }
        let response = if scenario == "session_error" {
            ResponseTemplate::new(503).set_body_json(
                serde_json::json!({"error":"session unavailable signature=upload-secret"}),
            )
        } else {
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":session}))
        };
        Mock::given(method("POST"))
            .and(path("/v1/organizations/org-1/udfUploads/url"))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        let response = if scenario == "redirect" {
            ResponseTemplate::new(307).insert_header(
                "location",
                format!("{}/redirected?signature=upload-secret", storage.uri()),
            )
        } else {
            ResponseTemplate::new(403).set_body_string("signature=upload-secret")
        };
        Mock::given(method("PUT"))
            .and(path("/code"))
            .respond_with(response)
            .mount(&storage)
            .await;
        let output = udf_test_command(
            &server,
            project.path(),
            false,
            true,
            &[
                "create",
                "--file",
                "definition.json",
                "--artifact",
                "code.zip",
                "--debug",
            ],
        )
        .output()
        .unwrap();
        assert_eq!(output.status.code(), Some(1), "{scenario}");
        let logged = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!logged.contains("upload-secret"), "{scenario}: {logged}");
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        assert!(storage.received_requests().await.unwrap().len() <= 1);
    }
}

// A `udf.json` reused with `version create --file` may keep a functionName
// that matches NAME (stripped from the body); a different one is a usage error.
#[tokio::test]
async fn udf_version_create_file_strips_a_matching_function_name() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("code.zip"), b"PK\x03\x04archive").unwrap();
    std::fs::write(
        project.path().join("udf.json"),
        udf_definition("executable", true).to_string(),
    )
    .unwrap();
    let args = [
        "version",
        "create",
        "my_udf",
        "--file",
        "udf.json",
        "--artifact",
        "code.zip",
    ];
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfUploads/url"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":{
            "uploadId":"fresh-upload", "uploadUrl":format!("{}/artifact?signature=s", storage.uri())
        }})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/artifact"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&storage)
        .await;
    let mut expected = udf_definition("executable", false);
    expected["uploadId"] = serde_json::json!("fresh-upload");
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
        .and(body_json(expected))
        .respond_with(ResponseTemplate::new(201).set_body_json(
            serde_json::json!({"result":{"functionName":"my_udf","version":2,"status":"building"}}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(&server, project.path(), false, true, &args)
        .output()
        .unwrap();
    assert_success(&output);

    let mismatch = MockServer::start().await;
    let mut renamed = args;
    renamed[2] = "other_udf";
    let output = udf_test_command(&mismatch, project.path(), false, true, &renamed)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("udf.json"), "{stderr}");
    assert!(mismatch.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn udf_create_api_failures_consume_one_upload_attempt() {
    // Create and version create share `upload_artifact`; one run covers both.
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("code.zip"), b"PK\x03\x04archive").unwrap();
    std::fs::write(
        project.path().join("definition.json"),
        udf_definition("executable", true).to_string(),
    )
    .unwrap();
    Mock::given(method("POST")).and(path("/v1/organizations/org-1/udfUploads/url"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":{
            "uploadId":"one-attempt", "uploadUrl":format!("{}/code?signature=upload-secret", storage.uri())
        }}))).expect(1).mount(&server).await;
    Mock::given(method("PUT"))
        .and(path("/code"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&storage)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .respond_with(
            ResponseTemplate::new(424)
                .set_body_json(serde_json::json!({"error":"build dependency unavailable"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let args = [
        "create",
        "--file",
        "definition.json",
        "--artifact",
        "code.zip",
    ];
    let output = udf_test_command(&server, project.path(), false, true, &args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("build dependency unavailable"));
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn udf_stdin_minimal_definition_and_sparse_human_output() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("code.zip"), b"PK\x03\x04archive").unwrap();
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfUploads/url"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result":{
                "uploadId":"fresh-upload", "uploadUrl":format!("{}/code", storage.uri())
            }})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/code"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&storage)
        .await;
    let mut expected = udf_definition("executable", true);
    expected["uploadId"] = serde_json::json!("fresh-upload");
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .and(body_json(expected))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"result":{}})))
        .expect(1)
        .mount(&server)
        .await;
    let mut cmd = udf_test_command(
        &server,
        project.path(),
        false,
        false,
        &["create", "--file", "-", "--artifact", "code.zip"],
    );
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(udf_definition("executable", true).to_string().as_bytes())
        .unwrap();
    assert_success(&child.wait_with_output().unwrap());
    for (args, suffix, result) in [
        (
            vec!["get", "my_udf"],
            "/my_udf",
            serde_json::json!({"functionName":null}),
        ),
        (vec!["list"], "", serde_json::json!([{}])),
        (
            vec!["attachment", "list", "my_udf"],
            "/my_udf/attachments",
            serde_json::json!([
                {"functionName": "my_udf", "serviceId": "svc-1", "version": 2, "status": "deployed"},
                {}
            ]),
        ),
        (
            vec!["version", "list", "my_udf"],
            "/my_udf/versions",
            serde_json::json!([]),
        ),
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/v1/organizations/org-1/udfs{suffix}")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"result":result,"nextCursor":"next"})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let output = udf_test_command(&server, project.path(), false, false, &args)
            .output()
            .unwrap();
        assert_success(&output);
        if args == vec!["list"] {
            assert!(String::from_utf8_lossy(&output.stdout).contains("next"));
        }
        if args[0] == "attachment" {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let rows: Vec<Vec<&str>> = stdout
                .lines()
                .filter(|line| line.starts_with('|') && !line.starts_with("|-"))
                .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
                .collect();
            assert_eq!(
                rows,
                [
                    ["name", "service_id", "version", "status"],
                    ["my_udf", "svc-1", "2", "deployed"],
                    ["-", "-", "-", "-"],
                ],
                "{stdout}"
            );
        }
        let request = server.received_requests().await.unwrap().pop().unwrap();
        assert!(request.url.query().is_none());
    }
}

// ── UDF directories, native archives and waking (#1030) ─────────────────────

fn udf_dir(parent: &Path, name: &str, runtime: &str) -> std::path::PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("udf.json"),
        serde_json::json!({
            "functionName": name, "type": "executable", "runtime": runtime,
            "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64"
        })
        .to_string(),
    )
    .unwrap();
    if runtime == "native" {
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\n").unwrap();
        for arch in ["amd64", "arm64"] {
            std::fs::create_dir_all(dir.join(arch)).unwrap();
            std::fs::write(dir.join(arch).join("main"), "binary\n").unwrap();
        }
        std::fs::write(dir.join("amd64/model.bin"), "weights\n").unwrap();
    } else {
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("main.py"), "import sys\n").unwrap();
        std::fs::write(dir.join("lib/helper.py"), "x = 1\n").unwrap();
        std::fs::write(dir.join(".env"), "SECRET=1\n").unwrap();
    }
    dir
}

fn project_udf(project: &Path, name: &str, runtime: &str) -> std::path::PathBuf {
    udf_dir(&project.join("clickhouse/udfs"), name, runtime)
}

async fn mount_udf_upload_session(server: &MockServer, storage: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfUploads/url"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
                "uploadId": "fresh-upload",
                "uploadUrl": format!("{}/artifact?signature=upload-secret", storage.uri())
            }})),
        )
        .expect(1)
        .mount(server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/artifact"))
        .and(header("content-type", "application/zip"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(storage)
        .await;
}

fn zip_entry_names(bytes: &[u8]) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).unwrap();
    (0..archive.len())
        .map(|index| archive.by_index(index).unwrap().name().unwrap().to_string())
        .collect()
}

fn udf_result(name: &str, version: i64, status: &str) -> ResponseTemplate {
    ResponseTemplate::new(201).set_body_json(serde_json::json!({"result": {
        "functionName": name, "version": version, "status": status
    }}))
}

#[tokio::test]
async fn udf_create_name_packages_the_directory_and_reads_its_udf_json() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    // `--dir` names another parent; `version create` below covers the default.
    let shared = tempfile::tempdir().unwrap();
    udf_dir(&shared.path().join("udfs"), "my_udf", "python3.11");
    mount_udf_upload_session(&server, &storage).await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .and(body_json(serde_json::json!({
            "functionName": "my_udf", "type": "executable", "runtime": "python3.11",
            "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64",
            "uploadId": "fresh-upload"
        })))
        .respond_with(udf_result("my_udf", 1, "building"))
        .expect(1)
        .mount(&server)
        .await;

    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &[
            "create",
            "my_udf",
            "--dir",
            shared.path().join("udfs").to_str().unwrap(),
        ],
    )
    .output()
    .unwrap();
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["status"],
        "building"
    );
    let upload = storage.received_requests().await.unwrap().pop().unwrap();
    assert!(upload.body.starts_with(b"PK\x03\x04"));
    assert_eq!(
        zip_entry_names(&upload.body),
        ["lib/", "lib/helper.py", "main.py"]
    );
    assert!(!upload.headers.contains_key("authorization"));
}

#[tokio::test]
async fn udf_version_create_name_archives_native_layout_and_drops_the_name() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "native");
    mount_udf_upload_session(&server, &storage).await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
        .and(body_json(serde_json::json!({
            "type": "executable", "runtime": "native",
            "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64",
            "uploadId": "fresh-upload"
        })))
        .respond_with(udf_result("my_udf", 2, "building"))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["version", "create", "my_udf"],
    )
    .output()
    .unwrap();
    assert_success(&output);
    assert_eq!(
        zip_entry_names(
            &storage
                .received_requests()
                .await
                .unwrap()
                .pop()
                .unwrap()
                .body
        ),
        [
            "amd64/",
            "amd64/main",
            "amd64/model.bin",
            "arm64/",
            "arm64/main"
        ]
    );
}

// The UDF directory failures that render JSON carry the codes `local udf
// deploy` reports for the same problems, before any request is made.
#[tokio::test]
async fn udf_directory_problems_report_local_udf_codes() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let symlinked = project_udf(project.path(), "symlinked", "python3.11");
    std::os::unix::fs::symlink(symlinked.join("lib/helper.py"), symlinked.join("link.py")).unwrap();
    let not_json = project_udf(project.path(), "not_json", "python3.11");
    std::fs::write(not_json.join("udf.json"), "{ not json").unwrap();
    let no_binary = project_udf(project.path(), "no_binary", "native");
    std::fs::remove_file(no_binary.join("amd64/main")).unwrap();

    // Which problem gets which code is `udf_json_failures_carry_the_local_udf_codes`
    // and `package_source_dir_rejects_*`; one run per command reaches JSON.
    for (args, code) in [
        (vec!["create", "symlinked"], "udf_source_invalid"),
        (
            vec!["version", "create", "not_json"],
            "udf_definition_invalid",
        ),
        (
            vec!["deploy", "no_binary", "--service", "svc-1"],
            "udf_source_invalid",
        ),
    ] {
        let output = udf_test_command(&server, project.path(), false, true, &args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let error: Value = serde_json::from_slice(&output.stderr)
            .unwrap_or_else(|_| panic!("{args:?}: {}", String::from_utf8_lossy(&output.stderr)));
        assert_eq!(error["error"]["code"], code, "{args:?}: {error}");
    }

    // The functionName check runs in each handler (version create: its own
    // test); a mismatch is a usage error before anything is uploaded.
    let renamed = project_udf(project.path(), "renamed", "python3.11");
    std::fs::write(
        renamed.join("udf.json"),
        serde_json::json!({
            "functionName": "my_udf", "type": "executable", "runtime": "python3.11",
            "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64"
        })
        .to_string(),
    )
    .unwrap();
    for args in [
        vec!["create", "renamed"],
        vec!["deploy", "renamed", "--service", "svc-1"],
    ] {
        let output = udf_test_command(&server, project.path(), false, true, &args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn udf_attach_reports_idle_services_and_wakes_them_with_wake() {
    let idle_424 = ResponseTemplate::new(424).set_body_json(serde_json::json!({
        "error": "service is idle", "code": "SERVICE_IDLE", "serviceState": "idle",
        "canWake": true, "status": 424
    }));

    // Without --wake: the typed reason and the remedy are in the message, and
    // no wake request is sent.
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(idle_424.clone())
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["attach", "my_udf", "svc-1"],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("SERVICE_IDLE"), "{message}");
    assert!(message.contains("--wake"), "{message}");
    assert_eq!(error["error"]["code"], "service_idle", "{error}");
    assert_eq!(
        error["error"]["command"],
        "clickhousectl cloud service wake svc-1"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    // With --wake: 424 → PATCH awake → service running → attach accepted.
    // The command returns the accepted attachment without polling it.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(idle_424)
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/state"))
        .and(body_json(serde_json::json!({"command": "awake"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "awaking"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "running"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/attachments/svc-1"))
        .and(body_json(serde_json::json!({"version": 2})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
            "functionName": "my_udf", "serviceId": "svc-1", "version": 2, "status": "provisioning"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["attach", "my_udf", "svc-1", "--version", "2", "--wake"],
    )
    .output()
    .unwrap();
    assert_success(&output);
    let attachment: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(attachment["status"], "provisioning");
    assert_eq!(attachment["version"], 2);
    let calls: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| format!("{} {}", request.method, request.url.path()))
        .collect();
    assert_eq!(
        calls,
        [
            "PUT /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
            "PATCH /v1/organizations/org-1/services/svc-1/state",
            "GET /v1/organizations/org-1/services/svc-1",
            "PUT /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ]
    );
}

// A stopped or starting service gets its own JSON code, derived from the
// typed 424 payload, and exit 1; so does a --wake that finds the service
// stopped while waiting for it.
#[tokio::test]
async fn udf_attach_reports_stopped_and_not_running_services_with_their_codes() {
    for (body, code) in [
        (
            serde_json::json!({
                "error": "service is stopped", "code": "SERVICE_STOPPED",
                "serviceState": "stopped", "canWake": false, "status": 424
            }),
            "service_stopped",
        ),
        (
            serde_json::json!({
                "error": "service is not running", "code": "SERVICE_NOT_RUNNING",
                "serviceState": "starting", "canWake": false, "status": 424
            }),
            "service_not_running",
        ),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        Mock::given(method("PUT"))
            .and(path(
                "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
            ))
            .respond_with(ResponseTemplate::new(424).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let output = udf_test_command(
            &server,
            project.path(),
            false,
            true,
            &["attach", "my_udf", "svc-1"],
        )
        .output()
        .unwrap();
        assert_eq!(output.status.code(), Some(1), "{code}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], code, "{error}");
    }

    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(424).set_body_json(serde_json::json!({
            "error": "service is idle", "code": "SERVICE_IDLE", "serviceState": "idle",
            "canWake": true, "status": 424
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/state"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "awaking"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "stopped"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["attach", "my_udf", "svc-1", "--wake"],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "service_stopped", "{error}");
}

// A failed wake PATCH is not fatal while the service is on its way up
// (here `provisioning`): the CLI waits for `running` and retries the attach.
#[tokio::test]
async fn udf_attach_wake_waits_out_a_failed_wake_while_the_service_comes_up() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(424).set_body_json(serde_json::json!({
            "error": "service is idle", "code": "SERVICE_IDLE", "serviceState": "idle",
            "canWake": true, "status": 424
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/state"))
        .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "status": 409, "error": "service state is changing"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "provisioning"}})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": "running"}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
            "functionName": "my_udf", "serviceId": "svc-1", "version": 1, "status": "provisioning"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["attach", "my_udf", "svc-1", "--wake"],
    )
    .output()
    .unwrap();
    assert_success(&output);
    let calls: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| format!("{} {}", request.method, request.url.path()))
        .collect();
    assert_eq!(
        calls,
        [
            "PUT /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
            "PATCH /v1/organizations/org-1/services/svc-1/state",
            "GET /v1/organizations/org-1/services/svc-1",
            "GET /v1/organizations/org-1/services/svc-1",
            "PUT /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ]
    );
}

/// The service `udf deploy` checks before it uploads anything.
async fn mount_deploy_service(server: &MockServer, service: &str, state: &str) {
    Mock::given(method("GET"))
        .and(path(format!("/v1/organizations/org-1/services/{service}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": {"state": state}})),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn udf_deploy_creates_or_versions_then_attaches_the_built_version() {
    for existing in [false, true] {
        let server = MockServer::start().await;
        let storage = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        let source = project_udf(project.path(), "my_udf", "python3.11");
        if existing {
            // Cloud no longer accepts sandboxVersion: deploy warns once and
            // omits it (the publish mocks below match the body exactly).
            let file = source.join("udf.json");
            let mut definition: Value =
                serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
            definition["sandboxVersion"] = serde_json::json!("v3");
            std::fs::write(&file, definition.to_string()).unwrap();
        }
        mount_udf_upload_session(&server, &storage).await;
        mount_deploy_service(&server, "svc-1", "running").await;
        let new_version = if existing { 2 } else { 1 };
        let udf_body = |version: i64, status: &str| {
            serde_json::json!({"result": {
                "functionName": "my_udf", "version": version, "status": status, "runtime": "python3.11"
            }})
        };

        // The first GET decides between create and version create; later
        // GETs follow the build of the new version.
        let first_get = if existing {
            ResponseTemplate::new(200).set_body_json(udf_body(1, "ready"))
        } else {
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "NOT_FOUND"}))
        };
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/udfs/my_udf"))
            .respond_with(first_get)
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/udfs/my_udf"))
            .respond_with(ResponseTemplate::new(200).set_body_json(udf_body(
                if existing {
                    new_version + 1
                } else {
                    new_version
                },
                "ready",
            )))
            .mount(&server)
            .await;
        if existing {
            // A concurrent publisher made GET point at version 3. The wait
            // must find the created version 2 on a subsequent list page.
            for (cursor, result, next) in [
                (
                    None,
                    serde_json::json!([{"functionName":"my_udf","version":3,"status":"ready"}]),
                    Some("page /+?"),
                ),
                (
                    Some("page /+?"),
                    serde_json::json!([{"functionName":"my_udf","version":2,"status":"ready"}]),
                    None,
                ),
            ] {
                let mock = Mock::given(method("GET"))
                    .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
                    .and(query_param("limit", "100"));
                let mock = if let Some(cursor) = cursor {
                    mock.and(query_param("cursor", cursor))
                } else {
                    mock.and(wiremock::matchers::query_param_is_missing("cursor"))
                };
                mock.respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result":result,"limit":100,"totalCount":null,"nextCursor":next
                })))
                .expect(1)
                .mount(&server)
                .await;
            }
        }
        let definition = serde_json::json!({
            "type": "executable", "runtime": "python3.11",
            "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64",
            "uploadId": "fresh-upload"
        });
        if existing {
            Mock::given(method("POST"))
                .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
                .and(body_json(definition))
                .respond_with(ResponseTemplate::new(201).set_body_json(udf_body(2, "building")))
                .expect(1)
                .mount(&server)
                .await;
        } else {
            let mut create = definition;
            create["functionName"] = serde_json::json!("my_udf");
            Mock::given(method("POST"))
                .and(path("/v1/organizations/org-1/udfs"))
                .and(body_json(create))
                .respond_with(ResponseTemplate::new(201).set_body_json(udf_body(1, "building")))
                .expect(1)
                .mount(&server)
                .await;
        }
        Mock::given(method("PUT"))
            .and(path("/v1/organizations/org-1/udfs/my_udf/attachments/svc-1"))
            .and(body_json(serde_json::json!({"version": new_version})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
                "functionName": "my_udf", "serviceId": "svc-1", "version": new_version, "status": "provisioning"
            }})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/udfs/my_udf/attachments/svc-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
                "functionName": "my_udf", "serviceId": "svc-1", "version": new_version, "status": "deployed"
            }})))
            .expect(1)
            .mount(&server)
            .await;

        let json = !existing;
        let output = udf_test_command(
            &server,
            project.path(),
            false,
            json,
            &["deploy", "my_udf", "--service", "svc-1", "--timeout", "30"],
        )
        .output()
        .unwrap();
        assert_success(&output);
        let warnings = String::from_utf8_lossy(&output.stderr)
            .lines()
            .filter(|line| line.starts_with("Warning:") && line.contains("sandboxVersion"))
            .count();
        assert_eq!(warnings, usize::from(existing));
        let stdout = String::from_utf8_lossy(&output.stdout);
        if json {
            let result: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(result["action"], "created");
            assert_eq!(result["udf"]["version"], 1);
            assert_eq!(result["udf"]["status"], "ready");
            assert_eq!(result["attachment"]["status"], "deployed");
            assert_eq!(result["attachment"]["version"], 1);
        } else {
            // One result line naming the UDF, version and service.
            assert_eq!(stdout.lines().count(), 1, "{stdout}");
            for identifier in ["my_udf", "2", "svc-1"] {
                assert!(stdout.contains(identifier), "{identifier}: {stdout}");
            }
            // Two waits: the build (carrying `--timeout 30`, not the shipped
            // default), then the deployment to the service.
            let stderr = String::from_utf8_lossy(&output.stderr);
            let waits: Vec<&str> = stderr
                .lines()
                .filter(|line| !line.starts_with("Warning: "))
                .collect();
            assert_eq!(waits.len(), 2, "{stderr}");
            assert!(
                waits[0].contains("my_udf") && waits[0].contains("30s"),
                "{stderr}"
            );
            assert!(!waits[0].contains("svc-1"), "{stderr}");
            assert!(waits[1].contains("svc-1"), "{stderr}");
        }
        let calls: Vec<String> = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| format!("{} {}", request.method, request.url.path()))
            .collect();
        let publish = if existing {
            "POST /v1/organizations/org-1/udfs/my_udf/versions"
        } else {
            "POST /v1/organizations/org-1/udfs"
        };
        let mut expected_calls = vec![
            "GET /v1/organizations/org-1/services/svc-1",
            "GET /v1/organizations/org-1/udfs/my_udf",
            "POST /v1/organizations/org-1/udfUploads/url",
            publish,
            "GET /v1/organizations/org-1/udfs/my_udf",
            "PUT /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
            "GET /v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ];
        if existing {
            expected_calls.splice(
                5..5,
                [
                    "GET /v1/organizations/org-1/udfs/my_udf/versions",
                    "GET /v1/organizations/org-1/udfs/my_udf/versions",
                ],
            );
        }
        assert_eq!(calls, expected_calls, "existing={existing}");
        assert_eq!(
            zip_entry_names(
                &storage
                    .received_requests()
                    .await
                    .unwrap()
                    .pop()
                    .unwrap()
                    .body
            ),
            ["lib/", "lib/helper.py", "main.py"]
        );
    }
}

#[tokio::test]
async fn udf_deploy_native_archives_only_the_architecture_directories() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "native_udf", "native");
    mount_udf_upload_session(&server, &storage).await;
    mount_deploy_service(&server, "svc-1", "running").await;
    let ready = serde_json::json!({"result": {"functionName": "native_udf", "version": 1, "status": "ready"}});
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/native_udf"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "NOT_FOUND"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/native_udf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ready.clone()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(ready))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/v1/organizations/org-1/udfs/native_udf/attachments/svc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
            "functionName": "native_udf", "serviceId": "svc-1", "version": 1, "status": "deployed"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/native_udf/attachments/svc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {
            "functionName": "native_udf", "serviceId": "svc-1", "version": 1, "status": "deployed"
        }})))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["deploy", "native_udf", "--service", "svc-1"],
    )
    .output()
    .unwrap();
    assert_success(&output);
    assert_eq!(
        zip_entry_names(
            &storage
                .received_requests()
                .await
                .unwrap()
                .pop()
                .unwrap()
                .body
        ),
        [
            "amd64/",
            "amd64/main",
            "amd64/model.bin",
            "arm64/",
            "arm64/main"
        ]
    );
}

#[tokio::test]
async fn udf_deploy_failures_after_publish_name_the_built_version() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "python3.11");
    mount_udf_upload_session(&server, &storage).await;
    mount_deploy_service(&server, "svc-1", "running").await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "NOT_FOUND"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let ready =
        serde_json::json!({"result": {"functionName": "my_udf", "version": 1, "status": "ready"}});
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ready.clone()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(ready))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(424).set_body_json(serde_json::json!({
            "error": "service is stopped", "code": "SERVICE_STOPPED", "serviceState": "stopped",
            "canWake": false, "status": 424
        })))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["deploy", "my_udf", "--service", "svc-1"],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("SERVICE_STOPPED"), "{message}");
    assert!(message.contains("cloud service start svc-1"), "{message}");
    assert!(
        message.contains("`clickhousectl cloud udf attach my_udf svc-1 --version 1`"),
        "{message}"
    );
}

/// A fresh `my_udf` that `udf deploy` creates as version 1, its build
/// reporting `status` on every poll.
async fn mount_deploy_new_udf(server: &MockServer, status: &str) {
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "NOT_FOUND"})),
        )
        .up_to_n_times(1)
        .mount(server)
        .await;
    let built = serde_json::json!({"result": {
        "functionName": "my_udf", "version": 1, "status": status
    }});
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(built.clone()))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(built))
        .expect(1)
        .mount(server)
        .await;
}

// The service idled during the build and --no-wake kept the attach from
// waking it: deploy has no --wake to rerun with, and rerunning would build
// again, so the way out is the attach of the built version, with --wake.
#[tokio::test]
async fn udf_deploy_idle_at_attach_resumes_with_a_waking_attach() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "python3.11");
    mount_udf_upload_session(&server, &storage).await;
    mount_deploy_service(&server, "svc-1", "running").await;
    mount_deploy_new_udf(&server, "ready").await;
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(424).set_body_json(serde_json::json!({
            "error": "service is idle", "code": "SERVICE_IDLE", "serviceState": "idle",
            "canWake": true, "status": 424
        })))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["deploy", "my_udf", "--service", "svc-1", "--no-wake"],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "service_idle", "{error}");
    assert_eq!(
        error["error"]["command"], "clickhousectl cloud udf attach my_udf svc-1 --version 1 --wake",
        "{error}"
    );
    let message = error["error"]["message"].as_str().unwrap();
    assert!(!message.contains("cloud service wake"), "{message}");
    assert!(
        message.contains("`clickhousectl cloud udf attach my_udf svc-1 --version 1 --wake`"),
        "{message}"
    );
    assert!(
        !deploy_calls(&server)
            .await
            .iter()
            .any(|call| call.starts_with("PATCH ")),
        "--no-wake must not wake the service"
    );
}

// A build still running when the wait gives up can finish later; the error
// names the attach that resumes the deploy once it does.
// A build that reported an error has nothing to resume.
#[tokio::test]
async fn udf_deploy_failed_build_offers_no_resume() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "python3.11");
    mount_udf_upload_session(&server, &storage).await;
    mount_deploy_service(&server, "svc-1", "running").await;
    mount_deploy_new_udf(&server, "error").await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["deploy", "my_udf", "--service", "svc-1"],
    )
    .output()
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("failed"), "{message}");
    assert!(!message.contains("cloud udf attach"), "{message}");
    assert!(
        !deploy_calls(&server)
            .await
            .iter()
            .any(|call| call.starts_with("PUT ")),
        "nothing is attached before the build is ready"
    );
}

/// Paths `udf deploy` requested, in order.
async fn deploy_calls(server: &MockServer) -> Vec<String> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| format!("{} {}", request.method, request.url.path()))
        .collect()
}

// The service is checked before anything is uploaded: a bad ID must not
// leave an orphan UDF version behind.
#[tokio::test]
async fn udf_deploy_rejects_an_unknown_service_before_publishing() {
    let org = "00000000-0000-4000-8000-000000000001";
    let service = "11111111-2222-3333-4444-555555555555";
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "python3.11");
    Mock::given(method("GET"))
        .and(path(format!("/v1/organizations/{org}/services/{service}")))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400, "error": "not a valid UUID"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let home = project.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut cmd = Command::new(clickhousectl_binary());
    cmd.isolated_env();
    let output = cmd
        .env("HOME", home)
        .env("DO_NOT_TRACK", "1")
        .current_dir(project.path())
        .args(["cloud", "--url", &server.uri()])
        .args([
            "--api-key",
            "udf-key",
            "--api-secret",
            "udf-secret",
            "--json",
        ])
        .args([
            "udf",
            "--org-id",
            org,
            "deploy",
            "my_udf",
            "--service",
            service,
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "resource_not_found", "{error}");
    assert_eq!(
        deploy_calls(&server).await,
        [format!("GET /v1/organizations/{org}/services/{service}")]
    );
}

// `--no-wake` on an idle service, or a service in a state a wake cannot
// fix, fails before the upload with the command that unblocks it.
#[tokio::test]
async fn udf_deploy_rejects_an_unusable_service_before_publishing() {
    for (state, no_wake, code, command) in [
        (
            "idle",
            true,
            "service_idle",
            Some("clickhousectl cloud service wake svc-1"),
        ),
        (
            "stopped",
            false,
            "service_stopped",
            Some("clickhousectl cloud service start svc-1"),
        ),
        (
            "stopped",
            true,
            "service_stopped",
            Some("clickhousectl cloud service start svc-1"),
        ),
        ("terminated", false, "service_not_running", None),
    ] {
        let server = MockServer::start().await;
        let project = tempfile::tempdir().unwrap();
        project_udf(project.path(), "my_udf", "python3.11");
        mount_deploy_service(&server, "svc-1", state).await;
        let mut args = vec!["deploy", "my_udf", "--service", "svc-1"];
        if no_wake {
            args.push("--no-wake");
        }
        let output = udf_test_command(&server, project.path(), false, true, &args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{state}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], code, "{error}");
        assert_eq!(error["error"]["command"].as_str(), command, "{error}");
        assert_eq!(
            deploy_calls(&server).await,
            ["GET /v1/organizations/org-1/services/svc-1"],
            "{state}"
        );
    }
}

// Without --no-wake an idle service is not woken up front: the build takes
// minutes and the service could idle again, so the attach wakes it.
#[tokio::test]
async fn udf_deploy_proceeds_on_an_idle_service_without_waking_it_early() {
    let server = MockServer::start().await;
    let storage = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    project_udf(project.path(), "my_udf", "python3.11");
    mount_udf_upload_session(&server, &storage).await;
    mount_deploy_service(&server, "svc-1", "idle").await;
    let ready =
        serde_json::json!({"result": {"functionName": "my_udf", "version": 1, "status": "ready"}});
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(serde_json::json!({"error": "NOT_FOUND"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ready.clone()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(ready))
        .expect(1)
        .mount(&server)
        .await;
    let attached = serde_json::json!({"result": {
        "functionName": "my_udf", "serviceId": "svc-1", "version": 1, "status": "deployed"
    }});
    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(attached.clone()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(attached))
        .expect(1)
        .mount(&server)
        .await;
    let output = udf_test_command(
        &server,
        project.path(),
        false,
        true,
        &["deploy", "my_udf", "--service", "svc-1"],
    )
    .output()
    .unwrap();
    assert_success(&output);
    let calls = deploy_calls(&server).await;
    assert_eq!(calls[0], "GET /v1/organizations/org-1/services/svc-1");
    assert!(
        !calls.iter().any(|call| call.starts_with("PATCH ")),
        "{calls:?}"
    );
}
