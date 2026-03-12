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
}

#[test]
fn test_organization_roundtrip() {
    let org = Organization {
        id: "org-1".to_string(),
        name: "My Org".to_string(),
        created_at: Some("2024-01-01T00:00:00Z".to_string()),
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
        "idleScaling": true,
        "idleTimeoutMinutes": 10,
        "ipAccessList": [{"source": "0.0.0.0/0", "description": "Allow all"}],
        "createdAt": "2024-01-01T00:00:00Z",
        "endpoints": [{"protocol": "https", "host": "abc.clickhouse.cloud", "port": 8443}],
        "minReplicaMemoryGb": 24,
        "maxReplicaMemoryGb": 48,
        "numReplicas": 3
    });
    let svc: Service = serde_json::from_value(json).unwrap();
    assert_eq!(svc.id, "svc-1");
    assert_eq!(svc.tier.as_deref(), Some("production"));
    assert_eq!(svc.idle_scaling, Some(true));
    assert_eq!(svc.idle_timeout_minutes, Some(10));
    assert_eq!(svc.ip_access_list.as_ref().unwrap().len(), 1);
    assert_eq!(svc.ip_access_list.as_ref().unwrap()[0].source, "0.0.0.0/0");
    assert_eq!(svc.endpoints.as_ref().unwrap()[0].port, 8443);
    assert_eq!(svc.min_replica_memory_gb, Some(24));
    assert_eq!(svc.max_replica_memory_gb, Some(48));
    assert_eq!(svc.num_replicas, Some(3));
}

#[test]
fn test_service_deserialize_minimal() {
    let json = r#"{"id":"svc-1","name":"svc","provider":"aws","region":"us-east-1","state":"idle"}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.id, "svc-1");
    assert!(svc.tier.is_none());
    assert!(svc.endpoints.is_none());
}

#[test]
fn test_endpoint_deserialize() {
    let json = r#"{"protocol":"https","host":"abc.clickhouse.cloud","port":8443}"#;
    let ep: Endpoint = serde_json::from_str(json).unwrap();
    assert_eq!(ep.protocol, "https");
    assert_eq!(ep.host, "abc.clickhouse.cloud");
    assert_eq!(ep.port, 8443);
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
        "sizeInBytes": 1048576
    });
    let backup: Backup = serde_json::from_value(json).unwrap();
    assert_eq!(backup.id, "backup-1");
    assert_eq!(backup.service_id.as_deref(), Some("svc-1"));
    assert_eq!(backup.status, "done");
    assert_eq!(backup.size_in_bytes, Some(1048576));
    assert_eq!(backup.started_at.as_deref(), Some("2024-01-01T00:00:00Z"));
    assert_eq!(backup.finished_at.as_deref(), Some("2024-01-01T00:05:00Z"));
}

#[test]
fn test_backup_deserialize_minimal() {
    let json = r#"{"id":"backup-1","status":"in_progress"}"#;
    let backup: Backup = serde_json::from_str(json).unwrap();
    assert_eq!(backup.id, "backup-1");
    assert!(backup.service_id.is_none());
    assert!(backup.size_in_bytes.is_none());
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
    // Optional fields should be absent
    assert!(json.get("ipAccessList").is_none());
    assert!(json.get("minReplicaMemoryGb").is_none());
    assert!(json.get("backupId").is_none());
    assert!(json.get("tags").is_none());
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
        byoc_id: Some("byoc-1".to_string()),
        compliance_type: Some("hipaa".to_string()),
        profile: Some("v1-default".to_string()),
        private_preview_terms_checked: Some(true),
    };
    let json = serde_json::to_value(&req).unwrap();

    // Verify camelCase serialization
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
    assert_eq!(json["byocId"], "byoc-1");
    assert_eq!(json["complianceType"], "hipaa");
    assert_eq!(json["profile"], "v1-default");
    assert_eq!(json["privatePreviewTermsChecked"], true);
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

// ── Phase 2: New type tests ────────────────────────────────────────

#[test]
fn test_update_service_request_minimal() {
    let req = UpdateServiceRequest {
        name: Some("new-name".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "new-name");
    assert!(json.get("ipAccessList").is_none());
    assert!(json.get("idleScaling").is_none());
    assert!(json.get("tags").is_none());
}

#[test]
fn test_update_service_request_full() {
    let req = UpdateServiceRequest {
        name: Some("updated".to_string()),
        ip_access_list: Some(vec![IpAccessEntry {
            source: "10.0.0.0/8".to_string(),
            description: Some("VPN".to_string()),
        }]),
        idle_scaling: Some(false),
        idle_timeout_minutes: Some(15),
        tags: Some(vec![ResourceTag {
            key: "env".to_string(),
            value: "staging".to_string(),
        }]),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "updated");
    assert_eq!(json["ipAccessList"][0]["source"], "10.0.0.0/8");
    assert_eq!(json["idleScaling"], false);
    assert_eq!(json["idleTimeoutMinutes"], 15);
    assert_eq!(json["tags"][0]["key"], "env");
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
    assert_eq!(resp.password, "new-secret-123");
}

#[test]
fn test_service_query_endpoint_deserialize() {
    let json = serde_json::json!({"openApiEnabled": true, "roles": ["admin", "reader"]});
    let ep: ServiceQueryEndpoint = serde_json::from_value(json).unwrap();
    assert_eq!(ep.open_api_enabled, Some(true));
    assert_eq!(ep.roles.as_ref().unwrap(), &["admin", "reader"]);
}

#[test]
fn test_create_query_endpoint_request_serialize() {
    let req = CreateQueryEndpointRequest {
        roles: Some(vec!["admin".to_string()]),
        open_api_enabled: Some(true),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["roles"], serde_json::json!(["admin"]));
    assert_eq!(json["openApiEnabled"], true);
}

#[test]
fn test_create_query_endpoint_request_empty() {
    let req = CreateQueryEndpointRequest {
        roles: None,
        open_api_enabled: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("roles").is_none());
    assert!(json.get("openApiEnabled").is_none());
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

// ── Phase 3: Org type tests ────────────────────────────────────────

#[test]
fn test_update_org_request_serialize() {
    let req = UpdateOrgRequest {
        name: Some("New Org Name".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "New Org Name");

    // Verify skip_serializing when None
    let req_empty = UpdateOrgRequest { name: None };
    let json_empty = serde_json::to_value(&req_empty).unwrap();
    assert!(json_empty.get("name").is_none());
}

#[test]
fn test_org_prometheus_deserialize() {
    let json = serde_json::json!({
        "host": "prometheus.example.com",
        "port": "9090",
        "protocol": "https"
    });
    let prom: OrgPrometheus = serde_json::from_value(json).unwrap();
    assert_eq!(prom.host.as_deref(), Some("prometheus.example.com"));
    assert_eq!(prom.port.as_deref(), Some("9090"));
    assert_eq!(prom.protocol.as_deref(), Some("https"));
}

#[test]
fn test_usage_cost_deserialize() {
    let json = serde_json::json!({
        "totalCost": 123.45,
        "currency": "USD",
        "billingPeriodStart": "2024-01-01T00:00:00Z",
        "billingPeriodEnd": "2024-01-31T23:59:59Z",
        "usageDetails": [
            {
                "serviceName": "my-svc",
                "serviceId": "svc-1",
                "cost": 50.0,
                "unit": "credits"
            }
        ]
    });
    let cost: UsageCost = serde_json::from_value(json).unwrap();
    assert_eq!(cost.total_cost, Some(123.45));
    assert_eq!(cost.currency.as_deref(), Some("USD"));
    assert_eq!(cost.billing_period_start.as_deref(), Some("2024-01-01T00:00:00Z"));
    assert_eq!(cost.billing_period_end.as_deref(), Some("2024-01-31T23:59:59Z"));
    let details = cost.usage_details.unwrap();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].service_name.as_deref(), Some("my-svc"));
}

#[test]
fn test_usage_cost_detail_deserialize() {
    let json = serde_json::json!({
        "serviceName": "prod-db",
        "serviceId": "svc-42",
        "cost": 99.99,
        "unit": "credits"
    });
    let detail: UsageCostDetail = serde_json::from_value(json).unwrap();
    assert_eq!(detail.service_name.as_deref(), Some("prod-db"));
    assert_eq!(detail.service_id.as_deref(), Some("svc-42"));
    assert_eq!(detail.cost, Some(99.99));
    assert_eq!(detail.unit.as_deref(), Some("credits"));
}

// ── Phase 4: Member type tests ─────────────────────────────────────

#[test]
fn test_member_deserialize() {
    let json = serde_json::json!({
        "userId": "user-1",
        "email": "alice@example.com",
        "role": "admin",
        "name": "Alice",
        "createdAt": "2024-03-01T12:00:00Z"
    });
    let member: Member = serde_json::from_value(json).unwrap();
    assert_eq!(member.user_id, "user-1");
    assert_eq!(member.email, "alice@example.com");
    assert_eq!(member.role, "admin");
    assert_eq!(member.name.as_deref(), Some("Alice"));
    assert_eq!(member.created_at.as_deref(), Some("2024-03-01T12:00:00Z"));
}

#[test]
fn test_member_deserialize_minimal() {
    let json = r#"{"userId":"user-2","email":"bob@example.com","role":"developer"}"#;
    let member: Member = serde_json::from_str(json).unwrap();
    assert_eq!(member.user_id, "user-2");
    assert_eq!(member.email, "bob@example.com");
    assert_eq!(member.role, "developer");
    assert!(member.name.is_none());
    assert!(member.created_at.is_none());
}

#[test]
fn test_update_member_request_serialize() {
    let req = UpdateMemberRequest {
        role: "admin".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["role"], "admin");
}

#[test]
fn test_invitation_deserialize() {
    let json = serde_json::json!({
        "id": "inv-1",
        "email": "carol@example.com",
        "role": "developer",
        "createdAt": "2024-06-01T00:00:00Z",
        "expiresAt": "2024-06-08T00:00:00Z"
    });
    let inv: Invitation = serde_json::from_value(json).unwrap();
    assert_eq!(inv.id, "inv-1");
    assert_eq!(inv.email, "carol@example.com");
    assert_eq!(inv.role, "developer");
    assert_eq!(inv.created_at.as_deref(), Some("2024-06-01T00:00:00Z"));
    assert_eq!(inv.expires_at.as_deref(), Some("2024-06-08T00:00:00Z"));
}

#[test]
fn test_create_invitation_request_serialize() {
    let req = CreateInvitationRequest {
        email: "dave@example.com".to_string(),
        role: "admin".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["email"], "dave@example.com");
    assert_eq!(json["role"], "admin");
}

// ── Phase 5: API Key type tests ────────────────────────────────────

#[test]
fn test_api_key_deserialize() {
    let json = serde_json::json!({
        "id": "key-1",
        "name": "my-key",
        "state": "enabled",
        "roles": ["admin", "developer"],
        "createdAt": "2024-01-15T10:00:00Z",
        "expiresAt": "2025-01-15T10:00:00Z"
    });
    let key: ApiKey = serde_json::from_value(json).unwrap();
    assert_eq!(key.id, "key-1");
    assert_eq!(key.name, "my-key");
    assert_eq!(key.state, "enabled");
    assert_eq!(key.roles.as_ref().unwrap(), &["admin", "developer"]);
    assert_eq!(key.created_at.as_deref(), Some("2024-01-15T10:00:00Z"));
    assert_eq!(key.expires_at.as_deref(), Some("2025-01-15T10:00:00Z"));
}

#[test]
fn test_create_api_key_request_serialize() {
    let req = CreateApiKeyRequest {
        name: "ci-key".to_string(),
        roles: None,
        expires_at: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "ci-key");
    // Verify optional fields are omitted
    assert!(json.get("roles").is_none());
    assert!(json.get("expiresAt").is_none());
}

#[test]
fn test_create_api_key_response_deserialize() {
    let json = serde_json::json!({
        "apiKey": {
            "id": "key-2",
            "name": "new-key",
            "state": "enabled"
        },
        "keyId": "kid-abc",
        "keySecret": "secret-xyz"
    });
    let resp: CreateApiKeyResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.api_key.id, "key-2");
    assert_eq!(resp.api_key.name, "new-key");
    assert_eq!(resp.api_key.state, "enabled");
    assert_eq!(resp.key_id, "kid-abc");
    assert_eq!(resp.key_secret, "secret-xyz");
}

#[test]
fn test_update_api_key_request_serialize() {
    let req = UpdateApiKeyRequest {
        name: Some("renamed-key".to_string()),
        roles: Some(vec!["query_endpoints".to_string()]),
        state: Some("disabled".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "renamed-key");
    assert_eq!(json["roles"], serde_json::json!(["query_endpoints"]));
    assert_eq!(json["state"], "disabled");
}

// ── Phase 6: Activity, BYOC, Backup Bucket, Backup Config, Prometheus ──

#[test]
fn test_activity_deserialize() {
    let json = serde_json::json!({
        "id": "act-1",
        "type": "service_create",
        "actorType": "user",
        "actorId": "user-5",
        "createdAt": "2024-07-01T08:00:00Z"
    });
    let act: Activity = serde_json::from_value(json).unwrap();
    assert_eq!(act.id, "act-1");
    assert_eq!(act.activity_type, "service_create");
    assert_eq!(act.actor_type.as_deref(), Some("user"));
    assert_eq!(act.actor_id.as_deref(), Some("user-5"));
    assert_eq!(act.created_at.as_deref(), Some("2024-07-01T08:00:00Z"));
}

#[test]
fn test_byoc_infrastructure_deserialize() {
    let json = serde_json::json!({
        "id": "byoc-1",
        "cloudProvider": "aws",
        "regionId": "eu-west-1",
        "state": "infra-ready",
        "accountName": "my-account",
        "displayName": "Production BYOC"
    });
    let byoc: ByocInfrastructure = serde_json::from_value(json).unwrap();
    assert_eq!(byoc.id.as_deref(), Some("byoc-1"));
    assert_eq!(byoc.cloud_provider.as_deref(), Some("aws"));
    assert_eq!(byoc.region_id.as_deref(), Some("eu-west-1"));
    assert_eq!(byoc.state.as_deref(), Some("infra-ready"));
    assert_eq!(byoc.account_name.as_deref(), Some("my-account"));
    assert_eq!(byoc.display_name.as_deref(), Some("Production BYOC"));
}

#[test]
fn test_create_byoc_request_serialize() {
    let req = CreateByocRequest {
        region_id: "us-central1".to_string(),
        account_id: "acct-123".to_string(),
        availability_zone_suffixes: Some(vec!["a".to_string(), "b".to_string()]),
        vpc_cidr_range: Some("10.0.0.0/16".to_string()),
        display_name: Some("My BYOC".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["regionId"], "us-central1");
    assert_eq!(json["accountId"], "acct-123");
    assert_eq!(json["availabilityZoneSuffixes"], serde_json::json!(["a", "b"]));
    assert_eq!(json["vpcCidrRange"], "10.0.0.0/16");
    assert_eq!(json["displayName"], "My BYOC");
}

#[test]
fn test_update_byoc_request_serialize() {
    let req = UpdateByocRequest {
        display_name: Some("Renamed BYOC".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["displayName"], "Renamed BYOC");
}

#[test]
fn test_backup_bucket_deserialize_aws() {
    let json = serde_json::json!({
        "id": "bb-1",
        "bucketProvider": "AWS",
        "bucketPath": "/path/to/backups",
        "iamRoleArn": "arn:aws:iam::123456789012:role/backup-role",
        "iamRoleSessionName": "clickhouse-backup"
    });
    let bucket: BackupBucket = serde_json::from_value(json).unwrap();
    assert_eq!(bucket.id.as_deref(), Some("bb-1"));
    assert_eq!(bucket.bucket_provider, "AWS");
    assert_eq!(bucket.bucket_path.as_deref(), Some("/path/to/backups"));
    assert_eq!(bucket.iam_role_arn.as_deref(), Some("arn:aws:iam::123456789012:role/backup-role"));
}

#[test]
fn test_backup_bucket_deserialize_gcp() {
    let json = serde_json::json!({
        "id": "bb-2",
        "bucketProvider": "GCP",
        "bucketPath": "/gcp/backups",
        "accessKeyId": "GOOG1234"
    });
    let bucket: BackupBucket = serde_json::from_value(json).unwrap();
    assert_eq!(bucket.bucket_provider, "GCP");
    assert_eq!(bucket.access_key_id.as_deref(), Some("GOOG1234"));
}

#[test]
fn test_create_backup_bucket_request_serialize() {
    let req = CreateBackupBucketRequest {
        bucket_provider: "AWS".to_string(),
        bucket_path: "/backups".to_string(),
        iam_role_arn: Some("arn:aws:iam::123456789012:role/backup".to_string()),
        iam_role_session_name: Some("clickhouse".to_string()),
        access_key_id: None,
        secret_access_key: None,
        container_name: None,
        connection_string: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["bucketProvider"], "AWS");
    assert_eq!(json["bucketPath"], "/backups");
    assert_eq!(json["iamRoleArn"], "arn:aws:iam::123456789012:role/backup");
    assert!(json.get("accessKeyId").is_none());
    assert!(json.get("containerName").is_none());
}

#[test]
fn test_update_backup_bucket_request_serialize() {
    let req = UpdateBackupBucketRequest {
        bucket_path: Some("/new/path".to_string()),
        iam_role_arn: Some("arn:aws:iam::123456789012:role/new-role".to_string()),
        iam_role_session_name: None,
        access_key_id: None,
        secret_access_key: None,
        container_name: None,
        connection_string: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["bucketPath"], "/new/path");
    assert_eq!(json["iamRoleArn"], "arn:aws:iam::123456789012:role/new-role");
}

#[test]
fn test_backup_configuration_deserialize() {
    let json = serde_json::json!({
        "schedule": "0 2 * * *",
        "retentionPeriodDays": 30
    });
    let config: BackupConfiguration = serde_json::from_value(json).unwrap();
    assert_eq!(config.schedule.as_deref(), Some("0 2 * * *"));
    assert_eq!(config.retention_period_days, Some(30));
}

#[test]
fn test_update_backup_config_request_serialize() {
    let req = UpdateBackupConfigRequest {
        schedule: Some("0 3 * * *".to_string()),
        retention_period_days: Some(14),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["schedule"], "0 3 * * *");
    assert_eq!(json["retentionPeriodDays"], 14);
}

#[test]
fn test_prometheus_config_deserialize() {
    let json = serde_json::json!({
        "host": "metrics.clickhouse.cloud",
        "port": 9363,
        "protocol": "https"
    });
    let config: PrometheusConfig = serde_json::from_value(json).unwrap();
    assert_eq!(config.host.as_deref(), Some("metrics.clickhouse.cloud"));
    assert_eq!(config.port, Some(9363));
    assert_eq!(config.protocol.as_deref(), Some("https"));
}

#[test]
fn test_setup_prometheus_request_serialize() {
    let req = SetupPrometheusRequest {};
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json, serde_json::json!({}));
}
