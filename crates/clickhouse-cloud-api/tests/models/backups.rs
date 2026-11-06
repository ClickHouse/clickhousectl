use clickhouse_cloud_api::models::*;

use crate::assert_unknown_variant_round_trips;

#[test]
fn aws_backup_external_id_response_tolerates_absence_and_null() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({"iamRoleExternalId": null}),
    ] {
        let response: AwsBackupBucket = serde_json::from_value(wire).unwrap();
        assert_eq!(response.iam_role_external_id, None);
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({})
        );
    }
    let response: AwsBackupBucket = serde_json::from_value(
        serde_json::json!({"iamRoleExternalId": "trust-policy-id", "futureField": 1}),
    )
    .unwrap();
    assert_eq!(
        response.iam_role_external_id.as_deref(),
        Some("trust-policy-id")
    );
}

#[test]
fn aws_backup_external_id_create_and_properties_are_strict() {
    let mut wire = serde_json::json!({
        "bucketProvider": "AWS", "bucketPath": "s3://backups",
        "iamRoleArn": "arn:aws:iam::123:role/backup", "iamRoleSessionName": "backups",
        "iamRoleExternalId": "trust-policy-id"
    });
    let request: AwsBackupBucketPostRequestV1 = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(request.iam_role_external_id, "trust-policy-id");
    assert_eq!(serde_json::to_value(request).unwrap(), wire);
    let properties: AwsBackupBucketProperties = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(properties.iam_role_external_id, "trust-policy-id");
    wire.as_object_mut().unwrap().remove("iamRoleExternalId");
    assert!(serde_json::from_value::<AwsBackupBucketPostRequestV1>(wire.clone()).is_err());
    assert!(serde_json::from_value::<AwsBackupBucketProperties>(wire.clone()).is_err());
    wire["iamRoleExternalId"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<AwsBackupBucketPostRequestV1>(wire.clone()).is_err());
    assert!(serde_json::from_value::<AwsBackupBucketProperties>(wire).is_err());
}

#[test]
fn aws_backup_external_id_update_preserves_omit_set_and_clear() {
    for external_id in [
        None,
        Some(serde_json::json!("trust-policy-id")),
        Some(serde_json::Value::Null),
    ] {
        let mut wire = serde_json::json!({
            "bucketProvider": "AWS", "bucketPath": "s3://backups",
            "iamRoleArn": "arn:aws:iam::123:role/backup"
        });
        if let Some(value) = &external_id {
            wire["iamRoleExternalId"] = value.clone();
        }
        let request: AwsBackupBucketPatchRequestV1 = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(
            request.iam_role_external_id,
            external_id.map(|value| value.as_str().map(str::to_owned))
        );
        assert_eq!(serde_json::to_value(request).unwrap(), wire);
    }
    for wire in [
        r#"{"bucketProvider":"AWS","bucketPath":"s3://backups","iamRoleArn":"arn","iamRoleExternalId":42}"#,
        r#"{"bucketProvider":"AWS","bucketPath":"s3://backups","iamRoleArn":"arn","iamRoleExternalId":null,"iamRoleExternalId":"id"}"#,
    ] {
        assert!(serde_json::from_str::<AwsBackupBucketPatchRequestV1>(wire).is_err());
    }
    assert!(
        serde_json::from_value::<AwsBackupBucketPatchRequestV1>(serde_json::json!({})).is_err()
    );
}

#[test]
fn snapshot_configuration_tolerates_missing_null_and_extra_fields() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "enabled": null, "gap": null, "timeFrame": null, "futureProperty": 1
        }),
    ] {
        let configuration: SnapshotConfiguration = serde_json::from_value(wire).unwrap();
        assert_eq!(configuration, SnapshotConfiguration::default());
        assert_eq!(
            serde_json::to_value(configuration).unwrap(),
            serde_json::json!({})
        );
    }
}

#[test]
fn snapshot_configuration_preserves_false_zero_and_fractional_numbers() {
    let configuration: SnapshotConfiguration = serde_json::from_value(serde_json::json!({
        "enabled": false, "gap": 0, "timeFrame": 1440.5
    }))
    .unwrap();
    assert_eq!(configuration.enabled, Some(false));
    assert_eq!(configuration.gap, Some(0.0));
    assert_eq!(configuration.time_frame, Some(1440.5));
    assert_eq!(
        serde_json::to_value(configuration).unwrap(),
        serde_json::json!({"enabled": false, "gap": 0.0, "timeFrame": 1440.5})
    );
}

#[test]
fn snapshot_configuration_patch_omits_none_and_preserves_provided_values() {
    for (request, expected) in [
        (
            SnapshotConfigurationPatchRequest::default(),
            serde_json::json!({}),
        ),
        (
            SnapshotConfigurationPatchRequest {
                enabled: Some(false),
                ..Default::default()
            },
            serde_json::json!({"enabled": false}),
        ),
        (
            SnapshotConfigurationPatchRequest {
                enabled: None,
                gap: Some(0.0),
                time_frame: Some(1440.5),
            },
            serde_json::json!({"gap": 0.0, "timeFrame": 1440.5}),
        ),
        (
            SnapshotConfigurationPatchRequest {
                enabled: Some(true),
                gap: Some(30.0),
                time_frame: Some(1440.0),
            },
            serde_json::json!({"enabled": true, "gap": 30.0, "timeFrame": 1440.0}),
        ),
    ] {
        let serialized = serde_json::to_value(&request).unwrap();
        assert_eq!(serialized, expected);
        assert_eq!(
            serde_json::from_value::<SnapshotConfigurationPatchRequest>(serialized).unwrap(),
            request
        );
    }
}

#[test]
fn snapshot_round_trips_complete_response_and_unknown_bucket_properties() {
    let value = serde_json::json!({
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "status": "done", "type": "full", "serviceId": "svc-1",
        "startedAt": "2026-09-15T01:00:00Z", "finishedAt": "2026-09-15T01:00:01Z",
        "sizeInBytes": 2048.0, "durationInSeconds": 1.5, "backupName": "snapshot-1",
        "bucket": {"bucketProvider": "FUTURE", "newProperty": {"region": "test"}}
    });
    let snapshot: Snapshot = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(snapshot.status, Some(SnapshotStatus::Done));
    assert_eq!(snapshot.r#type, Some(SnapshotType::Full));
    assert_eq!(snapshot.duration_in_seconds, Some(1.5));
    assert_eq!(serde_json::to_value(snapshot).unwrap(), value);
}

#[test]
fn snapshot_tolerates_missing_null_and_extra_fields() {
    let dropped: Snapshot = serde_json::from_str("{}").unwrap();
    let nulled: Snapshot = serde_json::from_value(serde_json::json!({
        "id": null, "status": null, "type": null, "serviceId": null,
        "startedAt": null, "finishedAt": null, "sizeInBytes": null,
        "durationInSeconds": null, "backupName": null, "bucket": null,
        "futureProperty": 1
    }))
    .unwrap();
    assert_eq!(dropped, Snapshot::default());
    assert_eq!(nulled, Snapshot::default());
    assert_eq!(serde_json::to_value(nulled).unwrap(), serde_json::json!({}));
}

#[test]
fn snapshot_enums_preserve_known_and_unknown_values() {
    for (wire, expected) in [
        ("done", SnapshotStatus::Done),
        ("error", SnapshotStatus::Error),
        ("in_progress", SnapshotStatus::InProgress),
        ("throttled", SnapshotStatus::Throttled),
        ("queued", SnapshotStatus::Unknown("queued".to_string())),
    ] {
        let parsed: SnapshotStatus = serde_json::from_value(serde_json::json!(wire)).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.to_string(), wire);
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
    }
    for (wire, expected) in [
        ("full", SnapshotType::Full),
        ("future", SnapshotType::Unknown("future".to_string())),
    ] {
        let parsed: SnapshotType = serde_json::from_value(serde_json::json!(wire)).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.to_string(), wire);
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
    }
}

#[test]
fn deserialize_backup() {
    let json = r#"{
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "status": "done",
        "serviceId": "11111111-2222-3333-4444-555555555555",
        "startedAt": "2024-06-01T02:00:00Z",
        "finishedAt": "2024-06-01T02:05:00Z",
        "sizeInBytes": 1073741824,
        "durationInSeconds": 300,
        "type": "full",
        "backupName": "backup-2024-06-01"
    }"#;
    let backup: Backup = serde_json::from_str(json).unwrap();
    assert_eq!(backup.status, Some(BackupStatus::Done));
    assert_eq!(backup.r#type, Some(BackupType::Full));
    assert_eq!(backup.size_in_bytes, Some(1073741824.0));
    assert_eq!(backup.duration_in_seconds, Some(300.0));
}

#[test]
fn deserialize_backup_configuration() {
    let json = r#"{
        "backupPeriodInHours": 24,
        "backupRetentionPeriodInHours": 168,
        "backupStartTime": "02:00"
    }"#;
    let config: BackupConfiguration = serde_json::from_str(json).unwrap();
    assert_eq!(config.backup_period_in_hours, Some(24.0));
    assert_eq!(config.backup_retention_period_in_hours, Some(168.0));
    assert_eq!(config.backup_start_time.as_deref(), Some("02:00"));
}

#[test]
fn serialize_backup_configuration_patch_request() {
    let req = BackupConfigurationPatchRequest {
        backup_period_in_hours: Some(12.0),
        backup_retention_period_in_hours: Some(336.0),
        backup_start_time: Some(Some("03:00".to_string())),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["backupPeriodInHours"], 12.0);
    assert_eq!(json["backupRetentionPeriodInHours"], 336.0);
    assert_eq!(json["backupStartTime"], "03:00");
}

/// The three states of `backupStartTime` are what let a caller both set and
/// unset the start time: only an explicit `null` clears the stored value, and
/// omitting the key must leave it untouched.
#[test]
fn serialize_backup_configuration_patch_request_omits_an_untouched_start_time() {
    let req = BackupConfigurationPatchRequest {
        backup_period_in_hours: Some(12.0),
        backup_retention_period_in_hours: None,
        backup_start_time: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json, serde_json::json!({ "backupPeriodInHours": 12.0 }));
}

#[test]
fn serialize_backup_configuration_patch_request_clears_the_start_time_with_null() {
    let req = BackupConfigurationPatchRequest {
        backup_period_in_hours: Some(12.0),
        backup_retention_period_in_hours: None,
        backup_start_time: Some(None),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "backupPeriodInHours": 12.0, "backupStartTime": null })
    );
}

#[test]
fn backup_ignores_extra_fields() {
    let json = r#"{"status":"done","type":"full","compressionRatio":0.85}"#;
    let backup: Backup = serde_json::from_str(json).unwrap();
    assert_eq!(backup.status, Some(BackupStatus::Done));
}

#[test]
fn backup_configuration_ignores_extra_fields() {
    let json = r#"{"backupPeriodInHours":24,"backupRetentionPeriodInHours":168,"compressionEnabled":true}"#;
    let c: BackupConfiguration = serde_json::from_str(json).unwrap();
    assert_eq!(c.backup_period_in_hours, Some(24.0));
}

#[test]
fn backup_minimal_response() {
    let b: Backup = serde_json::from_str("{}").unwrap();
    assert_eq!(b.id, None);
    assert_eq!(b.status, None);
    assert_eq!(b.size_in_bytes, None);
}

#[test]
fn deserialize_aws_backup_bucket() {
    let json = r#"{
        "bucketPath": "s3://my-bucket/prefix",
        "bucketProvider": "AWS",
        "iamRoleArn": "arn:aws:iam::123:role/backup",
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    }"#;
    let b: AwsBackupBucket = serde_json::from_str(json).unwrap();
    assert_eq!(b.bucket_path.as_deref(), Some("s3://my-bucket/prefix"));
    assert_eq!(
        b.iam_role_arn.as_deref(),
        Some("arn:aws:iam::123:role/backup")
    );
}

#[test]
fn deserialize_backup_bucket_dispatches_aws() {
    let json = r#"{
        "bucketPath": "s3://my-bucket/prefix",
        "bucketProvider": "AWS",
        "iamRoleArn": "arn:aws:iam::123:role/backup",
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    }"#;
    let b: BackupBucket = serde_json::from_str(json).unwrap();
    assert!(matches!(b, BackupBucket::AwsBackupBucket(_)));
    if let BackupBucket::AwsBackupBucket(aws) = b {
        assert_eq!(aws.bucket_path.as_deref(), Some("s3://my-bucket/prefix"));
        assert_eq!(
            aws.iam_role_arn.as_deref(),
            Some("arn:aws:iam::123:role/backup")
        );
    }
}

#[test]
fn deserialize_backup_bucket_dispatches_gcp() {
    let json = r#"{
        "accessKeyId": "GOOG1234567890",
        "bucketPath": "gs://my-gcp-bucket/prefix",
        "bucketProvider": "GCP",
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    }"#;
    let b: BackupBucket = serde_json::from_str(json).unwrap();
    assert!(matches!(b, BackupBucket::GcpBackupBucket(_)));
    if let BackupBucket::GcpBackupBucket(gcp) = b {
        assert_eq!(gcp.access_key_id.as_deref(), Some("GOOG1234567890"));
        assert_eq!(
            gcp.bucket_path.as_deref(),
            Some("gs://my-gcp-bucket/prefix")
        );
    }
}

#[test]
fn deserialize_backup_bucket_dispatches_azure() {
    let json = r#"{
        "bucketProvider": "AZURE",
        "containerName": "my-container",
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    }"#;
    let b: BackupBucket = serde_json::from_str(json).unwrap();
    assert!(matches!(b, BackupBucket::AzureBackupBucket(_)));
    if let BackupBucket::AzureBackupBucket(azure) = b {
        assert_eq!(azure.container_name.as_deref(), Some("my-container"));
    }
}

#[test]
fn deserialize_backup_bucket_unknown_provider() {
    // The Unknown payload round-trips losslessly as the original object, not a
    // JSON string.
    let json = r#"{
        "bucketProvider": "NEW_PROVIDER",
        "somefield": "somevalue",
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    }"#;
    assert_unknown_variant_round_trips(json, |b: &BackupBucket| {
        matches!(b, BackupBucket::Unknown(_))
    });
}

#[test]
fn backup_bucket_patch_request_unknown_provider_round_trips() {
    let json = r#"{
        "bucketProvider": "NEW_PROVIDER",
        "somefield": "somevalue"
    }"#;
    assert_unknown_variant_round_trips(json, |b: &BackupBucketPatchRequest| {
        matches!(b, BackupBucketPatchRequest::Unknown(_))
    });
}

#[test]
fn backup_bucket_post_request_unknown_provider_round_trips() {
    let json = r#"{
        "bucketProvider": "NEW_PROVIDER",
        "somefield": "somevalue"
    }"#;
    assert_unknown_variant_round_trips(json, |b: &BackupBucketPostRequest| {
        matches!(b, BackupBucketPostRequest::Unknown(_))
    });
}

#[test]
fn backup_bucket_properties_unknown_provider_round_trips() {
    let json = r#"{
        "bucketProvider": "NEW_PROVIDER",
        "somefield": "somevalue"
    }"#;
    assert_unknown_variant_round_trips(json, |b: &BackupBucketProperties| {
        matches!(b, BackupBucketProperties::Unknown(_))
    });
}

#[test]
fn backup_bucket_unknown_display_emits_compact_json() {
    let json = r#"{"bucketProvider":"NEW_PROVIDER","somefield":"somevalue"}"#;
    let b: BackupBucket = serde_json::from_str(json).unwrap();
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    // Display emits the raw compact-JSON payload carried by the Unknown variant.
    assert_eq!(b.to_string(), value.to_string());
}
