//! `cloud backup` (src/cloud/backups.rs).

use std::process::Command;

use serde_json::Value;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

// ── Backup configuration validation (issue #425) ────────────────────────────

#[tokio::test]
async fn backup_list_uses_utc_created_times_only_in_human_output() {
    let mock = MockServer::start().await;
    let result = serde_json::json!([
        {
            "id": "11111111-2222-3333-4444-555555555555",
            "sizeInBytes": 1024,
            "startedAt": "2026-09-15T08:34:56.123Z"
        },
        {}
    ]);
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/backups"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-backup-list"
        })))
        .expect(2)
        .mount(&mock)
        .await;

    let args = ["backup", "list", "svc-1", "--org-id", "org-1"];
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert!(stdout.contains("| Created"), "{stdout}");
    assert!(stdout.contains("2026-09-15T08:34:56.123Z"), "{stdout}");
    assert!(!stdout.contains("+00:00"), "{stdout}");
    assert!(
        stdout.lines().any(|line| {
            line.split('|').map(str::trim).collect::<Vec<_>>() == ["", "-", "-", "-", "-", ""]
        }),
        "{stdout}"
    );

    let json = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&json);
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).unwrap(),
        serde_json::json!([
            {
                "id": "11111111-2222-3333-4444-555555555555",
                "sizeInBytes": 1024.0,
                "startedAt": "2026-09-15T08:34:56.123Z"
            },
            {}
        ])
    );
}

// ── Bring-your-own backup buckets (issue #576) ─────────────────────────────

const BACKUP_BUCKET_PATH: &str = "/v1/organizations/org-1/services/svc-1/backupBucket";

#[tokio::test]
async fn backup_bucket_get_uses_oauth_and_preserves_sparse_output() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BACKUP_BUCKET_PATH))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "bucketProvider": "AWS" },
            "status": 200,
            "requestId": "bucket-get",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "backup",
            "bucket",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ])
        .output()
        .expect("run backup bucket get");

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({ "bucketProvider": "AWS" })
    );
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].url.query().is_none());
}

#[tokio::test]
async fn backup_bucket_create_reads_secret_from_stdin_and_sends_exact_gcp_body() {
    // The --config-file and --config aliases are clap's; see
    // `primary_json_file_argument_contract` in `cloud::backups`.
    backup_bucket_create_reads_secret_from_stdin_and_sends_exact_gcp_body_with_file_flag("--file")
        .await;
}

async fn backup_bucket_create_reads_secret_from_stdin_and_sends_exact_gcp_body_with_file_flag(
    file_flag: &str,
) {
    let mock = MockServer::start().await;
    let expected = serde_json::json!({
        "bucketProvider": "GCP",
        "bucketPath": "gs://company-backups/clickhouse",
        "accessKeyId": "gcp-access",
        "secretAccessKey": "gcp-secret",
    });
    Mock::given(method("POST"))
        .and(path(BACKUP_BUCKET_PATH))
        .and(header("authorization", BASIC_TEST_AUTH))
        .and(body_json(expected.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "bucketProvider": "GCP",
                "bucketPath": "gs://company-backups/clickhouse",
                "accessKeyId": "gcp-access"
            },
            "status": 200,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let body = invoke_cli_capture_body_with_stdin(
        &mock,
        &[
            "backup", "bucket", "create", "svc-1", "--org-id", "org-1", file_flag, "-",
        ],
        serde_json::to_string(&expected).unwrap().as_bytes(),
    )
    .await;
    assert_eq!(body, expected);
}

#[tokio::test]
async fn backup_bucket_update_reads_file_and_delete_use_the_service_resource_path() {
    backup_bucket_update_reads_file_and_delete_use_the_service_resource_path_with_file_flag(
        "--file",
    )
    .await;
}

async fn backup_bucket_update_reads_file_and_delete_use_the_service_resource_path_with_file_flag(
    file_flag: &str,
) {
    let mock = MockServer::start().await;
    let expected = serde_json::json!({
        "bucketProvider": "AZURE",
        "containerName": "backups",
        "connectionString": "DefaultEndpointsProtocol=https;AccountName=company",
    });
    Mock::given(method("PATCH"))
        .and(path(BACKUP_BUCKET_PATH))
        .and(header("authorization", BASIC_TEST_AUTH))
        .and(body_json(expected.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": { "bucketProvider": "AZURE", "containerName": "backups" },
            "status": 200,
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(BACKUP_BUCKET_PATH))
        .and(header("authorization", BASIC_TEST_AUTH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "bucket-delete",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let config_dir = tempfile::tempdir().unwrap();
    let config = config_dir.path().join("azure-bucket.json");
    std::fs::write(&config, serde_json::to_vec(&expected).unwrap()).unwrap();
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "backup",
            "bucket",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            file_flag,
            config.to_str().unwrap(),
        ],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({ "bucketProvider": "AZURE", "containerName": "backups" })
    );

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["backup", "bucket", "delete", "svc-1", "--org-id", "org-1"],
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({ "status": 200, "requestId": "bucket-delete" })
    );

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| request.url.query().is_none()));
}

// ── Backup start time against the stored period (issue #642) ────────────────

/// The API keeps the stored period when a start-time update omits one, and
/// rejects the PATCH if that period is not 24 or 48. The CLI reads the
/// configuration first so the user gets a message naming the flag to pass.
const BACKUP_CONFIG_PATH: &str = "/v1/organizations/org-1/services/svc-1/backupConfiguration";

async fn mount_stored_backup_config(mock: &MockServer, period_hours: f64) {
    Mock::given(method("GET"))
        .and(path(BACKUP_CONFIG_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "backupPeriodInHours": period_hours,
                "backupRetentionPeriodInHours": 48.0,
            },
            "status": 200,
            "requestId": "stub-backup-config-get",
        })))
        .mount(mock)
        .await;
}

async fn mount_backup_config_patch(mock: &MockServer) {
    Mock::given(method("PATCH"))
        .and(path(BACKUP_CONFIG_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "backupPeriodInHours": 24.0,
                "backupRetentionPeriodInHours": 48.0,
                "backupStartTime": "02:00",
            },
            "status": 200,
            "requestId": "stub-backup-config-patch",
        })))
        .mount(mock)
        .await;
}

fn backup_config_update_args<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "service",
        "backup-config",
        "update",
        "svc-1",
        "--org-id",
        "org-1",
        "--backup-start-time",
        "02:00",
    ];
    args.extend_from_slice(extra);
    args
}

#[tokio::test]
async fn backup_config_start_time_refuses_an_incompatible_stored_period() {
    let mock = MockServer::start().await;
    mount_stored_backup_config(&mock, 12.0).await;

    let output = invoke_cli_with_cloud_credentials(&mock, &backup_config_update_args(&[]));

    assert_eq!(output.status.code(), Some(1));
    let error = cloud_runtime_error(&output);
    assert_eq!(error["code"], "other");
    // Names the stored period, the flag it blocks and the flag that fixes it.
    let message = error["message"].as_str().unwrap();
    for identifier in ["12", "--backup-start-time", "--backup-period-hours 24"] {
        assert!(message.contains(identifier), "{identifier}: {message}");
    }

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "expected only the configuration read");
    assert_eq!(requests[0].method, wiremock::http::Method::GET);
}

#[tokio::test]
async fn backup_config_start_time_patches_when_the_stored_period_is_compatible() {
    let mock = MockServer::start().await;
    mount_stored_backup_config(&mock, 24.0).await;
    mount_backup_config_patch(&mock).await;

    let output = invoke_cli_with_cloud_credentials(&mock, &backup_config_update_args(&[]));

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, wiremock::http::Method::GET);
    assert_eq!(requests[1].method, wiremock::http::Method::PATCH);

    let body: Value = serde_json::from_slice(&requests[1].body).expect("PATCH body wasn't JSON");
    assert_eq!(body["backupStartTime"], "02:00");
    assert!(
        body.get("backupPeriodInHours").is_none(),
        "the period must stay absent so the API keeps the stored one: {body}"
    );

    let printed: Value = serde_json::from_slice(&output.stdout).expect("stdout wasn't JSON");
    assert_eq!(printed["backupStartTime"], "02:00");
}

#[tokio::test]
async fn backup_config_start_time_with_an_explicit_period_skips_the_read() {
    let mock = MockServer::start().await;
    mount_backup_config_patch(&mock).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &backup_config_update_args(&["--backup-period-hours", "48"]),
    );

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "an explicit period needs no read");
    assert_eq!(requests[0].method, wiremock::http::Method::PATCH);

    let body: Value = serde_json::from_slice(&requests[0].body).expect("PATCH body wasn't JSON");
    assert_eq!(body["backupStartTime"], "02:00");
    assert_eq!(body["backupPeriodInHours"], 48.0);
}

// ── Clearing the backup start time (issue #564) ─────────────────────────────

/// The Cloud API clears a stored start time only on an explicit JSON `null`
/// (an empty string is rejected as an invalid time), and clearing is the only
/// way back to a backup period other than 24 or 48 hours. The flag therefore
/// has to put the key in the body with a null value, and must not be talked
/// out of it by the stored-period read.
async fn mount_cleared_backup_config_patch(mock: &MockServer) {
    Mock::given(method("PATCH"))
        .and(path(BACKUP_CONFIG_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "backupPeriodInHours": 12.0,
                "backupRetentionPeriodInHours": 48.0,
            },
            "status": 200,
            "requestId": "stub-backup-config-clear",
        })))
        .mount(mock)
        .await;
}

#[tokio::test]
async fn backup_config_clear_start_time_sends_an_explicit_null() {
    let mock = MockServer::start().await;
    mount_cleared_backup_config_patch(&mock).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--org-id",
            "org-1",
            "--clear-backup-start-time",
        ],
    );

    assert_success(&output);
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1, "clearing needs no stored-period read");
    assert_eq!(requests[0].method, wiremock::http::Method::PATCH);

    let body: Value = serde_json::from_slice(&requests[0].body).expect("PATCH body wasn't JSON");
    assert_eq!(
        body,
        serde_json::json!({ "backupStartTime": null }),
        "the key must be present and null, not omitted: {body}"
    );
}
