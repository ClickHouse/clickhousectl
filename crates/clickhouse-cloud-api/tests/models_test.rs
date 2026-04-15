use clickhouse_cloud_api::models::*;

#[test]
fn deserialize_organization() {
    let json = r#"{
        "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "name": "My Organization",
        "createdAt": "2024-01-15T10:30:00Z",
        "privateEndpoints": [],
        "byocConfig": [],
        "enableCoreDumps": false
    }"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.name, "My Organization");
    assert_eq!(
        org.id,
        "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
            .parse::<uuid::Uuid>()
            .unwrap()
    );
    assert!(!org.enable_core_dumps);
}

#[test]
fn serialize_organization() {
    let org = Organization {
        id: "a1b2c3d4-e5f6-7890-abcd-ef1234567890".parse().unwrap(),
        name: "Test Org".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_value(&org).unwrap();
    assert_eq!(json["name"], "Test Org");
    assert_eq!(json["id"], "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
    // Default fields are still serialized (no skip_serializing_if on required fields)
    assert!(json.get("createdAt").is_some());
    assert!(json.get("enableCoreDumps").is_some());
}

#[test]
fn deserialize_api_response_with_org_list() {
    let json = r#"{
        "status": 200,
        "requestId": "req-uuid-123",
        "result": [
            {
                "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                "name": "Org 1"
            },
            {
                "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
                "name": "Org 2"
            }
        ]
    }"#;
    let resp: ApiResponse<Vec<Organization>> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, Some(200.0));
    assert_eq!(resp.request_id, Some("req-uuid-123".to_string()));
    let result = resp.result.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "Org 1");
    assert_eq!(result[1].name, "Org 2");
}

#[test]
fn deserialize_api_response_error() {
    let json = r#"{
        "status": 401,
        "error": "Unauthorized",
        "requestId": "req-uuid-456"
    }"#;
    let resp: ApiResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, Some(401.0));
    assert_eq!(resp.error, Some("Unauthorized".to_string()));
    assert!(resp.result.is_none());
}

#[test]
fn deserialize_service() {
    let json = r#"{
        "id": "11111111-2222-3333-4444-555555555555",
        "name": "my-service",
        "provider": "aws",
        "region": "us-east-1",
        "state": "running",
        "tier": "production",
        "clickhouseVersion": "24.1",
        "endpoints": [
            {
                "protocol": "nativesecure",
                "host": "abc123.clickhouse.cloud",
                "port": 9440
            }
        ],
        "minTotalMemoryGb": 24,
        "maxTotalMemoryGb": 48,
        "numReplicas": 3,
        "idleScaling": true,
        "idleTimeoutMinutes": 5,
        "ipAccessList": [
            {"source": "0.0.0.0/0", "description": "Anywhere"}
        ],
        "createdAt": "2024-03-01T00:00:00Z",
        "privateEndpointIds": [],
        "isPrimary": true,
        "isReadonly": false,
        "releaseChannel": "default",
        "hasTransparentDataEncryption": false,
        "tags": []
    }"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.name, "my-service");
    assert_eq!(svc.provider, ServiceProvider::Aws);
    assert_eq!(svc.region, ServiceRegion::Us_east_1);
    assert_eq!(svc.state, ServiceState::Running);
    assert_eq!(svc.tier, ServiceTier::Production);
    assert_eq!(svc.num_replicas, 3.0);
    assert!(svc.idle_scaling);
    assert!(svc.is_primary);
}

#[test]
fn serialize_service_post_request() {
    let req = ServicePostRequest {
        name: "new-service".to_string(),
        provider: ServicePostRequestProvider::Aws,
        region: ServicePostRequestRegion::Us_east_1,
        tier: Some(ServicePostRequestTier::Production),
        min_total_memory_gb: Some(24.0),
        max_total_memory_gb: Some(48.0),
        num_replicas: Some(3.0),
        idle_scaling: Some(true),
        idle_timeout_minutes: Some(5.0),
        ip_access_list: vec![IpAccessListEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some("Anywhere".to_string()),
        }],
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "new-service");
    assert_eq!(json["provider"], "aws");
    assert_eq!(json["region"], "us-east-1");
    assert_eq!(json["tier"], "production");
    assert_eq!(json["minTotalMemoryGb"], 24.0);
    assert_eq!(json["ipAccessList"][0]["source"], "0.0.0.0/0");
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
    assert_eq!(backup.status, BackupStatus::Done);
    assert_eq!(backup.r#type, BackupType::Full);
    assert_eq!(backup.size_in_bytes, 1073741824.0);
    assert_eq!(backup.duration_in_seconds, 300.0);
}

#[test]
fn deserialize_api_key() {
    let json = r#"{
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "name": "My API Key",
        "state": "enabled",
        "roles": ["admin"],
        "createdAt": "2024-01-01T00:00:00Z",
        "expireAt": "2025-01-01T00:00:00Z"
    }"#;
    let key: ApiKey = serde_json::from_str(json).unwrap();
    assert_eq!(key.name, "My API Key");
    assert_eq!(key.state, ApiKeyState::Enabled);
}

#[test]
fn deserialize_clickpipe() {
    let json = r#"{
        "id": "11111111-1111-1111-1111-111111111111",
        "serviceId": "22222222-2222-2222-2222-222222222222",
        "name": "my-pipe",
        "state": "Running",
        "createdAt": "2024-06-01T00:00:00Z",
        "updatedAt": "2024-06-01T01:00:00Z"
    }"#;
    let pipe: ClickPipe = serde_json::from_str(json).unwrap();
    assert_eq!(pipe.name, "my-pipe");
    assert_eq!(pipe.state, ClickPipeState::Running);
}

#[test]
fn deserialize_member() {
    let json = r#"{
        "userId": "user-123",
        "name": "John Doe",
        "email": "john@example.com",
        "role": "admin",
        "joinedAt": "2024-01-01T00:00:00Z"
    }"#;
    let member: Member = serde_json::from_str(json).unwrap();
    assert_eq!(member.name, "John Doe");
    assert_eq!(member.email, "john@example.com");
    assert_eq!(member.role, MemberRole::Admin);
}

#[test]
fn deserialize_invitation() {
    let json = r#"{
        "id": "33333333-4444-5555-6666-777777777777",
        "email": "new@example.com",
        "role": "developer",
        "createdAt": "2024-06-01T00:00:00Z"
    }"#;
    let inv: Invitation = serde_json::from_str(json).unwrap();
    assert_eq!(inv.email, "new@example.com");
    assert_eq!(inv.role, InvitationRole::Developer);
}

#[test]
fn deserialize_backup_configuration() {
    let json = r#"{
        "backupPeriodInHours": 24,
        "backupRetentionPeriodInHours": 168,
        "backupStartTime": "02:00"
    }"#;
    let config: BackupConfiguration = serde_json::from_str(json).unwrap();
    assert_eq!(config.backup_period_in_hours, 24.0);
    assert_eq!(config.backup_retention_period_in_hours, 168.0);
    assert_eq!(config.backup_start_time, "02:00");
}

#[test]
fn roundtrip_service_state_patch_request() {
    let req = ServiceStatePatchRequest {
        command: Some(ServiceStatePatchRequestCommand::Start),
    };
    let json = serde_json::to_string(&req).unwrap();
    let deserialized: ServiceStatePatchRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(
        deserialized.command,
        Some(ServiceStatePatchRequestCommand::Start)
    );
}

#[test]
fn deserialize_usage_cost() {
    let json = r#"{
        "costs": [],
        "grandTotalCHC": 50.25
    }"#;
    let cost: UsageCost = serde_json::from_str(json).unwrap();
    assert_eq!(cost.grand_total_chc, 50.25);
}

#[test]
fn deserialize_clickpipe_settings() {
    let json = r#"{
        "streaming_max_insert_wait_ms": 5000,
        "object_storage_concurrency": null,
        "clickhouse_max_threads": 4
    }"#;
    let settings: ClickPipeSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.streaming_max_insert_wait_ms, Some(5000));
    assert_eq!(settings.object_storage_concurrency, None);
    assert_eq!(settings.clickhouse_max_threads, Some(4));
}

#[test]
fn deserialize_private_endpoint_config() {
    let json = r#"{
        "endpointServiceId": "vpce-svc-123456",
        "privateDnsHostname": "abc.vpce.clickhouse.cloud"
    }"#;
    let config: PrivateEndpointConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.endpoint_service_id, "vpce-svc-123456");
}

#[test]
fn required_fields_always_serialized() {
    let org = Organization {
        name: "Test".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_value(&org).unwrap();
    // Required fields are always present (even with default values)
    assert!(json.get("id").is_some());
    assert!(json.get("createdAt").is_some());
    assert_eq!(json["name"], "Test");
}

#[test]
fn deserialize_service_endpoint() {
    let json = r#"{
        "protocol": "nativesecure",
        "host": "abc123.clickhouse.cloud",
        "port": 9440
    }"#;
    let ep: ServiceEndpoint = serde_json::from_str(json).unwrap();
    assert_eq!(ep.protocol, ServiceEndpointProtocol::Nativesecure);
    assert_eq!(ep.host, "abc123.clickhouse.cloud");
    assert_eq!(ep.port, 9440.0);
}

#[test]
fn serialize_api_key_post_request() {
    let req = ApiKeyPostRequest {
        name: "test-key".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "test-key");
}

#[test]
fn deserialize_clickstack_dashboard_response() {
    let json = r#"{
        "id": "dash-123",
        "name": "My Dashboard",
        "tiles": [],
        "filters": [],
        "createdAt": "2024-01-01T00:00:00Z",
        "updatedAt": "2024-01-02T00:00:00Z"
    }"#;
    let dash: ClickStackDashboardResponse = serde_json::from_str(json).unwrap();
    assert_eq!(dash.name, "My Dashboard");
}

#[test]
fn service_provider_enum_values() {
    let aws: ServiceProvider = serde_json::from_str(r#""aws""#).unwrap();
    let gcp: ServiceProvider = serde_json::from_str(r#""gcp""#).unwrap();
    let azure: ServiceProvider = serde_json::from_str(r#""azure""#).unwrap();
    assert_eq!(aws, ServiceProvider::Aws);
    assert_eq!(gcp, ServiceProvider::Gcp);
    assert_eq!(azure, ServiceProvider::Azure);
}

#[test]
fn service_state_enum_roundtrip() {
    let states = [
        ("starting", ServiceState::Starting),
        ("stopping", ServiceState::Stopping),
        ("running", ServiceState::Running),
        ("stopped", ServiceState::Stopped),
        ("idle", ServiceState::Idle),
    ];
    for (json_val, expected) in states {
        let parsed: ServiceState =
            serde_json::from_str(&format!(r#""{json_val}""#)).unwrap();
        assert_eq!(parsed, expected);

        let serialized = serde_json::to_string(&expected).unwrap();
        assert_eq!(serialized, format!(r#""{json_val}""#));
    }
}

#[test]
fn clickpipe_state_all_variants() {
    let states = [
        "Unknown",
        "Provisioning",
        "Running",
        "Stopping",
        "Stopped",
        "Failed",
        "Completed",
        "InternalError",
        "Setup",
        "Snapshot",
        "Paused",
        "Pausing",
        "Modifying",
        "Resync",
    ];
    for s in states {
        let parsed: ClickPipeState =
            serde_json::from_str(&format!(r#""{s}""#)).unwrap();
        let serialized = serde_json::to_string(&parsed).unwrap();
        assert_eq!(serialized, format!(r#""{s}""#));
    }
}

#[test]
fn deserialize_activity() {
    let json = r#"{
        "actorType": "api",
        "actorId": "actor-123",
        "createdAt": "2024-06-01T00:00:00Z"
    }"#;
    let activity: Activity = serde_json::from_str(json).unwrap();
    assert_eq!(activity.actor_type, ActivityActortype::Api);
}

#[test]
fn default_struct_has_defaults() {
    let svc = Service::default();
    assert_eq!(svc.id, uuid::Uuid::default());
    assert_eq!(svc.name, "");
    assert_eq!(svc.provider, ServiceProvider::default());
    assert_eq!(svc.state, ServiceState::default());
}

#[test]
fn deserialize_postgres_service() {
    let json = r#"{
        "id": "44444444-5555-6666-7777-888888888888",
        "name": "my-postgres",
        "provider": "aws",
        "region": "us-east-1",
        "state": "running"
    }"#;
    let pg: PostgresService = serde_json::from_str(json).unwrap();
    assert_eq!(pg.name, "my-postgres");
}

#[test]
fn unknown_enum_variant_deserializes() {
    // An unknown service state from the API should deserialize into Unknown(String)
    let json = r#"{"state": "brand-new-state"}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.state, ServiceState::Unknown("brand-new-state".to_string()));
}

#[test]
fn unknown_enum_variant_roundtrips() {
    let state = ServiceState::Unknown("future-state".to_string());
    let json = serde_json::to_string(&state).unwrap();
    assert_eq!(json, r#""future-state""#);
    let back: ServiceState = serde_json::from_str(&json).unwrap();
    assert_eq!(back, state);
}

#[test]
fn known_enum_variant_still_deserializes() {
    let json = r#""running""#;
    let state: ServiceState = serde_json::from_str(json).unwrap();
    assert_eq!(state, ServiceState::Running);
}

#[test]
fn unknown_enum_display() {
    assert_eq!(ServiceState::Running.to_string(), "running");
    assert_eq!(ServiceState::Unknown("brand-new".to_string()).to_string(), "brand-new");
}

// ===========================================================================
// ApiResponse envelope edge cases
// ===========================================================================

#[test]
fn api_response_result_explicitly_null() {
    let json = r#"{"status": 200, "requestId": "req-1", "result": null}"#;
    let resp: ApiResponse<Vec<Organization>> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, Some(200.0));
    assert!(resp.result.is_none());
}

#[test]
fn api_response_missing_status() {
    let json = r#"{"result": []}"#;
    let resp: ApiResponse<Vec<Organization>> = serde_json::from_str(json).unwrap();
    assert!(resp.status.is_none());
    assert!(resp.request_id.is_none());
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[test]
fn api_response_extra_fields_ignored() {
    let json = r#"{
        "status": 200,
        "requestId": "req-1",
        "result": {"name": "Test"},
        "extraField": true,
        "anotherField": 42,
        "nestedExtra": {"a": 1}
    }"#;
    let resp: ApiResponse<Organization> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, Some(200.0));
    let org = resp.result.unwrap();
    assert_eq!(org.name, "Test");
}

#[test]
fn api_response_empty_object() {
    let json = r#"{}"#;
    let resp: ApiResponse<Organization> = serde_json::from_str(json).unwrap();
    assert!(resp.status.is_none());
    assert!(resp.request_id.is_none());
    assert!(resp.result.is_none());
    assert!(resp.error.is_none());
}

// ===========================================================================
// Request body serialization (camelCase, None omission, enum variants)
// ===========================================================================

#[test]
fn serialize_service_patch_request() {
    let req = ServicePatchRequest {
        name: Some("renamed".to_string()),
        release_channel: Some(ServicePatchRequestReleasechannel::Default),
        enable_core_dumps: Some(false),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "renamed");
    assert_eq!(json["releaseChannel"], "default");
    assert_eq!(json["enableCoreDumps"], false);
    // None fields must be omitted
    assert!(json.get("ipAccessList").is_none());
    assert!(json.get("privateEndpointIds").is_none());
    assert!(json.get("endpoints").is_none());
    assert!(json.get("tags").is_none());
}

#[test]
fn serialize_service_state_patch_request_start() {
    let req = ServiceStatePatchRequest {
        command: Some(ServiceStatePatchRequestCommand::Start),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["command"], "start");
}

#[test]
fn serialize_service_state_patch_request_stop() {
    let req = ServiceStatePatchRequest {
        command: Some(ServiceStatePatchRequestCommand::Stop),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["command"], "stop");
}

#[test]
fn serialize_service_replica_scaling_patch_request() {
    let req = ServiceReplicaScalingPatchRequest {
        num_replicas: Some(5.0),
        min_replica_memory_gb: Some(16.0),
        max_replica_memory_gb: Some(64.0),
        idle_scaling: Some(true),
        idle_timeout_minutes: Some(10.0),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["numReplicas"], 5.0);
    assert_eq!(json["minReplicaMemoryGb"], 16.0);
    assert_eq!(json["maxReplicaMemoryGb"], 64.0);
    assert_eq!(json["idleScaling"], true);
    assert_eq!(json["idleTimeoutMinutes"], 10.0);
}

#[test]
fn serialize_service_scaling_patch_request() {
    let req = ServiceScalingPatchRequest {
        num_replicas: Some(3.0),
        min_total_memory_gb: Some(24.0),
        max_total_memory_gb: Some(48.0),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["numReplicas"], 3.0);
    assert_eq!(json["minTotalMemoryGb"], 24.0);
    assert_eq!(json["maxTotalMemoryGb"], 48.0);
    assert!(json.get("idleScaling").is_none());
}

#[test]
fn serialize_service_password_patch_request_default() {
    let req = ServicePasswordPatchRequest::default();
    let json = serde_json::to_value(&req).unwrap();
    // All fields should be omitted, leaving just {}
    assert_eq!(json, serde_json::json!({}));
}

#[test]
fn serialize_clickpipe_state_patch_request() {
    let req = ClickPipeStatePatchRequest {
        command: Some(ClickPipeStatePatchRequestCommand::Start),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["command"], "start");

    let stop = ClickPipeStatePatchRequest {
        command: Some(ClickPipeStatePatchRequestCommand::Stop),
    };
    let json = serde_json::to_value(&stop).unwrap();
    assert_eq!(json["command"], "stop");
}

#[test]
fn serialize_clickpipes_cdc_scaling_patch_request() {
    let req = ClickPipesCdcScalingPatchRequest {
        replica_cpu_millicores: Some(4000),
        replica_memory_gb: Some(16.0),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["replicaCpuMillicores"], 4000);
    assert_eq!(json["replicaMemoryGb"], 16.0);
}

#[test]
fn serialize_backup_configuration_patch_request() {
    let req = BackupConfigurationPatchRequest {
        backup_period_in_hours: Some(12.0),
        backup_retention_period_in_hours: Some(336.0),
        backup_start_time: Some("03:00".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["backupPeriodInHours"], 12.0);
    assert_eq!(json["backupRetentionPeriodInHours"], 336.0);
    assert_eq!(json["backupStartTime"], "03:00");
}

#[test]
fn serialize_postgres_service_post_request() {
    let req = PostgresServicePostRequest {
        name: "pg-new".to_string(),
        provider: PgProvider::Aws,
        region: "us-east-1".to_string(),
        size: PgSize::C6gd_medium,
        storage_size: 100,
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "pg-new");
    assert_eq!(json["provider"], "aws");
    assert_eq!(json["region"], "us-east-1");
    assert_eq!(json["size"], "c6gd.medium");
    assert_eq!(json["storageSize"], 100);
    // Optional fields omitted
    assert!(json.get("haType").is_none());
    assert!(json.get("pgConfig").is_none());
    assert!(json.get("pgBouncerConfig").is_none());
}

#[test]
fn serialize_postgres_service_set_state() {
    let req = PostgresServiceSetState {
        command: PostgresServiceSetStateCommand::Restart,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["command"], "restart");
}

#[test]
fn serialize_postgres_service_set_password() {
    let req = PostgresServiceSetPassword {
        password: "s3cur3".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["password"], "s3cur3");
}

#[test]
fn serialize_postgres_read_replica_request() {
    let req = PostgresServiceReadReplicaRequest {
        name: "pg-replica".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "pg-replica");
    assert!(json.get("pgConfig").is_none());
    assert!(json.get("pgBouncerConfig").is_none());
}

#[test]
fn serialize_byoc_infrastructure_post_request() {
    let req = ByocInfrastructurePostRequest {
        account_id: "123456789012".to_string(),
        display_name: "My BYOC".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["accountId"], "123456789012");
    assert_eq!(json["displayName"], "My BYOC");
}

#[test]
fn serialize_byoc_infrastructure_patch_request() {
    let req = ByocInfrastructurePatchRequest {
        display_name: Some("Renamed".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["displayName"], "Renamed");
}

#[test]
fn serialize_invitation_post_request() {
    let req = InvitationPostRequest {
        email: "alice@example.com".to_string(),
        role: InvitationPostRequestRole::Developer,
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["email"], "alice@example.com");
    assert_eq!(json["role"], "developer");
}

#[test]
fn serialize_member_patch_request() {
    let req = MemberPatchRequest {
        role: Some(MemberPatchRequestRole::Admin),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["role"], "admin");
}

#[test]
fn serialize_clickpipe_patch_request() {
    let req = ClickPipePatchRequest {
        name: Some("renamed-pipe".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "renamed-pipe");
    assert!(json.get("source").is_none());
    assert!(json.get("destination").is_none());
}

#[test]
fn serialize_create_reverse_private_endpoint() {
    let req = CreateReversePrivateEndpoint {
        description: "Test RPE".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["description"], "Test RPE");
}

#[test]
fn serialize_instance_query_endpoint_post_request() {
    let req = InstanceServiceQueryApiEndpointsPostRequest {
        allowed_origins: "https://example.com".to_string(),
        roles: vec!["reader".to_string()],
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["allowedOrigins"], "https://example.com");
    assert_eq!(json["roles"], serde_json::json!(["reader"]));
}

#[test]
fn serialize_servic_private_endpointe_post_request() {
    let req = ServicPrivateEndpointePostRequest {
        id: "vpce-abc".to_string(),
        description: "My PE".to_string(),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["id"], "vpce-abc");
    assert_eq!(json["description"], "My PE");
}

#[test]
fn serialize_postgres_instance_config() {
    let config = PostgresInstanceConfig {
        pg_config: PgConfig {
            max_connections: 200,
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig {},
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["pgConfig"]["max_connections"], 200);
    assert!(json.get("pgBouncerConfig").is_some());
}

// ===========================================================================
// Forward compatibility: extra unknown fields ignored
// ===========================================================================

#[test]
fn organization_ignores_extra_fields() {
    let json = r#"{"name":"Test","brandNewField":"surprise","anotherNew":42}"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.name, "Test");
}

#[test]
fn service_ignores_extra_fields() {
    let json = r#"{"name":"svc","state":"running","futureField":"v2","nested":{"a":1}}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.name, "svc");
    assert_eq!(svc.state, ServiceState::Running);
}

#[test]
fn clickpipe_ignores_extra_fields() {
    let json = r#"{"name":"pipe","state":"Running","newFeatureFlag":true}"#;
    let pipe: ClickPipe = serde_json::from_str(json).unwrap();
    assert_eq!(pipe.name, "pipe");
    assert_eq!(pipe.state, ClickPipeState::Running);
}

#[test]
fn backup_ignores_extra_fields() {
    let json = r#"{"status":"done","type":"full","compressionRatio":0.85}"#;
    let backup: Backup = serde_json::from_str(json).unwrap();
    assert_eq!(backup.status, BackupStatus::Done);
}

#[test]
fn api_key_ignores_extra_fields() {
    let json = r#"{"name":"key","state":"enabled","rotationPolicy":"weekly"}"#;
    let key: ApiKey = serde_json::from_str(json).unwrap();
    assert_eq!(key.name, "key");
    assert_eq!(key.state, ApiKeyState::Enabled);
}

#[test]
fn member_ignores_extra_fields() {
    let json = r#"{"name":"Alice","role":"admin","department":"eng","mfa":true}"#;
    let m: Member = serde_json::from_str(json).unwrap();
    assert_eq!(m.name, "Alice");
    assert_eq!(m.role, MemberRole::Admin);
}

#[test]
fn invitation_ignores_extra_fields() {
    let json = r#"{"email":"a@b.com","role":"developer","expiresIn":"7d"}"#;
    let inv: Invitation = serde_json::from_str(json).unwrap();
    assert_eq!(inv.email, "a@b.com");
}

#[test]
fn postgres_service_ignores_extra_fields() {
    let json = r#"{"name":"pg","state":"running","maintenanceWindow":"sun-02:00"}"#;
    let pg: PostgresService = serde_json::from_str(json).unwrap();
    assert_eq!(pg.name, "pg");
}

#[test]
fn activity_ignores_extra_fields() {
    let json = r#"{"actorType":"user","sourceIp":"1.2.3.4"}"#;
    let a: Activity = serde_json::from_str(json).unwrap();
    assert_eq!(a.actor_type, ActivityActortype::User);
}

#[test]
fn backup_configuration_ignores_extra_fields() {
    let json = r#"{"backupPeriodInHours":24,"backupRetentionPeriodInHours":168,"compressionEnabled":true}"#;
    let c: BackupConfiguration = serde_json::from_str(json).unwrap();
    assert_eq!(c.backup_period_in_hours, 24.0);
}

// ===========================================================================
// Minimal/partial response deserialization
// ===========================================================================

#[test]
fn service_minimal_response() {
    let json = r#"{"id":"11111111-2222-3333-4444-555555555555"}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(
        svc.id,
        "11111111-2222-3333-4444-555555555555"
            .parse::<uuid::Uuid>()
            .unwrap()
    );
    // Missing fields get their default values
    assert_eq!(svc.name, "");
    assert_eq!(svc.provider, ServiceProvider::default());
    assert_eq!(svc.state, ServiceState::default());
    assert!(svc.endpoints.is_empty());
}

#[test]
fn service_empty_object() {
    let svc: Service = serde_json::from_str("{}").unwrap();
    assert_eq!(svc.id, uuid::Uuid::default());
    assert_eq!(svc.name, "");
}

#[test]
fn organization_minimal_response() {
    let org: Organization = serde_json::from_str(r#"{"name":"X"}"#).unwrap();
    assert_eq!(org.name, "X");
    assert_eq!(org.id, uuid::Uuid::default());
    assert_eq!(org.created_at, chrono::DateTime::<chrono::Utc>::default());
}

#[test]
fn clickpipe_minimal_response() {
    let pipe: ClickPipe = serde_json::from_str("{}").unwrap();
    assert_eq!(pipe.id, uuid::Uuid::default());
    assert_eq!(pipe.name, "");
    assert_eq!(pipe.state, ClickPipeState::default());
}

#[test]
fn postgres_service_minimal_response() {
    let pg: PostgresService = serde_json::from_str(r#"{"id":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"}"#).unwrap();
    assert_eq!(pg.name, "");
    assert_eq!(pg.state, PgStateProperty::default());
}

#[test]
fn backup_minimal_response() {
    let b: Backup = serde_json::from_str("{}").unwrap();
    assert_eq!(b.id, uuid::Uuid::default());
    assert_eq!(b.status, BackupStatus::default());
    assert_eq!(b.size_in_bytes, 0.0);
}

#[test]
fn api_key_minimal_response() {
    let k: ApiKey = serde_json::from_str(r#"{"name":"k"}"#).unwrap();
    assert_eq!(k.name, "k");
    assert_eq!(k.id, uuid::Uuid::default());
    assert_eq!(k.state, ApiKeyState::default());
}

#[test]
fn clickstack_dashboard_minimal_response() {
    let d: ClickStackDashboardResponse = serde_json::from_str("{}").unwrap();
    assert_eq!(d.id, "");
    assert_eq!(d.name, "");
}

// ===========================================================================
// Extended model deserialization (complex/nested types)
// ===========================================================================

#[test]
fn deserialize_aws_backup_bucket() {
    let json = r#"{
        "bucketPath": "s3://my-bucket/prefix",
        "bucketProvider": "AWS",
        "iamRoleArn": "arn:aws:iam::123:role/backup",
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    }"#;
    let b: AwsBackupBucket = serde_json::from_str(json).unwrap();
    assert_eq!(b.bucket_path, "s3://my-bucket/prefix");
    assert_eq!(b.iam_role_arn, "arn:aws:iam::123:role/backup");
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
        assert_eq!(aws.bucket_path, "s3://my-bucket/prefix");
        assert_eq!(aws.iam_role_arn, "arn:aws:iam::123:role/backup");
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
        assert_eq!(gcp.access_key_id, "GOOG1234567890");
        assert_eq!(gcp.bucket_path, "gs://my-gcp-bucket/prefix");
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
        assert_eq!(azure.container_name, "my-container");
    }
}

#[test]
fn deserialize_backup_bucket_unknown_provider() {
    let json = r#"{
        "bucketProvider": "NEW_PROVIDER",
        "somefield": "somevalue",
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
    }"#;
    let b: BackupBucket = serde_json::from_str(json).unwrap();
    assert!(matches!(b, BackupBucket::Unknown(_)));
}

#[test]
fn deserialize_service_post_response() {
    let json = r#"{
        "service": {
            "id": "11111111-2222-3333-4444-555555555555",
            "name": "new-svc",
            "state": "provisioning"
        },
        "password": "gen-pw-123"
    }"#;
    let resp: ServicePostResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.password, "gen-pw-123");
    assert_eq!(resp.service.name, "new-svc");
    assert_eq!(resp.service.state, ServiceState::Provisioning);
}

#[test]
fn deserialize_usage_cost_with_records() {
    let json = r#"{
        "costs": [
            {
                "name": "compute",
                "totalCHC": 25.5
            },
            {
                "name": "storage",
                "totalCHC": 10.0
            }
        ],
        "grandTotalCHC": 35.5
    }"#;
    let cost: UsageCost = serde_json::from_str(json).unwrap();
    assert_eq!(cost.grand_total_chc, 35.5);
    assert_eq!(cost.costs.len(), 2);
}

#[test]
fn deserialize_postgres_instance_config() {
    let json = r#"{
        "pgConfig": {
            "max_connections": 200,
            "shared_buffers": "256MB"
        },
        "pgBouncerConfig": {}
    }"#;
    let config: PostgresInstanceConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.pg_config.max_connections, 200);
}

#[test]
fn deserialize_reverse_private_endpoint() {
    let json = r#"{
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "description": "MSK endpoint",
        "status": "available"
    }"#;
    let rpe: ReversePrivateEndpoint = serde_json::from_str(json).unwrap();
    assert_eq!(rpe.description, "MSK endpoint");
    assert_eq!(rpe.status, ReversePrivateEndpointStatus::Other("available".to_string()));
}

#[test]
fn deserialize_clickpipe_kafka_source() {
    let json = r#"{
        "brokers": "broker1:9092,broker2:9092",
        "topics": "my-topic",
        "groupId": "my-group",
        "securityProtocol": "SASL_SSL"
    }"#;
    let src: ClickPipeKafkaSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.brokers, "broker1:9092,broker2:9092");
    assert_eq!(src.topics, "my-topic");
}

#[test]
fn deserialize_clickpipe_destination() {
    let json = r#"{
        "database": "default",
        "table": "events",
        "managedTable": true,
        "columns": [
            {"name": "id", "type": "UInt64"},
            {"name": "ts", "type": "DateTime"}
        ]
    }"#;
    let dest: ClickPipeDestination = serde_json::from_str(json).unwrap();
    assert_eq!(dest.database, "default");
    assert_eq!(dest.table, "events");
    assert_eq!(dest.columns.len(), 2);
    assert_eq!(dest.columns[0].name, "id");
}

#[test]
fn deserialize_clickpipe_scaling() {
    let json = r#"{
        "replicas": 3,
        "concurrency": 2
    }"#;
    let s: ClickPipeScaling = serde_json::from_str(json).unwrap();
    assert_eq!(s.replicas, 3);
    assert_eq!(s.concurrency, 2);
}
