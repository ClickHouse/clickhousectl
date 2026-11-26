//! `cloud access`, the experimental access-negotiation flow (src/cloud/access.rs). Compiled only
//! with `--features experimental-access-negotiation`.
#![allow(clippy::too_many_lines)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;
// ── Access negotiation's OAuth-only identity lane ──────────────────────────

fn access_roles_command(dir: &Path, json: bool) -> Command {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.join("home"))
        .current_dir(dir)
        .args(["cloud", "access", "roles", "--policies"]);
    if json {
        command.arg("--json");
    }
    command
}

fn assignable_roles_fixture() -> Value {
    serde_json::json!([
        {"id": "00000000-0000-0000-0000-000000000001", "name": "System reader", "type": "system", "policies": [
            {"allowDeny": "ALLOW", "permissions": ["service:view"], "resources": ["*"], "tags": {"roleV2": "sql-console-readonly"}},
            {"allowDeny": "DENY", "permissions": ["service:delete"], "resources": ["instance/private"]}
        ]},
        {"id": "55555555-5555-4555-8555-555555555555", "name": "Custom SQL", "type": "custom", "policies": [
            {"allowDeny": "ALLOW", "permissions": ["sql-console:database:access"], "resources": ["instance/service-a"], "tags": {"roleV2": "sql-console-admin", "grants": ["GRANT SELECT ON db.* TO role", "REVOKE SELECT ON db.secret FROM role"]}},
            {"allowDeny": "future-effect", "permissions": [], "resources": [], "tags": {"roleV2": "future-role", "grants": []}}
        ]},
        {"id": "22222222-2222-2222-2222-222222222222", "name": "Future type", "type": "future-type", "policies": []}
    ])
}

#[tokio::test]
async fn access_roles_outputs_complete_oauth_menu_with_selected_organization() {
    const ORG: &str = "11111111-2222-3333-4444-555555555555";
    for selector in ["explicit", "auto", "name"] {
        for json in [false, true] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/v1/organizations"))
                .and(header("authorization", "Bearer test-bearer-token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"result": [{"id": ORG, "name": "Selected org"}]}),
                ))
                .expect(if selector == "explicit" { 0 } else { 1 })
                .mount(&server)
                .await;
            let roles = assignable_roles_fixture();
            Mock::given(method("GET")).and(path(format!("/v1/organizations/{ORG}/roles")))
                .and(header("authorization", "Bearer test-bearer-token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": roles, "requestId": "correlation", "nextCursor": "must-not-page"})))
                .expect(1).mount(&server).await;
            let dir = tempfile::tempdir().unwrap();
            let tokens = access_credentials(dir.path());
            write_oauth_tokens(&tokens, &server.uri());
            let mut command = access_roles_command(dir.path(), json);
            let debug = selector != "explicit";
            if debug {
                command.arg("--debug");
            }
            if selector == "explicit" {
                command.args(["--org", ORG]);
            }
            if selector == "name" {
                command.args(["--org-name", "Selected org"]);
            }
            let output = command.output().unwrap();
            assert_eq!(
                output.status.code(),
                Some(0),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_access_output_has_no_credentials(&output);
            if debug {
                assert_access_debug_diagnostics(&output, &server.uri());
            } else {
                assert!(!String::from_utf8_lossy(&output.stderr).contains("[debug]"));
            }
            if json {
                assert_eq!(
                    serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                    roles
                );
            } else {
                let text = String::from_utf8_lossy(&output.stdout);
                for value in [
                    ORG,
                    "System reader",
                    "Custom SQL",
                    "DENY",
                    "instance/private",
                    "Unknown effect (future-effect)",
                    "Unknown (future-type)",
                    "00000000-0000-0000-0000-000000000001",
                ] {
                    assert!(text.contains(value), "missing {value}: {text}");
                }
            }
            assert_eq!(
                server.received_requests().await.unwrap().len(),
                if selector == "explicit" { 1 } else { 2 }
            );
        }
    }
}

/// Without `--policies`, JSON is the compact summary: one role per line,
/// fewest writes first.
#[tokio::test]
async fn access_roles_defaults_to_the_compact_summary() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/roles"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"result": assignable_roles_fixture()})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    write_oauth_tokens(&access_credentials(dir.path()), &server.uri());
    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .current_dir(dir.path())
        .args(["cloud", "access", "roles", "--json", "--org", "org"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert_eq!(text.lines().count(), 5, "{text}");
    let roles: Value = serde_json::from_str(&text).unwrap();
    let names: Vec<_> = roles
        .as_array()
        .unwrap()
        .iter()
        .map(|role| role["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Future type", "System reader", "Custom SQL"]);
    assert_eq!(
        roles[1]["denied"],
        serde_json::json!({"service": ["delete"]})
    );
    assert_eq!(
        roles[2]["write"],
        serde_json::json!({"sql-console": ["database:access (sql-console-admin)"]})
    );
}

#[tokio::test]
async fn access_roles_empty_and_optional_tags_are_valid() {
    for roles in [
        serde_json::json!([]),
        serde_json::json!([
            {"id": "00000000-0000-0000-0000-000000000001", "name": "empty", "type": "system", "policies": [
                {"allowDeny": "DENY", "permissions": [], "resources": [], "tags": null},
                {"allowDeny": "ALLOW", "permissions": [], "resources": [], "tags": {"grants": null, "roleV2": null}}
            ]}
        ]),
    ] {
        for json in [false, true] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/v1/organizations/org/roles"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": roles})),
                )
                .expect(1)
                .mount(&server)
                .await;
            let dir = tempfile::tempdir().unwrap();
            write_oauth_tokens(&access_credentials(dir.path()), &server.uri());
            let output = access_roles_command(dir.path(), json)
                .args(["--org", "org"])
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(0),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            if json {
                let expected: Vec<clickhouse_cloud_api::models::AssignableRole> =
                    serde_json::from_value(roles.clone()).unwrap();
                assert_eq!(
                    serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                    serde_json::to_value(expected).unwrap()
                );
            } else if roles.as_array().unwrap().is_empty() {
                assert!(String::from_utf8_lossy(&output.stdout).contains("No assignable roles"));
            }
        }
    }
}

#[tokio::test]
async fn access_roles_rejects_incomplete_menus_before_any_success_output() {
    let fixture = assignable_roles_fixture();
    let mut invalid = vec![
        serde_json::json!({}),
        serde_json::json!({"result": null}),
        serde_json::json!({"result": fixture, "error": "envelope failure"}),
        serde_json::json!({"result": fixture, "status": 500}),
        serde_json::json!({"result": {"roles": []}}),
    ];
    for field in ["id", "name", "type", "policies"] {
        for null in [false, true] {
            let mut roles = fixture.clone();
            if null {
                roles[1][field] = Value::Null;
            } else {
                roles[1].as_object_mut().unwrap().remove(field);
            }
            invalid.push(serde_json::json!({"result": roles}));
        }
    }
    for field in ["allowDeny", "permissions", "resources"] {
        for null in [false, true] {
            let mut roles = fixture.clone();
            if null {
                roles[1]["policies"][0][field] = Value::Null;
            } else {
                roles[1]["policies"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
            }
            invalid.push(serde_json::json!({"result": roles}));
        }
    }
    let mut roles = fixture.clone();
    roles[1]["id"] = "not-a-uuid".into();
    invalid.push(serde_json::json!({"result": roles}));
    for response in invalid {
        for json in [false, true] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/v1/organizations/org/roles"))
                .and(header("authorization", "Bearer test-bearer-token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(response.clone()))
                .expect(1)
                .mount(&server)
                .await;
            let dir = tempfile::tempdir().unwrap();
            write_oauth_tokens(&access_credentials(dir.path()), &server.uri());
            let output = access_roles_command(dir.path(), json)
                .args(["--org", "org"])
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(1),
                "{response}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                output.stdout.is_empty(),
                "partial success: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            assert!(!output.stderr.is_empty());
            assert_access_output_has_no_credentials(&output);
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
        }
    }
}

#[tokio::test]
async fn access_roles_preserves_server_denials_without_fallback() {
    for (status, message, exit) in [
        (401, "non-CLI identity", 4),
        // A 403 is not an auth failure (#1225).
        (403, "cross-org denial", 1),
        (400, "RBAC prerequisite", 1),
        (500, "upstream error", 1),
    ] {
        for json in [false, true] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/v1/organizations/other-org/roles"))
                .and(header("authorization", "Bearer test-bearer-token"))
                .respond_with(
                    ResponseTemplate::new(status)
                        .set_body_json(serde_json::json!({"error": message})),
                )
                .expect(1)
                .mount(&server)
                .await;
            let dir = tempfile::tempdir().unwrap();
            write_oauth_tokens(&access_credentials(dir.path()), &server.uri());
            let output = access_roles_command(dir.path(), json)
                .args(["--org", "other-org"])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(exit));
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains(message));
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
        }
    }
}

#[tokio::test]
async fn access_roles_requires_and_refreshes_oauth_despite_basic_credentials() {
    for state in ["missing", "unrefreshable", "refresh"] {
        let issuer = MockServer::start().await;
        let target = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let tokens = access_credentials(dir.path());
        if state != "missing" {
            std::fs::write(tokens.join("tokens.json"), serde_json::to_vec(&serde_json::json!({
                "access_token": "test-bearer-token", "refresh_token": "refresh-secret", "expires_at": 1,
                "api_url": format!("{}/v1", issuer.uri()),
                "auth_config": {"auth_url": issuer.uri(), "client_id": "test-client"}
            })).unwrap()).unwrap();
            Mock::given(method("POST")).and(path("/oauth/token"))
                .respond_with(if state == "refresh" {
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token": "refreshed-bearer-token", "expires_in": 3600, "token_type": "Bearer"}))
                } else { ResponseTemplate::new(401).set_body_string("refresh-secret") })
                .expect(1).mount(&issuer).await;
        }
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org/roles"))
            .and(header("authorization", "Bearer refreshed-bearer-token"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": []})),
            )
            .expect(if state == "refresh" { 1 } else { 0 })
            .mount(&target)
            .await;
        let output = access_roles_command(dir.path(), true)
            .args([
                "--org",
                "org",
                "--url",
                &target.uri(),
                "--api-key",
                "flag-basic-key",
                "--api-secret",
                "flag-basic-secret",
            ])
            .env("CLICKHOUSECTL_AUTH_URL", issuer.uri())
            .env("CLICKHOUSECTL_CLIENT_ID", "test-client")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(if state == "refresh" { 0 } else { 4 })
        );
        assert_access_output_has_no_credentials(&output);
        if state == "refresh" {
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                serde_json::json!([])
            );
        } else {
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("cloud auth login"));
        }
        assert_eq!(
            target.received_requests().await.unwrap().len(),
            usize::from(state == "refresh")
        );
    }
}

#[test]
fn access_roles_selector_conflicts_exit_usage_before_oauth() {
    for args in [
        vec!["cloud", "--org-id=one", "access", "roles", "--org=two"],
        vec![
            "cloud",
            "--org-name=one",
            "access",
            "roles",
            "--org-name=two",
        ],
        vec!["cloud", "access", "--org=one", "roles", "--org-id=two"],
        vec!["cloud", "--org-name=one", "access", "roles", "--org=one"],
    ] {
        let dir = tempfile::tempdir().unwrap();
        access_credentials(dir.path());
        let output = Command::new(clickhousectl_binary())
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", dir.path().join("home"))
            .current_dir(dir.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[tokio::test]
async fn access_request_authorization_code_handoff_keeps_stdout_final_and_credentials_safe() {
    use base64::Engine as _;
    use sha2::{Digest as _, Sha256};
    const A: &str = "21111111-1111-1111-9999-000000000007";
    const B: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    const C: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    const FILE_PATH: &str = "/v1/organizations/org/accessRequests/authorizationCode";
    const TOKEN_PATH: &str = "/v1/organizations/org/accessRequests/authorizationCode/token";
    const STATUS_PATH: &str = "/v1/organizations/org/accessRequests/request-1";
    let mut tasks = Vec::new();
    for json in [false, true] {
        for scenario in [
            "success",
            "success-proposed",
            "success-no-expiry",
            "tampered-state",
            "callback-no-code",
            "callback-bad-code",
            "callback-denied",
            "create-401",
            "create-403",
            "server-expiry",
            "pending-expired",
            "timeout-env",
            "timeout-flag",
            "exchange-replay",
            "exchange-redirect-mismatch",
            "exchange-401",
            "exchange-403",
            "exchange-echo",
            "exchange-malformed",
            "exchange-empty",
            "exchange-expiry-invalid",
            "exchange-301",
            "exchange-302",
            "exchange-303",
            "exchange-307",
            "exchange-308",
            "create-deadline-missing",
            "create-envelope-error",
            "create-proposal-missing",
            "create-proposal-null",
            "create-proposal-malformed",
            "create-proposal-duplicate",
            "create-proposal-oversized",
            "create-proposal-mismatch",
            "create-proposal-unexpected",
            "grants-missing",
            "grants-null",
            "grants-empty",
            "grants-malformed",
            "grants-duplicate",
            "grants-many",
        ] {
            tasks.push(tokio::spawn(async move {
                let server = MockServer::start().await;
                let dir = tempfile::tempdir().unwrap();
                let tokens = access_credentials(dir.path());
                write_oauth_tokens(&tokens, &server.uri());
                let credentials_path = dir.path().join(".clickhouse/credentials.json");
                let before = std::fs::read(&credentials_path).unwrap();
                let oauth_before = std::fs::read(tokens.join("tokens.json")).unwrap();
                // An absent local path makes the best-effort opener fail without
                // launching a real browser; the test plays the browser's redirect.
                let approval_url = dir.path().join("absent-browser-target").to_string_lossy().into_owned();
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
                let pending_expiry = match scenario {
                    "pending-expired" => 1_i64,
                    "server-expiry" => now + 1500,
                    _ => 4102444800000_i64,
                };
                let proposed = scenario == "success-proposed" || scenario.starts_with("create-proposal");
                let proposal = if proposed {serde_json::json!([A, B.to_uppercase()])} else {serde_json::json!([])};
                let mut create = serde_json::json!({"result": {
                    "requestedRoleIds":proposal, "requestId":"request-1", "status":"pending", "approvalUrl":approval_url, "expiresAt":pending_expiry
                }});
                match scenario {
                    "create-deadline-missing" => create["result"]["expiresAt"] = Value::Null,
                    "create-envelope-error" => create["error"] = serde_json::json!("secret-envelope-error"),
                    "create-proposal-missing" => { create["result"].as_object_mut().unwrap().remove("requestedRoleIds"); },
                    "create-proposal-null" => create["result"]["requestedRoleIds"] = Value::Null,
                    "create-proposal-malformed" => create["result"]["requestedRoleIds"] = serde_json::json!(["secret-invalid-role"]),
                    "create-proposal-duplicate" => create["result"]["requestedRoleIds"] = serde_json::json!([B, B.to_uppercase()]),
                    "create-proposal-oversized" => create["result"]["requestedRoleIds"] = serde_json::json!((1..=101).map(|id| format!("00000000-0000-0000-0000-{id:012x}")).collect::<Vec<_>>()),
                    "create-proposal-mismatch" => create["result"]["requestedRoleIds"] = serde_json::json!([B, A]),
                    _ => {},
                }
                let filing = match scenario {
                    "create-401" | "create-403" => ResponseTemplate::new(scenario[7..].parse::<u16>().unwrap()).set_body_string("auth-code-secret"),
                    _ => ResponseTemplate::new(200).set_body_json(create),
                };
                Mock::given(method("POST")).and(path(FILE_PATH))
                    .and(header("authorization", "Bearer test-bearer-token"))
                    .respond_with(filing).expect(1).mount(&server).await;

                // Scenarios that complete the browser redirect, and those that then exchange.
                let redirects = scenario.starts_with("success") || scenario.starts_with("exchange-") || scenario.starts_with("grants-")
                    || matches!(scenario, "tampered-state" | "callback-no-code" | "callback-bad-code" | "callback-denied");
                let exchanges = redirects && !scenario.starts_with("callback-");
                let successful = scenario.starts_with("success") || matches!(scenario, "tampered-state" | "grants-many");
                let redirect_status = scenario.strip_prefix("exchange-").and_then(|code| code.parse::<u16>().ok())
                    .filter(|status| (300..400).contains(status));
                let stolen = format!("{}/stolen", server.uri());
                Mock::given(method("POST")).and(path(TOKEN_PATH))
                    .and(header("authorization", "Bearer test-bearer-token"))
                    .respond_with(move |request: &wiremock::Request| {
                        let error = |status: u16, message: &str| ResponseTemplate::new(status)
                            .set_body_json(serde_json::json!({"error":message,"status":status,"requestId":"corr-1"}));
                        if let Some(status) = redirect_status {
                            return ResponseTemplate::new(status).insert_header("Location", stolen.as_str());
                        }
                        match scenario {
                            "exchange-replay" => return error(400, "BAD_REQUEST: code already redeemed"),
                            "exchange-redirect-mismatch" => return error(400, "BAD_REQUEST: invalid_grant: redirect URI does not match the filed value"),
                            "exchange-401" => return error(401, "UNAUTHORIZED: PKCE verification failed"),
                            "exchange-403" => return error(403, "FORBIDDEN: access requests require permission to create API keys"),
                            "exchange-echo" | "exchange-malformed" => {
                                // Deliberately echo the submitted code and verifier.
                                let echoed = String::from_utf8_lossy(&request.body).to_string();
                                return ResponseTemplate::new(if scenario == "exchange-echo" {401} else {200})
                                    .set_body_json(serde_json::json!({"error":echoed,"result":echoed}));
                            },
                            _ => {},
                        }
                        let mut issued = serde_json::json!({"result": {
                            "keyId":"issued-id", "key":"issued-key", "secret":if scenario == "exchange-empty" {""} else {"issued-secret"},
                            "keyExpiresAt":1794231456974_i64, "grantedRoleIds":[B.to_uppercase(), C]
                        }});
                        match scenario {
                            "success-no-expiry" => {issued["result"].as_object_mut().unwrap().remove("keyExpiresAt");},
                            "exchange-expiry-invalid" => issued["result"]["keyExpiresAt"] = serde_json::json!(-1),
                            "grants-missing" => {issued["result"].as_object_mut().unwrap().remove("grantedRoleIds");},
                            "grants-null" => issued["result"]["grantedRoleIds"] = Value::Null,
                            "grants-empty" => issued["result"]["grantedRoleIds"] = serde_json::json!([]),
                            "grants-malformed" => issued["result"]["grantedRoleIds"] = serde_json::json!(["secret-invalid-role"]),
                            "grants-duplicate" => issued["result"]["grantedRoleIds"] = serde_json::json!([B, B.to_uppercase()]),
                            "grants-many" => issued["result"]["grantedRoleIds"] = serde_json::json!((1..=101).map(|id| format!("00000000-0000-0000-0000-{id:012x}")).collect::<Vec<_>>()),
                            _ => {},
                        }
                        ResponseTemplate::new(200).set_body_json(issued)
                    }).expect(u64::from(exchanges)).mount(&server).await;
                // The granted roles' names come from the OAuth role listing;
                // `grants-many` leaves it unmocked, so the lookup fails and ids stay.
                if scenario != "grants-many" {
                    Mock::given(method("GET")).and(path("/v1/organizations/org/roles"))
                        .and(header("authorization", "Bearer test-bearer-token"))
                        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": [
                            {"id": B, "name": "Service Admin", "type": "system", "policies": [
                                {"allowDeny": "ALLOW", "permissions": ["control-plane:service:manage"], "resources": ["instance/*"]}
                            ]},
                            {"id": A, "name": "Member", "type": "system", "policies": []}
                        ]})))
                        .mount(&server).await;
                }

                let mut command = Command::new(clickhousectl_binary());
                command.env_clear().env("DO_NOT_TRACK", "1").env("HOME", dir.path().join("home"))
                    .env("PATH", dir.path().join("empty-path")).current_dir(dir.path())
                    .args(["cloud", "access", "request", "--org", "org", "--debug", "--purpose", "test purpose"])
                    .stdout(Stdio::piped()).stderr(Stdio::piped());
                if json { command.arg("--json"); }
                match scenario {
                    "timeout-env" => { command.env("CLICKHOUSE_CLOUD_ACCESS_TIMEOUT_SECONDS", "1"); },
                    "timeout-flag" => {
                        command.env("CLICKHOUSE_CLOUD_ACCESS_TIMEOUT_SECONDS", "invalid")
                            .args(["--timeout-seconds", "1"]);
                    },
                    _ => {},
                }
                if proposed && scenario != "create-proposal-unexpected" { command.args(["--perms", &format!("role:{A},role:{B}")]); }
                let started = std::time::Instant::now();
                let child = command.spawn().unwrap();
                let output_task = tokio::task::spawn_blocking(move || {
                    let mut child = child;
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(12);
                    while child.try_wait().unwrap().is_none() {
                        if std::time::Instant::now() >= deadline {
                            child.kill().unwrap();
                            let output = child.wait_with_output().unwrap();
                            panic!("request failed to terminate promptly: {}", String::from_utf8_lossy(&output.stderr));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    child.wait_with_output().unwrap()
                });
                if redirects {
                    let filed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                        loop {
                            if let Some(request) = server.received_requests().await.unwrap().first() {
                                break serde_json::from_slice::<Value>(&request.body).unwrap();
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        }
                    }).await.unwrap();
                    let redirect_uri = filed["redirectUri"].as_str().unwrap();
                    let state = filed["state"].as_str().unwrap();
                    let http = reqwest::Client::new();
                    if scenario == "tampered-state" {
                        // A forged redirect is refused and the wait goes on.
                        let forged = http.get(redirect_uri).query(&[("code", "forged-code"), ("state", "wrong-state")]).send().await.unwrap();
                        assert_eq!(forged.status().as_u16(), 400);
                    }
                    let query: Vec<(&str, &str)> = match scenario {
                        "callback-no-code" => vec![("state", state)],
                        "callback-bad-code" => vec![("code", "bad\u{1}code"), ("state", state)],
                        // The approval page's denial redirect (RFC 6749 §4.1.2.1).
                        "callback-denied" => vec![("error", "access_denied"), ("state", state)],
                        _ => vec![("code", "auth-code-secret"), ("state", state)],
                    };
                    let response = http.get(redirect_uri).query(&query).send().await.unwrap();
                    let expected = if matches!(scenario, "callback-no-code" | "callback-bad-code") {400} else {200};
                    assert_eq!(response.status().as_u16(), expected, "{scenario}");
                }
                let output = output_task.await.unwrap();
                if scenario.starts_with("timeout-") {
                    assert!(started.elapsed() >= std::time::Duration::from_secs(1));
                }
                // A 403 is a missing permission, not missing authentication (#1225).
                let expected_exit = if successful {0} else if scenario == "create-401" {4} else {1};
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert_eq!(output.status.code(), Some(expected_exit), "{scenario}: {stderr}");
                assert_eq!(stderr.contains(&approval_url), !scenario.starts_with("create-"), "{scenario}: {stderr}");
                assert!(!String::from_utf8_lossy(&output.stdout).contains(&approval_url));
                assert_access_output_has_no_credentials(&output);
                assert_access_debug_diagnostics(&output, &server.uri());
                let combined = format!("{}{}", String::from_utf8_lossy(&output.stdout), stderr);
                let expected_message = match scenario {
                    "callback-denied" => Some("access request denied"),
                    "server-expiry" | "pending-expired" => Some("access request expired"),
                    "timeout-env" | "timeout-flag" => Some("access request approval wait timed out after 1 seconds"),
                    "callback-no-code" | "callback-bad-code" => Some("no usable authorization code"),
                    "exchange-replay" => Some("code already redeemed"),
                    "exchange-redirect-mismatch" => Some("redirect URI does not match"),
                    "exchange-401" => Some("PKCE verification failed"),
                    "exchange-403" => Some("permission to create API keys"),
                    "exchange-expiry-invalid" => Some("invalid keyExpiresAt"),
                    _ => None,
                };
                if let Some(message) = expected_message {
                    assert!(stderr.contains(message), "{scenario}: {stderr}");
                }
                // A refused code or verifier is not an identity problem, so it never suggests login.
                if matches!(scenario, "exchange-replay" | "exchange-401" | "exchange-echo") {
                    assert!(!stderr.contains("cloud auth login"), "{scenario}: {stderr}");
                }
                assert!(!combined.contains("secret-envelope-error"));
                assert!(!combined.contains("secret-invalid-role"));
                let requests = server.received_requests().await.unwrap();
                assert!(requests.iter().all(|r| r.url.path() != "/stolen"), "{scenario}: followed a redirect");
                // Only this machine's redirect and the deadline end the wait.
                assert!(requests.iter().all(|r| r.url.path() != STATUS_PATH), "{scenario}: read the request status");
                let filed: Value = serde_json::from_slice(&requests[0].body).unwrap();
                assert_eq!(requests[0].url.path(), FILE_PATH);
                assert!(filed.get("flow").is_none() && filed.get("agent").is_none(), "{filed}");
                assert_eq!(filed["codeChallengeMethod"], "S256");
                assert_eq!(filed["codeChallenge"].as_str().unwrap().len(), 43);
                let redirect = filed["redirectUri"].as_str().unwrap();
                assert!(redirect.starts_with("http://127.0.0.1:") && redirect.ends_with("/callback"), "{redirect}");
                let state = filed["state"].as_str().unwrap();
                assert!((1..=512).contains(&state.len()));
                assert!(!combined.contains(state));
                let token_requests: Vec<_> = requests.iter().filter(|r| r.url.path() == TOKEN_PATH).collect();
                assert_eq!(token_requests.len(), usize::from(exchanges), "{scenario}");
                for token in token_requests {
                    let body: Value = serde_json::from_slice(&token.body).unwrap();
                    assert_eq!(body.as_object().unwrap().len(), 3);
                    assert_eq!(body["code"], "auth-code-secret");
                    // Byte for byte the value filed.
                    assert_eq!(body["redirectUri"], filed["redirectUri"]);
                    let verifier = body["codeVerifier"].as_str().unwrap();
                    assert!((43..=128).contains(&verifier.len()));
                    assert!(!combined.contains(verifier));
                    assert_eq!(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())), filed["codeChallenge"]);
                }
                if successful {
                    let expiry = scenario != "success-no-expiry";
                    if json {
                        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
                        assert_eq!(result.as_object().unwrap().len(), 12);
                        assert_eq!(result["activeIn"], dir.path().canonicalize().unwrap().display().to_string());
                        assert_eq!(result["keyExpiresAtUtc"], if expiry {serde_json::json!("2026-11-09T13:37:36Z")} else {Value::Null});
                        if scenario == "grants-many" {
                            assert!(result["grantedRoles"].as_array().unwrap().iter().all(|role| role["name"].is_null()));
                        } else {
                            assert_eq!(result["grantedRoles"], serde_json::json!([{"id": B, "name": "Service Admin"}, {"id": C, "name": null}]));
                        }
                        assert_eq!(result["requestId"], "request-1");
                        assert_eq!(result["organizationId"], "org");
                        assert_eq!(result["credentialsPath"], credentials_path.canonicalize().unwrap().display().to_string());
                        assert_eq!(result["requestedRoleIds"], if proposed {serde_json::json!([A, B])} else {serde_json::json!([])});
                        if scenario == "grants-many" {
                            assert_eq!(result["grantedRoleIds"].as_array().unwrap().len(), 101);
                        } else { assert_eq!(result["grantedRoleIds"], serde_json::json!([B, C])); }
                        assert_eq!(result["status"], "approved");
                        assert_eq!(result["keyId"], "issued-id");
                        assert_eq!(result["key"], "issued-key");
                        assert_eq!(result["keyExpiresAt"], if expiry {serde_json::json!(1794231456974_i64)} else {Value::Null});
                    } else {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let canonical = credentials_path.canonicalize().unwrap();
                        assert!(stdout.starts_with(&format!("Approved. Key saved: {}\n", canonical.display())), "{stdout}");
                        if scenario != "grants-many" {
                            assert!(stdout.contains(&format!("  Granted: Service Admin ({B}), {C}\n")), "{stdout}");
                        }
                        // The approver granted other roles than proposed.
                        if proposed { assert!(stdout.contains("  Requested: Member, Service Admin; the approver changed the roles."), "{stdout}"); }
                        else { assert!(stdout.contains("  Requested: none; the approver changed the roles."), "{stdout}"); }
                        // The fixture expiry is in the past, so 0 minutes remain.
                        assert_eq!(stdout.contains("  Expires: 2026-11-09T13:37:36Z ("), expiry, "{stdout}");
                        assert!(stdout.contains(&format!("  Active in {} and below.", dir.path().canonicalize().unwrap().display())), "{stdout}");
                    }
                    let stored: Value = serde_json::from_slice(&std::fs::read(&credentials_path).unwrap()).unwrap();
                    assert_eq!(stored["api_key"], "issued-key");
                    assert_eq!(stored["api_secret"], "issued-secret");
                    let grant = &stored["access_grant"];
                    assert_eq!(grant["api_key"], "issued-key");
                    assert_eq!(grant["key_id"], "issued-id");
                    assert_eq!(grant["organization_id"], "org");
                    assert_eq!(grant["request_id"], "request-1");
                    assert_eq!(grant["expires_at"], if expiry {serde_json::json!("2026-11-09T13:37:36Z")} else {Value::Null});
                    assert!(!grant.to_string().contains("issued-secret"));
                    if scenario != "grants-many" {
                        assert_eq!(grant["roles"][0]["name"], "Service Admin");
                        assert_eq!(grant["roles"][0]["policies"][0]["permissions"], serde_json::json!(["control-plane:service:manage"]));
                        assert!(grant["roles"][1].get("policies").is_none());
                    }
                } else {
                    assert!(output.stdout.is_empty());
                    assert_eq!(std::fs::read(&credentials_path).unwrap(), before);
                }
                assert_eq!(std::fs::read(tokens.join("tokens.json")).unwrap(), oauth_before);
            }));
        }
    }
    for task in tasks {
        task.await.unwrap();
    }
}

#[test]
fn access_request_without_purpose_is_a_usage_error_before_authentication() {
    let dir = tempfile::tempdir().unwrap();
    let output = access_command_without_purpose(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--purpose"));
    assert!(output.stdout.is_empty());
    assert!(!dir.path().join(".clickhouse").exists());
}

#[test]
fn access_request_invalid_timeout_environment_fails_before_authentication() {
    for value in ["0", "-1", "", "1.5", "5m", "4294967296"] {
        let dir = tempfile::tempdir().unwrap();
        let output = access_command(dir.path())
            .env("CLICKHOUSE_CLOUD_ACCESS_TIMEOUT_SECONDS", value)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{value:?}");
        assert!(output.stdout.is_empty());
        assert!(!dir.path().join(".clickhouse").exists());
    }
}

/// `cloud access request` with the required `--purpose`.
fn access_command(dir: &Path) -> Command {
    let mut command = access_command_without_purpose(dir);
    command.args(["--purpose", "test purpose"]);
    command
}

fn access_command_without_purpose(dir: &Path) -> Command {
    let mut command = Command::new(clickhousectl_binary());
    command
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.join("home"))
        .current_dir(dir)
        .args(["cloud", "access", "request", "--json", "--debug"]);
    command
}

fn access_credentials(dir: &Path) -> PathBuf {
    let project = dir.join(".clickhouse");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("credentials.json"),
        r#"{"api_key":"saved-basic-key","api_secret":"saved-basic-secret"}"#,
    )
    .unwrap();
    let tokens = dir.join("home/.clickhouse");
    std::fs::create_dir_all(&tokens).unwrap();
    tokens
}

fn assert_access_debug_diagnostics(output: &std::process::Output, api_url: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr
            .matches("[debug] auth source: OAuth tokens (")
            .count(),
        1,
        "{stderr}"
    );
    assert!(stderr.contains("home/.clickhouse/tokens.json)"), "{stderr}");
    assert_eq!(stderr.matches("[debug] api url:").count(), 1, "{stderr}");
    assert!(
        stderr
            .lines()
            .any(|line| line == format!("[debug] api url: {api_url}/v1")),
        "{stderr}"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("[debug]"));
}

fn assert_access_output_has_no_credentials(output: &std::process::Output) {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for secret in [
        "saved-basic-key",
        "saved-basic-secret",
        "flag-basic-secret",
        "env-basic-secret",
        "test-bearer-token",
        "refreshed-bearer-token",
        "refresh-secret",
        // Access handshake fixtures: the authorization code and the issued
        // secret. Per-run values (verifier, state) are checked by their callers.
        "auth-code-secret",
        "issued-secret",
    ] {
        assert!(
            !text.contains(secret),
            "credential leaked in command output"
        );
    }
}

#[tokio::test]
async fn access_request_perms_files_canonical_proposal_once_without_grant_or_subset() {
    let first = "abcdefab-1234-1234-1234-123456789abc";
    let system = "21111111-1111-1111-9999-000000000007";
    for json in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/organizations/selected/accessRequests/authorizationCode",
            ))
            .and(header("authorization", "Bearer test-bearer-token"))
            .respond_with(
                ResponseTemplate::new(403).set_body_json(
                    serde_json::json!({"error": "role selection is not assignable"}),
                ),
            )
            .expect(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let tokens = access_credentials(dir.path());
        write_oauth_tokens(&tokens, &server.uri());
        let credentials_path = dir.path().join(".clickhouse/credentials.json");
        let before = std::fs::read(&credentials_path).unwrap();
        let mut command = Command::new(clickhousectl_binary());
        command
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", dir.path().join("home"))
            .current_dir(dir.path())
            .args([
                "cloud",
                "access",
                "request",
                "--debug",
                "--org",
                "selected",
                "--perms",
                &format!("role:{first},role:{system}"),
                "--perms",
                &format!("role:{}", first.to_uppercase()),
                "--key-name",
                "proposed-key",
                "--purpose",
                "bounded proposal",
            ]);
        let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        if json {
            // Under a detected agent (which also forces JSON), the handshake
            // carries the agent session/trace headers like other Cloud calls.
            command
                .arg("--json")
                .env("AGENT", "claude-code")
                .env("CLAUDE_CODE_SESSION_ID", "sess-access")
                .env("TRACEPARENT", traceparent);
        }
        let output = command.output().unwrap();
        // A 403 is a refusal, not missing authentication.
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("role selection is not assignable"));
        if json {
            // `--debug` diagnostics precede the pretty-printed envelope.
            let envelope = &stderr[stderr.rfind("\n{").map_or(0, |start| start + 1)..];
            let error: Value = serde_json::from_str(envelope).unwrap();
            assert!(error["error"]["message"].is_string(), "{stderr}");
            assert_ne!(error["error"]["code"], "auth_required", "{stderr}");
        }
        assert_access_output_has_no_credentials(&output);
        assert_eq!(std::fs::read(&credentials_path).unwrap(), before);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "no role catalog lookup or subset retry");
        let agent_header = |name: &str| {
            requests[0]
                .headers
                .get(name)
                .map(|value| value.to_str().unwrap().to_string())
        };
        if json {
            assert_eq!(
                agent_header("agent-session-id").as_deref(),
                Some("sess-access")
            );
            assert_eq!(agent_header("traceparent").as_deref(), Some(traceparent));
        } else {
            assert_eq!(agent_header("agent-session-id"), None);
        }
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["requestedRoleIds"], serde_json::json!([first, system]));
        assert_eq!(body["keyName"], "proposed-key");
        assert_eq!(body["purpose"], "bounded proposal");
        assert_eq!(body["codeChallengeMethod"], "S256");
        assert_eq!(body["codeChallenge"].as_str().unwrap().len(), 43);
        assert!(!body["state"].as_str().unwrap().is_empty());
        assert!(
            body["redirectUri"]
                .as_str()
                .unwrap()
                .starts_with("http://127.0.0.1:")
        );
        assert!(body["redirectUri"].as_str().unwrap().ends_with("/callback"));
        assert!(body.get("flow").is_none());
        assert_eq!(body.as_object().unwrap().len(), 7);
        for field in ["codeChallenge", "state"] {
            assert!(
                !String::from_utf8_lossy(&output.stderr).contains(body[field].as_str().unwrap())
            );
        }
    }
}

#[tokio::test]
async fn access_request_perms_usage_validation_precedes_oauth_and_network() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let tokens = access_credentials(dir.path());
    // Expired identity would attempt refresh if usage validation were late.
    std::fs::write(
        tokens.join("tokens.json"),
        serde_json::to_vec(&serde_json::json!({
            "access_token": "test-bearer-token", "refresh_token": "refresh-secret", "expires_at": 1,
            "api_url": format!("{}/v1", server.uri())
        }))
        .unwrap(),
    )
    .unwrap();
    let oversized = (0..101)
        .map(|i| format!("role:00000000-0000-0000-0000-{i:012x}"))
        .collect::<Vec<_>>()
        .join(",");
    for value in [
        "",
        ",",
        "role:Admin",
        "Role:00000000-0000-0000-0000-000000000001",
        "role:00000000-0000-0000-0000-000000000001,",
        "role:00000000-0000-0000-0000-000000000001 ,role:*",
        &oversized,
    ] {
        let output = access_command(dir.path())
            .args(["--perms", value])
            .env("CLICKHOUSECTL_AUTH_URL", server.uri())
            .env("CLICKHOUSECTL_CLIENT_ID", "test-client")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert_access_output_has_no_credentials(&output);
    }
    let output = Command::new(clickhousectl_binary())
        .env_clear()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .current_dir(dir.path())
        .args([
            "cloud",
            "--org-id=one",
            "access",
            "request",
            "--org=two",
            "--perms",
            "role:00000000-0000-0000-0000-000000000001",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(server.received_requests().await.unwrap().is_empty());
}

/// Empty or over-long `--purpose` or `--key-name` fail as usage errors
/// before OAuth and the network.
#[tokio::test]
async fn access_request_length_limits_fail_before_oauth_and_network() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let tokens = access_credentials(dir.path());
    // Expired identity would attempt refresh if usage validation were late.
    std::fs::write(
        tokens.join("tokens.json"),
        serde_json::to_vec(&serde_json::json!({
            "access_token": "test-bearer-token", "refresh_token": "refresh-secret", "expires_at": 1,
            "api_url": format!("{}/v1", server.uri())
        }))
        .unwrap(),
    )
    .unwrap();
    let purpose = "p".repeat(201);
    let key_name = "k".repeat(101);
    for (args, message) in [
        (
            vec!["--purpose", ""],
            "--purpose must be 1 to 200 characters",
        ),
        (
            vec!["--purpose", &purpose],
            "--purpose must be 1 to 200 characters",
        ),
        (
            vec!["--key-name", ""],
            "--key-name must be 1 to 100 characters",
        ),
        (
            vec!["--key-name", &key_name],
            "--key-name must be 1 to 100 characters",
        ),
    ] {
        let mut command = if args[0] == "--purpose" {
            access_command_without_purpose(dir.path())
        } else {
            access_command(dir.path())
        };
        let output = command
            .args(&args)
            .env("CLICKHOUSECTL_AUTH_URL", server.uri())
            .env("CLICKHOUSECTL_CLIENT_ID", "test-client")
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(stderr.contains(message), "{args:?}: {stderr}");
        assert!(output.stdout.is_empty());
        assert_access_output_has_no_credentials(&output);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

/// Discovery runs on the typed client directly, outside the handshake's
/// `operation` wrapper, so `for_access` must hand the client the 30-second
/// access-operation bound itself: a server holding the response open
/// fails the command near the bound instead of stalling until the mock's
/// delay elapses. The mock delays longer than the bound; without the
/// client timeout the command would only finish after it.
#[tokio::test]
async fn access_roles_discovery_gives_up_within_the_operation_bound() {
    let started = std::time::Instant::now();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/roles"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!([]))
                .set_delay(std::time::Duration::from_secs(90)),
        )
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let tokens = access_credentials(dir.path());
    write_oauth_tokens(&tokens, &server.uri());
    let output = access_roles_command(dir.path(), true)
        .args(["--org", "org"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(60),
        "discovery must give up within the operation bound, not wait out the mock"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("test-bearer-token"));
    assert_access_output_has_no_credentials(&output);
}

#[tokio::test]
async fn access_request_uses_oauth_for_discovery_and_filing_with_saved_or_explicit_url() {
    for explicit_url in [false, true] {
        let selected = MockServer::start().await;
        let saved = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations"))
            .and(header("authorization", "Bearer test-bearer-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": [{"id": "11111111-2222-3333-4444-555555555555", "name": "Only org"}]
            })))
            .expect(1)
            .mount(&selected)
            .await;
        // Stop before browser launch, while exercising the real PKCE POST.
        Mock::given(method("POST"))
            .and(path(
                "/v1/organizations/11111111-2222-3333-4444-555555555555/accessRequests/authorizationCode",
            ))
            .and(header("authorization", "Bearer test-bearer-token"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "error": "fixture rejection"
            })))
            .expect(1)
            .mount(&selected)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let token_dir = access_credentials(dir.path());
        write_oauth_tokens(
            &token_dir,
            &if explicit_url {
                saved.uri()
            } else {
                selected.uri()
            },
        );
        let mut command = access_command(dir.path());
        // Even explicit/environment Basic credentials cannot change the user lane.
        command
            .args(["--api-key", "flag-key", "--api-secret", "flag-basic-secret"])
            .env("CLICKHOUSE_CLOUD_API_KEY", "env-key")
            .env("CLICKHOUSE_CLOUD_API_SECRET", "env-basic-secret");
        if explicit_url {
            command.args(["--url", &format!("{}/v1/", selected.uri())]);
        }
        let output = command.output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_access_output_has_no_credentials(&output);
        assert_access_debug_diagnostics(&output, &selected.uri());
        assert!(saved.received_requests().await.unwrap().is_empty());
        let requests = selected.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
        let body: Value = serde_json::from_slice(&requests[1].body).unwrap();
        assert_eq!(body["codeChallengeMethod"], "S256");
        assert_eq!(body["codeChallenge"].as_str().unwrap().len(), 43);
        assert!(!body["state"].as_str().unwrap().is_empty());
        assert!(
            body["redirectUri"]
                .as_str()
                .unwrap()
                .starts_with("http://127.0.0.1:")
        );
        assert!(body.get("codeVerifier").is_none());
        assert!(body.get("flow").is_none());
        // The required --purpose is the fifth field.
        assert_eq!(body.as_object().unwrap().len(), 5);
        assert_eq!(body["purpose"], "test purpose");
        let credentials =
            std::fs::read_to_string(dir.path().join(".clickhouse/credentials.json")).unwrap();
        assert!(credentials.contains("saved-basic-secret"));
        assert!(token_dir.join("tokens.json").exists());
    }
}

#[tokio::test]
async fn access_request_missing_or_unrefreshable_identity_never_falls_back_to_basic() {
    for token_state in ["missing", "no-refresh", "unknown-host", "rejected"] {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let token_dir = access_credentials(dir.path());
        if token_state != "missing" {
            // `auth_config` is required for the record to load at all. The
            // saved config only drives refresh for the states that must
            // reach it; `unknown-host` keeps a legacy record without one,
            // which no environment can repair.
            let mut record = serde_json::json!({
                "access_token": "test-bearer-token",
                "refresh_token": if token_state == "no-refresh" { "" } else { "refresh-secret" },
                "expires_at": 1,
                "api_url": format!("{}/v1", server.uri()),
            });
            if token_state != "unknown-host" {
                record["auth_config"] =
                    serde_json::json!({"auth_url": server.uri(), "client_id": "test-client"});
            }
            std::fs::write(
                token_dir.join("tokens.json"),
                serde_json::to_vec(&record).unwrap(),
            )
            .unwrap();
        }
        let mut command = access_command(dir.path());
        command.args([
            "--url",
            &server.uri(),
            "--org-id",
            "11111111-2222-3333-4444-555555555555",
        ]);
        if token_state == "rejected" {
            Mock::given(method("POST"))
                .and(path("/oauth/token"))
                // Even an identity provider echoing a secret must not leak it.
                .respond_with(ResponseTemplate::new(401).set_body_string("refresh-secret"))
                .expect(1)
                .mount(&server)
                .await;
            command
                .env("CLICKHOUSECTL_AUTH_URL", server.uri())
                .env("CLICKHOUSECTL_CLIENT_ID", "test-client");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(4), "{token_state}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert!(
            error["error"]["message"]
                .as_str()
                .unwrap()
                .contains("cloud auth login")
        );
        assert_access_output_has_no_credentials(&output);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), usize::from(token_state == "rejected"));
        assert!(dir.path().join(".clickhouse/credentials.json").exists());
        // A failed refresh never removes the saved login (#1224).
        assert_eq!(
            token_dir.join("tokens.json").exists(),
            token_state != "missing",
            "{token_state}"
        );
    }
}

#[tokio::test]
async fn access_request_refreshes_oauth_despite_stored_basic_and_preserves_login_environment() {
    let issuer = MockServer::start().await;
    let target = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "refreshed-bearer-token", "expires_in": 3600, "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&issuer)
        .await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/11111111-2222-3333-4444-555555555555/accessRequests/authorizationCode",
        ))
        .and(header("authorization", "Bearer refreshed-bearer-token"))
        .respond_with(ResponseTemplate::new(403))
        .expect(1)
        .mount(&target)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let token_dir = access_credentials(dir.path());
    std::fs::write(
        token_dir.join("tokens.json"),
        serde_json::to_vec(&serde_json::json!({
            "access_token": "test-bearer-token", "refresh_token": "refresh-secret",
            "expires_at": 1, "api_url": format!("{}/v1", issuer.uri()),
            "auth_config": {"auth_url": issuer.uri(), "client_id": "test-client"},
        }))
        .unwrap(),
    )
    .unwrap();
    let output = access_command(dir.path())
        .args([
            "--url",
            &target.uri(),
            "--org-id",
            "11111111-2222-3333-4444-555555555555",
        ])
        .env("CLICKHOUSECTL_AUTH_URL", format!("{}/", issuer.uri()))
        .env("CLICKHOUSECTL_CLIENT_ID", "test-client")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_access_output_has_no_credentials(&output);
    let requests = issuer.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let form: std::collections::HashMap<_, _> = url::form_urlencoded::parse(&requests[0].body)
        .into_owned()
        .collect();
    assert_eq!(form["grant_type"], "refresh_token");
    assert_eq!(form["client_id"], "test-client");
    assert_eq!(form["refresh_token"], "refresh-secret");
    let tokens: Value =
        serde_json::from_slice(&std::fs::read(token_dir.join("tokens.json")).unwrap()).unwrap();
    assert_eq!(tokens["access_token"], "refreshed-bearer-token");
    assert_eq!(tokens["refresh_token"], "refresh-secret");
    assert_eq!(tokens["api_url"], format!("{}/v1", issuer.uri()));
    assert!(tokens["expires_at"].as_i64().unwrap() > jiff::Timestamp::now().as_second() + 60);
}

#[tokio::test]
async fn access_organization_resolution_reads_complete_user_memberships() {
    const ONE: &str = "11111111-2222-3333-4444-555555555555";
    const TWO: &str = "22222222-2222-3333-4444-555555555555";
    for scenario in [
        "zero",
        "one",
        "multi",
        "paged-single",
        "paged-multi",
        "named",
        "case-mismatch",
        "ambiguous-name",
        "repeat",
        "no-progress",
        "empty-progress",
        "page-error",
        "missing-result",
        "missing-id",
        "explicit",
    ] {
        let server = MockServer::start().await;
        let first_rows = match scenario {
            "zero" | "explicit" => serde_json::json!([]),
            "multi" => {
                serde_json::json!([{"id": ONE, "name": "First"}, {"id": TWO, "name": "Second"}])
            }
            _ => serde_json::json!([{"id": ONE, "name": "First"}]),
        };
        let paged = [
            "paged-single",
            "paged-multi",
            "named",
            "case-mismatch",
            "ambiguous-name",
            "repeat",
            "no-progress",
            "empty-progress",
            "page-error",
            "missing-result",
            "missing-id",
        ]
        .contains(&scenario);
        let mut first = serde_json::json!({"result": first_rows});
        if paged {
            first["nextCursor"] = "opaque+/= &?".into();
        }
        Mock::given(method("GET"))
            .and(path("/v1/organizations"))
            .and(header("authorization", "Bearer test-bearer-token"))
            .and(|request: &wiremock::Request| request.url.query().is_none())
            .respond_with(ResponseTemplate::new(200).set_body_json(first))
            .expect(if scenario == "explicit" { 0 } else { 1 })
            .mount(&server)
            .await;
        if paged {
            let second = match scenario {
                "paged-single" => serde_json::json!({"result": [], "nextCursor": null}),
                "repeat" => serde_json::json!({"result": [], "nextCursor": "opaque+/= &?"}),
                "no-progress" => {
                    serde_json::json!({"result": [{"id": ONE}], "nextCursor": "different"})
                }
                "empty-progress" => serde_json::json!({"result": [], "nextCursor": "different"}),
                "missing-result" => serde_json::json!({"nextCursor": null}),
                "missing-id" => serde_json::json!({"result": [{"name": "Second"}]}),
                "ambiguous-name" => {
                    serde_json::json!({"result": [{"id": TWO, "name": "First"}]})
                }
                _ => serde_json::json!({"result": [{"id": TWO, "name": "Second"}]}),
            };
            Mock::given(method("GET"))
                .and(path("/v1/organizations"))
                .and(query_param("cursor", "opaque+/= &?"))
                .and(header("authorization", "Bearer test-bearer-token"))
                .respond_with(
                    ResponseTemplate::new(if scenario == "page-error" { 500 } else { 200 })
                        .set_body_json(second),
                )
                .expect(1)
                .mount(&server)
                .await;
        }
        let files = ["one", "paged-single", "named", "explicit"].contains(&scenario);
        let selected = if ["named", "explicit"].contains(&scenario) {
            TWO
        } else {
            ONE
        };
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/organizations/{selected}/accessRequests/authorizationCode"
            )))
            .and(header("authorization", "Bearer test-bearer-token"))
            .respond_with(ResponseTemplate::new(403))
            .expect(if files { 1 } else { 0 })
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let token_dir = access_credentials(dir.path());
        write_oauth_tokens(&token_dir, &server.uri());
        let mut command = access_command(dir.path());
        match scenario {
            "named" => {
                command.args(["--org-name", "Second"]);
            }
            "case-mismatch" => {
                command.args(["--org-name", "second"]);
            }
            "ambiguous-name" => {
                command.args(["--org-name", "First"]);
            }
            "explicit" => {
                command.args(["--org", TWO]);
            }
            _ => {}
        }
        let output = command.output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "{scenario}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_access_output_has_no_credentials(&output);
        if ["zero", "multi", "paged-multi"].contains(&scenario) {
            assert!(String::from_utf8_lossy(&output.stderr).contains("cloud org list --json"));
        }
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests.len(),
            usize::from(scenario != "explicit") + usize::from(paged) + usize::from(files),
            "{scenario}"
        );
    }
}

#[test]
fn access_organization_conflicts_fail_usage_before_authentication() {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        vec![
            "cloud",
            "--org-id=first",
            "access",
            "request",
            "--org-id=second",
        ],
        vec![
            "cloud",
            "--org-id=first",
            "access",
            "request",
            "--org=second",
        ],
        vec![
            "cloud",
            "access",
            "--org=first",
            "request",
            "--org-id=second",
        ],
        vec![
            "cloud",
            "--org-name=first",
            "access",
            "request",
            "--org-name=second",
        ],
        vec![
            "cloud",
            "--org-name=same",
            "access",
            "request",
            "--org=same",
        ],
    ] {
        let output = Command::new(clickhousectl_binary())
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", dir.path())
            .current_dir(dir.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
    }
}

/// A bare `FORBIDDEN` under the saved key from `cloud access request` names
/// the running command's declared need and what the granted role lacks; the
/// classification and exit code stay those of a 403. An empty `clickhouse list`
/// under a role without `service:view` gets a stderr note.
#[tokio::test]
async fn forbidden_under_a_granted_key_names_what_the_role_lacks() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org/services/svc-1"))
        .respond_with(
            ResponseTemplate::new(403)
                .set_body_json(serde_json::json!({"error": "FORBIDDEN", "status": 403})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/services"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": []})))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(dir.path().join("home")).unwrap();
    std::fs::write(
        project.join("credentials.json"),
        serde_json::to_vec(&serde_json::json!({
            "api_key": "granted-key", "api_secret": "granted-secret",
            "api_url": format!("{}/v1", server.uri()),
            "access_grant": {
                "api_key": "granted-key", "key_id": "key-id", "organization_id": "org",
                "request_id": "request-1", "expires_at": "2099-01-01T09:56:00Z",
                "roles": [{"id": "21111111-1111-1111-9999-000000000003", "name": "Member", "policies": [
                    {"effect": "ALLOW", "permissions": ["control-plane:organization:view"], "resources": ["*"]}
                ]}]
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let run = |args: &[&str]| {
        Command::new(clickhousectl_binary())
            .env_clear()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", dir.path().join("home"))
            .current_dir(dir.path())
            .args(["cloud", "--org-id", "org"])
            .args(args)
            .output()
            .unwrap()
    };
    for json in [false, true] {
        let mut args = vec!["clickhouse", "update", "svc-1", "--new-name", "renamed"];
        if json {
            args.push("--json");
        }
        let output = run(&args);
        assert_eq!(output.status.code(), Some(1));
        let message = if json {
            let error = cloud_runtime_error(&output);
            assert_eq!(error["code"], "http_4xx");
            error["message"].as_str().unwrap().to_string()
        } else {
            String::from_utf8_lossy(&output.stderr).into_owned()
        };
        assert!(
            message.contains("FORBIDDEN (the Cloud API gives no reason)."),
            "{message}"
        );
        assert!(
            message.contains(
                "Key: from `cloud access request`, role Member, expires 2099-01-01T09:56Z."
            ),
            "{message}"
        );
        assert!(
            message.contains("`cloud clickhouse update` needs control-plane:service:manage"),
            "{message}"
        );
        assert!(
            message.contains("clickhousectl cloud access request --perms role:<UUID>"),
            "{message}"
        );
        assert!(!message.contains("granted-secret"));
    }
    let output = run(&["clickhouse", "list", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).contains(
        "note: this key's role lacks control-plane:service:view; an empty list may mean no access."
    ));
}

/// A saved `cloud access request` key past its expiry outranks the OAuth
/// login, so a read refuses it before any request (exit 4, `auth_required`),
/// names the file and says how to replace or clear it; `cloud access roles`
/// keeps working on the OAuth login.
#[tokio::test]
async fn an_expired_saved_grant_is_refused_before_any_request() {
    let server = MockServer::start().await;
    for oauth in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let cloud_dir = dir.path().join("home/.clickhouse");
        std::fs::create_dir_all(&cloud_dir).unwrap();
        if oauth {
            write_oauth_tokens(&cloud_dir, &server.uri());
        }
        let project = dir.path().join(".clickhouse");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(
            project.join("credentials.json"),
            serde_json::to_vec(&serde_json::json!({
                "api_key": "granted-key", "api_secret": "granted-secret",
                "api_url": format!("{}/v1", server.uri()),
                "access_grant": {
                    "api_key": "granted-key", "key_id": "key-id", "organization_id": "org",
                    "request_id": "request-1", "expires_at": "2026-01-02T17:45:00Z",
                    "roles": [{"id": "role-1", "name": "Service API Reader"}]
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let mut messages = Vec::new();
        for json in [false, true] {
            let mut command = Command::new(clickhousectl_binary());
            command
                .isolated_env()
                .env("DO_NOT_TRACK", "1")
                .env("HOME", dir.path().join("home"))
                .current_dir(dir.path())
                .args(["cloud", "org", "list"]);
            if json {
                command.arg("--json");
            }
            let output = command.stdin(Stdio::null()).output().unwrap();
            assert_eq!(output.status.code(), Some(4), "{output:?}");
            assert!(output.stdout.is_empty());
            let message = if json {
                let error = cloud_runtime_error(&output);
                assert_eq!(error["code"], "auth_required");
                error["message"].as_str().unwrap().to_string()
            } else {
                let stderr = String::from_utf8(output.stderr).unwrap();
                // After the note naming the API saved with the key.
                let (_, error) = stderr
                    .split_once("Error: ")
                    .unwrap_or_else(|| panic!("{stderr}"));
                error.trim_end().to_string()
            };
            messages.push(message);
        }
        assert_eq!(messages[0], messages[1]);
        let message = &messages[0];
        assert!(message.contains("expired 2026-01-02T17:45Z"), "{message}");
        let key_line = format!(
            "  Key: {} (from `cloud access request`, role Service API Reader)",
            project.join("credentials.json").display()
        );
        assert!(message.lines().any(|line| line == key_line), "{message}");
        assert!(
            message.contains("clickhousectl cloud access request --perms role:<UUID>"),
            "{message}"
        );
        if oauth {
            assert!(message.contains("clickhousectl cloud auth logout --api-keys"));
        } else {
            assert!(
                message.contains("`clickhousectl cloud auth login`"),
                "{message}"
            );
        }
        assert!(!message.contains("granted-secret"));
        assert!(server.received_requests().await.unwrap().is_empty());

        if oauth {
            Mock::given(method("GET"))
                .and(path("/v1/organizations/org/roles"))
                .and(header("authorization", "Bearer test-bearer-token"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(serde_json::json!({"result": assignable_roles_fixture()})),
                )
                .expect(1)
                .mount(&server)
                .await;
            let output = Command::new(clickhousectl_binary())
                .isolated_env()
                .env("DO_NOT_TRACK", "1")
                .env("HOME", dir.path().join("home"))
                .current_dir(dir.path())
                .args(["cloud", "access", "roles", "--json", "--org", "org"])
                .output()
                .unwrap();
            assert_success(&output);
        }
    }
}

/// An expired saved grant beside an OAuth login that only needs a refresh:
/// the refusal refreshes it first and reports it valid, offering a new key or
/// clearing the old one. When the issuer refuses the refresh, the refusal is
/// unchanged (no valid login, log in first) and the saved login is kept.
#[tokio::test]
async fn an_expired_saved_grant_refreshes_a_lapsed_oauth_login_before_refusing() {
    let server = MockServer::start().await;
    for refreshes in [true, false] {
        let issuer = MockServer::start().await;
        mount_refresh(&issuer, refreshes, 1).await;
        let dir = tempfile::tempdir().unwrap();
        let token_path = write_expired_oauth_tokens(
            &dir.path().join("home/.clickhouse"),
            &server.uri(),
            &issuer,
        );
        let original = std::fs::read(&token_path).unwrap();
        let project = dir.path().join(".clickhouse");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(
            project.join("credentials.json"),
            serde_json::to_vec(&serde_json::json!({
                "api_key": "granted-key", "api_secret": "granted-secret",
                "api_url": format!("{}/v1", server.uri()),
                "access_grant": {
                    "api_key": "granted-key", "key_id": "key-id", "organization_id": "org",
                    "request_id": "request-1", "expires_at": "2026-01-02T17:45:00Z",
                    "roles": [{"id": "role-1", "name": "Service API Reader"}]
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let output = Command::new(clickhousectl_binary())
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", dir.path().join("home"))
            .current_dir(dir.path())
            .args(["cloud", "org", "list", "--json"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(4), "{output:?}");
        let error = cloud_runtime_error(&output);
        assert_eq!(error["code"], "auth_required");
        let message = error["message"].as_str().unwrap();
        assert!(message.contains("expired 2026-01-02T17:45Z"), "{message}");
        let lines: Vec<_> = message.lines().skip(2).collect();
        if refreshes {
            assert_eq!(
                lines,
                [
                    "  OAuth login: valid (read-only).",
                    "  New key: `clickhousectl cloud access roles`, then `clickhousectl cloud access request --perms role:<UUID> --purpose \"<what, why; max 200 chars>\"`",
                    "  Or clear it to use the OAuth login: `clickhousectl cloud auth logout --api-keys`",
                ],
                "{message}"
            );
            assert_eq!(saved_access_token(&token_path), "refreshed-access");
        } else {
            assert_eq!(
                lines,
                [
                    "  OAuth login: none valid.",
                    "  First log in (needs the user): `clickhousectl cloud auth login`. Then: `clickhousectl cloud access request --perms role:<UUID> --purpose \"<what, why; max 200 chars>\"`",
                ],
                "{message}"
            );
            assert_eq!(std::fs::read(&token_path).unwrap(), original);
        }
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}
