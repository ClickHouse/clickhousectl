use super::types::*;
use std::str::FromStr;

// Contract policy for this suite:
// - mirror GA request/response schemas and query surfaces only
// - exclude beta endpoints, deprecated endpoints, and deprecated request fields
// - allow deprecated or BYOC fields in responses when they appear in GA schemas
// - keep BYOC request/command surface out of scope until explicitly added

// ── Response deserialization tests ──────────────────────────────────

#[test]
fn test_delete_response_deserialize() {
    let json = serde_json::json!({
        "status": 200,
        "requestId": "0182edf5-8c5b-4586-a6f8-78452320e4b1"
    });
    let response: DeleteResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.status, 200.0);
    assert_eq!(response.request_id, "0182edf5-8c5b-4586-a6f8-78452320e4b1");
}

#[test]
fn test_delete_response_serialize() {
    let response = DeleteResponse {
        status: 200.0,
        request_id: "0182edf5-8c5b-4586-a6f8-78452320e4b1".to_string(),
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["status"], 200.0);
    assert_eq!(json["requestId"], "0182edf5-8c5b-4586-a6f8-78452320e4b1");
    assert!(json.get("request_id").is_none());
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
        "byocId": "byoc-1",
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
    assert_eq!(svc.min_total_memory_gb, Some(72.0));
    assert_eq!(svc.max_total_memory_gb, Some(144.0));
    assert_eq!(svc.idle_scaling, Some(true));
    assert_eq!(svc.idle_timeout_minutes, Some(10.0));
    assert_eq!(svc.ip_access_list.as_ref().unwrap().len(), 1);
    assert_eq!(svc.ip_access_list.as_ref().unwrap()[0].source, "0.0.0.0/0");
    assert_eq!(svc.endpoints.as_ref().unwrap()[0].port, 8443.0);
    assert_eq!(svc.min_replica_memory_gb, Some(24.0));
    assert_eq!(svc.max_replica_memory_gb, Some(48.0));
    assert_eq!(svc.num_replicas, Some(3.0));
    assert_eq!(svc.clickhouse_version.as_deref(), Some("24.3.1"));
    assert_eq!(svc.release_channel.as_deref(), Some("default"));
    assert_eq!(svc.encryption_key.as_deref(), Some("key-1"));
    assert_eq!(svc.iam_role.as_deref(), Some("arn:aws:iam::123:role/test"));
    assert_eq!(svc.private_endpoint_ids.as_ref().unwrap().len(), 2);
    assert_eq!(svc.data_warehouse_id.as_deref(), Some("dw-1"));
    assert_eq!(svc.is_primary, Some(true));
    assert_eq!(svc.is_readonly, Some(false));
    assert_eq!(svc.byoc_id.as_deref(), Some("byoc-1"));
    assert_eq!(svc.has_transparent_data_encryption, Some(false));
    assert_eq!(svc.profile.as_deref(), Some("v1-default"));
    assert_eq!(
        svc.transparent_data_encryption_key_id.as_deref(),
        Some("tde-1")
    );
    assert_eq!(svc.encryption_role_id.as_deref(), Some("role-1"));
    assert_eq!(svc.compliance_type.as_deref(), Some("hipaa"));
    assert_eq!(svc.tags.as_ref().unwrap()[0].key, "env");
    assert_eq!(svc.enable_core_dumps, Some(true));
}

#[test]
fn test_service_deserialize_minimal() {
    let json =
        r#"{"id":"svc-1","name":"svc","provider":"aws","region":"us-east-1","state":"idle"}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.id, "svc-1");
    assert!(svc.tier.is_none());
    assert!(svc.min_total_memory_gb.is_none());
    assert!(svc.max_total_memory_gb.is_none());
    assert!(svc.endpoints.is_none());
    assert!(svc.clickhouse_version.is_none());
    assert!(svc.byoc_id.is_none());
    assert!(svc.tags.is_none());
}

#[test]
fn test_endpoint_deserialize() {
    let json = r#"{"protocol":"https","host":"abc.clickhouse.cloud","port":8443}"#;
    let ep: Endpoint = serde_json::from_str(json).unwrap();
    assert_eq!(ep.protocol, "https");
    assert_eq!(ep.host, "abc.clickhouse.cloud");
    assert_eq!(ep.port, 8443.0);
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
fn test_resource_tag_roundtrip() {
    let tag = ResourceTag {
        key: "env".to_string(),
        value: Some("production".to_string()),
    };
    let json = serde_json::to_string(&tag).unwrap();
    let tag2: ResourceTag = serde_json::from_str(&json).unwrap();
    assert_eq!(tag.key, tag2.key);
    assert_eq!(tag.value, tag2.value);
}

#[test]
fn test_resource_tag_null_value() {
    let json = serde_json::json!({
        "key": "team",
        "value": null
    });
    let tag: ResourceTag = serde_json::from_value(json).unwrap();
    assert_eq!(tag.key, "team");
    assert!(tag.value.is_none());

    let serialized = serde_json::to_value(&tag).unwrap();
    assert_eq!(serialized["key"], "team");
    assert!(serialized.get("value").is_none());
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
            value: Some("prod".to_string()),
        }]),
        remove: Some(vec![ResourceTag {
            key: "old".to_string(),
            value: Some("tag".to_string()),
        }]),
    };
    let json = serde_json::to_value(&patch).unwrap();
    assert_eq!(json["add"][0]["key"], "env");
    assert_eq!(json["remove"][0]["key"], "old");
}

#[test]
fn test_service_endpoint_change_serialize() {
    let change = ServiceEndpointChange {
        protocol: ServiceToggleableEndpointProtocol::Mysql,
        enabled: true,
    };
    let json = serde_json::to_value(&change).unwrap();
    assert_eq!(json["protocol"], "mysql");
    assert_eq!(json["enabled"], true);
}

#[test]
fn test_service_toggleable_endpoint_protocol_values() {
    for protocol in &["mysql"] {
        let change = ServiceEndpointChange {
            protocol: ServiceToggleableEndpointProtocol::Mysql,
            enabled: false,
        };
        let json = serde_json::to_value(&change).unwrap();
        assert_eq!(json["protocol"], *protocol);
    }
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
        provider: CloudProvider::Aws,
        region: CloudRegion::UsEast1,
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
        provider: CloudProvider::Aws,
        region: CloudRegion::UsEast1,
        ip_access_list: Some(vec![IpAccessEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some("All".to_string()),
        }]),
        min_replica_memory_gb: Some(24.0),
        max_replica_memory_gb: Some(48.0),
        num_replicas: Some(3.0),
        idle_scaling: Some(true),
        idle_timeout_minutes: Some(10.0),
        backup_id: Some("backup-1".to_string()),
        release_channel: Some(ReleaseChannel::Default),
        tags: Some(vec![ResourceTag {
            key: "env".to_string(),
            value: Some("prod".to_string()),
        }]),
        data_warehouse_id: Some("dw-1".to_string()),
        is_readonly: Some(true),
        encryption_key: Some("key-1".to_string()),
        encryption_assumed_role_identifier: Some("role-1".to_string()),
        has_transparent_data_encryption: Some(true),
        compliance_type: Some(ComplianceType::Hipaa),
        profile: Some(ServiceProfile::V1Default),
        private_preview_terms_checked: Some(true),
        endpoints: Some(vec![ServiceEndpointChange {
            protocol: ServiceToggleableEndpointProtocol::Mysql,
            enabled: true,
        }]),
        enable_core_dumps: Some(true),
    };
    let json = serde_json::to_value(&req).unwrap();

    assert_eq!(json["ipAccessList"][0]["source"], "0.0.0.0/0");
    assert_eq!(json["minReplicaMemoryGb"], 24.0);
    assert_eq!(json["maxReplicaMemoryGb"], 48.0);
    assert_eq!(json["numReplicas"], 3.0);
    assert_eq!(json["idleScaling"], true);
    assert_eq!(json["idleTimeoutMinutes"], 10.0);
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
fn test_create_service_request_excludes_deprecated_and_byoc_fields() {
    let req = CreateServiceRequest {
        name: "my-svc".to_string(),
        provider: CloudProvider::Aws,
        region: CloudRegion::UsEast1,
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("tier").is_none());
    assert!(json.get("minTotalMemoryGb").is_none());
    assert!(json.get("maxTotalMemoryGb").is_none());
    assert!(json.get("privateEndpointIds").is_none());
    assert!(json.get("byocId").is_none());
}

#[test]
fn test_state_change_request_serialize() {
    let req = StateChangeRequest {
        command: ServiceStateCommand::Start,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["command"], "start");
}

#[test]
fn test_service_state_command_values() {
    let cases = [
        (ServiceStateCommand::Start, "start"),
        (ServiceStateCommand::Stop, "stop"),
    ];
    for (command, expected) in cases {
        let req = StateChangeRequest { command };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["command"], expected);
    }
}

// ── ApiResponse wrapper tests ───────────────────────────────────────

#[test]
fn test_api_response_with_result() {
    let json = r#"{"result":{"key":"value"}}"#;
    let resp: ApiResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
    assert_eq!(resp.result.unwrap()["key"], "value");
}

#[test]
fn test_api_response_with_error() {
    let json = r#"{"error":{"code":"NOT_FOUND","message":"Not found"}}"#;
    let resp: ApiResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
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
        release_channel: Some(ReleaseChannel::Fast),
        endpoints: Some(vec![ServiceEndpointChange {
            protocol: ServiceToggleableEndpointProtocol::Mysql,
            enabled: true,
        }]),
        transparent_data_encryption_key_id: Some("tde-key-1".to_string()),
        // The published schema currently shows an array here, but the live API
        // expects a single patch object and rejects array payloads.
        tags: Some(InstanceTagsPatch {
            add: Some(vec![ResourceTag {
                key: "env".to_string(),
                value: Some("staging".to_string()),
            }]),
            remove: None,
        }),
        enable_core_dumps: Some(false),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "updated");
    assert_eq!(json["ipAccessList"]["add"][0]["source"], "10.0.0.0/8");
    assert_eq!(
        json["privateEndpointIds"]["add"],
        serde_json::json!(["pe-1"])
    );
    assert_eq!(json["releaseChannel"], "fast");
    assert_eq!(json["endpoints"][0]["protocol"], "mysql");
    assert_eq!(json["transparentDataEncryptionKeyId"], "tde-key-1");
    assert!(json["tags"].is_object());
    assert_eq!(json["tags"]["add"][0]["key"], "env");
    assert_eq!(json["enableCoreDumps"], false);
}

#[test]
fn test_replica_scaling_request_serialize() {
    let req = ReplicaScalingRequest {
        min_replica_memory_gb: Some(24.0),
        max_replica_memory_gb: Some(48.0),
        num_replicas: Some(3.0),
        idle_scaling: None,
        idle_timeout_minutes: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["minReplicaMemoryGb"], 24.0);
    assert_eq!(json["maxReplicaMemoryGb"], 48.0);
    assert_eq!(json["numReplicas"], 3.0);
    assert!(json.get("idleScaling").is_none());
    assert!(json.get("idleTimeoutMinutes").is_none());
}

#[test]
fn test_service_scaling_patch_response_deserialize() {
    let json = serde_json::json!({
        "id": "svc-1",
        "name": "scaled-svc",
        "provider": "aws",
        "region": "us-east-1",
        "state": "running",
        "minReplicaMemoryGb": 24,
        "maxReplicaMemoryGb": 48,
        "numReplicas": 3,
        "idleScaling": true,
        "idleTimeoutMinutes": 10,
        "releaseChannel": "default",
        "profile": "v1-default",
        "complianceType": "pci",
        "enableCoreDumps": false
    });
    let resp: ServiceScalingPatchResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.id, "svc-1");
    assert_eq!(resp.name, "scaled-svc");
    assert_eq!(resp.provider, CloudProvider::Aws);
    assert_eq!(resp.region, CloudRegion::UsEast1);
    assert_eq!(resp.state, ServiceState::Running);
    assert_eq!(resp.min_replica_memory_gb, Some(24.0));
    assert_eq!(resp.max_replica_memory_gb, Some(48.0));
    assert_eq!(resp.num_replicas, Some(3.0));
    assert_eq!(resp.idle_scaling, Some(true));
    assert_eq!(resp.idle_timeout_minutes, Some(10.0));
    assert_eq!(resp.release_channel, Some(ReleaseChannel::Default));
    assert_eq!(resp.profile, Some(ServiceProfile::V1Default));
    assert_eq!(resp.compliance_type, Some(ComplianceType::Pci));
    assert_eq!(resp.enable_core_dumps, Some(false));
}

#[test]
fn test_service_password_patch_response_deserialize() {
    let json = r#"{"password":"new-secret-123"}"#;
    let resp: ServicePasswordPatchResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.password.as_deref(), Some("new-secret-123"));
}

#[test]
fn test_service_password_patch_response_without_password() {
    let json = r#"{}"#;
    let resp: ServicePasswordPatchResponse = serde_json::from_str(json).unwrap();
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
fn test_instance_private_endpoint_deserialize() {
    let json = serde_json::json!({
        "id": "vpce-123",
        "description": "My endpoint",
        "cloudProvider": "aws",
        "region": "us-east-1"
    });
    let ep: InstancePrivateEndpoint = serde_json::from_value(json).unwrap();
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
        state: Some(ApiKeyState::Enabled),
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
    assert!(json.get("roles").is_none());
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
        state: Some(ApiKeyState::Disabled),
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
    assert!(json.get("roles").is_none());
}

// ── Activity, Backup Config ─────────────────────────────────────────

#[test]
fn test_activity_deserialize() {
    use clickhouse_cloud_api::models::Activity;
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
    assert_eq!(act.id.as_deref(), Some("act-1"));
    assert_eq!(act.r#type.as_ref().unwrap().to_string(), "service_create");
    assert_eq!(act.actor_type.as_ref().unwrap().to_string(), "user");
    assert_eq!(act.actor_id.as_deref(), Some("user-5"));
    assert!(act.created_at.is_some());
    assert_eq!(act.actor_details.as_deref(), Some("Alice Smith"));
    assert_eq!(act.actor_ip_address.as_deref(), Some("1.2.3.4"));
    assert_eq!(act.organization_id.as_deref(), Some("org-1"));
    assert_eq!(act.service_id.as_deref(), Some("svc-1"));
    assert_eq!(act.user_agent.as_deref(), Some("clickhousectl/0.1.0"));
}

#[test]
fn test_activity_key_update() {
    use clickhouse_cloud_api::models::Activity;
    let json = serde_json::json!({
        "id": "act-2",
        "type": "openapi_key_update",
        "actorType": "api",
        "targetKeyId": "key-1",
        "keyUpdateType": "state-changed"
    });
    let act: Activity = serde_json::from_value(json).unwrap();
    assert_eq!(act.r#type.as_ref().unwrap().to_string(), "openapi_key_update");
    assert_eq!(act.target_key_id.as_deref(), Some("key-1"));
    assert_eq!(act.key_update_type.as_ref().unwrap().to_string(), "state-changed");
}

#[test]
fn test_activity_minimal() {
    use clickhouse_cloud_api::models::Activity;
    let json = serde_json::json!({
        "id": "act-3",
        "type": "user_login"
    });
    let act: Activity = serde_json::from_value(json).unwrap();
    assert_eq!(act.id.as_deref(), Some("act-3"));
    assert_eq!(act.r#type.as_ref().unwrap().to_string(), "user_login");
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
    assert_eq!(config.backup_period_in_hours, Some(24.0));
    assert_eq!(config.backup_retention_period_in_hours, Some(720.0));
    assert_eq!(config.backup_start_time.as_deref(), Some("02:00"));
}

#[test]
fn test_update_backup_config_request_serialize() {
    let req = UpdateBackupConfigRequest {
        backup_period_in_hours: Some(12.0),
        backup_retention_period_in_hours: Some(336.0),
        backup_start_time: Some("03:00".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["backupPeriodInHours"], 12.0);
    assert_eq!(json["backupRetentionPeriodInHours"], 336.0);
    assert_eq!(json["backupStartTime"], "03:00");
}

// ── Service state enum values ───────────────────────────────────────

#[test]
fn test_service_state_values() {
    let states = [
        "starting",
        "stopping",
        "terminating",
        "softdeleting",
        "awaking",
        "partially_running",
        "provisioning",
        "running",
        "stopped",
        "terminated",
        "softdeleted",
        "degraded",
        "failed",
        "idle",
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

// ── UsageCost wrapper tests ─────────────────────────────────────────

// ── PrivateEndpointConfig tests ─────────────────────────────────────

#[test]
fn test_private_endpoint_config_deserialize() {
    let json = serde_json::json!({
        "endpointServiceId": "com.amazonaws.vpce.us-east-1.vpce-svc-123",
        "privateDnsHostname": "abc.clickhouse.cloud"
    });
    let config: PrivateEndpointConfig = serde_json::from_value(json).unwrap();
    assert_eq!(
        config.endpoint_service_id,
        "com.amazonaws.vpce.us-east-1.vpce-svc-123"
    );
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
    use clickhouse_cloud_api::models::Activity;
    let types = [
        "create_organization",
        "organization_update_name",
        "transfer_service_in",
        "transfer_service_out",
        "save_payment_method",
        "marketplace_subscription",
        "migrate_marketplace_billing_details_in",
        "migrate_marketplace_billing_details_out",
        "organization_update_tier",
        "organization_invite_create",
        "organization_invite_delete",
        "organization_member_join",
        "organization_member_add",
        "organization_member_leave",
        "organization_member_delete",
        "organization_member_update_role",
        "organization_member_update_mfa_method",
        "user_login",
        "user_login_failed",
        "user_logout",
        "key_create",
        "key_delete",
        "openapi_key_update",
        "service_create",
        "service_start",
        "service_stop",
        "service_awaken",
        "service_idle",
        "service_running",
        "service_partially_running",
        "service_delete",
        "service_update_name",
        "service_update_ip_access_list",
        "service_update_autoscaling_memory",
        "service_update_autoscaling_idling",
        "service_update_password",
        "service_update_autoscaling_replicas",
        "service_update_max_allowable_replicas",
        "service_update_backup_configuration",
        "service_restore_backup",
        "service_update_release_channel",
        "service_update_gpt_usage_consent",
        "service_update_private_endpoints",
        "service_import_to_organization",
        "service_export_from_organization",
        "service_maintenance_start",
        "service_maintenance_end",
        "service_update_core_dump",
        "backup_delete",
    ];
    for t in &types {
        let json = serde_json::json!({"id": "act-1", "type": t});
        let act: Activity = serde_json::from_value(json).unwrap();
        assert_eq!(act.r#type.as_ref().unwrap().to_string(), *t);
    }
}

#[test]
fn test_activity_actor_type_values() {
    use clickhouse_cloud_api::models::Activity;
    for actor_type in &["user", "support", "system", "api"] {
        let json =
            serde_json::json!({"id": "act-1", "type": "user_login", "actorType": actor_type});
        let act: Activity = serde_json::from_value(json).unwrap();
        assert_eq!(act.actor_type.as_ref().unwrap().to_string(), *actor_type);
    }
}

#[test]
fn test_activity_key_update_type_values() {
    use clickhouse_cloud_api::models::Activity;
    let update_types = [
        "created",
        "deleted",
        "name-changed",
        "role-changed",
        "state-changed",
        "date-changed",
        "ip-access-list-changed",
        "org-role-changed",
        "default-service-role-changed",
        "service-role-changed",
        "roles-v2-changed",
    ];
    for t in &update_types {
        let json = serde_json::json!({
            "id": "act-1",
            "type": "openapi_key_update",
            "keyUpdateType": t
        });
        let act: Activity = serde_json::from_value(json).unwrap();
        assert_eq!(act.key_update_type.as_ref().unwrap().to_string(), *t);
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
fn test_service_region_values() {
    let regions = [
        "ap-northeast-1",
        "ap-northeast-2",
        "ap-south-1",
        "ap-southeast-1",
        "ap-southeast-2",
        "eu-central-1",
        "eu-west-1",
        "eu-west-2",
        "il-central-1",
        "us-east-1",
        "us-east-2",
        "us-west-2",
        "us-east1",
        "us-central1",
        "europe-west4",
        "asia-southeast1",
        "asia-northeast1",
        "eastus",
        "eastus2",
        "westus3",
        "germanywestcentral",
        "centralus",
    ];
    for region in &regions {
        let json = serde_json::json!({
            "id": "svc-1",
            "name": "svc",
            "provider": "aws",
            "region": region,
            "state": "running"
        });
        let svc: Service = serde_json::from_value(json).unwrap();
        assert_eq!(svc.region, *region);
    }
}

#[test]
fn test_service_tier_values() {
    let tiers = [
        "development",
        "production",
        "dedicated_high_mem",
        "dedicated_high_cpu",
        "dedicated_standard",
        "dedicated_standard_n2d_standard_4",
        "dedicated_standard_n2d_standard_8",
        "dedicated_standard_n2d_standard_32",
        "dedicated_standard_n2d_standard_128",
        "dedicated_standard_n2d_standard_32_16SSD",
        "dedicated_standard_n2d_standard_64_24SSD",
    ];
    for tier in &tiers {
        let json = serde_json::json!({
            "id": "svc-1",
            "name": "svc",
            "provider": "aws",
            "region": "us-east-1",
            "state": "running",
            "tier": tier
        });
        let svc: Service = serde_json::from_value(json).unwrap();
        assert_eq!(svc.tier.as_deref(), Some(*tier));
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
        "v1-default",
        "v1-highmem-xs",
        "v1-highmem-s",
        "v1-highmem-m",
        "v1-highmem-l",
        "v1-highmem-xl",
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

// --- FromStr rejects unknown values for user input validation ---

#[test]
fn test_cloud_provider_fromstr_rejects_unknown() {
    assert!(CloudProvider::from_str("aws").is_ok());
    assert!(CloudProvider::from_str("gcp").is_ok());
    assert!(CloudProvider::from_str("azure").is_ok());
    let err = CloudProvider::from_str("awss").unwrap_err();
    assert!(err.contains("awss"), "error should mention the bad value");
    assert!(err.contains("aws"), "error should list valid values");
}

#[test]
fn test_cloud_region_fromstr_rejects_unknown() {
    assert!(CloudRegion::from_str("us-east-1").is_ok());
    let err = CloudRegion::from_str("us-east-99").unwrap_err();
    assert!(err.contains("us-east-99"));
}

#[test]
fn test_service_tier_fromstr_rejects_unknown() {
    assert!(ServiceTier::from_str("production").is_ok());
    assert!(ServiceTier::from_str("development").is_ok());
    let err = ServiceTier::from_str("productoin").unwrap_err();
    assert!(err.contains("productoin"));
}

#[test]
fn test_release_channel_fromstr_rejects_unknown() {
    assert!(ReleaseChannel::from_str("slow").is_ok());
    assert!(ReleaseChannel::from_str("default").is_ok());
    assert!(ReleaseChannel::from_str("fast").is_ok());
    let err = ReleaseChannel::from_str("turbo").unwrap_err();
    assert!(err.contains("turbo"));
    assert!(err.contains("slow"));
}

#[test]
fn test_compliance_type_fromstr_rejects_unknown() {
    assert!(ComplianceType::from_str("hipaa").is_ok());
    assert!(ComplianceType::from_str("pci").is_ok());
    let err = ComplianceType::from_str("soc2").unwrap_err();
    assert!(err.contains("soc2"));
}

#[test]
fn test_service_profile_fromstr_rejects_unknown() {
    assert!(ServiceProfile::from_str("v1-default").is_ok());
    let err = ServiceProfile::from_str("v2-default").unwrap_err();
    assert!(err.contains("v2-default"));
}

#[test]
fn test_flexible_enum_known_values() {
    let values = CloudProvider::known_values();
    assert_eq!(values, &["aws", "gcp", "azure"]);
}

// --- Deserialization still accepts unknown values from API responses ---

#[test]
fn test_flexible_enum_deserialize_accepts_unknown() {
    // API responses can contain new values the CLI doesn't know about
    let json = serde_json::json!({
        "id": "svc-1",
        "name": "svc",
        "provider": "oracle",
        "region": "mars-west-1",
        "state": "quantum-superposition"
    });
    let svc: Service = serde_json::from_value(json).unwrap();
    assert_eq!(svc.provider, "oracle");
    assert_eq!(svc.region, "mars-west-1");
    assert_eq!(svc.state, "quantum-superposition");
}
