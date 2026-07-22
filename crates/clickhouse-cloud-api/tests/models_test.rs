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
    #[cfg(feature = "deprecated-fields")]
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
        #[cfg(feature = "deprecated-fields")]
        tier: Some(ServicePostRequestTier::Production),
        #[cfg(feature = "deprecated-fields")]
        min_total_memory_gb: Some(24.0),
        #[cfg(feature = "deprecated-fields")]
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
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(json["tier"], "production");
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(json["minTotalMemoryGb"], 24.0);
    assert_eq!(json["ipAccessList"][0]["source"], "0.0.0.0/0");
}

#[test]
fn serialize_service_post_request_horizontal_autoscaling() {
    let req = ServicePostRequest {
        name: "horizontal-service".to_string(),
        autoscaling_mode: Some(AutoscalingMode::Horizontal),
        min_replicas: Some(1.0),
        max_replicas: Some(5.0),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["autoscalingMode"], "horizontal");
    assert_eq!(json["minReplicas"], 1.0);
    assert_eq!(json["maxReplicas"], 5.0);

    // Omitted entirely when unset — mutually exclusive with the vertical
    // scaling fields, so they must not serialize as null/defaults.
    let json = serde_json::to_value(ServicePostRequest::default()).unwrap();
    assert!(json.get("minReplicas").is_none());
    assert!(json.get("maxReplicas").is_none());
    assert!(json.get("autoscalingMode").is_none());
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
    #[cfg(feature = "deprecated-fields")]
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
    #[cfg(feature = "deprecated-fields")]
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
        let parsed: ServiceState = serde_json::from_str(&format!(r#""{json_val}""#)).unwrap();
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
        let parsed: ClickPipeState = serde_json::from_str(&format!(r#""{s}""#)).unwrap();
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
    assert_eq!(
        svc.state,
        ServiceState::Unknown("brand-new-state".to_string())
    );
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
    assert_eq!(
        ServiceState::Unknown("brand-new".to_string()).to_string(),
        "brand-new"
    );
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
        min_replicas: None,
        max_replicas: None,
        min_replica_memory_gb: Some(16.0),
        max_replica_memory_gb: Some(64.0),
        idle_scaling: Some(true),
        idle_timeout_minutes: Some(10.0),
        ..Default::default()
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
        #[cfg(feature = "deprecated-fields")]
        min_total_memory_gb: Some(24.0),
        #[cfg(feature = "deprecated-fields")]
        max_total_memory_gb: Some(48.0),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["numReplicas"], 3.0);
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(json["minTotalMemoryGb"], 24.0);
    #[cfg(feature = "deprecated-fields")]
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
        size: PgSize::C6gd_large,
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "pg-new");
    assert_eq!(json["provider"], "aws");
    assert_eq!(json["region"], "us-east-1");
    assert_eq!(json["size"], "c6gd.large");
    assert!(json.get("storageSize").is_none());
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
        #[cfg(feature = "deprecated-fields")]
        role: Some(InvitationPostRequestRole::Developer),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["email"], "alice@example.com");
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(json["role"], "developer");
    // By default the deprecated `role` field is gated out and never serialized.
    #[cfg(not(feature = "deprecated-fields"))]
    assert!(json.get("role").is_none());
}

#[cfg(feature = "deprecated-fields")]
#[test]
fn serialize_member_patch_request() {
    let req = MemberPatchRequest {
        role: Some(MemberPatchRequestRole::Admin),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["role"], "admin");
}

/// In the default build the deprecated request fields don't exist on the
/// struct, so callers can't set them and they never reach the wire.
#[cfg(not(feature = "deprecated-fields"))]
#[test]
fn deprecated_request_fields_absent_by_default() {
    let member = MemberPatchRequest {
        assigned_role_ids: Some(vec!["admin".to_string()]),
    };
    assert!(serde_json::to_value(&member).unwrap().get("role").is_none());

    let invitation = InvitationPostRequest {
        email: "alice@example.com".to_string(),
        assigned_role_ids: vec!["admin".to_string()],
    };
    assert!(
        serde_json::to_value(&invitation)
            .unwrap()
            .get("role")
            .is_none()
    );

    let scaling = ServiceScalingPatchRequest {
        num_replicas: Some(3.0),
        ..Default::default()
    };
    let scaling = serde_json::to_value(&scaling).unwrap();
    assert!(scaling.get("minTotalMemoryGb").is_none());
    assert!(scaling.get("maxTotalMemoryGb").is_none());
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
            max_connections: Some(serde_json::json!(200)),
            autovacuum_max_workers: Some(serde_json::json!(5)),
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig::default(),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert_eq!(json["pgConfig"]["max_connections"], 200);
    assert_eq!(json["pgConfig"]["autovacuum_max_workers"], 5);
    assert!(json.get("pgBouncerConfig").is_some());
}

#[test]
fn serialize_postgres_instance_config_always_includes_both_nested() {
    // The live API rejects PATCH/POST bodies that omit either `pgConfig` or
    // `pgBouncerConfig` with `BAD_REQUEST: ... 'undefined'`, so the envelope
    // always serializes both — defaulting to `{}` — while inner pgConfig
    // fields stay opt-in. See #163 for the matrix evidence.
    let config = PostgresInstanceConfig {
        pg_config: PgConfig {
            max_connections: Some(serde_json::json!(200)),
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig::default(),
    };
    let json = serde_json::to_value(&config).unwrap();
    assert!(
        json.get("pgBouncerConfig").is_some(),
        "pgBouncerConfig must always be present"
    );
    assert_eq!(json["pgBouncerConfig"], serde_json::json!({}));
    assert_eq!(json["pgConfig"]["max_connections"], 200);
    let pg = json["pgConfig"].as_object().unwrap();
    assert_eq!(
        pg.len(),
        1,
        "PgConfig should only serialize the one set field, got {pg:?}"
    );
}

#[test]
fn serialize_clickpipe_object_storage_ingestion_controls() {
    let source = ClickPipePostObjectStorageSource {
        url: "https://bucket.s3.amazonaws.com/events/*.json".to_string(),
        skip_initial_load: Some(true),
        start_after: Some("events/2026-06-01/".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&source).unwrap();
    assert_eq!(json["skipInitialLoad"], true);
    assert_eq!(json["startAfter"], "events/2026-06-01/");

    // Omitted from the wire when unset.
    let json = serde_json::to_value(ClickPipePostObjectStorageSource::default()).unwrap();
    assert!(json.get("skipInitialLoad").is_none());
    assert!(json.get("startAfter").is_none());
}

#[test]
fn deserialize_scaling_schedule_entry_fixed_scaling_fields() {
    let json = r#"{
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "name": "weekday-peak",
        "weekdays": [1, 2, 3, 4, 5],
        "startHourUtc": 8,
        "endHourUtc": 18,
        "isActiveNow": false,
        "autoscalingMode": "vertical",
        "minReplicaMemoryGb": 16,
        "maxReplicaMemoryGb": 32
    }"#;
    let entry: ScalingScheduleEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.autoscaling_mode, AutoscalingMode::Vertical);
    assert_eq!(entry.min_replica_memory_gb, Some(16.0));
    assert_eq!(entry.max_replica_memory_gb, Some(32.0));

    let req = ScalingScheduleEntryRequest {
        name: entry.name.clone(),
        weekdays: entry.weekdays.clone(),
        start_hour_utc: entry.start_hour_utc,
        end_hour_utc: entry.end_hour_utc,
        autoscaling_mode: Some(entry.autoscaling_mode.clone()),
        min_replica_memory_gb: entry.min_replica_memory_gb,
        max_replica_memory_gb: entry.max_replica_memory_gb,
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["autoscalingMode"], "vertical");
    assert_eq!(json["minReplicaMemoryGb"], 16.0);
    assert_eq!(json["maxReplicaMemoryGb"], 32.0);
}

#[test]
fn serialize_postgres_instance_config_default_envelope() {
    // Default envelope serializes to the minimal accepted body shape.
    let json = serde_json::to_value(PostgresInstanceConfig::default()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "pgConfig": {}, "pgBouncerConfig": {} })
    );
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
    #[cfg(feature = "deprecated-fields")]
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

#[cfg(feature = "deprecated-fields")]
#[test]
fn service_deserializes_deprecated_fields() {
    // With the `deprecated-fields` feature on, deprecated fields exist on the
    // struct and deserialize normally. Without the feature they are absent from
    // the struct entirely (see `deprecated_fields_absent_by_default`).
    let json = r#"{"tier":"production","minTotalMemoryGb":24,"maxTotalMemoryGb":48}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.min_total_memory_gb, 24.0);
    assert_eq!(svc.max_total_memory_gb, 48.0);
}

/// In the default build (no `deprecated-fields` feature) deprecated response
/// fields don't exist on the struct, so they can't be read and never appear in
/// serialized output. Deserializing a payload that contains them simply ignores
/// the extra keys.
#[cfg(not(feature = "deprecated-fields"))]
#[test]
fn deprecated_fields_absent_by_default() {
    let svc: Service = serde_json::from_str(
        r#"{"name":"svc","tier":"production","minTotalMemoryGb":24,"maxTotalMemoryGb":48}"#,
    )
    .unwrap();
    let v = serde_json::to_value(&svc).unwrap();
    assert!(v.get("tier").is_none());
    assert!(v.get("minTotalMemoryGb").is_none());
    assert!(v.get("maxTotalMemoryGb").is_none());

    let m: Member = serde_json::from_str(r#"{"name":"Alice","role":"admin"}"#).unwrap();
    assert!(serde_json::to_value(&m).unwrap().get("role").is_none());
}

#[test]
#[cfg(not(feature = "deprecated-fields"))]
fn service_hides_deprecated_fields_when_serializing() {
    let svc: Service = serde_json::from_str(
        r#"{"name":"svc","tier":"production","minTotalMemoryGb":24,"maxTotalMemoryGb":48}"#,
    )
    .unwrap();
    let value = serde_json::to_value(&svc).unwrap();
    let obj = value.as_object().unwrap();
    // Deprecated fields are omitted from serialized output by default.
    assert!(!obj.contains_key("tier"), "tier should be hidden");
    assert!(!obj.contains_key("minTotalMemoryGb"));
    assert!(!obj.contains_key("maxTotalMemoryGb"));
    // Non-deprecated fields are still present.
    assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("svc"));
}

#[test]
#[cfg(feature = "deprecated-fields")]
fn service_shows_deprecated_fields_with_feature() {
    let svc: Service = serde_json::from_str(
        r#"{"tier":"production","minTotalMemoryGb":24,"maxTotalMemoryGb":48}"#,
    )
    .unwrap();
    let value = serde_json::to_value(&svc).unwrap();
    let obj = value.as_object().unwrap();
    assert!(obj.contains_key("tier"));
    assert!(obj.contains_key("minTotalMemoryGb"));
    assert!(obj.contains_key("maxTotalMemoryGb"));
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
    let pg: PostgresService =
        serde_json::from_str(r#"{"id":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"}"#).unwrap();
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
    assert_eq!(
        config.pg_config.max_connections,
        Some(serde_json::json!(200))
    );
}

#[test]
fn deserialize_postgres_instance_config_string_wrapped_numbers() {
    // The live GET endpoint returns numeric pgConfig values wrapped in JSON
    // strings (e.g. "max_connections": "100"). The spec types these fields
    // as string-or-number, so they are modelled as serde_json::Value and
    // both representations must deserialize.
    let json = r#"{
        "pgConfig": {
            "max_connections": "100",
            "random_page_cost": "1.1",
            "max_worker_processes": 8,
            "autovacuum_naptime": "5s",
            "autovacuum_vacuum_scale_factor": "0.2",
            "autovacuum_max_workers": 3
        },
        "pgBouncerConfig": {}
    }"#;
    let config: PostgresInstanceConfig = serde_json::from_str(json).unwrap();
    assert_eq!(
        config.pg_config.max_connections,
        Some(serde_json::json!("100"))
    );
    assert_eq!(
        config.pg_config.random_page_cost,
        Some(serde_json::json!("1.1"))
    );
    assert_eq!(
        config.pg_config.max_worker_processes,
        Some(serde_json::json!(8))
    );
    assert_eq!(
        config.pg_config.autovacuum_naptime,
        Some(serde_json::json!("5s"))
    );
    assert_eq!(
        config.pg_config.autovacuum_vacuum_scale_factor,
        Some(serde_json::json!("0.2"))
    );
    assert_eq!(
        config.pg_config.autovacuum_max_workers,
        Some(serde_json::json!(3))
    );
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
    assert_eq!(
        rpe.status,
        ReversePrivateEndpointStatus::Other("available".to_string())
    );
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
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(s.concurrency, 2);
}

// ===========================================================================
// Upgrade window (issue #203 drift)
// ===========================================================================

#[test]
fn deserialize_upgrade_window() {
    let json = r#"{
        "weekday": 2,
        "startHourUtc": 6,
        "duration": 21600
    }"#;
    let w: UpgradeWindow = serde_json::from_str(json).unwrap();
    assert_eq!(w.weekday, 2);
    assert_eq!(w.start_hour_utc, 6);
    assert_eq!(w.duration, 21600);

    let round_tripped = serde_json::to_value(&w).unwrap();
    assert_eq!(round_tripped["startHourUtc"], 6);
    assert_eq!(round_tripped["weekday"], 2);
    assert_eq!(round_tripped["duration"], 21600);
}

#[test]
fn serialize_upgrade_window_put_request() {
    let req = UpgradeWindowPutRequest {
        weekday: 5,
        start_hour_utc: 18,
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["weekday"], 5);
    assert_eq!(v["startHourUtc"], 18);
    assert!(v.get("start_hour_utc").is_none());
}

// ===========================================================================
// ClickPipe Pub/Sub source (issue #203 drift)
// ===========================================================================

#[test]
fn deserialize_clickpipe_pubsub_source() {
    let json = r#"{
        "topic": "projects/p/topics/t",
        "projectId": "my-project",
        "authentication": "SERVICE_ACCOUNT",
        "format": "JSONEachRow",
        "seekType": "latest",
        "ackDeadline": 60,
        "enableOrdering": true,
        "filter": "attribute.foo = \"bar\""
    }"#;
    let src: ClickPipePubSubSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.topic, "projects/p/topics/t");
    assert_eq!(src.project_id, "my-project");
    assert_eq!(
        src.authentication,
        ClickPipePubSubSourceAuthentication::ServiceAccount
    );
    assert_eq!(src.format, ClickPipePubSubSourceFormat::JSONEachRow);
    assert_eq!(src.seek_type, ClickPipePubSubSourceSeektype::Latest);
    assert_eq!(src.ack_deadline, Some(60));
    assert_eq!(src.enable_ordering, Some(true));
}

#[test]
fn deserialize_clickpipe_post_pubsub_source_required_fields() {
    let json = r#"{
        "topic": "projects/p/topics/t",
        "projectId": "my-project",
        "authentication": "SERVICE_ACCOUNT",
        "format": "JSONEachRow",
        "seekType": "earliest",
        "serviceAccountKey": {
            "serviceAccountFile": "/path/to/key.json"
        }
    }"#;
    let src: ClickPipePostPubSubSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.topic, "projects/p/topics/t");
    assert_eq!(src.seek_type, ClickPipePostPubSubSourceSeektype::Earliest);
    assert_eq!(
        src.service_account_key.service_account_file,
        "/path/to/key.json"
    );
}

#[test]
fn deserialize_clickpipe_source_with_pubsub() {
    let json = r#"{
        "pubsub": {
            "topic": "projects/p/topics/t",
            "projectId": "p",
            "authentication": "SERVICE_ACCOUNT",
            "format": "JSONEachRow",
            "seekType": "latest"
        }
    }"#;
    let src: ClickPipeSource = serde_json::from_str(json).unwrap();
    let pubsub = src.pubsub.expect("pubsub field should populate");
    assert_eq!(pubsub.topic, "projects/p/topics/t");
    assert_eq!(pubsub.format, ClickPipePubSubSourceFormat::JSONEachRow);
}

// ===========================================================================
// ClickStack dashboard containers, heatmap, on-click (issue #203 drift)
// ===========================================================================

#[test]
fn deserialize_clickstack_dashboard_with_containers() {
    let json = r#"{
        "id": "dash-1",
        "name": "Overview",
        "tiles": [
            {
                "id": "tile-1",
                "name": "T1",
                "x": 0, "y": 0, "w": 4, "h": 4,
                "containerId": "c-1",
                "tabId": "t-1"
            }
        ],
        "filters": [],
        "createdAt": "2024-01-01T00:00:00Z",
        "updatedAt": "2024-01-02T00:00:00Z",
        "containers": [
            {
                "id": "c-1",
                "title": "Container A",
                "collapsed": false,
                "tabs": [{"id": "t-1", "title": "Tab 1"}]
            }
        ]
    }"#;
    let dash: ClickStackDashboardResponse = serde_json::from_str(json).unwrap();
    let containers = dash.containers.expect("containers should populate");
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].id, "c-1");
    assert!(!containers[0].collapsed);
    let tabs = containers[0].tabs.as_ref().expect("tabs populated");
    assert_eq!(tabs[0].title, "Tab 1");
    assert_eq!(dash.tiles[0].container_id.as_deref(), Some("c-1"));
    assert_eq!(dash.tiles[0].tab_id.as_deref(), Some("t-1"));
}

#[test]
fn deserialize_clickstack_tile_config_heatmap_variant() {
    // Untagged-enum dispatch must reach the new ClickStackHeatmapChartConfig
    // arm. The discriminator is `displayType: "heatmap"` plus the heatmap-
    // specific `select` shape with `valueExpression`.
    let json = r#"{
        "displayType": "heatmap",
        "sourceId": "src-1",
        "select": [{"valueExpression": "latency_ms"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackHeatmapChartConfig(h) => {
            assert_eq!(h.source_id, "src-1");
            assert_eq!(
                h.display_type,
                ClickStackHeatmapChartConfigDisplaytype::Heatmap
            );
            assert_eq!(h.select.len(), 1);
            assert_eq!(h.select[0].value_expression, "latency_ms");
        }
        other => panic!("expected heatmap variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_search_variant() {
    // ClickStackOnClick is an untagged enum; the Search variant comes first
    // so a "search"-typed payload deserializes through it cleanly.
    let json = r#"{
        "type": "search",
        "target": {"mode": "id", "id": "search-1"}
    }"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    match on_click {
        ClickStackOnClick::ClickStackOnClickSearch(s) => {
            assert_eq!(s.r#type, ClickStackOnClickSearchType::Search);
        }
        other => panic!("expected search variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_dashboard_struct() {
    // We deserialize directly into ClickStackOnClickDashboard rather than the
    // untagged parent because the parent's first variant catches anything
    // with the search/dashboard shape (both inline `type` enums have an
    // Unknown(String) catch-all).
    let json = r#"{
        "type": "dashboard",
        "target": {"mode": "template", "template": "{{x}}"},
        "whereLanguage": "sql",
        "whereTemplate": "x = {{y}}"
    }"#;
    let dash: ClickStackOnClickDashboard = serde_json::from_str(json).unwrap();
    assert_eq!(dash.r#type, ClickStackOnClickDashboardType::Dashboard);
    assert_eq!(dash.where_template.as_deref(), Some("x = {{y}}"));
    match dash.target {
        ClickStackOnClickTarget::ClickStackOnClickTargetTemplateVariant(t) => {
            assert_eq!(t.template, "{{x}}");
        }
        other => panic!("expected template target, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_target_id_variant() {
    let json = r#"{"mode": "id", "id": "abc"}"#;
    let target: ClickStackOnClickTarget = serde_json::from_str(json).unwrap();
    match target {
        ClickStackOnClickTarget::ClickStackOnClickTargetIdVariant(v) => {
            assert_eq!(v.id, "abc");
            assert_eq!(v.mode, ClickStackOnClickTargetIdVariantMode::Id);
        }
        other => panic!("expected id variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_target_template_variant() {
    let json = r#"{"mode": "template", "template": "{{q}}"}"#;
    let target: ClickStackOnClickTarget = serde_json::from_str(json).unwrap();
    match target {
        ClickStackOnClickTarget::ClickStackOnClickTargetTemplateVariant(v) => {
            assert_eq!(v.template, "{{q}}");
            assert_eq!(v.mode, ClickStackOnClickTargetTemplateVariantMode::Template);
        }
        other => panic!("expected template variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_log_source_with_metadata_materialized_views() {
    let json = r#"{
        "id": "src-1",
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts",
        "metadataMaterializedViews": {
            "granularity": "1 hour",
            "keyRollupTable": "logs_keys_1h",
            "kvRollupTable": "logs_kv_1h"
        }
    }"#;
    let src: ClickStackLogSource = serde_json::from_str(json).unwrap();
    let mv = src
        .metadata_materialized_views
        .expect("metadataMaterializedViews should populate");
    assert_eq!(mv.granularity, "1 hour");
    assert_eq!(mv.key_rollup_table, "logs_keys_1h");
    assert_eq!(mv.kv_rollup_table, "logs_kv_1h");
}

#[test]
fn deserialize_clickstack_alert_with_note() {
    let json = r#"{
        "id": "alert-1",
        "name": "High CPU",
        "note": "investigate runaway queries"
    }"#;
    let alert: ClickStackAlertResponse = serde_json::from_str(json).unwrap();
    assert_eq!(alert.note.as_deref(), Some("investigate runaway queries"));
    // Round-trip the optional `note` to confirm it serializes (no rename, but
    // its skip_serializing_if=None gate must let Some(_) through).
    let v = serde_json::to_value(&alert).unwrap();
    assert_eq!(v["note"], "investigate runaway queries");
}

#[test]
fn autoscaling_mode_round_trip() {
    let v = serde_json::to_value(AutoscalingMode::Vertical).unwrap();
    assert_eq!(v, "vertical");
    let v = serde_json::to_value(AutoscalingMode::Horizontal).unwrap();
    assert_eq!(v, "horizontal");
    let parsed: AutoscalingMode = serde_json::from_str("\"vertical\"").unwrap();
    assert_eq!(parsed, AutoscalingMode::Vertical);
    let parsed: AutoscalingMode = serde_json::from_str("\"horizontal\"").unwrap();
    assert_eq!(parsed, AutoscalingMode::Horizontal);
    assert_eq!(AutoscalingMode::default(), AutoscalingMode::Vertical);
    assert_eq!(AutoscalingMode::Vertical.to_string(), "vertical");
    assert_eq!(AutoscalingMode::Horizontal.to_string(), "horizontal");
}

#[test]
fn autoscaling_mode_unknown_catch_all() {
    let parsed: AutoscalingMode = serde_json::from_str("\"crystal\"").unwrap();
    assert_eq!(parsed, AutoscalingMode::Unknown("crystal".to_string()));
    assert_eq!(parsed.to_string(), "crystal");
}

#[test]
fn pg_state_property_stopped() {
    let parsed: PgStateProperty = serde_json::from_str("\"stopped\"").unwrap();
    assert_eq!(parsed, PgStateProperty::Stopped);
    assert_eq!(parsed.to_string(), "stopped");
}

#[test]
fn service_region_ca_central_1() {
    let parsed: ServiceRegion = serde_json::from_str("\"ca-central-1\"").unwrap();
    assert_eq!(parsed, ServiceRegion::Ca_central_1);
    assert_eq!(parsed.to_string(), "ca-central-1");
}

#[test]
fn byoc_config_regionid_ca_central_1() {
    let parsed: ByocConfigRegionid = serde_json::from_str("\"ca-central-1\"").unwrap();
    assert_eq!(parsed, ByocConfigRegionid::Ca_central_1);
    assert_eq!(parsed.to_string(), "ca-central-1");
}

#[test]
fn click_pipe_schema_discovery_response_round_trip() {
    let json = r#"{
        "fields": [
            {"name": "user_id", "type": "Int64", "optional": false},
            {"name": "event", "type": "String", "optional": true}
        ]
    }"#;
    let resp: ClickPipeSchemaDiscoveryResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.fields.len(), 2);
    assert_eq!(resp.fields[0].name, "user_id");
    assert_eq!(resp.fields[0].r#type, "Int64");
    assert_eq!(resp.fields[0].optional, Some(false));
    assert_eq!(resp.fields[1].optional, Some(true));

    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["fields"][0]["name"], "user_id");
    assert_eq!(v["fields"][0]["type"], "Int64");
    assert_eq!(v["fields"][1]["optional"], true);
}

#[test]
fn click_pipe_schema_discovery_request_kafka_source() {
    let req = ClickPipeSchemaDiscoveryRequest {
        source: ClickPipeSchemaDiscoverySource {
            kafka: Some(ClickPipePostKafkaSource::default()),
            kinesis: None,
        },
    };
    let v = serde_json::to_value(&req).unwrap();
    assert!(v["source"]["kafka"].is_object());
    assert!(v["source"].get("kinesis").is_none());
}

#[test]
fn click_pipe_schema_discovery_request_default_omits_sources() {
    let v = serde_json::to_value(ClickPipeSchemaDiscoveryRequest::default()).unwrap();
    assert!(v["source"].get("kafka").is_none());
    assert!(v["source"].get("kinesis").is_none());
}

#[test]
fn click_pipe_schema_discovery_field_nullable_optional() {
    let json = r#"{"name": "col", "type": "Nullable(String)", "optional": null}"#;
    let field: ClickPipeSchemaDiscoveryField = serde_json::from_str(json).unwrap();
    assert_eq!(field.optional, None);
    let v = serde_json::to_value(&field).unwrap();
    assert!(v.get("optional").is_none());
}

#[test]
fn mysql_source_server_id_optional() {
    let json = r#"{"host": "h", "port": 3306, "settings": {"replicationMode": "gtid"}, "tableMappings": [], "serverId": 4242}"#;
    let src: ClickPipeMySQLSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.server_id, Some(4242));

    let json = r#"{"host": "h", "port": 3306, "settings": {"replicationMode": "gtid"}, "tableMappings": []}"#;
    let src: ClickPipeMySQLSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.server_id, None);

    let v = serde_json::to_value(ClickPipeMySQLSource {
        server_id: Some(99),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(v["serverId"], 99);
    let v = serde_json::to_value(ClickPipeMySQLSource::default()).unwrap();
    assert!(v.get("serverId").is_none());
}

#[test]
fn mysql_patch_source_server_id_nullable() {
    let json = r#"{"serverId": null}"#;
    let src: ClickPipePatchMySQLSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.server_id, None);

    let json = r#"{"serverId": 100}"#;
    let src: ClickPipePatchMySQLSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.server_id, Some(100));
}
