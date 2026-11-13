//! `cloud auth` and the OAuth login lifecycle (src/cloud/auth.rs).

use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

fn invoke_cli_without_cloud_credentials(
    mock: &MockServer,
    cli_args: &[String],
) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let mut args = vec!["cloud".to_string(), "--url".to_string(), mock.uri()];
    args.extend_from_slice(cli_args);
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(dir.path())
        .args(args)
        .output()
        .expect("failed to spawn clickhousectl")
}

fn invoke_api_key_login(project_dir: &Path, url: &str) -> std::process::Output {
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project_dir.join("home"))
        .current_dir(project_dir)
        .args([
            "cloud",
            "auth",
            "login",
            "--api-key",
            "new-key",
            "--api-secret",
            "new-secret",
            "--url",
            url,
        ])
        .output()
        .expect("failed to spawn clickhousectl")
}

#[tokio::test]
async fn api_key_login_saves_a_nonempty_pair_and_status_selects_it() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .mount(&mock)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let output = invoke_api_key_login(dir.path(), &mock.uri());
    assert_success(&output);
    let saved: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".clickhouse/credentials.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved["api_key"], "new-key");
    assert_eq!(saved["api_secret"], "new-secret");
    // The key was verified against a non-prod API, so that API is saved with
    // it and later commands go there without `--url` (#1202).
    let api_url = format!("{}/v1", mock.uri());
    assert_eq!(saved["api_url"], api_url.as_str());
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .current_dir(dir.path())
        .args(["cloud", "auth", "status", "--json"])
        .output()
        .unwrap();
    assert_success(&output);
    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status["verification"], "verified");
    let rows = status["sources"].as_array().unwrap();
    let saved = rows.iter().find(|row| row["type"] == "API key").unwrap();
    assert_eq!(saved["status"], "Active");
    assert_eq!(saved["scope"], "read/write");
    assert_eq!(saved["active"], "yes");
    assert_eq!(saved["api_url"], api_url.as_str());
}

#[tokio::test]
async fn auth_status_never_selects_empty_saved_credentials() {
    let mock = MockServer::start().await;
    // The empty-field forms are `client::empty_saved_credentials_do_not_shadow_usable_sources`.
    for (key, secret, with_env) in [("", "saved-secret", false), ("saved-key", "", true)] {
        let dir = tempfile::tempdir().unwrap();
        write_project_api_credentials(dir.path(), key, secret, &mock.uri());
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", dir.path().join("home"))
            .current_dir(dir.path())
            .args(["cloud", "auth", "status", "--json", "--url", &mock.uri()]);
        if with_env {
            command.env("CLICKHOUSE_CLOUD_API_KEY", "env-key");
            command.env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret");
        }
        let output = command.output().unwrap();
        assert_success(&output);
        let status: Value = serde_json::from_slice(&output.stdout).unwrap();
        let rows = status["sources"].as_array().unwrap();
        let saved = rows.iter().find(|row| row["type"] == "API key").unwrap();
        assert!(saved["status"].as_str().unwrap().starts_with("Incomplete"));
        assert_eq!(saved["scope"], "-");
        assert_eq!(saved["active"], "-");
        let active: Vec<_> = rows.iter().filter(|row| row["active"] == "yes").collect();
        if with_env {
            assert_eq!(active.len(), 1);
            assert_eq!(active[0]["type"], "Env vars");
        } else {
            assert!(active.is_empty());
        }
    }
}

#[tokio::test]
async fn api_key_login_preserves_malformed_credentials() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .mount(&mock)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let credentials_dir = dir.path().join(".clickhouse");
    let credentials_path = credentials_dir.join("credentials.json");
    std::fs::create_dir_all(&credentials_dir).unwrap();
    let original = b"{malformed credentials\n";
    std::fs::write(&credentials_path, original).unwrap();

    let output = invoke_api_key_login(dir.path(), &mock.uri());

    assert_eq!(output.status.code(), Some(1));
    // The parser's own diagnostic for the file, naming it.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let source = serde_json::from_slice::<Value>(original).unwrap_err();
    assert!(
        stderr.starts_with("Error: ")
            && stderr.contains(".clickhouse/credentials.json")
            && stderr.contains(&source.to_string()),
        "unexpected stderr: {stderr}",
    );
    assert_eq!(std::fs::read(credentials_path).unwrap(), original);
}

#[test]
fn logout_does_not_report_success_when_credentials_removal_fails() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let credentials_dir = dir.path().join(".clickhouse");
    std::fs::create_dir(&home).unwrap();
    std::fs::create_dir(&credentials_dir).unwrap();
    std::fs::create_dir(credentials_dir.join("credentials.json")).unwrap();

    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(dir.path())
        .args(["cloud", "auth", "logout", "--api-keys"])
        .output()
        .expect("failed to spawn clickhousectl");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "no success confirmation");
    assert!(credentials_dir.join("credentials.json").is_dir());
}

#[tokio::test]
async fn auth_status_explicit_flags_win_without_revealing_credentials() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .mount(&mock)
        .await;
    // Both output formats render the same rows, so each runs once.
    for (configure_other_sources, json) in [(false, true), (true, false)] {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir(&home).unwrap();
        if configure_other_sources {
            write_project_api_credentials(dir.path(), "file-key", "file-secret", &mock.uri());
            let token_dir = home.join(".clickhouse");
            std::fs::create_dir(&token_dir).unwrap();
            write_oauth_tokens(&token_dir, &mock.uri());
        }
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home)
            .current_dir(dir.path())
            .args([
                "cloud",
                "auth",
                "status",
                "--api-key",
                "flag-key",
                "--api-secret",
                "flag-secret",
                "--url",
                &mock.uri(),
                "--debug",
            ]);
        if configure_other_sources {
            command.env("CLICKHOUSE_CLOUD_API_KEY", "env-key");
            command.env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret");
        }
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        for credential in [
            "flag-key",
            "flag-secret",
            "file-key",
            "file-secret",
            "env-key",
            "env-secret",
            "test-bearer-token",
            "unused",
        ] {
            assert!(!stdout.contains(credential), "credential exposed on stdout");
            assert!(!stderr.contains(credential), "credential exposed on stderr");
        }
        assert!(stderr.contains("CLI flags"), "{stderr}");
        // Compare the same source, scope and selection fields in both formats.
        let rows: Vec<(String, String, String, String)> = if json {
            let status: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(status["verification"], "verified");
            assert_eq!(status["identity"]["keyId"], "key-123");
            status["sources"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    (
                        row["type"].as_str().unwrap().to_owned(),
                        row["status"].as_str().unwrap().to_owned(),
                        row["scope"].as_str().unwrap().to_owned(),
                        row["active"].as_str().unwrap().to_owned(),
                    )
                })
                .collect()
        } else {
            assert!(stdout.contains("Identity:\n"), "{stdout}");
            assert!(stdout.contains("key-123"), "{stdout}");
            stdout
                .lines()
                .skip(2)
                .take_while(|line| !line.is_empty())
                .map(|line| {
                    let cells: Vec<_> = line.trim_matches('|').split('|').map(str::trim).collect();
                    assert_eq!(cells.len(), 5, "{line}");
                    (
                        cells[0].into(),
                        cells[1].into(),
                        cells[2].into(),
                        cells[3].into(),
                    )
                })
                .collect()
        };
        assert_eq!(rows.len(), 4);
        assert_eq!(rows.iter().filter(|row| row.3 == "yes").count(), 1);
        let flags = rows.iter().find(|row| row.0 == "CLI flags").unwrap();
        assert_eq!(flags.1, "Active");
        assert_eq!(flags.2, "read/write");
        assert_eq!(flags.3, "yes");
        for source in ["API key", "Env vars", "OAuth"] {
            let row = rows.iter().find(|row| row.0 == source).unwrap();
            assert_eq!(row.3, "-");
            if configure_other_sources {
                assert!(row.1.starts_with("Configured"), "{row:?}");
                assert_eq!(
                    row.2,
                    if source == "OAuth" {
                        "read-only"
                    } else {
                        "read/write"
                    }
                );
            } else {
                assert_eq!(row.1, "Not configured");
                assert_eq!(row.2, "-");
            }
        }
    }
    // The identity check uses the winning source: the explicit flags.
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests {
        assert_eq!(request.url.path(), "/v1/whoami");
        assert_eq!(
            request.headers.get("authorization").unwrap(),
            "Basic ZmxhZy1rZXk6ZmxhZy1zZWNyZXQ=" // flag-key:flag-secret
        );
    }
}

#[tokio::test]
async fn auth_status_incomplete_flags_explain_missing_credential_without_falling_back() {
    let mock = MockServer::start().await;
    // Each configured source once, rotating which half of the pair is missing
    // and the output format.
    for (configured_source, (supplied, missing), json) in [
        (None, ("--api-key", "--api-secret"), false),
        (Some("API key"), ("--api-secret", "--api-key"), true),
        (Some("Env vars"), ("--api-key", "--api-secret"), true),
        (Some("OAuth"), ("--api-secret", "--api-key"), false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        std::fs::create_dir(&home).unwrap();
        if configured_source == Some("API key") {
            write_project_api_credentials(dir.path(), "file-key", "file-secret", &mock.uri());
        }
        if configured_source == Some("OAuth") {
            let token_dir = home.join(".clickhouse");
            std::fs::create_dir(&token_dir).unwrap();
            write_oauth_tokens(&token_dir, &mock.uri());
        }
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home)
            .current_dir(dir.path())
            .args([
                "cloud",
                "auth",
                "status",
                supplied,
                "flag-credential",
                "--url",
                &mock.uri(),
                "--debug",
            ]);
        if configured_source == Some("Env vars") {
            command.env("CLICKHOUSE_CLOUD_API_KEY", "env-key");
            command.env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret");
        }
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        // One debug line naming the missing flag.
        let context =
            format!("supplied={supplied}, configured_source={configured_source:?}, json={json}");
        assert_eq!(stderr.lines().count(), 1, "{context}: {stderr}");
        assert!(stderr.starts_with("[debug] "), "{context}: {stderr}");
        assert!(stderr.contains(missing), "{context}: {stderr}");
        for credential in [
            "flag-credential",
            "file-key",
            "file-secret",
            "env-key",
            "env-secret",
            "test-bearer-token",
            "unused",
        ] {
            assert!(!stdout.contains(credential), "credential exposed on stdout");
            assert!(!stderr.contains(credential), "credential exposed on stderr");
        }
        let rows: Vec<Vec<String>> = if json {
            let status: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(status["verification"], "skipped");
            assert!(status["identity"].is_null());
            status["sources"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    ["type", "status", "scope", "active"]
                        .map(|field| row[field].as_str().unwrap().to_owned())
                        .to_vec()
                })
                .collect()
        } else {
            assert!(!stdout.contains("Identity"), "{stdout}");
            stdout
                .lines()
                .skip(2)
                .map(|line| {
                    // The fifth column, the API URL, has no JSON counterpart
                    // for these rows.
                    let cells: Vec<String> = line
                        .trim_matches('|')
                        .split('|')
                        .map(|cell| cell.trim().to_owned())
                        .collect();
                    assert_eq!(cells.len(), 5, "{line}");
                    cells.into_iter().take(4).collect()
                })
                .collect()
        };
        assert_eq!(rows.len(), 4);
        for row in rows {
            assert_eq!(row.len(), 4);
            assert_eq!(row[3], "-", "no source should be active: {row:?}");
            if row[0] == "CLI flags" {
                assert!(row[1].starts_with("Incomplete"), "{row:?}");
                assert!(row[1].contains(missing), "{row:?}");
                assert_eq!(row[2], "-");
            } else if Some(row[0].as_str()) == configured_source {
                assert_eq!(row[1], "Configured (inactive)");
                assert_eq!(
                    row[2],
                    if row[0] == "OAuth" {
                        "read-only"
                    } else {
                        "read/write"
                    }
                );
            } else {
                assert_eq!(row[1], "Not configured");
                assert_eq!(row[2], "-");
            }
        }
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn auth_status_marks_outranked_environment_credentials_inactive() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .mount(&mock)
        .await;
    let dir = tempfile::tempdir().unwrap();
    write_project_api_credentials(dir.path(), "file-key", "file-secret", &mock.uri());
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", dir.path().join("home"))
        .env("CLICKHOUSE_CLOUD_API_KEY", "env-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "env-secret")
        .current_dir(dir.path())
        .args(["cloud", "--json", "--url", &mock.uri(), "auth", "status"])
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);
    assert!(output.stderr.is_empty());
    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
    let row = |source: &str| {
        status["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["type"] == source)
            .unwrap()
            .clone()
    };
    let env = row("Env vars");
    assert_eq!(env["active"], "-");
    assert_eq!(row("API key")["active"], "yes");
    // The inactive row names the source that outranks it.
    let env_status = env["status"].as_str().unwrap();
    assert!(env_status.starts_with("Configured"), "{env_status}");
    assert!(env_status.contains("credentials file"), "{env_status}");
}

/// The one end-to-end copy of the OAuth write check (`cloud::run`); which
/// commands count as writes is `is_write_command_classifies_reads_and_writes_in_every_domain`.
/// The check runs before any input is read and before organization or target
/// names are resolved, so neither run makes a request.
#[tokio::test]
async fn oauth_writes_are_rejected_before_input_org_or_target_lookups() {
    let mock = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    for (args, json) in [
        (
            vec![
                "udf",
                "create",
                "--file",
                "missing",
                "--artifact",
                "missing",
                "--org-id",
                "org-1",
            ],
            false,
        ),
        (
            vec![
                "service",
                "update",
                "--new-name",
                "new",
                "--name",
                "target",
                "--org-name",
                "production",
            ],
            true,
        ),
    ] {
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", &home)
            .current_dir(project.path())
            .args(["cloud", "--url", &mock.uri()])
            .args(&args);
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(4), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("clickhousectl cloud auth login --api-key"),
            "{stderr}"
        );
        if json {
            assert_eq!(cloud_runtime_error(&output)["code"], "auth_required");
        }
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

// An expired login that cannot be refreshed is kept (#1224): no refresh
// token, a rejected refresh, and an unreachable issuer all exit 4 without
// deleting the file or calling the API. The resolver unit test
// `client::expired_oauth_login_reports_expiry_not_missing_credentials` owns
// which message is reported.
#[tokio::test]
async fn unrefreshable_expired_oauth_login_is_kept_and_reported() {
    for case in ["no-refresh-token", "rejected", "unreachable"] {
        let api = MockServer::start().await;
        let issuer = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({"error": "invalid_grant"})),
            )
            .expect(u64::from(case == "rejected"))
            .mount(&issuer)
            .await;
        let auth_url = if case == "unreachable" {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            drop(listener);
            url
        } else {
            issuer.uri()
        };
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
        let original = serde_json::to_vec(&serde_json::json!({
            "access_token": "expired",
            "refresh_token": if case == "no-refresh-token" { "" } else { "refresh" },
            "expires_at": 1,
            "api_url": api.uri(),
            "auth_config": {"auth_url": auth_url, "client_id": "saved-client"}
        }))
        .unwrap();
        std::fs::write(&token_path, &original).unwrap();
        let output = custom_oauth_command(dir.path(), &api.uri())
            .args(["org", "list"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(4), "{case}: {output:?}");
        assert!(output.stdout.is_empty(), "{case}");
        assert_eq!(cloud_runtime_error(&output)["code"], "auth_required");
        assert_eq!(std::fs::read(&token_path).unwrap(), original, "{case}");
        assert!(api.received_requests().await.unwrap().is_empty(), "{case}");
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn old_oauth_records_require_login_even_for_known_hosts_and_fresh_tokens() {
    let mock = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let token_dir = dir.path().join("home/.clickhouse");
    std::fs::create_dir_all(&token_dir).unwrap();
    // `auth::load_tokens_rejects_old_records_for_known_and_custom_hosts` covers
    // custom hosts and expired tokens; a fresh token for a known host is the
    // case a refresh could otherwise mask.
    let old = serde_json::json!({
        "access_token": "old-access", "refresh_token": "old-refresh",
        "expires_at": 4102444800_i64, "api_url": "https://api.clickhouse.cloud/v1"
    });
    std::fs::write(
        token_dir.join("tokens.json"),
        serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    let output = custom_oauth_command(dir.path(), &mock.uri())
        .env("CLICKHOUSECTL_AUTH_URL", mock.uri())
        .env("CLICKHOUSECTL_CLIENT_ID", "must-not-be-used")
        .args(["auth", "whoami"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4), "{output:?}");
    assert_eq!(cloud_runtime_error(&output)["code"], "auth_required");
    assert!(mock.received_requests().await.unwrap().is_empty());
}

// Legacy production records cannot be refreshed: issuer configuration must be
// saved by a new login, not reconstructed from the built-in table or current env.
#[tokio::test]
async fn expired_legacy_production_tokens_require_login_instead_of_refresh() {
    let mock = MockServer::start().await;
    // The oldest production format also omitted api_url.
    for (mode, includes_api_url) in [("human", false), ("explicit", true)] {
        let dir = tempfile::tempdir().unwrap();
        let mut command = cloud_runtime_error_command(dir.path(), &mock.uri(), mode);
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
        let mut legacy = serde_json::json!({
            "access_token": "expired-production-access",
            "refresh_token": "legacy-production-refresh",
            "expires_at": 1
        });
        if includes_api_url {
            legacy["api_url"] = serde_json::json!("https://api.clickhouse.cloud/v1");
        }
        let original = serde_json::to_vec(&legacy).unwrap();
        std::fs::write(&token_path, &original).unwrap();
        let output = command
            .env_remove("CLICKHOUSE_CLOUD_API_KEY")
            .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
            // No environment or built-in configuration may repair the record.
            .env("CLICKHOUSECTL_AUTH_URL", mock.uri())
            .env("CLICKHOUSECTL_CLIENT_ID", "must-not-be-used")
            .args(["auth", "whoami"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(4), "{mode}: {output:?}");
        assert!(output.stdout.is_empty());
        let message = if mode == "human" {
            String::from_utf8_lossy(&output.stderr).into_owned()
        } else {
            let error = cloud_runtime_error(&output);
            assert_eq!(error["code"], "auth_required");
            error["message"].as_str().unwrap().to_owned()
        };
        assert_unusable_login_message(&message);
        assert_eq!(std::fs::read(&token_path).unwrap(), original);
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

/// An unusable saved login points only at `cloud auth login`; missing
/// credentials list every source, API-key flags and variables included.
fn assert_unusable_login_message(message: &str) {
    assert!(
        message.contains("`clickhousectl cloud auth login`"),
        "{message}"
    );
    assert!(!message.contains("--api-key"), "{message}");
    assert!(!message.contains("CLICKHOUSE_CLOUD_API_KEY"), "{message}");
}

fn assert_missing_credentials_message(message: &str) {
    assert!(message.contains("--api-key"), "{message}");
    assert!(message.contains("CLICKHOUSE_CLOUD_API_KEY"), "{message}");
}

#[tokio::test]
async fn unusable_oauth_login_is_distinct_from_missing_credentials() {
    let mock = MockServer::start().await;
    // One run per distinct outcome: no login, and an unusable one in each
    // output format. Which records are unusable is
    // `auth::load_tokens_reports_unusable_files_without_modifying_them` and
    // `auth::token_deserialization_requires_complete_configuration`.
    for (mode, format) in [
        ("human", "absent"),
        ("explicit", "malformed"),
        ("human", "incomplete"),
        ("explicit", "old"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        let original = match format {
            "absent" => None,
            "old" => Some(
                serde_json::to_vec(&serde_json::json!({
                    "access_token": "private-access", "refresh_token": "private-refresh",
                    "expires_at": 4102444800_i64, "api_url": mock.uri()
                }))
                .unwrap(),
            ),
            "incomplete" => Some(
                serde_json::to_vec(&serde_json::json!({
                    "access_token": "private-access", "refresh_token": "private-refresh",
                    "expires_at": 4102444800_i64, "api_url": mock.uri(),
                    "auth_config": {"auth_url": mock.uri()}
                }))
                .unwrap(),
            ),
            "malformed" => Some(b"not json: private-access".to_vec()),
            _ => unreachable!(),
        };
        if let Some(data) = &original {
            std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
            std::fs::write(&token_path, data).unwrap();
        }
        let command = || {
            let mut cmd = cloud_runtime_error_command(dir.path(), &mock.uri(), mode);
            cmd.env_remove("CLICKHOUSE_CLOUD_API_KEY")
                .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
                .env("CLICKHOUSECTL_AUTH_URL", mock.uri())
                .env("CLICKHOUSECTL_CLIENT_ID", "must-not-be-used");
            cmd
        };
        let output = command().args(["auth", "status"]).output().unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let expected = if format == "absent" {
            "Not configured"
        } else {
            "Login required"
        };
        if mode == "human" {
            let row = stdout
                .lines()
                .find(|line| line.contains("| OAuth"))
                .unwrap();
            let cells: Vec<_> = row.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells, ["OAuth", expected, "-", "-", "-"]);
        } else {
            let status: Value = serde_json::from_str(&stdout).unwrap();
            let row = status["sources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["type"] == "OAuth")
                .unwrap();
            assert_eq!(row["status"], expected);
            assert_eq!(row["scope"], "-");
            assert_eq!(row["active"], "-");
            assert_eq!(status["verification"], "skipped");
            assert!(status.get("warning").is_none());
            if format == "absent" {
                assert!(status.get("oauth_warning").is_none());
            } else {
                assert_unusable_login_message(status["oauth_warning"].as_str().unwrap());
            }
        }
        if format == "absent" {
            assert!(
                !stdout.contains("`clickhousectl cloud auth login`"),
                "{stdout}"
            );
        } else {
            assert!(
                stdout.contains("`clickhousectl cloud auth login`"),
                "{stdout}"
            );
        }
        let output = command()
            .args(["service", "list", "--org-id", "org-1"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(4), "{output:?}");
        assert!(output.stdout.is_empty());
        let message = if mode == "human" {
            String::from_utf8_lossy(&output.stderr).into_owned()
        } else {
            let error = cloud_runtime_error(&output);
            assert_eq!(error["code"], "auth_required");
            error["message"].as_str().unwrap().to_owned()
        };
        if format == "absent" {
            assert_missing_credentials_message(&message);
        } else {
            assert_unusable_login_message(&message);
        }
        for secret in ["private-access", "private-refresh"] {
            assert!(!stdout.contains(secret));
            assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
        }
        if let Some(data) = original {
            assert_eq!(std::fs::read(&token_path).unwrap(), data);
        }
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn auth_status_reports_unusable_oauth_alongside_api_key_identity() {
    for mode in ["human", "explicit"] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/whoami"))
            .and(wiremock::matchers::basic_auth("test-key", "test-secret"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "status": 401, "error": "key rejected"
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
        std::fs::write(&token_path, b"not json").unwrap();
        let output = cloud_runtime_error_command(dir.path(), &mock.uri(), mode)
            .args(["auth", "status", "--debug"])
            .output()
            .unwrap();
        assert_success(&output);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.contains("`clickhousectl cloud auth login`"),
            "{stdout}"
        );
        if mode == "human" {
            assert!(stdout.contains("Identity: rejected"));
        } else {
            let status: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(status["verification"], "rejected");
            assert!(status["warning"].as_str().unwrap().contains("key rejected"));
            assert_unusable_login_message(status["oauth_warning"].as_str().unwrap());
            let sources = status["sources"].as_array().unwrap();
            let env = sources
                .iter()
                .find(|row| row["type"] == "Env vars")
                .unwrap();
            assert_eq!(env["active"], "yes");
            let oauth = sources.iter().find(|row| row["type"] == "OAuth").unwrap();
            assert_eq!(oauth["status"], "Login required");
            assert_eq!(oauth["active"], "-");
        }
        assert_eq!(std::fs::read(&token_path).unwrap(), b"not json");
    }
}

#[tokio::test]
async fn oauth_login_and_refresh_never_follow_redirects() {
    let api = MockServer::start().await;
    let destination = MockServer::start().await;
    // One client builder serves both flows, and its redirect policy treats
    // every 3xx alike; 307/308 are the ones that would replay the body.
    for (status, login) in [(307, true), (308, false)] {
        let issuer = MockServer::start().await;
        let endpoint = if login {
            "/oauth/device/code"
        } else {
            "/oauth/token"
        };
        Mock::given(method("POST"))
            .and(path(endpoint))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("Location", format!("{}/stolen", destination.uri())),
            )
            .expect(1)
            .mount(&issuer)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let mut command = custom_oauth_command(dir.path(), &api.uri());
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        if login {
            command
                .env("CLICKHOUSECTL_AUTH_URL", issuer.uri())
                .env("CLICKHOUSECTL_CLIENT_ID", "client")
                .args(["auth", "login"]);
        } else {
            std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
            std::fs::write(
                &token_path,
                serde_json::to_vec(&serde_json::json!({
                    "access_token": "expired", "refresh_token": "secret-refresh", "expires_at": 1,
                    "api_url": api.uri(),
                    "auth_config": {"auth_url": issuer.uri(), "client_id": "client"}
                }))
                .unwrap(),
            )
            .unwrap();
            command.args(["auth", "whoami"]);
        }
        let output = command.output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(if login { 1 } else { 4 }),
            "{status}: {output:?}"
        );
        assert!(destination.received_requests().await.unwrap().is_empty());
        // A refused login saves nothing (a refused refresh keeps the saved
        // login: `unrefreshable_expired_oauth_login_is_kept_and_reported`),
        // and in JSON mode stderr holds only the error object.
        if login {
            assert!(!token_path.exists());
        }
        cloud_runtime_error(&output);
    }
    assert!(api.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn oauth_rejects_insecure_authorization_config_before_requests() {
    let api = MockServer::start().await;
    // Other schemes: `auth::authorization_url_requires_https_except_for_loopback`.
    let auth_url = "http://auth.example.invalid";
    let dir = tempfile::tempdir().unwrap();
    let output = custom_oauth_command(dir.path(), &api.uri())
        .env("CLICKHOUSECTL_AUTH_URL", auth_url)
        .env("CLICKHOUSECTL_CLIENT_ID", "client")
        .args(["auth", "login"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        cloud_runtime_error(&output)["message"]
            .as_str()
            .unwrap()
            .contains("HTTPS")
    );
    let token_path = dir.path().join("home/.clickhouse/tokens.json");
    assert!(!token_path.exists());
    std::fs::create_dir_all(token_path.parent().unwrap()).unwrap();
    std::fs::write(
        &token_path,
        serde_json::to_vec(&serde_json::json!({
            "access_token": "expired", "refresh_token": "secret-refresh", "expires_at": 1,
            "api_url": api.uri(),
            "auth_config": {"auth_url": auth_url, "client_id": "client"}
        }))
        .unwrap(),
    )
    .unwrap();
    let output = custom_oauth_command(dir.path(), &api.uri())
        .args(["auth", "whoami"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(cloud_runtime_error(&output)["code"], "auth_required");
    assert!(api.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn custom_oauth_login_pins_config_through_refresh_and_relogin() {
    let api = MockServer::start().await;
    let issuer_a = MockServer::start().await;
    let issuer_b = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let token_path = dir.path().join("home/.clickhouse/tokens.json");
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"actorType": "user", "email": "test@example.com", "organizations": []}
        })))
        .expect(4)
        .mount(&api)
        .await;

    // An absent local path makes the best-effort opener fail without launching
    // a real browser. This test drives the token response, not interactive login.
    let verification_uri = dir.path().join("no-browser-target");
    for (issuer, client_id, access, refresh) in [
        (&issuer_a, "client-a", "access-a", "refresh-a"),
        (&issuer_b, "client-b", "access-b", "refresh-b"),
    ] {
        Mock::given(method("POST"))
            .and(path("/oauth/device/code"))
            .and(wiremock::matchers::body_string_contains(format!(
                "client_id={client_id}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "test-device", "user_code": "TEST",
                "verification_uri": verification_uri.to_string_lossy(),
                "expires_in": 60, "interval": 0
            })))
            .expect(1)
            .mount(issuer)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(wiremock::matchers::body_string_contains(
                "device_code=test-device",
            ))
            .and(wiremock::matchers::body_string_contains(format!(
                "client_id={client_id}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": access, "refresh_token": refresh,
                "expires_in": 3600, "token_type": "Bearer"
            })))
            .expect(1)
            .mount(issuer)
            .await;
    }

    let output = custom_oauth_command(dir.path(), &api.uri())
        .env("CLICKHOUSECTL_AUTH_URL", format!("{}/", issuer_a.uri()))
        .env("CLICKHOUSECTL_CLIENT_ID", "client-a")
        .args(["auth", "login"])
        .output()
        .unwrap();
    assert_success(&output);
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&token_path).unwrap()).unwrap();
    let config_a = serde_json::json!({"auth_url": issuer_a.uri(), "client_id": "client-a"});
    assert_eq!(saved["auth_config"], config_a);
    assert_eq!(saved["api_url"], format!("{}/v1", api.uri()));
    assert_eq!(saved["refresh_token"], "refresh-a");

    // First refresh removes the variables and rotates the token. The second
    // changes both variables to B and omits rotation: both must still use A.
    for (changed_env, old_refresh, response) in [
        (
            false,
            "refresh-a",
            serde_json::json!({
                "access_token": "refreshed-a", "refresh_token": "rotated-a",
                "expires_in": 3600, "token_type": "Bearer"
            }),
        ),
        (
            true,
            "rotated-a",
            serde_json::json!({
                "access_token": "refreshed-a-again", "expires_in": 3600, "token_type": "Bearer"
            }),
        ),
    ] {
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(wiremock::matchers::body_string_contains(
                "grant_type=refresh_token",
            ))
            .and(wiremock::matchers::body_string_contains(
                "client_id=client-a",
            ))
            .and(wiremock::matchers::body_string_contains(format!(
                "refresh_token={old_refresh}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&issuer_a)
            .await;
        saved["expires_at"] = serde_json::json!(1);
        std::fs::write(&token_path, serde_json::to_vec(&saved).unwrap()).unwrap();
        let mut command = custom_oauth_command(dir.path(), &api.uri());
        if changed_env {
            command
                .env("CLICKHOUSECTL_AUTH_URL", issuer_b.uri())
                .env("CLICKHOUSECTL_CLIENT_ID", "client-b");
        }
        let output = command.args(["auth", "whoami"]).output().unwrap();
        assert_success(&output);
        saved = serde_json::from_slice(&std::fs::read(&token_path).unwrap()).unwrap();
        assert_eq!(saved["auth_config"], config_a);
        assert_eq!(saved["refresh_token"], "rotated-a");
        assert!(saved["expires_at"].as_i64().unwrap() > chrono::Utc::now().timestamp());
        assert!(issuer_b.received_requests().await.unwrap().is_empty());
    }

    // Only a deliberate new login changes the saved issuer/token pairing.
    let output = custom_oauth_command(dir.path(), &api.uri())
        .env("CLICKHOUSECTL_AUTH_URL", issuer_b.uri())
        .env("CLICKHOUSECTL_CLIENT_ID", "client-b")
        .args(["auth", "login"])
        .output()
        .unwrap();
    assert_success(&output);
    saved = serde_json::from_slice(&std::fs::read(&token_path).unwrap()).unwrap();
    assert_eq!(
        saved["auth_config"],
        serde_json::json!({
            "auth_url": issuer_b.uri(), "client_id": "client-b"
        })
    );
    assert_eq!(saved["access_token"], "access-b");
    assert_eq!(saved["refresh_token"], "refresh-b");
    assert_eq!(issuer_a.received_requests().await.unwrap().len(), 4);
    assert_eq!(issuer_b.received_requests().await.unwrap().len(), 2);
}

// ── auth login/logout keep stdout to one JSON object (#1233) ──────────────

#[tokio::test]
async fn oauth_device_login_in_json_mode_puts_instructions_on_stderr() {
    let api = MockServer::start().await;
    let issuer = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(whoami_envelope(serde_json::json!({
            "actorType": "user", "email": "test@example.com", "organizations": []
        })))
        .expect(1)
        .mount(&api)
        .await;
    // An absent local path makes the best-effort opener fail without
    // launching a real browser.
    let verification_uri = dir.path().join("no-browser-target");
    Mock::given(method("POST"))
        .and(path("/oauth/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "test-device", "user_code": "WXYZ-1234",
            "verification_uri": verification_uri.to_string_lossy(),
            "expires_in": 60, "interval": 0
        })))
        .expect(1)
        .mount(&issuer)
        .await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "access", "refresh_token": "refresh",
            "expires_in": 3600, "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&issuer)
        .await;

    let output = custom_oauth_command(dir.path(), &api.uri())
        .env("CLICKHOUSECTL_AUTH_URL", issuer.uri())
        .env("CLICKHOUSECTL_CLIENT_ID", "client")
        .args(["auth", "login"])
        .output()
        .unwrap();
    assert_success(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not one JSON object ({error}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    assert_eq!(report["verification"], "verified");
    assert!(report["saved"].as_str().unwrap().ends_with("tokens.json"));
    // The code and URL a human must act on are still relayed, on stderr.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("WXYZ-1234"), "{stderr}");
    assert!(
        stderr.contains(verification_uri.to_string_lossy().as_ref()),
        "{stderr}"
    );
}

#[test]
fn logout_in_json_mode_reports_each_credential_source() {
    for (flags, oauth, api_keys) in [
        (&[][..], "cleared", "cleared"),
        (&["--oauth"][..], "cleared", "unchanged"),
        (&["--api-keys"][..], "unchanged", "cleared"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let token_path = dir.path().join("home/.clickhouse/tokens.json");
        let credentials_path = dir.path().join(".clickhouse/credentials.json");
        for path in [&token_path, &credentials_path] {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"{}").unwrap();
        }
        let output = invoke_auth(
            dir.path(),
            "http://127.0.0.1:9",
            true,
            &[&["logout"], flags].concat(),
        );
        assert_success(&output);
        assert!(output.stderr.is_empty(), "{flags:?}: {output:?}");
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            report,
            serde_json::json!({"oauth_tokens": oauth, "api_keys": api_keys}),
            "{flags:?}"
        );
        assert_eq!(token_path.exists(), oauth == "unchanged", "{flags:?}");
        assert_eq!(
            credentials_path.exists(),
            api_keys == "unchanged",
            "{flags:?}"
        );
    }
}

const WHOAMI_USER_ID: &str = "44444444-5555-4666-8777-888888888888";

#[tokio::test]
async fn auth_whoami_renders_both_identities_without_an_organization() {
    let user = serde_json::json!({
        "actorType": "user",
        "userId": WHOAMI_USER_ID,
        "email": "ada@example.com",
        "name": "Ada Lovelace",
        "organizations": [
            {"organizationId": WHOAMI_ORG_ID, "organizationName": "Analytical Engines"}
        ]
    });
    let api_key = serde_json::json!({
        "actorType": "apiKey",
        "keyId": "key-123",
        "name": "ci deploy",
        "organizationId": WHOAMI_ORG_ID
    });
    let future = serde_json::json!({"actorType": "robot", "robotId": "r2"});

    // (identity, oauth, output in JSON, human lines). JSON and human output
    // alternate: both print the identity unchanged.
    for (identity, oauth, json, human_expected) in [
        (
            user.clone(),
            true,
            true,
            vec![
                "actorType: user",
                WHOAMI_USER_ID,
                "ada@example.com",
                "Ada Lovelace",
                WHOAMI_ORG_ID,
                "Analytical Engines",
            ],
        ),
        (
            api_key.clone(),
            false,
            false,
            vec!["actorType: apiKey", "key-123", "ci deploy", WHOAMI_ORG_ID],
        ),
        (
            future.clone(),
            false,
            true,
            vec!["actorType: robot", "robotId: r2"],
        ),
    ] {
        let server = MockServer::start().await;
        let auth = if oauth {
            header("authorization", "Bearer test-bearer-token")
        } else {
            header(
                "authorization",
                "Basic ZHJpZnQta2V5OmRyaWZ0LXNlY3JldA==", // drift-key:drift-secret
            )
        };
        Mock::given(method("GET"))
            .and(path("/v1/whoami"))
            .and(auth)
            .respond_with(whoami_envelope(identity.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let project = tempfile::tempdir().unwrap();

        let output = invoke_cloud_as(&server, project.path(), oauth, json, &["auth", "whoami"]);
        assert_success(&output);
        if json {
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                identity
            );
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for expected in human_expected {
                assert!(stdout.contains(expected), "missing {expected}: {stdout}");
            }
        }

        // No organization lookup or other call: exactly the one whoami request.
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(
            requests
                .iter()
                .all(|request| request.url.path() == "/v1/whoami" && request.url.query().is_none())
        );
    }
}

#[tokio::test]
async fn auth_whoami_auth_failures_exit_4() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "status": 401,
            "requestId": "whoami-401",
            "error": "Invalid credentials"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let project = tempfile::tempdir().unwrap();
    // Human rendering of a 401 is `dispatched_cloud_401_exits_with_auth_required`'s.
    let output = invoke_cloud_as(&server, project.path(), false, true, &["auth", "whoami"]);
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid credentials"), "{stderr}");
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error"]["code"], "auth_required");

    // With no credentials at all, whoami fails as auth-required before any request.
    let empty = MockServer::start().await;
    let output = invoke_cli_without_cloud_credentials(&empty, &["auth".into(), "whoami".into()]);
    assert_eq!(
        output.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(empty.received_requests().await.unwrap().is_empty());
}

// ── auth login / status verify the identity via whoami (#1044) ────────────

fn whoami_rejected() -> ResponseTemplate {
    ResponseTemplate::new(401).set_body_json(serde_json::json!({
        "status": 401,
        "requestId": "whoami-401",
        "error": "Invalid credentials"
    }))
}

/// A URL nothing listens on, so requests fail before any HTTP response.
fn unreachable_url() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

fn invoke_auth(project: &Path, url: &str, json: bool, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project.join("home"))
        .current_dir(project)
        .args(["cloud", "--url", url, "auth"])
        .args(args);
    if json {
        command.arg("--json");
    }
    command.stdin(Stdio::null()).output().unwrap()
}

const LOGIN_ARGS: &[&str] = &[
    "login",
    "--api-key",
    "new-key",
    "--api-secret",
    "new-secret",
];

#[tokio::test]
async fn api_key_login_verifies_and_prints_the_identity_before_saving() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/whoami"))
        .and(header(
            "authorization",
            "Basic bmV3LWtleTpuZXctc2VjcmV0", // new-key:new-secret
        ))
        .respond_with(whoami_envelope(whoami_api_key_identity()))
        .expect(2)
        .mount(&server)
        .await;
    for json in [false, true] {
        let project = tempfile::tempdir().unwrap();
        let output = invoke_auth(project.path(), &server.uri(), json, LOGIN_ARGS);
        assert_success(&output);
        assert!(output.stderr.is_empty(), "json={json}");
        let saved = project.path().join(".clickhouse/credentials.json");
        assert!(saved.exists());
        let stdout = String::from_utf8_lossy(&output.stdout);
        for secret in ["new-key", "new-secret"] {
            assert!(!stdout.contains(secret), "credential exposed: {stdout}");
        }
        if json {
            let report: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(report["verification"], "verified");
            assert_eq!(report["identity"], whoami_api_key_identity());
            assert!(
                report["saved"]
                    .as_str()
                    .unwrap()
                    .ends_with("credentials.json")
            );
            assert!(report.get("warning").is_none());
        } else {
            for expected in [
                ".clickhouse/credentials.json",
                "key-123",
                "ci deploy",
                WHOAMI_ORG_ID,
            ] {
                assert!(stdout.contains(expected), "missing {expected}: {stdout}");
            }
        }
    }
}

#[tokio::test]
async fn api_key_login_rejected_by_whoami_exits_4_and_saves_nothing() {
    // whoami refusing the key with a 403 blocks the save as a 401 does, even
    // though a 403 elsewhere is a request refusal, not auth_required (#1225).
    for status in [401, 403] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/whoami"))
            .respond_with(
                ResponseTemplate::new(status).set_body_json(serde_json::json!({
                    "status": status,
                    "requestId": "whoami-rejected",
                    "error": "Invalid credentials"
                })),
            )
            .expect(1)
            .mount(&server)
            .await;
        let project = tempfile::tempdir().unwrap();
        let output = invoke_auth(project.path(), &server.uri(), true, LOGIN_ARGS);
        assert_eq!(output.status.code(), Some(4), "status {status}");
        assert!(output.stdout.is_empty());
        assert!(!project.path().join(".clickhouse/credentials.json").exists());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Invalid credentials"), "{stderr}");
        let error: Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(error["error"]["code"], "auth_required");
    }
}

#[tokio::test]
async fn api_key_login_saves_with_a_warning_when_whoami_is_unreachable() {
    let url = unreachable_url();
    for json in [false, true] {
        let project = tempfile::tempdir().unwrap();
        let output = invoke_auth(project.path(), &url, json, LOGIN_ARGS);
        assert_success(&output);
        let saved: Value = serde_json::from_slice(
            &std::fs::read(project.path().join(".clickhouse/credentials.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(saved["api_key"], "new-key");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if json {
            assert!(stderr.is_empty(), "{stderr}");
            let report: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(report["verification"], "unverified");
            assert!(report["identity"].is_null());
            assert!(report["warning"].is_string(), "{report}");
        } else {
            assert!(stdout.contains(".clickhouse/credentials.json"), "{stdout}");
            assert!(stderr.starts_with("warning: "), "{stderr}");
        }
    }
}

#[tokio::test]
async fn auth_status_without_credentials_skips_the_identity_check() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let output = invoke_auth(project.path(), &server.uri(), true, &["status"]);
    assert_success(&output);
    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status["verification"], "skipped");
    assert!(status["identity"].is_null());
    assert_eq!(status["sources"].as_array().unwrap().len(), 4);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn auth_status_reports_the_identity_and_never_fails_on_it() {
    let user = serde_json::json!({
        "actorType": "user",
        "userId": WHOAMI_USER_ID,
        "email": "ada@example.com",
        "name": "Ada Lovelace",
        "organizations": [
            {"organizationId": WHOAMI_ORG_ID, "organizationName": "Analytical Engines"}
        ]
    });
    // (response, oauth, JSON output, expected verification, expected human line).
    // Each outcome once; the output format alternates.
    for (response, oauth, json, verification, human) in [
        (
            Some(whoami_envelope(whoami_api_key_identity())),
            false,
            true,
            "verified",
            "key-123",
        ),
        (
            Some(whoami_envelope(user.clone())),
            true,
            false,
            "verified",
            "ada@example.com",
        ),
        (
            Some(whoami_rejected()),
            false,
            true,
            "rejected",
            "Identity: rejected (",
        ),
        (None, false, false, "unavailable", "Identity: unavailable ("),
    ] {
        let server = MockServer::start().await;
        let url = match response {
            Some(response) => {
                Mock::given(method("GET"))
                    .and(path("/v1/whoami"))
                    .respond_with(response)
                    .expect(1)
                    .mount(&server)
                    .await;
                server.uri()
            }
            None => unreachable_url(),
        };
        let project = tempfile::tempdir().unwrap();
        if oauth {
            let cloud_dir = project.path().join("home/.clickhouse");
            std::fs::create_dir_all(&cloud_dir).unwrap();
            write_oauth_tokens(&cloud_dir, &url);
        } else {
            write_project_api_credentials(project.path(), "file-key", "file-secret", &url);
        }
        let output = invoke_auth(project.path(), &url, json, &["status"]);
        assert_success(&output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if json {
            let status: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(status["verification"], verification, "{stdout}");
            assert_eq!(status["identity"].is_null(), verification != "verified");
            assert_eq!(status["warning"].is_string(), verification != "verified");
            let active = status["sources"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["active"] == "yes")
                .count();
            assert_eq!(active, 1);
        } else {
            assert!(stdout.contains(human), "missing {human}: {stdout}");
        }
    }
}
