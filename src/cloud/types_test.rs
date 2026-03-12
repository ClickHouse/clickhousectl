use super::types::*;

// ── Response deserialization tests ──────────────────────────────────

#[test]
fn test_organization_deserialize() {
    let json = r#"{"id":"org-1","name":"My Org","createdAt":"2024-01-01T00:00:00Z"}"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.id, "org-1");
    assert_eq!(org.name, "My Org");
    assert_eq!(org.created_at.as_deref(), Some("2024-01-01T00:00:00Z"));
}

#[test]
fn test_organization_deserialize_minimal() {
    let json = r#"{"id":"org-1","name":"My Org"}"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.id, "org-1");
    assert!(org.created_at.is_none());
    assert!(org.private_endpoints.is_none());
    assert!(org.enable_core_dumps.is_none());
}

#[test]
fn test_organization_with_new_fields() {
    let json = serde_json::json!({
        "id": "org-1",
        "name": "My Org",
        "createdAt": "2024-01-01T00:00:00Z",
        "privateEndpoints": [{"id": "pe-1", "description": "VPC endpoint", "cloudProvider": "aws", "region": "us-east-1"}],
        "enableCoreDumps": true
    });
    let org: Organization = serde_json::from_value(json).unwrap();
    let pe = &org.private_endpoints.as_ref().unwrap()[0];
    assert_eq!(pe.id.as_deref(), Some("pe-1"));
    assert_eq!(pe.description.as_deref(), Some("VPC endpoint"));
    assert_eq!(pe.cloud_provider.as_deref(), Some("aws"));
    assert_eq!(pe.region.as_deref(), Some("us-east-1"));
    assert_eq!(org.enable_core_dumps, Some(true));
}

#[test]
fn test_organization_roundtrip() {
    let org = Organization {
        id: "org-1".to_string(),
        name: "My Org".to_string(),
        created_at: Some("2024-01-01T00:00:00Z".to_string()),
        private_endpoints: None,
        enable_core_dumps: None,
    };
    let json = serde_json::to_string(&org).unwrap();
    let org2: Organization = serde_json::from_str(&json).unwrap();
    assert_eq!(org.id, org2.id);
    assert_eq!(org.name, org2.name);
    assert_eq!(org.created_at, org2.created_at);
}

#[test]
fn test_service_deserialize_full() {
    let json = serde_json::json!({
        "id": "svc-1",
        "name": "my-service",
        "provider": "aws",
        "region": "us-east-1",
        "state": "running",
        "tier": "production",
        "minTotalMemoryGb": 72,
        "maxTotalMemoryGb": 144,
        "idleScaling": true,
        "idleTimeoutMinutes": 10,
        "ipAccessList": [{"source": "0.0.0.0/0", "description": "Allow all"}],
        "createdAt": "2024-01-01T00:00:00Z",
        "endpoints": [{"protocol": "https", "host": "abc.clickhouse.cloud", "port": 8443}],
        "minReplicaMemoryGb": 24,
        "maxReplicaMemoryGb": 48,
        "numReplicas": 3,
        "clickhouseVersion": "24.3.1",
        "releaseChannel": "default",
        "encryptionKey": "key-1",
        "iamRole": "arn:aws:iam::123:role/test",
        "privateEndpointIds": ["pe-1", "pe-2"],
        "availablePrivateEndpointIds": ["pe-3"],
        "dataWarehouseId": "dw-1",
        "isPrimary": true,
        "isReadonly": false,
        "hasTransparentDataEncryption": false,
        "profile": "v1-default",
        "transparentDataEncryptionKeyId": "tde-1",
        "encryptionRoleId": "role-1",
        "complianceType": "hipaa",
        "tags": [{"key": "env", "value": "prod"}],
        "enableCoreDumps": true
    });
    let svc: Service = serde_json::from_value(json).unwrap();
    assert_eq!(svc.id, "svc-1");
    assert_eq!(svc.tier.as_deref(), Some("production"));
    assert_eq!(svc.min_total_memory_gb, Some(72));
    assert_eq!(svc.max_total_memory_gb, Some(144));
    assert_eq!(svc.idle_scaling, Some(true));
    assert_eq!(svc.idle_timeout_minutes, Some(10));
    assert_eq!(svc.ip_access_list.as_ref().unwrap().len(), 1);
    assert_eq!(svc.ip_access_list.as_ref().unwrap()[0].source, "0.0.0.0/0");
    assert_eq!(svc.endpoints.as_ref().unwrap()[0].port, 8443);
    assert_eq!(svc.min_replica_memory_gb, Some(24));
    assert_eq!(svc.max_replica_memory_gb, Some(48));
    assert_eq!(svc.num_replicas, Some(3));
    assert_eq!(svc.clickhouse_version.as_deref(), Some("24.3.1"));
    assert_eq!(svc.release_channel.as_deref(), Some("default"));
    assert_eq!(svc.encryption_key.as_deref(), Some("key-1"));
    assert_eq!(svc.iam_role.as_deref(), Some("arn:aws:iam::123:role/test"));
    assert_eq!(svc.private_endpoint_ids.as_ref().unwrap().len(), 2);
    assert_eq!(svc.data_warehouse_id.as_deref(), Some("dw-1"));
    assert_eq!(svc.is_primary, Some(true));
    assert_eq!(svc.is_readonly, Some(false));
    assert_eq!(svc.has_transparent_data_encryption, Some(false));
    assert_eq!(svc.profile.as_deref(), Some("v1-default"));
    assert_eq!(svc.transparent_data_encryption_key_id.as_deref(), Some("tde-1"));
    assert_eq!(svc.encryption_role_id.as_deref(), Some("role-1"));
    assert_eq!(svc.compliance_type.as_deref(), Some("hipaa"));
    assert_eq!(svc.tags.as_ref().unwrap()[0].key, "env");
    assert_eq!(svc.enable_core_dumps, Some(true));
}

#[test]
fn test_service_deserialize_minimal() {
    let json = r#"{"id":"svc-1","name":"svc","provider":"aws","region":"us-east-1","state":"idle"}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.id, "svc-1");
    assert!(svc.tier.is_none());
    assert!(svc.min_total_memory_gb.is_none());
    assert!(svc.max_total_memory_gb.is_none());
    assert!(svc.endpoints.is_none());
    assert!(svc.clickhouse_version.is_none());
    assert!(svc.tags.is_none());
}

#[test]
fn test_endpoint_deserialize() {
    let json = r#"{"protocol":"https","host":"abc.clickhouse.cloud","port":8443}"#;
    let ep: Endpoint = serde_json::from_str(json).unwrap();
    assert_eq!(ep.protocol, "https");
    assert_eq!(ep.host, "abc.clickhouse.cloud");
    assert_eq!(ep.port, 8443);
    assert!(ep.username.is_none());
}

#[test]
fn test_endpoint_with_username() {
    let json = serde_json::json!({
        "protocol": "mysql",
        "host": "abc.clickhouse.cloud",
        "port": 3306,
        "username": "default"
    });
    let ep: Endpoint = serde_json::from_value(json).unwrap();
    assert_eq!(ep.protocol, "mysql");
    assert_eq!(ep.username.as_deref(), Some("default"));
}

#[test]
fn test_ip_access_entry_roundtrip() {
    let entry = IpAccessEntry {
        source: "10.0.0.0/8".to_string(),
        description: Some("Internal".to_string()),
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("\"source\":\"10.0.0.0/8\""));
    assert!(json.contains("\"description\":\"Internal\""));

    let entry2: IpAccessEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(entry.source, entry2.source);
    assert_eq!(entry.description, entry2.description);
}

#[test]
fn test_ip_access_entry_skip_none_description() {
    let entry = IpAccessEntry {
        source: "0.0.0.0/0".to_string(),
        description: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(!json.contains("description"));
}

#[test]
fn test_backup_deserialize() {
    let json = serde_json::json!({
        "id": "backup-1",
        "serviceId": "svc-1",
        "status": "done",
        "startedAt": "2024-01-01T00:00:00Z",
        "finishedAt": "2024-01-01T00:05:00Z",
        "sizeInBytes": 1048576,
        "durationInSeconds": 300.5,
        "type": "full",
        "backupName": "backup_20240101",
        "bucket": {"bucketProvider": "AWS", "bucketPath": "/backups"}
    });
    let backup: Backup = serde_json::from_value(json).unwrap();
    assert_eq!(backup.id, "backup-1");
    assert_eq!(backup.service_id.as_deref(), Some("svc-1"));
    assert_eq!(backup.status, "done");
    assert_eq!(backup.size_in_bytes, Some(1048576));
    assert_eq!(backup.started_at.as_deref(), Some("2024-01-01T00:00:00Z"));
    assert_eq!(backup.finished_at.as_deref(), Some("2024-01-01T00:05:00Z"));
    assert_eq!(backup.duration_in_seconds, Some(300.5));
    assert_eq!(backup.backup_type.as_deref(), Some("full"));
    assert_eq!(backup.backup_name.as_deref(), Some("backup_20240101"));
    match backup.bucket.as_ref() {
        Some(BackupBucket::AWS {
            bucket_path,
            iam_role_arn,
            iam_role_session_name,
        }) => {
            assert_eq!(bucket_path.as_deref(), Some("/backups"));
            assert!(iam_role_arn.is_none());
            assert!(iam_role_session_name.is_none());
        }
        other => panic!("expected AWS backup bucket, got {other:?}"),
    }
}

#[test]
fn test_backup_deserialize_minimal() {
    let json = r#"{"id":"backup-1","status":"in_progress"}"#;
    let backup: Backup = serde_json::from_str(json).unwrap();
    assert_eq!(backup.id, "backup-1");
    assert!(backup.service_id.is_none());
    assert!(backup.size_in_bytes.is_none());
    assert!(backup.duration_in_seconds.is_none());
    assert!(backup.backup_type.is_none());
    assert!(backup.backup_name.is_none());
    assert!(backup.bucket.is_none());
}

#[test]
fn test_backup_bucket_gcp_deserialize() {
    let json = serde_json::json!({
        "bucketProvider": "GCP",
        "bucketPath": "gs://backup-bucket/path",
        "accessKeyId": "gcp-hmac-id"
    });
    let bucket: BackupBucket = serde_json::from_value(json).unwrap();
    match bucket {
        BackupBucket::GCP {
            bucket_path,
            access_key_id,
        } => {
            assert_eq!(bucket_path.as_deref(), Some("gs://backup-bucket/path"));
            assert_eq!(access_key_id.as_deref(), Some("gcp-hmac-id"));
        }
        other => panic!("expected GCP backup bucket, got {other:?}"),
    }
}

#[test]
fn test_backup_bucket_azure_roundtrip() {
    let bucket = BackupBucket::AZURE {
        container_name: Some("backups".to_string()),
    };
    let json = serde_json::to_value(&bucket).unwrap();
    assert_eq!(json["bucketProvider"], "AZURE");
    assert_eq!(json["containerName"], "backups");

    let roundtrip: BackupBucket = serde_json::from_value(json).unwrap();
    match roundtrip {
        BackupBucket::AZURE { container_name } => {
            assert_eq!(container_name.as_deref(), Some("backups"));
        }
        other => panic!("expected AZURE backup bucket, got {other:?}"),
    }
}

#[test]
fn test_resource_tag_roundtrip() {
    let tag = ResourceTag {
        key: "env".to_string(),
        value: "production".to_string(),
    };
    let json = serde_json::to_string(&tag).unwrap();
    let tag2: ResourceTag = serde_json::from_str(&json).unwrap();
    assert_eq!(tag.key, tag2.key);
    assert_eq!(tag.value, tag2.value);
}

// ── New helper type tests ───────────────────────────────────────────

#[test]
fn test_ip_access_list_patch_serialize() {
    let patch = IpAccessListPatch {
        add: Some(vec![IpAccessEntry {
            source: "10.0.0.0/8".to_string(),
            description: Some("VPN".to_string()),
        }]),
        remove: Some(vec![IpAccessEntry {
            source: "0.0.0.0/0".to_string(),
            description: None,
        }]),
    };
    let json = serde_json::to_value(&patch).unwrap();
    assert_eq!(json["add"][0]["source"], "10.0.0.0/8");
    assert_eq!(json["remove"][0]["source"], "0.0.0.0/0");
}

#[test]
fn test_ip_access_list_patch_empty() {
    let patch = IpAccessListPatch::default();
    let json = serde_json::to_value(&patch).unwrap();
    assert!(json.get("add").is_none());
    assert!(json.get("remove").is_none());
}

#[test]
fn test_instance_private_endpoints_patch_serialize() {
    let patch = InstancePrivateEndpointsPatch {
        add: Some(vec!["pe-1".to_string()]),
        remove: Some(vec!["pe-2".to_string()]),
    };
    let json = serde_json::to_value(&patch).unwrap();
    assert_eq!(json["add"], serde_json::json!(["pe-1"]));
    assert_eq!(json["remove"], serde_json::json!(["pe-2"]));
}

#[test]
fn test_instance_tags_patch_serialize() {
    let patch = InstanceTagsPatch {
        add: Some(vec![ResourceTag {
            key: "env".to_string(),
            value: "prod".to_string(),
        }]),
        remove: Some(vec![ResourceTag {
            key: "old".to_string(),
            value: "tag".to_string(),
        }]),
    };
    let json = serde_json::to_value(&patch).unwrap();
    assert_eq!(json["add"][0]["key"], "env");
    assert_eq!(json["remove"][0]["key"], "old");
}

#[test]
fn test_service_endpoint_change_serialize() {
    let change = ServiceEndpointChange {
        protocol: "mysql".to_string(),
        enabled: true,
    };
    let json = serde_json::to_value(&change).unwrap();
    assert_eq!(json["protocol"], "mysql");
    assert_eq!(json["enabled"], true);
}

#[test]
fn test_assigned_role_deserialize() {
    let json = serde_json::json!({
        "roleId": "role-uuid-1",
        "roleName": "Admin",
        "roleType": "system"
    });
    let role: AssignedRole = serde_json::from_value(json).unwrap();
    assert_eq!(role.role_id, "role-uuid-1");
    assert_eq!(role.role_name.as_deref(), Some("Admin"));
    assert_eq!(role.role_type.as_deref(), Some("system"));
}

#[test]
fn test_assigned_role_minimal() {
    let json = serde_json::json!({"roleId": "role-uuid-1"});
    let role: AssignedRole = serde_json::from_value(json).unwrap();
    assert_eq!(role.role_id, "role-uuid-1");
    assert!(role.role_name.is_none());
    assert!(role.role_type.is_none());
}

// ── Request serialization tests ─────────────────────────────────────

#[test]
fn test_create_service_request_minimal() {
    let req = CreateServiceRequest {
        name: "my-svc".to_string(),
        provider: "aws".to_string(),
        region: "us-east-1".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "my-svc");
    assert_eq!(json["provider"], "aws");
    assert_eq!(json["region"], "us-east-1");
    assert!(json.get("ipAccessList").is_none());
    assert!(json.get("minReplicaMemoryGb").is_none());
    assert!(json.get("backupId").is_none());
    assert!(json.get("tags").is_none());
    assert!(json.get("endpoints").is_none());
    assert!(json.get("enableCoreDumps").is_none());
}

#[test]
fn test_create_service_request_full() {
    let req = CreateServiceRequest {
        name: "my-svc".to_string(),
        provider: "aws".to_string(),
        region: "us-east-1".to_string(),
        ip_access_list: Some(vec![IpAccessEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some("All".to_string()),
        }]),
        min_replica_memory_gb: Some(24),
        max_replica_memory_gb: Some(48),
        num_replicas: Some(3),
        idle_scaling: Some(true),
        idle_timeout_minutes: Some(10),
        backup_id: Some("backup-1".to_string()),
        release_channel: Some("default".to_string()),
        tags: Some(vec![ResourceTag {
            key: "env".to_string(),
            value: "prod".to_string(),
        }]),
        data_warehouse_id: Some("dw-1".to_string()),
        is_readonly: Some(true),
        encryption_key: Some("key-1".to_string()),
        encryption_assumed_role_identifier: Some("role-1".to_string()),
        has_transparent_data_encryption: Some(true),
        compliance_type: Some("hipaa".to_string()),
        profile: Some("v1-default".to_string()),
        private_preview_terms_checked: Some(true),
        endpoints: Some(vec![ServiceEndpointChange {
            protocol: "mysql".to_string(),
            enabled: true,
        }]),
        enable_core_dumps: Some(true),
    };
    let json = serde_json::to_value(&req).unwrap();

    assert_eq!(json["ipAccessList"][0]["source"], "0.0.0.0/0");
    assert_eq!(json["minReplicaMemoryGb"], 24);
    assert_eq!(json["maxReplicaMemoryGb"], 48);
    assert_eq!(json["numReplicas"], 3);
    assert_eq!(json["idleScaling"], true);
    assert_eq!(json["idleTimeoutMinutes"], 10);
    assert_eq!(json["backupId"], "backup-1");
    assert_eq!(json["releaseChannel"], "default");
    assert_eq!(json["tags"][0]["key"], "env");
    assert_eq!(json["dataWarehouseId"], "dw-1");
    assert_eq!(json["isReadonly"], true);
    assert_eq!(json["encryptionKey"], "key-1");
    assert_eq!(json["encryptionAssumedRoleIdentifier"], "role-1");
    assert_eq!(json["hasTransparentDataEncryption"], true);
    assert_eq!(json["complianceType"], "hipaa");
    assert_eq!(json["profile"], "v1-default");
    assert_eq!(json["privatePreviewTermsChecked"], true);
    assert_eq!(json["endpoints"][0]["protocol"], "mysql");
    assert_eq!(json["endpoints"][0]["enabled"], true);
    assert_eq!(json["enableCoreDumps"], true);
}

#[test]
fn test_state_change_request_serialize() {
    let req = StateChangeRequest {
        command: "start".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["command"], "start");
}

// ── ApiResponse wrapper tests ───────────────────────────────────────

#[test]
fn test_api_response_with_result() {
    let json = r#"{"result":{"id":"org-1","name":"My Org"}}"#;
    let resp: ApiResponse<Organization> = serde_json::from_str(json).unwrap();
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
    assert_eq!(resp.result.unwrap().id, "org-1");
}

#[test]
fn test_api_response_with_error() {
    let json = r#"{"error":{"code":"NOT_FOUND","message":"Not found"}}"#;
    let resp: ApiResponse<Organization> = serde_json::from_str(json).unwrap();
    assert!(resp.result.is_none());
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().message, "Not found");
}

#[test]
fn test_create_service_response_deserialize() {
    let json = serde_json::json!({
        "service": {
            "id": "svc-1",
            "name": "new-svc",
            "provider": "aws",
            "region": "us-east-1",
            "state": "provisioning"
        },
        "password": "secret123"
    });
    let resp: CreateServiceResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.service.id, "svc-1");
    assert_eq!(resp.password, "secret123");
}

// ── UpdateServiceRequest tests ──────────────────────────────────────

#[test]
fn test_update_service_request_minimal() {
    let req = UpdateServiceRequest {
        name: Some("new-name".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "new-name");
    assert!(json.get("ipAccessList").is_none());
    assert!(json.get("tags").is_none());
    assert!(json.get("releaseChannel").is_none());
    assert!(json.get("endpoints").is_none());
    assert!(json.get("privateEndpointIds").is_none());
}

#[test]
fn test_update_service_request_full() {
    let req = UpdateServiceRequest {
        name: Some("updated".to_string()),
        ip_access_list: Some(IpAccessListPatch {
            add: Some(vec![IpAccessEntry {
                source: "10.0.0.0/8".to_string(),
                description: Some("VPN".to_string()),
            }]),
            remove: None,
        }),
        private_endpoint_ids: Some(InstancePrivateEndpointsPatch {
            add: Some(vec!["pe-1".to_string()]),
            remove: None,
        }),
        release_channel: Some("fast".to_string()),
        endpoints: Some(vec![ServiceEndpointChange {
            protocol: "mysql".to_string(),
            enabled: true,
        }]),
        transparent_data_encryption_key_id: Some("tde-key-1".to_string()),
        tags: Some(vec![InstanceTagsPatch {
            add: Some(vec![ResourceTag {
                key: "env".to_string(),
                value: "staging".to_string(),
            }]),
            remove: None,
        }]),
        enable_core_dumps: Some(false),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "updated");
    assert_eq!(json["ipAccessList"]["add"][0]["source"], "10.0.0.0/8");
    assert_eq!(json["privateEndpointIds"]["add"], serde_json::json!(["pe-1"]));
    assert_eq!(json["releaseChannel"], "fast");
    assert_eq!(json["endpoints"][0]["protocol"], "mysql");
    assert_eq!(json["transparentDataEncryptionKeyId"], "tde-key-1");
    assert_eq!(json["tags"][0]["add"][0]["key"], "env");
    assert_eq!(json["enableCoreDumps"], false);
}

#[test]
fn test_replica_scaling_request_serialize() {
    let req = ReplicaScalingRequest {
        min_replica_memory_gb: Some(24),
        max_replica_memory_gb: Some(48),
        num_replicas: Some(3),
        idle_scaling: None,
        idle_timeout_minutes: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["minReplicaMemoryGb"], 24);
    assert_eq!(json["maxReplicaMemoryGb"], 48);
    assert_eq!(json["numReplicas"], 3);
    assert!(json.get("idleScaling").is_none());
    assert!(json.get("idleTimeoutMinutes").is_none());
}

#[test]
fn test_password_reset_response_deserialize() {
    let json = r#"{"password":"new-secret-123"}"#;
    let resp: PasswordResetResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.password.as_deref(), Some("new-secret-123"));
}

#[test]
fn test_password_reset_response_without_password() {
    let json = r#"{}"#;
    let resp: PasswordResetResponse = serde_json::from_str(json).unwrap();
    assert!(resp.password.is_none());
}

#[test]
fn test_service_query_endpoint_deserialize() {
    let json = serde_json::json!({
        "id": "sqe-1",
        "openApiKeys": ["key-1", "key-2"],
        "roles": ["sql_console_admin"],
        "allowedOrigins": "https://example.com"
    });
    let ep: ServiceQueryEndpoint = serde_json::from_value(json).unwrap();
    assert_eq!(ep.id.as_deref(), Some("sqe-1"));
    assert_eq!(ep.open_api_keys.as_ref().unwrap(), &["key-1", "key-2"]);
    assert_eq!(ep.roles.as_ref().unwrap(), &["sql_console_admin"]);
    assert_eq!(ep.allowed_origins.as_deref(), Some("https://example.com"));
}

#[test]
fn test_create_query_endpoint_request_serialize() {
    let req = CreateQueryEndpointRequest {
        roles: Some(vec!["sql_console_admin".to_string()]),
        open_api_keys: Some(vec!["key-1".to_string()]),
        allowed_origins: Some("https://example.com".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["roles"], serde_json::json!(["sql_console_admin"]));
    assert_eq!(json["openApiKeys"], serde_json::json!(["key-1"]));
    assert_eq!(json["allowedOrigins"], "https://example.com");
}

#[test]
fn test_create_query_endpoint_request_empty() {
    let req = CreateQueryEndpointRequest {
        roles: None,
        open_api_keys: None,
        allowed_origins: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("roles").is_none());
    assert!(json.get("openApiKeys").is_none());
    assert!(json.get("allowedOrigins").is_none());
}

#[test]
fn test_private_endpoint_deserialize() {
    let json = serde_json::json!({
        "id": "vpce-123",
        "description": "My endpoint",
        "cloudProvider": "aws",
        "region": "us-east-1"
    });
    let ep: PrivateEndpoint = serde_json::from_value(json).unwrap();
    assert_eq!(ep.id.as_deref(), Some("vpce-123"));
    assert_eq!(ep.cloud_provider.as_deref(), Some("aws"));
    assert_eq!(ep.region.as_deref(), Some("us-east-1"));
}

#[test]
fn test_create_private_endpoint_request_serialize() {
    let req = CreatePrivateEndpointRequest {
        id: "vpce-456".to_string(),
        description: Some("Test".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["id"], "vpce-456");
    assert_eq!(json["description"], "Test");
}

#[test]
fn test_create_private_endpoint_request_no_desc() {
    let req = CreatePrivateEndpointRequest {
        id: "vpce-456".to_string(),
        description: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["id"], "vpce-456");
    assert!(json.get("description").is_none());
}

// ── Org type tests ──────────────────────────────────────────────────

#[test]
fn test_update_org_request_serialize() {
    let req = UpdateOrgRequest {
        name: Some("New Org Name".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "New Org Name");
    assert!(json.get("privateEndpoints").is_none());
    assert!(json.get("enableCoreDumps").is_none());

    let req_empty = UpdateOrgRequest::default();
    let json_empty = serde_json::to_value(&req_empty).unwrap();
    assert!(json_empty.get("name").is_none());
}

#[test]
fn test_update_org_request_full() {
    let req = UpdateOrgRequest {
        name: Some("Updated Org".to_string()),
        private_endpoints: Some(OrganizationPrivateEndpointsPatch {
            remove: Some(vec![OrganizationPatchPrivateEndpoint {
                id: Some("vpce-123".to_string()),
                description: Some("My endpoint".to_string()),
                cloud_provider: Some("aws".to_string()),
                region: Some("us-east-1".to_string()),
            }]),
        }),
        enable_core_dumps: Some(true),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "Updated Org");
    assert_eq!(json["privateEndpoints"]["remove"][0]["id"], "vpce-123");
    assert_eq!(json["privateEndpoints"]["remove"][0]["cloudProvider"], "aws");
    assert_eq!(json["enableCoreDumps"], true);
}

#[test]
fn test_organization_private_endpoints_patch_serialize() {
    let patch = OrganizationPrivateEndpointsPatch {
        remove: Some(vec![OrganizationPatchPrivateEndpoint {
            id: Some("vpce-2".to_string()),
            description: None,
            cloud_provider: None,
            region: None,
        }]),
    };
    let json = serde_json::to_value(&patch).unwrap();
    assert_eq!(json["remove"][0]["id"], "vpce-2");
}

#[test]
fn test_organization_private_endpoints_patch_empty() {
    let patch = OrganizationPrivateEndpointsPatch::default();
    let json = serde_json::to_value(&patch).unwrap();
    assert!(json.get("remove").is_none());
}

// ── UsageCostRecord tests ───────────────────────────────────────────

#[test]
fn test_usage_cost_record_deserialize() {
    let json = serde_json::json!({
        "dataWarehouseId": "dw-1",
        "serviceId": "svc-1",
        "date": "2024-01-15",
        "entityType": "service",
        "entityId": "svc-1",
        "entityName": "my-service",
        "metrics": {
            "computeCHC": 42.5,
            "storageCHC": 10.0
        },
        "totalCHC": 52.5,
        "locked": true
    });
    let record: UsageCostRecord = serde_json::from_value(json).unwrap();
    assert_eq!(record.data_warehouse_id.as_deref(), Some("dw-1"));
    assert_eq!(record.service_id.as_deref(), Some("svc-1"));
    assert_eq!(record.date.as_deref(), Some("2024-01-15"));
    assert_eq!(record.entity_type.as_deref(), Some("service"));
    assert_eq!(record.entity_id.as_deref(), Some("svc-1"));
    assert_eq!(record.entity_name.as_deref(), Some("my-service"));
    assert_eq!(record.metrics.as_ref().unwrap().compute_chc, Some(42.5));
    assert_eq!(record.metrics.as_ref().unwrap().storage_chc, Some(10.0));
    assert_eq!(record.total_chc, Some(52.5));
    assert_eq!(record.locked, Some(true));
}

#[test]
fn test_usage_cost_record_entity_types() {
    for entity_type in &["datawarehouse", "service", "clickpipe"] {
        let json = serde_json::json!({"entityType": entity_type});
        let record: UsageCostRecord = serde_json::from_value(json).unwrap();
        assert_eq!(record.entity_type.as_deref(), Some(*entity_type));
    }
}

#[test]
fn test_usage_cost_record_serialize_total_chc() {
    let record = UsageCostRecord {
        data_warehouse_id: None,
        service_id: None,
        date: None,
        entity_type: None,
        entity_id: None,
        entity_name: None,
        metrics: None,
        total_chc: Some(99.99),
        locked: None,
    };
    let json = serde_json::to_value(&record).unwrap();
    // Verify camelCase rename for totalCHC
    assert_eq!(json["totalCHC"], 99.99);
    assert!(json.get("totalChc").is_none());
}

#[test]
fn test_usage_cost_metrics_roundtrip() {
    let metrics = UsageCostMetrics {
        storage_chc: Some(1.25),
        backup_chc: Some(2.5),
        compute_chc: Some(3.75),
        data_transfer_chc: Some(4.0),
        initial_load_chc: Some(5.0),
        public_data_transfer_chc: Some(6.0),
        inter_region_tier1_data_transfer_chc: Some(7.0),
        inter_region_tier2_data_transfer_chc: Some(8.0),
        inter_region_tier3_data_transfer_chc: Some(9.0),
        inter_region_tier4_data_transfer_chc: Some(10.0),
    };
    let json = serde_json::to_value(&metrics).unwrap();
    assert_eq!(json["storageCHC"], 1.25);
    assert_eq!(json["backupCHC"], 2.5);
    assert_eq!(json["computeCHC"], 3.75);
    assert_eq!(json["dataTransferCHC"], 4.0);
    assert_eq!(json["initialLoadCHC"], 5.0);
    assert_eq!(json["publicDataTransferCHC"], 6.0);
    assert_eq!(json["interRegionTier1DataTransferCHC"], 7.0);
    assert_eq!(json["interRegionTier2DataTransferCHC"], 8.0);
    assert_eq!(json["interRegionTier3DataTransferCHC"], 9.0);
    assert_eq!(json["interRegionTier4DataTransferCHC"], 10.0);

    let roundtrip: UsageCostMetrics = serde_json::from_value(json).unwrap();
    assert_eq!(roundtrip.storage_chc, Some(1.25));
    assert_eq!(roundtrip.inter_region_tier4_data_transfer_chc, Some(10.0));
}

// ── Member type tests ───────────────────────────────────────────────

#[test]
fn test_member_deserialize() {
    let json = serde_json::json!({
        "userId": "user-1",
        "email": "alice@example.com",
        "role": "admin",
        "name": "Alice",
        "joinedAt": "2024-03-01T12:00:00Z",
        "assignedRoles": [
            {"roleId": "role-1", "roleName": "Admin", "roleType": "system"}
        ]
    });
    let member: Member = serde_json::from_value(json).unwrap();
    assert_eq!(member.user_id, "user-1");
    assert_eq!(member.email, "alice@example.com");
    assert_eq!(member.role, "admin");
    assert_eq!(member.name.as_deref(), Some("Alice"));
    assert_eq!(member.joined_at.as_deref(), Some("2024-03-01T12:00:00Z"));
    assert_eq!(member.assigned_roles.as_ref().unwrap().len(), 1);
    assert_eq!(member.assigned_roles.as_ref().unwrap()[0].role_id, "role-1");
}

#[test]
fn test_member_deserialize_minimal() {
    let json = r#"{"userId":"user-2","email":"bob@example.com","role":"developer"}"#;
    let member: Member = serde_json::from_str(json).unwrap();
    assert_eq!(member.user_id, "user-2");
    assert_eq!(member.email, "bob@example.com");
    assert_eq!(member.role, "developer");
    assert!(member.name.is_none());
    assert!(member.joined_at.is_none());
    assert!(member.assigned_roles.is_none());
}

#[test]
fn test_update_member_request_serialize_empty() {
    let req = UpdateMemberRequest::default();
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("assignedRoleIds").is_none());
}

#[test]
fn test_update_member_request_with_assigned_roles() {
    let req = UpdateMemberRequest {
        assigned_role_ids: Some(vec!["role-uuid-1".to_string(), "role-uuid-2".to_string()]),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["assignedRoleIds"], serde_json::json!(["role-uuid-1", "role-uuid-2"]));
}

#[test]
fn test_invitation_deserialize() {
    let json = serde_json::json!({
        "id": "inv-1",
        "email": "carol@example.com",
        "role": "developer",
        "createdAt": "2024-06-01T00:00:00Z",
        "expireAt": "2024-06-08T00:00:00Z",
        "assignedRoles": [
            {"roleId": "role-1", "roleName": "Developer", "roleType": "system"}
        ]
    });
    let inv: Invitation = serde_json::from_value(json).unwrap();
    assert_eq!(inv.id, "inv-1");
    assert_eq!(inv.email, "carol@example.com");
    assert_eq!(inv.role, "developer");
    assert_eq!(inv.created_at.as_deref(), Some("2024-06-01T00:00:00Z"));
    assert_eq!(inv.expire_at.as_deref(), Some("2024-06-08T00:00:00Z"));
    assert_eq!(inv.assigned_roles.as_ref().unwrap().len(), 1);
}

#[test]
fn test_create_invitation_request_serialize() {
    let req = CreateInvitationRequest {
        email: "dave@example.com".to_string(),
        assigned_role_ids: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["email"], "dave@example.com");
    assert!(json.get("assignedRoleIds").is_none());
}

#[test]
fn test_create_invitation_request_with_assigned_roles() {
    let req = CreateInvitationRequest {
        email: "dave@example.com".to_string(),
        assigned_role_ids: Some(vec!["role-uuid-1".to_string()]),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["email"], "dave@example.com");
    assert_eq!(json["assignedRoleIds"], serde_json::json!(["role-uuid-1"]));
}

// ── API Key type tests ──────────────────────────────────────────────

#[test]
fn test_api_key_deserialize() {
    let json = serde_json::json!({
        "id": "key-1",
        "name": "my-key",
        "state": "enabled",
        "roles": ["admin", "developer"],
        "assignedRoles": [
            {"roleId": "role-1", "roleName": "Admin", "roleType": "system"}
        ],
        "keySuffix": "abcd",
        "createdAt": "2024-01-15T10:00:00Z",
        "expireAt": "2025-01-15T10:00:00Z",
        "usedAt": "2024-06-01T08:00:00Z",
        "ipAccessList": [{"source": "10.0.0.0/8", "description": "VPN"}]
    });
    let key: ApiKey = serde_json::from_value(json).unwrap();
    assert_eq!(key.id, "key-1");
    assert_eq!(key.name, "my-key");
    assert_eq!(key.state, "enabled");
    assert_eq!(key.roles.as_ref().unwrap(), &["admin", "developer"]);
    assert_eq!(key.assigned_roles.as_ref().unwrap().len(), 1);
    assert_eq!(key.key_suffix.as_deref(), Some("abcd"));
    assert_eq!(key.created_at.as_deref(), Some("2024-01-15T10:00:00Z"));
    assert_eq!(key.expire_at.as_deref(), Some("2025-01-15T10:00:00Z"));
    assert_eq!(key.used_at.as_deref(), Some("2024-06-01T08:00:00Z"));
    assert_eq!(key.ip_access_list.as_ref().unwrap().len(), 1);
}

#[test]
fn test_api_key_state_values() {
    for state in &["enabled", "disabled"] {
        let json = serde_json::json!({
            "id": "key-1",
            "name": "test",
            "state": state
        });
        let key: ApiKey = serde_json::from_value(json).unwrap();
        assert_eq!(key.state, *state);
    }
}

#[test]
fn test_create_api_key_request_serialize() {
    let req = CreateApiKeyRequest {
        name: "ci-key".to_string(),
        expire_at: None,
        state: None,
        assigned_role_ids: None,
        ip_access_list: None,
        hash_data: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "ci-key");
    assert!(json.get("expireAt").is_none());
    assert!(json.get("state").is_none());
    assert!(json.get("assignedRoleIds").is_none());
    assert!(json.get("ipAccessList").is_none());
    assert!(json.get("hashData").is_none());
}

#[test]
fn test_create_api_key_request_full() {
    let req = CreateApiKeyRequest {
        name: "full-key".to_string(),
        expire_at: Some("2025-12-31T23:59:59Z".to_string()),
        state: Some("enabled".to_string()),
        assigned_role_ids: Some(vec!["role-uuid-1".to_string()]),
        ip_access_list: Some(vec![IpAccessEntry {
            source: "10.0.0.0/8".to_string(),
            description: None,
        }]),
        hash_data: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "full-key");
    assert_eq!(json["expireAt"], "2025-12-31T23:59:59Z");
    assert_eq!(json["state"], "enabled");
    assert_eq!(json["assignedRoleIds"], serde_json::json!(["role-uuid-1"]));
    assert_eq!(json["ipAccessList"][0]["source"], "10.0.0.0/8");
}

#[test]
fn test_create_api_key_request_with_hash_data() {
    let req = CreateApiKeyRequest {
        name: "prehashed-key".to_string(),
        expire_at: None,
        state: None,
        assigned_role_ids: None,
        ip_access_list: None,
        hash_data: Some(ApiKeyHashData {
            key_id_hash: "hash-of-id".to_string(),
            key_id_suffix: "abcd".to_string(),
            key_secret_hash: "hash-of-secret".to_string(),
        }),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "prehashed-key");
    assert_eq!(json["hashData"]["keyIdHash"], "hash-of-id");
    assert_eq!(json["hashData"]["keyIdSuffix"], "abcd");
    assert_eq!(json["hashData"]["keySecretHash"], "hash-of-secret");
}

#[test]
fn test_create_api_key_response_deserialize() {
    let json = serde_json::json!({
        "key": {
            "id": "key-2",
            "name": "new-key",
            "state": "enabled"
        },
        "keyId": "kid-abc",
        "keySecret": "secret-xyz"
    });
    let resp: CreateApiKeyResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.key.id, "key-2");
    assert_eq!(resp.key.name, "new-key");
    assert_eq!(resp.key.state, "enabled");
    assert_eq!(resp.key_id.as_deref(), Some("kid-abc"));
    assert_eq!(resp.key_secret.as_deref(), Some("secret-xyz"));
}

#[test]
fn test_create_api_key_response_without_generated_credentials() {
    let json = serde_json::json!({
        "key": {
            "id": "key-2",
            "name": "prehashed-key",
            "state": "enabled"
        }
    });
    let resp: CreateApiKeyResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.key.id, "key-2");
    assert!(resp.key_id.is_none());
    assert!(resp.key_secret.is_none());
}

#[test]
fn test_update_api_key_request_serialize() {
    let req = UpdateApiKeyRequest {
        name: Some("renamed-key".to_string()),
        assigned_role_ids: Some(vec!["role-uuid-1".to_string()]),
        expire_at: Some("2025-12-31T00:00:00Z".to_string()),
        state: Some("disabled".to_string()),
        ip_access_list: Some(vec![IpAccessEntry {
            source: "0.0.0.0/0".to_string(),
            description: None,
        }]),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "renamed-key");
    assert_eq!(json["assignedRoleIds"], serde_json::json!(["role-uuid-1"]));
    assert_eq!(json["expireAt"], "2025-12-31T00:00:00Z");
    assert_eq!(json["state"], "disabled");
    assert_eq!(json["ipAccessList"][0]["source"], "0.0.0.0/0");
}

// ── Activity, Backup Config ─────────────────────────────────────────

#[test]
fn test_activity_deserialize() {
    let json = serde_json::json!({
        "id": "act-1",
        "type": "service_create",
        "actorType": "user",
        "actorId": "user-5",
        "createdAt": "2024-07-01T08:00:00Z",
        "actorDetails": "Alice Smith",
        "actorIpAddress": "1.2.3.4",
        "organizationId": "org-1",
        "serviceId": "svc-1",
        "userAgent": "clickhousectl/0.1.0"
    });
    let act: Activity = serde_json::from_value(json).unwrap();
    assert_eq!(act.id, "act-1");
    assert_eq!(act.activity_type, "service_create");
    assert_eq!(act.actor_type.as_deref(), Some("user"));
    assert_eq!(act.actor_id.as_deref(), Some("user-5"));
    assert_eq!(act.created_at.as_deref(), Some("2024-07-01T08:00:00Z"));
    assert_eq!(act.actor_details.as_deref(), Some("Alice Smith"));
    assert_eq!(act.actor_ip_address.as_deref(), Some("1.2.3.4"));
    assert_eq!(act.organization_id.as_deref(), Some("org-1"));
    assert_eq!(act.service_id.as_deref(), Some("svc-1"));
    assert_eq!(act.user_agent.as_deref(), Some("clickhousectl/0.1.0"));
}

#[test]
fn test_activity_key_update() {
    let json = serde_json::json!({
        "id": "act-2",
        "type": "openapi_key_update",
        "actorType": "api",
        "targetKeyId": "key-1",
        "keyUpdateType": "state-changed"
    });
    let act: Activity = serde_json::from_value(json).unwrap();
    assert_eq!(act.activity_type, "openapi_key_update");
    assert_eq!(act.target_key_id.as_deref(), Some("key-1"));
    assert_eq!(act.key_update_type.as_deref(), Some("state-changed"));
}

#[test]
fn test_activity_minimal() {
    let json = serde_json::json!({
        "id": "act-3",
        "type": "user_login"
    });
    let act: Activity = serde_json::from_value(json).unwrap();
    assert_eq!(act.id, "act-3");
    assert_eq!(act.activity_type, "user_login");
    assert!(act.actor_type.is_none());
    assert!(act.actor_details.is_none());
    assert!(act.service_id.is_none());
}

#[test]
fn test_backup_configuration_deserialize() {
    let json = serde_json::json!({
        "backupPeriodInHours": 24,
        "backupRetentionPeriodInHours": 720,
        "backupStartTime": "02:00"
    });
    let config: BackupConfiguration = serde_json::from_value(json).unwrap();
    assert_eq!(config.backup_period_in_hours, Some(24));
    assert_eq!(config.backup_retention_period_in_hours, Some(720));
    assert_eq!(config.backup_start_time.as_deref(), Some("02:00"));
}

#[test]
fn test_update_backup_config_request_serialize() {
    let req = UpdateBackupConfigRequest {
        backup_period_in_hours: Some(12),
        backup_retention_period_in_hours: Some(336),
        backup_start_time: Some("03:00".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["backupPeriodInHours"], 12);
    assert_eq!(json["backupRetentionPeriodInHours"], 336);
    assert_eq!(json["backupStartTime"], "03:00");
}

// ── Service state enum values ───────────────────────────────────────

#[test]
fn test_service_state_values() {
    let states = [
        "starting", "stopping", "terminating", "softdeleting", "awaking",
        "partially_running", "provisioning", "running", "stopped",
        "terminated", "softdeleted", "degraded", "failed", "idle",
    ];
    for state in &states {
        let json = serde_json::json!({
            "id": "svc-1",
            "name": "svc",
            "provider": "aws",
            "region": "us-east-1",
            "state": state
        });
        let svc: Service = serde_json::from_value(json).unwrap();
        assert_eq!(svc.state, *state);
    }
}

// ── Backup status enum values ───────────────────────────────────────

#[test]
fn test_backup_status_values() {
    for status in &["done", "error", "in_progress"] {
        let json = serde_json::json!({
            "id": "backup-1",
            "status": status
        });
        let backup: Backup = serde_json::from_value(json).unwrap();
        assert_eq!(backup.status, *status);
    }
}

#[test]
fn test_backup_type_values() {
    for backup_type in &["full", "incremental"] {
        let json = serde_json::json!({
            "id": "backup-1",
            "status": "done",
            "type": backup_type
        });
        let backup: Backup = serde_json::from_value(json).unwrap();
        assert_eq!(backup.backup_type.as_deref(), Some(*backup_type));
    }
}

// ── UsageCost wrapper tests ─────────────────────────────────────────

#[test]
fn test_usage_cost_deserialize() {
    let json = serde_json::json!({
        "grandTotalCHC": 123.45,
        "costs": {
            "serviceId": "svc-1",
            "date": "2024-01-15",
            "totalCHC": 52.5
        }
    });
    let cost: UsageCost = serde_json::from_value(json).unwrap();
    assert_eq!(cost.grand_total_chc, Some(123.45));
    assert_eq!(cost.costs.as_ref().unwrap().total_chc, Some(52.5));
}

#[test]
fn test_usage_cost_serialize_grand_total() {
    let cost = UsageCost {
        grand_total_chc: Some(99.99),
        costs: None,
    };
    let json = serde_json::to_value(&cost).unwrap();
    assert_eq!(json["grandTotalCHC"], 99.99);
    assert!(json.get("grandTotalChc").is_none());
}

#[test]
fn test_usage_cost_minimal() {
    let json = serde_json::json!({});
    let cost: UsageCost = serde_json::from_value(json).unwrap();
    assert!(cost.grand_total_chc.is_none());
    assert!(cost.costs.is_none());
}

// ── PrivateEndpointConfig tests ─────────────────────────────────────

#[test]
fn test_private_endpoint_config_deserialize() {
    let json = serde_json::json!({
        "endpointServiceId": "com.amazonaws.vpce.us-east-1.vpce-svc-123",
        "privateDnsHostname": "abc.clickhouse.cloud"
    });
    let config: PrivateEndpointConfig = serde_json::from_value(json).unwrap();
    assert_eq!(config.endpoint_service_id, "com.amazonaws.vpce.us-east-1.vpce-svc-123");
    assert_eq!(config.private_dns_hostname, "abc.clickhouse.cloud");
}

#[test]
fn test_private_endpoint_config_roundtrip() {
    let config = PrivateEndpointConfig {
        endpoint_service_id: "vpce-svc-456".to_string(),
        private_dns_hostname: "xyz.clickhouse.cloud".to_string(),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["endpointServiceId"], "vpce-svc-456");
    assert_eq!(json["privateDnsHostname"], "xyz.clickhouse.cloud");
    let config2: PrivateEndpointConfig = serde_json::from_value(json).unwrap();
    assert_eq!(config.endpoint_service_id, config2.endpoint_service_id);
}

// ── ServicePasswordPatchRequest tests ───────────────────────────────

#[test]
fn test_service_password_patch_request_empty() {
    let req = ServicePasswordPatchRequest::default();
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("newPasswordHash").is_none());
    assert!(json.get("newDoubleSha1Hash").is_none());
}

#[test]
fn test_service_password_patch_request_with_hashes() {
    let req = ServicePasswordPatchRequest {
        new_password_hash: Some("sha256hash".to_string()),
        new_double_sha1_hash: Some("doublesha1hash".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["newPasswordHash"], "sha256hash");
    assert_eq!(json["newDoubleSha1Hash"], "doublesha1hash");
}

// ── ApiKeyHashData tests ────────────────────────────────────────────

#[test]
fn test_api_key_hash_data_roundtrip() {
    let data = ApiKeyHashData {
        key_id_hash: "idhash".to_string(),
        key_id_suffix: "suf1".to_string(),
        key_secret_hash: "secrethash".to_string(),
    };
    let json = serde_json::to_value(&data).unwrap();
    assert_eq!(json["keyIdHash"], "idhash");
    assert_eq!(json["keyIdSuffix"], "suf1");
    assert_eq!(json["keySecretHash"], "secrethash");
    let data2: ApiKeyHashData = serde_json::from_value(json).unwrap();
    assert_eq!(data.key_id_hash, data2.key_id_hash);
    assert_eq!(data.key_id_suffix, data2.key_id_suffix);
    assert_eq!(data.key_secret_hash, data2.key_secret_hash);
}

// ── Comprehensive enum value tests (from OpenAPI spec) ──────────────

#[test]
fn test_activity_type_values() {
    let types = [
        "create_organization", "organization_update_name",
        "transfer_service_in", "transfer_service_out",
        "save_payment_method", "marketplace_subscription",
        "migrate_marketplace_billing_details_in", "migrate_marketplace_billing_details_out",
        "organization_update_tier", "organization_invite_create",
        "organization_invite_delete", "organization_member_join",
        "organization_member_add", "organization_member_leave",
        "organization_member_delete", "organization_member_update_role",
        "organization_member_update_mfa_method", "user_login",
        "user_login_failed", "user_logout",
        "key_create", "key_delete", "openapi_key_update",
        "service_create", "service_start", "service_stop",
        "service_awaken", "service_idle", "service_running",
        "service_partially_running", "service_delete",
        "service_update_name", "service_update_ip_access_list",
        "service_update_autoscaling_memory", "service_update_autoscaling_idling",
        "service_update_password", "service_update_autoscaling_replicas",
        "service_update_max_allowable_replicas", "service_update_backup_configuration",
        "service_restore_backup", "service_update_release_channel",
        "service_update_gpt_usage_consent", "service_update_private_endpoints",
        "service_import_to_organization", "service_export_from_organization",
        "service_maintenance_start", "service_maintenance_end",
        "service_update_core_dump", "backup_delete",
    ];
    for t in &types {
        let json = serde_json::json!({"id": "act-1", "type": t});
        let act: Activity = serde_json::from_value(json).unwrap();
        assert_eq!(act.activity_type, *t);
    }
}

#[test]
fn test_activity_actor_type_values() {
    for actor_type in &["user", "support", "system", "api"] {
        let json = serde_json::json!({"id": "act-1", "type": "user_login", "actorType": actor_type});
        let act: Activity = serde_json::from_value(json).unwrap();
        assert_eq!(act.actor_type.as_deref(), Some(*actor_type));
    }
}

#[test]
fn test_activity_key_update_type_values() {
    let update_types = [
        "created", "deleted", "name-changed", "role-changed",
        "state-changed", "date-changed", "ip-access-list-changed",
        "org-role-changed", "default-service-role-changed",
        "service-role-changed", "roles-v2-changed",
    ];
    for t in &update_types {
        let json = serde_json::json!({
            "id": "act-1",
            "type": "openapi_key_update",
            "keyUpdateType": t
        });
        let act: Activity = serde_json::from_value(json).unwrap();
        assert_eq!(act.key_update_type.as_deref(), Some(*t));
    }
}

#[test]
fn test_service_endpoint_protocol_values() {
    for protocol in &["https", "nativesecure", "mysql"] {
        let json = serde_json::json!({
            "id": "svc-1",
            "name": "svc",
            "provider": "aws",
            "region": "us-east-1",
            "state": "running",
            "endpoints": [{"protocol": protocol, "host": "host.cloud", "port": 8443}]
        });
        let svc: Service = serde_json::from_value(json).unwrap();
        assert_eq!(svc.endpoints.as_ref().unwrap()[0].protocol, *protocol);
    }
}

#[test]
fn test_service_provider_values() {
    for provider in &["aws", "gcp", "azure"] {
        let json = serde_json::json!({
            "id": "svc-1",
            "name": "svc",
            "provider": provider,
            "region": "us-east-1",
            "state": "running"
        });
        let svc: Service = serde_json::from_value(json).unwrap();
        assert_eq!(svc.provider, *provider);
    }
}

#[test]
fn test_service_release_channel_values() {
    for channel in &["slow", "default", "fast"] {
        let json = serde_json::json!({
            "id": "svc-1",
            "name": "svc",
            "provider": "aws",
            "region": "us-east-1",
            "state": "running",
            "releaseChannel": channel
        });
        let svc: Service = serde_json::from_value(json).unwrap();
        assert_eq!(svc.release_channel.as_deref(), Some(*channel));
    }
}

#[test]
fn test_service_compliance_type_values() {
    for ct in &["hipaa", "pci"] {
        let json = serde_json::json!({
            "id": "svc-1",
            "name": "svc",
            "provider": "aws",
            "region": "us-east-1",
            "state": "running",
            "complianceType": ct
        });
        let svc: Service = serde_json::from_value(json).unwrap();
        assert_eq!(svc.compliance_type.as_deref(), Some(*ct));
    }
}

#[test]
fn test_service_profile_values() {
    let profiles = [
        "v1-default", "v1-highmem-xs", "v1-highmem-s",
        "v1-highmem-m", "v1-highmem-l", "v1-highmem-xl",
    ];
    for profile in &profiles {
        let json = serde_json::json!({
            "id": "svc-1",
            "name": "svc",
            "provider": "aws",
            "region": "us-east-1",
            "state": "running",
            "profile": profile
        });
        let svc: Service = serde_json::from_value(json).unwrap();
        assert_eq!(svc.profile.as_deref(), Some(*profile));
    }
}

#[test]
fn test_assigned_role_type_values() {
    for role_type in &["system", "custom"] {
        let json = serde_json::json!({
            "roleId": "role-1",
            "roleName": "Admin",
            "roleType": role_type
        });
        let role: AssignedRole = serde_json::from_value(json).unwrap();
        assert_eq!(role.role_type.as_deref(), Some(*role_type));
    }
}

#[test]
fn test_service_query_endpoint_role_values() {
    for role in &["sql_console_read_only", "sql_console_admin"] {
        let json = serde_json::json!({"roles": [role]});
        let ep: ServiceQueryEndpoint = serde_json::from_value(json).unwrap();
        assert_eq!(ep.roles.as_ref().unwrap()[0], *role);
    }
}
