//! `cloud key` (src/cloud/api_keys.rs).

use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

// ── Generated API key material is never silently dropped ───────────────────
//
// `key create` without the pre-hash flags asks the API to generate a key pair
// returned exactly once. A response missing `keyId`/`keySecret` loses that
// credential, so validation runs before either output branch — `--json` (which
// agent detection also turns on by itself) must not print the incomplete
// response and exit zero.

async fn run_key_create(result: Value, extra_args: &[&str]) -> std::process::Output {
    run_key_create_with_requests(result, extra_args).await.0
}

async fn run_key_create_with_requests(
    result: Value,
    extra_args: &[&str],
) -> (std::process::Output, Vec<Value>) {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-key-create",
        })))
        .mount(&mock)
        .await;

    let url = mock.uri();
    let mut args: Vec<&str> = vec![
        "cloud", "--url", &url, "key", "create", "--name", "ci", "--org-id", "org-1",
    ];
    args.extend(extra_args);

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .args(&args)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .output()
        .expect("failed to spawn clickhousectl");
    let bodies = mock
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| request.body_json().unwrap())
        .collect();
    (output, bodies)
}

#[tokio::test]
async fn key_create_fails_when_the_generated_material_is_absent() {
    for extra_args in [&[][..], &["--json"][..]] {
        let output = run_key_create(
            serde_json::json!({
                "key": { "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "name": "ci" },
                "keyId": "generated-key-id",
            }),
            extra_args,
        )
        .await;
        assert_eq!(
            output.status.code(),
            Some(1),
            "a generated key create with no secret must fail for args {extra_args:?}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout),
        );
        // The wording is unit-tested in `api_keys`. Neither the human
        // confirmation nor the incomplete response body may reach stdout
        // ahead of the failure.
        assert!(!output.stderr.is_empty(), "{extra_args:?}");
        assert!(
            output.stdout.is_empty(),
            "no success output may precede the failure for args {extra_args:?}:\n{}",
            String::from_utf8_lossy(&output.stdout),
        );
    }
}

#[tokio::test]
async fn key_create_json_prints_the_raw_response() {
    let result = serde_json::json!({
        "key": { "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "name": "ci" },
        "keyId": "generated-key-id",
        "keySecret": "generated-key-secret",
    });
    let output = run_key_create(result.clone(), &["--json"]).await;
    assert_success(&output);
    let body: Value =
        serde_json::from_slice(&output.stdout).expect("--json output wasn't valid JSON");
    // --json reflects the key set the API sent; nothing is synthesized from
    // the resolved material.
    assert_eq!(body, result);
}

#[tokio::test]
async fn key_create_human_output_shows_the_secret_and_warns_about_deny_all() {
    let output = run_key_create(
        serde_json::json!({
            "key": { "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "name": "ci" },
            "keyId": "generated-key-id",
            "keySecret": "generated-key-secret",
        }),
        &[],
    )
    .await;
    // Which label each value sits under is unit-tested on `render_key_created`
    // in `api_keys`; here the confirmation must reach stdout with the secret.
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("generated-key-secret"), "{stdout}");
    // No --ip-allow: one warning naming the flag and the command that adds one.
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("Warning: "), "{stderr}");
    assert!(
        stderr.contains("`cloud key update <resource-id> --ip-allow <IP_OR_CIDR>`"),
        "{stderr}"
    );
}

#[tokio::test]
async fn key_create_succeeds_for_a_pre_hashed_key_without_generated_material() {
    // The response has no key object either, so the absent name and resource
    // ID render as placeholders; an --ip-allow entry suppresses the warning.
    let (output, bodies) = run_key_create_with_requests(
        serde_json::json!({}),
        &[
            "--hash-key-id",
            "0f1e2d3c",
            "--hash-key-id-suffix",
            "3c",
            "--hash-key-secret",
            "4b5a6978",
            "--ip-allow",
            "198.51.100.4/32",
        ],
    )
    .await;
    // Placeholders for the absent name and ID, and no key material at all.
    assert_success(&output);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.lines().filter(|line| line.ends_with(": -")).count(),
        2,
        "{stdout}"
    );
    assert_eq!(stdout.lines().count(), 4, "{stdout}");
    assert!(!stdout.contains("4b5a6978"), "{stdout}");
    assert!(output.stderr.is_empty());
    assert_eq!(
        bodies,
        [serde_json::json!({
            "name": "ci",
            "state": "enabled",
            "assignedRoleIds": [],
            "ipAccessList": [{"source": "198.51.100.4/32"}],
            "hashData": {"keyIdHash": "0f1e2d3c", "keyIdSuffix": "3c", "keySecretHash": "4b5a6978"},
        })]
    );
}

// IP allowlist descriptions (#589).

#[tokio::test]
async fn api_key_create_passes_every_flag_and_allowlist_description() {
    let create = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .and(body_json(serde_json::json!({
            "name": "described-key",
            "state": "disabled",
            "assignedRoleIds": ["11111111-2222-3333-4444-555555555555"],
            "expireAt": "2030-01-01T00:00:00Z",
            "ipAccessList": [
                {"source": "198.51.100.4"},
                {"source": "2001:db8::/32", "description": "production"},
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "key": {"id": "11111111-2222-3333-4444-555555555555", "name": "described-key"},
                "keyId": "generated-key-id",
                "keySecret": "generated-key-secret"
            },
            "status": 200,
            "requestId": "stub-key-create"
        })))
        .expect(1)
        .mount(&create)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &create,
        &[
            "key",
            "create",
            "--name",
            "described-key",
            "--role-id",
            "11111111-2222-3333-4444-555555555555",
            "--expires-at",
            "2030-01-01T00:00:00Z",
            "--state",
            "disabled",
            "--ip-allow",
            "198.51.100.4",
            "--ip-allow",
            "2001:db8::/32=production",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&output);
}

// API key nullable expiry (#698) and explicit list clearing (#597).
async fn invoke_key_update(server: &MockServer, flags: &[&str]) -> std::process::Output {
    let project = tempfile::tempdir().unwrap();
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "expiry-test-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "expiry-test-secret")
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &server.uri(),
            "--json",
            "key",
            "update",
            "key-1",
            "--org-id",
            "org-1",
        ])
        .args(flags)
        .stdin(Stdio::null())
        .output()
        .expect("failed to run key update")
}

#[tokio::test]
async fn key_update_passes_every_flag_to_the_exact_body() {
    // Two runs pass every update flag through the dispatcher; the other
    // combinations are the `build_api_key_update_request_*` unit tests.
    let cases: &[(&[&str], Value)] = &[
        (
            &[
                "--clear-roles",
                "--clear-ip-allow",
                "--expires-at",
                "2030-01-01T00:00:00Z",
            ],
            serde_json::json!({
                "assignedRoleIds": [], "ipAccessList": [], "expireAt": "2030-01-01T00:00:00Z",
            }),
        ),
        (
            &[
                "--clear-expiry",
                "--new-name",
                "renamed",
                "--role-id",
                "11111111-2222-3333-4444-555555555555",
                "--state",
                "disabled",
                "--ip-allow",
                "10.0.0.0/8",
            ],
            serde_json::json!({
                "expireAt": null, "name": "renamed", "state": "disabled",
                "assignedRoleIds": ["11111111-2222-3333-4444-555555555555"],
                "ipAccessList": [{"source": "10.0.0.0/8"}],
            }),
        ),
    ];
    for (flags, expected) in cases {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/v1/organizations/org-1/keys/key-1"))
            .and(wiremock::matchers::basic_auth(
                "expiry-test-key",
                "expiry-test-secret",
            ))
            .and(body_json(expected.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"name": "updated", "expireAt": null}, "status": 200,
            })))
            .expect(1)
            .mount(&server)
            .await;
        let output = invoke_key_update(&server, flags).await;
        assert_success(&output);
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result, serde_json::json!({"name": "updated"}));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

// API-key pagination: explicit pages, all-page listing, and exhaustive name lookup.
const API_KEYS_PATH: &str = "/v1/organizations/org-1/keys";
const KEY_PAGE_CURSOR: &str = "opaque +/&?=雪";
const PAGED_KEY_ID: &str = "11111111-2222-3333-4444-555555555555";
const OTHER_PAGED_KEY_ID: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

async fn mount_key_page(mock: &MockServer, cursor: Option<&str>, status: u16, body: Value) {
    let matcher = Mock::given(method("GET")).and(path(API_KEYS_PATH));
    let matcher = match cursor {
        Some(cursor) => matcher.and(query_param("cursor", cursor)),
        None => matcher.and(wiremock::matchers::query_param_is_missing("cursor")),
    };
    matcher
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .expect(1)
        .mount(mock)
        .await;
}

async fn assert_key_page_requests(mock: &MockServer, cursors: &[Option<&str>]) {
    let requests = mock.received_requests().await.unwrap();
    let pages: Vec<_> = requests
        .iter()
        .filter(|request| request.url.path() == API_KEYS_PATH)
        .collect();
    assert_eq!(pages.len(), cursors.len());
    for (request, cursor) in pages.iter().zip(cursors) {
        assert_eq!(request.method, "GET");
        assert!(request.body.is_empty());
        let expected: Vec<_> = cursor
            .iter()
            .map(|cursor| ("cursor".to_owned(), (*cursor).to_owned()))
            .collect();
        assert_eq!(
            request.url.query_pairs().into_owned().collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            request.headers["authorization"].to_str().unwrap(),
            "Basic ZmFrZS1rZXktZm9yLXRlc3RzOmZha2Utc2VjcmV0LWZvci10ZXN0cw=="
        );
    }
}

#[tokio::test]
async fn key_pagination_list_collects_all_pages_in_json_and_human_output() {
    for human in [false, true] {
        let mock = MockServer::start().await;
        let first = serde_json::json!({"id": OTHER_PAGED_KEY_ID, "name": "first-key"});
        let last = serde_json::json!({"id": PAGED_KEY_ID, "name": "last-key"});
        mount_key_page(
            &mock,
            None,
            200,
            serde_json::json!({
                "result": [first], "nextCursor": KEY_PAGE_CURSOR, "limit": 1, "totalCount": 2
            }),
        )
        .await;
        // An empty page is not a terminator when the server supplies another cursor.
        mount_key_page(
            &mock,
            Some(KEY_PAGE_CURSOR),
            200,
            serde_json::json!({
                "result": [], "nextCursor": "last-page"
            }),
        )
        .await;
        mount_key_page(
            &mock,
            Some("last-page"),
            200,
            serde_json::json!({
                "result": [last], "nextCursor": null
            }),
        )
        .await;
        let args = ["key", "list", "--all", "--org-id", "org-1"];
        let output = if human {
            invoke_cli_with_cloud_credentials_human(&mock, &args)
        } else {
            invoke_cli_with_cloud_credentials(&mock, &args)
        };
        assert_success(&output);
        if human {
            let stdout = String::from_utf8(output.stdout).unwrap();
            assert!(stdout.contains("| first-key"), "{stdout}");
            assert!(stdout.contains("| last-key"), "{stdout}");
        } else {
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                serde_json::json!([first, last])
            );
        }
        assert_key_page_requests(&mock, &[None, Some(KEY_PAGE_CURSOR), Some("last-page")]).await;
        assert_eq!(mock.received_requests().await.unwrap().len(), 3);
    }
}

#[tokio::test]
async fn key_pagination_omits_a_null_cursor_from_the_page() {
    // An absent cursor is library serde; a null one must not be printed.
    let mock = MockServer::start().await;
    let result = serde_json::json!([{ "id": PAGED_KEY_ID, "name": "only-key" }]);
    mount_key_page(
        &mock,
        None,
        200,
        serde_json::json!({"result": result, "nextCursor": null}),
    )
    .await;
    let output = invoke_cli_with_cloud_credentials(&mock, &["key", "list", "--org-id", "org-1"]);
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({"result": result})
    );
    assert_key_page_requests(&mock, &[None]).await;
}

#[tokio::test]
async fn key_pagination_name_lookup_finds_later_match_and_rejects_cross_page_ambiguity() {
    // Each command resolves the name across pages; a cross-page duplicate
    // stops one of them before any write.
    for (command, http_method, ambiguous) in [
        ("get", "GET", false),
        ("get", "GET", true),
        ("update", "PATCH", false),
        ("delete", "DELETE", false),
    ] {
        let mock = MockServer::start().await;
        mount_key_page(&mock, None, 200, serde_json::json!({
        "result": [{"id": OTHER_PAGED_KEY_ID, "name": if ambiguous { "target" } else { "other" }}],
        "nextCursor": KEY_PAGE_CURSOR
        })).await;
        let target = serde_json::json!({"id": PAGED_KEY_ID, "name": "target"});
        mount_key_page(
            &mock,
            Some(KEY_PAGE_CURSOR),
            200,
            serde_json::json!({"result": [target]}),
        )
        .await;
        Mock::given(method(http_method))
            .and(path(format!("{API_KEYS_PATH}/{PAGED_KEY_ID}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": target})),
            )
            .expect(if ambiguous { 0 } else { 1 })
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &["key", command, "--name", "target", "--org-id", "org-1"],
        );
        if ambiguous {
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let error = cloud_runtime_error(&output);
            // Names the ambiguous name and the number of matches.
            let message = error["message"].as_str().unwrap();
            assert!(message.contains("'target'"), "{message}");
            assert_eq!(digits(message), "2", "{message}");
        } else {
            assert_success(&output);
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                if command == "delete" {
                    serde_json::json!({})
                } else {
                    target
                }
            );
        }
        assert_key_page_requests(&mock, &[None, Some(KEY_PAGE_CURSOR)]).await;
        assert_eq!(
            mock.received_requests().await.unwrap().len(),
            if ambiguous { 2 } else { 3 }
        );
    }
}

#[tokio::test]
async fn key_pagination_later_page_errors_prevent_partial_output_and_name_resolution() {
    // The listing and the name lookup each stop at a failed later page; the
    // status → exit code mapping is the `dispatched_cloud_*` tests'.
    for (lookup, status, exit) in [(false, 400, 1), (true, 401, 4)] {
        let mock = MockServer::start().await;
        mount_key_page(
            &mock,
            None,
            200,
            serde_json::json!({
                "result": [{"id": PAGED_KEY_ID, "name": "target"}], "nextCursor": KEY_PAGE_CURSOR
            }),
        )
        .await;
        mount_key_page(
            &mock,
            Some(KEY_PAGE_CURSOR),
            status,
            serde_json::json!({"error": "later page denied"}),
        )
        .await;
        let args = if lookup {
            vec!["key", "get", "--name", "target", "--org-id", "org-1"]
        } else {
            vec!["key", "list", "--all", "--org-id", "org-1"]
        };
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_eq!(output.status.code(), Some(exit), "{output:?}");
        assert!(output.stdout.is_empty());
        let error = cloud_runtime_error(&output);
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("later page denied")
        );
        assert_key_page_requests(&mock, &[None, Some(KEY_PAGE_CURSOR)]).await;
        assert_eq!(mock.received_requests().await.unwrap().len(), 2);
    }
}

#[tokio::test]
async fn key_pagination_default_is_one_page_with_metadata() {
    let mock = MockServer::start().await;
    let body = serde_json::json!({
        "result": [{"id": PAGED_KEY_ID, "name": "first"}],
        "limit": 250, "totalCount": 501, "nextCursor": KEY_PAGE_CURSOR
    });
    mount_key_page(&mock, None, 200, body.clone()).await;
    let output = invoke_cli_with_cloud_credentials(&mock, &["key", "list", "--org-id", "org-1"]);
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        body
    );
    assert_key_page_requests(&mock, &[None]).await;
}

#[tokio::test]
async fn key_pagination_explicit_page_forwards_limit_and_opaque_cursor() {
    // An empty cursor is passed through the same way (`collect_cursor_pages`
    // unit tests cover it as a page token).
    let cursor = "-opaque +/=&?'雪";
    let mock = MockServer::start().await;
    mount_key_page(
        &mock,
        Some(cursor),
        200,
        serde_json::json!({
            "result": [], "limit": 25, "totalCount": 0
        }),
    )
    .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "key", "list", "--org-id", "org-1", "--limit", "25", "--cursor", cursor,
        ],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({
            "result": [], "limit": 25, "totalCount": 0
        })
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url.query_pairs().collect::<Vec<_>>(),
        vec![
            ("limit".into(), "25".into()),
            ("cursor".into(), cursor.into())
        ]
    );
}

#[tokio::test]
async fn key_pagination_all_sends_limit_on_every_page() {
    let mock = MockServer::start().await;
    mount_key_page(
        &mock,
        None,
        200,
        serde_json::json!({"result": [], "nextCursor": ""}),
    )
    .await;
    mount_key_page(&mock, Some(""), 200, serde_json::json!({"result": []})).await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["key", "list", "--org-id", "org-1", "--all", "--limit", "1"],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!([])
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests {
        assert!(
            request
                .url
                .query_pairs()
                .any(|(key, value)| key == "limit" && value == "1")
        );
    }
}

#[tokio::test]
async fn key_pagination_human_continuation_preserves_shell_tokens_and_page_context() {
    // Quoting each kind of token is
    // `api_keys::quote_shell_argument_for_display_round_trips_or_refuses_control_characters`;
    // here one quoted and one refused cursor reach the printed hint.
    for cursor in ["-opaque +/=&?'雪", "\n\u{1b}[31m"] {
        let mock = MockServer::start().await;
        mount_key_page(
            &mock,
            None,
            200,
            serde_json::json!({"result": [], "nextCursor": cursor}),
        )
        .await;
        let output = invoke_cli_with_cloud_credentials_human(
            &mock,
            &["key", "list", "--org-id", "org-1", "--limit", "25"],
        );
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        if cursor.chars().any(char::is_control) {
            assert!(!stdout.contains(cursor), "{stdout}");
            assert!(stdout.contains("--json"), "{stdout}");
            assert!(!stdout.contains('\u{1b}'), "{stdout}");
        } else {
            let (_, flags) = stdout.split_once("with --org-id=").unwrap();
            let (flags, _) = flags.split_once(" to continue").unwrap();
            let roundtrip = Command::new("sh")
                .args(["-c", &format!("printf '%s\\n' --org-id={flags}")])
                .output()
                .unwrap();
            assert!(roundtrip.status.success());
            assert_eq!(
                String::from_utf8(roundtrip.stdout).unwrap(),
                format!("--org-id=org-1\n--limit=25\n--cursor={cursor}\n")
            );
        }
        assert_eq!(mock.received_requests().await.unwrap().len(), 1);
    }
}
