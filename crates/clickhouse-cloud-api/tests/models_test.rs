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
    assert_eq!(org.name, Some("My Organization".to_string()));
    assert_eq!(
        org.id,
        Some("a1b2c3d4-e5f6-7890-abcd-ef1234567890".parse().unwrap())
    );
    assert_eq!(org.enable_core_dumps, Some(false));
}

#[test]
fn serialize_organization() {
    let org = Organization {
        id: Some("a1b2c3d4-e5f6-7890-abcd-ef1234567890".parse().unwrap()),
        name: Some("Test Org".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&org).unwrap();
    assert_eq!(json["name"], "Test Org");
    assert_eq!(json["id"], "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
    // None fields should be omitted
    assert!(json.get("createdAt").is_none());
    assert!(json.get("enableCoreDumps").is_none());
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
    assert_eq!(result[0].name, Some("Org 1".to_string()));
    assert_eq!(result[1].name, Some("Org 2".to_string()));
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
    assert_eq!(svc.name, Some("my-service".to_string()));
    assert_eq!(svc.provider, Some(ServiceProvider::Aws));
    assert_eq!(svc.region, Some(ServiceRegion::Us_east_1));
    assert_eq!(svc.state, Some(ServiceState::Running));
    assert_eq!(svc.tier, Some(ServiceTier::Production));
    assert_eq!(svc.num_replicas, Some(3.0));
    assert_eq!(svc.idle_scaling, Some(true));
    assert_eq!(svc.is_primary, Some(true));
}

#[test]
fn serialize_service_post_request() {
    let req = ServicePostRequest {
        name: Some("new-service".to_string()),
        provider: Some(ServicePostRequestProvider::Aws),
        region: Some(ServicePostRequestRegion::Us_east_1),
        tier: Some(ServicePostRequestTier::Production),
        min_total_memory_gb: Some(24.0),
        max_total_memory_gb: Some(48.0),
        num_replicas: Some(3.0),
        idle_scaling: Some(true),
        idle_timeout_minutes: Some(5.0),
        ip_access_list: Some(vec![IpAccessListEntry {
            source: Some("0.0.0.0/0".to_string()),
            description: Some("Anywhere".to_string()),
        }]),
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
    assert_eq!(backup.status, Some(BackupStatus::Done));
    assert_eq!(backup.r#type, Some(BackupType::Full));
    assert_eq!(backup.size_in_bytes, Some(1073741824.0));
    assert_eq!(backup.duration_in_seconds, Some(300.0));
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
    assert_eq!(key.name, Some("My API Key".to_string()));
    assert_eq!(key.state, Some(ApiKeyState::Enabled));
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
    assert_eq!(pipe.name, Some("my-pipe".to_string()));
    assert_eq!(pipe.state, Some(ClickPipeState::Running));
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
    assert_eq!(member.name, Some("John Doe".to_string()));
    assert_eq!(member.email, Some("john@example.com".to_string()));
    assert_eq!(member.role, Some(MemberRole::Admin));
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
    assert_eq!(inv.email, Some("new@example.com".to_string()));
    assert_eq!(inv.role, Some(InvitationRole::Developer));
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
    assert_eq!(config.backup_start_time, Some("02:00".to_string()));
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
    assert_eq!(cost.grand_total_chc, Some(50.25));
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
    assert_eq!(
        config.endpoint_service_id,
        Some("vpce-svc-123456".to_string())
    );
}

#[test]
fn empty_optional_fields_omitted() {
    let org = Organization {
        name: Some("Test".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&org).unwrap();
    assert!(json.get("id").is_none());
    assert!(json.get("createdAt").is_none());
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
    assert_eq!(ep.protocol, Some(ServiceEndpointProtocol::Nativesecure));
    assert_eq!(ep.host, Some("abc123.clickhouse.cloud".to_string()));
    assert_eq!(ep.port, Some(9440.0));
}

#[test]
fn serialize_api_key_post_request() {
    let req = ApiKeyPostRequest {
        name: Some("test-key".to_string()),
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
    assert_eq!(dash.name, Some("My Dashboard".to_string()));
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
    assert_eq!(activity.actor_type, Some(ActivityActortype::Api));
}

#[test]
fn default_struct_all_none() {
    let svc = Service::default();
    assert!(svc.id.is_none());
    assert!(svc.name.is_none());
    assert!(svc.provider.is_none());
    assert!(svc.state.is_none());
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
    assert_eq!(pg.name, Some("my-postgres".to_string()));
}

#[test]
fn unknown_enum_variant_deserializes() {
    // An unknown service state from the API should deserialize into Unknown(String)
    let json = r#"{"state": "brand-new-state"}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.state, Some(ServiceState::Unknown("brand-new-state".to_string())));
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
