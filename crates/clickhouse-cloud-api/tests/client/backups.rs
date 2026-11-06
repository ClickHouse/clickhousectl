use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{basic_auth, body_json, body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{ok_empty, ok_json, setup};

#[tokio::test]
async fn list_snapshots_pagination_encodes_parameters_and_preserves_envelope() {
    let (server, client) = setup().await;
    let cursor = "opaque +/=&?雪";
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/services/svc/snapshots"))
        .and(basic_auth("key", "secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200.0, "requestId": "req-page", "result": [],
            "limit": 25, "totalCount": 100, "nextCursor": cursor
        })))
        .expect(4)
        .mount(&server)
        .await;
    for (limit, cursor_arg) in [
        (None, None),
        (Some(25), None),
        (None, Some(cursor)),
        (Some(25), Some(cursor)),
    ] {
        let response = client
            .snapshot_get_list("org", "svc", limit, cursor_arg)
            .await
            .unwrap();
        assert_eq!(response.status, Some(200));
        assert_eq!(response.request_id.as_deref(), Some("req-page"));
        assert_eq!(response.result, Some(vec![]));
        assert_eq!(response.error, None);
        assert_eq!(response.limit, Some(25));
        assert_eq!(response.total_count, Some(100));
        assert_eq!(response.next_cursor.as_deref(), Some(cursor));
    }
    let requests = server.received_requests().await.unwrap();
    let queries: Vec<Vec<(String, String)>> = requests
        .iter()
        .map(|request| request.url.query_pairs().into_owned().collect())
        .collect();
    assert_eq!(
        queries,
        vec![
            vec![],
            vec![("limit".into(), "25".into())],
            vec![("cursor".into(), cursor.into())],
            vec![
                ("limit".into(), "25".into()),
                ("cursor".into(), cursor.into())
            ],
        ]
    );
}

// ===========================================================================
// Backups & Backup Configuration
// ===========================================================================

#[tokio::test]
async fn get_snapshot_configuration() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/snapshotConfiguration",
        ))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({
            "enabled": true, "gap": 30, "timeFrame": 1440
        })))
        .expect(1)
        .mount(&server)
        .await;

    let configuration = client
        .snapshot_configuration_get("org-1", "svc-1")
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(configuration.enabled, Some(true));
    assert_eq!(configuration.gap, Some(30.0));
    assert_eq!(configuration.time_frame, Some(1440.0));
}

#[tokio::test]
async fn update_snapshot_configuration_sends_supported_presets() {
    for (gap, time_frame) in [(30.0, 1440.0), (60.0, 2880.0)] {
        let (server, client) = setup().await;
        let wire = serde_json::json!({
            "enabled": true, "gap": gap, "timeFrame": time_frame
        });
        Mock::given(method("PATCH"))
            .and(path(
                "/v1/organizations/org-1/services/svc-1/snapshotConfiguration",
            ))
            .and(basic_auth("key", "secret"))
            .and(body_json(wire.clone()))
            .respond_with(ok_json(wire))
            .expect(1)
            .mount(&server)
            .await;

        let configuration = client
            .snapshot_configuration_update(
                "org-1",
                "svc-1",
                &SnapshotConfigurationPatchRequest {
                    enabled: Some(true),
                    gap: Some(gap),
                    time_frame: Some(time_frame),
                },
            )
            .await
            .unwrap()
            .result
            .unwrap();
        assert_eq!(configuration.enabled, Some(true));
        assert_eq!(configuration.gap, Some(gap));
        assert_eq!(configuration.time_frame, Some(time_frame));
    }
}

#[tokio::test]
async fn update_snapshot_configuration_omits_unchanged_fields() {
    let (server, client) = setup().await;
    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/snapshotConfiguration",
        ))
        .and(basic_auth("key", "secret"))
        .and(body_json(serde_json::json!({"enabled": false})))
        .respond_with(ok_json(serde_json::json!({"enabled": false})))
        .expect(1)
        .mount(&server)
        .await;

    let configuration = client
        .snapshot_configuration_update(
            "org-1",
            "svc-1",
            &SnapshotConfigurationPatchRequest {
                enabled: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(configuration.enabled, Some(false));
    assert_eq!(configuration.gap, None);
    assert_eq!(configuration.time_frame, None);
}

#[tokio::test]
async fn update_snapshot_configuration_leaves_validation_to_the_server() {
    for (request, wire, message) in [
        (
            SnapshotConfigurationPatchRequest::default(),
            serde_json::json!({}),
            "Provide at least one field",
        ),
        (
            SnapshotConfigurationPatchRequest {
                enabled: Some(true),
                gap: Some(0.0),
                time_frame: Some(0.0),
            },
            serde_json::json!({"enabled": true, "gap": 0.0, "timeFrame": 0.0}),
            "Unsupported snapshot cadence",
        ),
    ] {
        let (server, client) = setup().await;
        Mock::given(method("PATCH"))
            .and(path(
                "/v1/organizations/org-1/services/svc-1/snapshotConfiguration",
            ))
            .and(basic_auth("key", "secret"))
            .and(body_json(wire))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "status": 400, "error": message
            })))
            .expect(1)
            .mount(&server)
            .await;

        let error = client
            .snapshot_configuration_update("org-1", "svc-1", &request)
            .await
            .unwrap_err();
        assert!(
            matches!(error, clickhouse_cloud_api::Error::Api { status: 400, message: actual } if actual == message)
        );
    }
}

#[tokio::test]
async fn snapshot_configuration_methods_preserve_api_errors() {
    for (status, response, message) in [
        (
            403,
            ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "status": 403, "error": "Access denied"
            })),
            "Access denied",
        ),
        (
            500,
            ResponseTemplate::new(500).set_body_string("Snapshot configuration unavailable"),
            "Snapshot configuration unavailable",
        ),
    ] {
        let (server, client) = setup().await;
        Mock::given(path(
            "/v1/organizations/org-1/services/svc-1/snapshotConfiguration",
        ))
        .respond_with(response)
        .expect(2)
        .mount(&server)
        .await;

        let errors = [
            client
                .snapshot_configuration_get("org-1", "svc-1")
                .await
                .unwrap_err(),
            client
                .snapshot_configuration_update(
                    "org-1",
                    "svc-1",
                    &SnapshotConfigurationPatchRequest {
                        enabled: Some(false),
                        ..Default::default()
                    },
                )
                .await
                .unwrap_err(),
        ];
        for error in errors {
            assert!(
                matches!(error, clickhouse_cloud_api::Error::Api { status: actual_status, message: actual_message } if actual_status == status && actual_message == message)
            );
        }
    }
}

#[tokio::test]
async fn snapshot_configuration_methods_reject_malformed_success_payloads() {
    let (server, client) = setup().await;
    Mock::given(path(
        "/v1/organizations/org-1/services/svc-1/snapshotConfiguration",
    ))
    .respond_with(ResponseTemplate::new(200).set_body_string("not JSON"))
    .expect(2)
    .mount(&server)
    .await;

    assert!(matches!(
        client.snapshot_configuration_get("org-1", "svc-1").await,
        Err(clickhouse_cloud_api::Error::Json(_))
    ));
    assert!(matches!(
        client
            .snapshot_configuration_update(
                "org-1",
                "svc-1",
                &SnapshotConfigurationPatchRequest {
                    enabled: Some(false),
                    ..Default::default()
                },
            )
            .await,
        Err(clickhouse_cloud_api::Error::Json(_))
    ));
}

#[tokio::test]
async fn list_snapshots() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/snapshots"))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!([
            {"status": "throttled", "type": "full", "durationInSeconds": 1.5},
            {"status": "done", "type": "full", "sizeInBytes": 2048}
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let snapshots = client
        .snapshot_get_list("org-1", "svc-1", None, None)
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].status, Some(SnapshotStatus::Throttled));
    assert_eq!(snapshots[0].r#type, Some(SnapshotType::Full));
    assert_eq!(snapshots[0].duration_in_seconds, Some(1.5));
    assert_eq!(snapshots[1].status, Some(SnapshotStatus::Done));
}

#[tokio::test]
async fn get_snapshot() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/snapshots/snapshot-1",
        ))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "serviceId": "svc-1",
            "status": "done",
            "type": "full",
            "sizeInBytes": 2048,
            "bucket": {"bucketProvider": "AWS", "bucketPath": "s3://backups"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let snapshot = client
        .snapshot_get("org-1", "svc-1", "snapshot-1")
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(
        snapshot.id.unwrap().to_string(),
        "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    );
    assert_eq!(snapshot.service_id.as_deref(), Some("svc-1"));
    assert_eq!(snapshot.size_in_bytes, Some(2048.0));
    assert_eq!(snapshot.bucket.unwrap()["bucketPath"], "s3://backups");
}

#[tokio::test]
async fn snapshot_methods_preserve_api_errors() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/snapshots"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "status": 403, "error": "Access denied"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/snapshots/missing",
        ))
        .respond_with(ResponseTemplate::new(500).set_body_string("Snapshot lookup failed"))
        .expect(1)
        .mount(&server)
        .await;

    let list_error = client
        .snapshot_get_list("org-1", "svc-1", None, None)
        .await
        .unwrap_err();
    assert!(
        matches!(list_error, clickhouse_cloud_api::Error::Api { status: 403, message } if message == "Access denied")
    );
    let get_error = client
        .snapshot_get("org-1", "svc-1", "missing")
        .await
        .unwrap_err();
    assert!(
        matches!(get_error, clickhouse_cloud_api::Error::Api { status: 500, message } if message == "Snapshot lookup failed")
    );
}

#[tokio::test]
async fn snapshot_methods_reject_malformed_success_payloads() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not JSON"))
        .expect(2)
        .mount(&server)
        .await;
    assert!(matches!(
        client.snapshot_get_list("org-1", "svc-1", None, None).await,
        Err(clickhouse_cloud_api::Error::Json(_))
    ));
    assert!(matches!(
        client.snapshot_get("org-1", "svc-1", "snapshot-1").await,
        Err(clickhouse_cloud_api::Error::Json(_))
    ));
}

#[tokio::test]
async fn list_backups() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/backups"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": [
                {
                    "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                    "status": "done",
                    "serviceId": "svc-1",
                    "startedAt": "2024-06-01T00:00:00Z",
                    "finishedAt": "2024-06-01T00:05:00Z",
                    "sizeInBytes": 1024,
                    "durationInSeconds": 300,
                    "type": "full"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client.backup_get_list("org-1", "svc-1").await.unwrap();
    let backups = resp.result.unwrap();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].status, Some(BackupStatus::Done));
    assert_eq!(backups[0].r#type, Some(BackupType::Full));
}

#[tokio::test]
async fn get_single_backup() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/backups/bak-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "status": "done",
            "sizeInBytes": 2048,
            "type": "full"
        })))
        .mount(&s)
        .await;

    let resp = c.backup_get("org-1", "svc-1", "bak-1").await.unwrap();
    let backup = resp.result.unwrap();
    assert_eq!(backup.status, Some(BackupStatus::Done));
    assert_eq!(backup.size_in_bytes, Some(2048.0));
}

#[tokio::test]
async fn get_backup_configuration() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/backupConfiguration",
        ))
        .respond_with(ok_json(serde_json::json!({
            "backupPeriodInHours": 24,
            "backupRetentionPeriodInHours": 168,
            "backupStartTime": "02:00"
        })))
        .mount(&s)
        .await;

    let resp = c.backup_configuration_get("org-1", "svc-1").await.unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.backup_period_in_hours, Some(24.0));
    assert_eq!(config.backup_start_time.as_deref(), Some("02:00"));
}

#[tokio::test]
async fn update_backup_configuration() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/backupConfiguration",
        ))
        .and(body_partial_json(serde_json::json!({"backupPeriodInHours": 12.0, "backupRetentionPeriodInHours": 336.0, "backupStartTime": "03:00"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "backupPeriodInHours": 12,
                "backupRetentionPeriodInHours": 336,
                "backupStartTime": "03:00"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let body = BackupConfigurationPatchRequest {
        backup_period_in_hours: Some(12.0),
        backup_retention_period_in_hours: Some(336.0),
        backup_start_time: Some(Some("03:00".to_string())),
    };
    let resp = client
        .backup_configuration_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.backup_period_in_hours, Some(12.0));
}

// ===========================================================================
// Backup Buckets
// ===========================================================================

#[tokio::test]
async fn get_backup_bucket() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/backupBucket"))
        .respond_with(ok_json(serde_json::json!({
            "bucketPath": "s3://my-backup-bucket/prefix",
            "bucketProvider": "AWS",
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "iamRoleArn": "arn:aws:iam::123:role/backup-role",
            "iamRoleExternalId": "trust-policy-id"
        })))
        .mount(&s)
        .await;

    let resp = c.backup_bucket_get("org-1", "svc-1").await.unwrap();
    let Some(BackupBucket::AwsBackupBucket(bucket)) = resp.result else {
        panic!("expected AWS bucket");
    };
    assert_eq!(
        bucket.iam_role_external_id.as_deref(),
        Some("trust-policy-id")
    );
}

#[tokio::test]
async fn create_backup_bucket() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services/svc-1/backupBucket"))
        .and(body_partial_json(
            serde_json::json!({"bucketPath": "s3://new-bucket", "iamRoleExternalId": "trust-policy-id"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "bucketPath": "s3://new-bucket",
            "bucketProvider": "AWS",
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        })))
        .mount(&s)
        .await;

    let body =
        BackupBucketPostRequest::AwsBackupBucketPostRequestV1(AwsBackupBucketPostRequestV1 {
            bucket_path: "s3://new-bucket".to_string(),
            iam_role_external_id: "trust-policy-id".to_string(),
            ..Default::default()
        });
    let resp = c
        .backup_bucket_create("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_backup_bucket() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/backupBucket"))
        .and(body_partial_json(
            serde_json::json!({"bucketPath": "s3://updated-bucket", "iamRoleExternalId": null}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "bucketPath": "s3://updated-bucket",
            "bucketProvider": "AWS",
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        })))
        .mount(&s)
        .await;

    let body =
        BackupBucketPatchRequest::AwsBackupBucketPatchRequestV1(AwsBackupBucketPatchRequestV1 {
            bucket_path: "s3://updated-bucket".to_string(),
            iam_role_external_id: Some(None),
            ..Default::default()
        });
    let resp = c
        .backup_bucket_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn delete_backup_bucket() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/svc-1/backupBucket"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.backup_bucket_delete("org-1", "svc-1").await.unwrap();
    assert_eq!(resp.status, Some(200));
}
