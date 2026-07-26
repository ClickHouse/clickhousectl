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
fn deserialize_clickstack_log_source_without_id() {
    // `id` is server-generated and omitted from create/update request payloads;
    // it must deserialize to None and be dropped on serialize.
    let json = r#"{
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts"
    }"#;
    let src: ClickStackLogSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.id, None);
    let v = serde_json::to_value(&src).unwrap();
    assert!(v.get("id").is_none(), "id must be omitted when None");
}

#[test]
fn deserialize_clickstack_trace_source_requires_default_table_select_expression() {
    let json = r#"{
        "id": "trace-1",
        "kind": "trace",
        "name": "traces",
        "connection": "conn-1",
        "defaultTableSelectExpression": "Timestamp, SpanName",
        "from": {"databaseName": "default", "tableName": "traces"},
        "timestampValueExpression": "Timestamp",
        "durationExpression": "Duration",
        "durationPrecision": 9,
        "traceIdExpression": "TraceId",
        "spanIdExpression": "SpanId",
        "parentSpanIdExpression": "ParentSpanId",
        "spanNameExpression": "SpanName",
        "spanKindExpression": "SpanKind"
    }"#;
    let src: ClickStackTraceSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.default_table_select_expression, "Timestamp, SpanName");
    let v = serde_json::to_value(&src).unwrap();
    assert_eq!(v["defaultTableSelectExpression"], "Timestamp, SpanName");
}

#[test]
fn round_trip_clickstack_log_source_new_fields() {
    let json = r#"{
        "id": "src-1",
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts",
        "section": "Billing",
        "disabled": false,
        "knownColumnsListExpression": "Timestamp, Body, ServiceName",
        "useTextIndexForImplicitColumn": "auto"
    }"#;
    let src: ClickStackLogSource = serde_json::from_str(json).unwrap();
    assert_eq!(src.section.as_deref(), Some("Billing"));
    assert_eq!(src.disabled, Some(false));
    assert_eq!(
        src.known_columns_list_expression.as_deref(),
        Some("Timestamp, Body, ServiceName")
    );
    assert_eq!(
        src.use_text_index_for_implicit_column,
        Some(ClickStackLogSourceUsetextindexforimplicitcolumn::Auto)
    );
    let v = serde_json::to_value(&src).unwrap();
    assert_eq!(v["section"], "Billing");
    assert_eq!(v["disabled"], false);
    assert_eq!(
        v["knownColumnsListExpression"],
        "Timestamp, Body, ServiceName"
    );
    assert_eq!(v["useTextIndexForImplicitColumn"], "auto");
}

#[test]
fn clickstack_source_dispatches_promql_variant() {
    let json = r#"{
        "id": "src-1",
        "kind": "promql",
        "name": "Prometheus Metrics",
        "connection": "conn-1",
        "from": {"databaseName": "default", "tableName": "metrics"},
        "timestampValueExpression": "timestamp"
    }"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match source {
        ClickStackSource::ClickStackPromqlSource(src) => {
            assert_eq!(src.kind, ClickStackPromqlSourceKind::Promql);
            assert_eq!(src.name, "Prometheus Metrics");
            assert_eq!(src.connection, "conn-1");
            assert_eq!(src.id.as_deref(), Some("src-1"));
        }
        other => panic!("expected Promql source, got {other}"),
    }
}

#[test]
fn clickstack_source_dispatches_log_variant() {
    // Regression: a log-source payload must not be swallowed by the new Promql
    // variant and must still resolve to the log-source variant.
    let json = r#"{
        "id": "src-1",
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts"
    }"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match source {
        ClickStackSource::ClickStackLogSource(src) => {
            assert_eq!(src.kind, ClickStackLogSourceKind::Log);
            assert_eq!(src.name, "logs");
        }
        other => panic!("expected log source, got {other}"),
    }
}

#[test]
fn clickstack_source_dispatches_trace_variant() {
    // Regression (PR #311 review P1-1): a spec-valid trace payload shares every
    // Log-required field (defaultTableSelectExpression et al.), so untagged
    // matching would greedily resolve it to the Log variant and drop trace-only
    // fields. Discriminator dispatch on `kind` must route it to the Trace
    // variant with its trace-only fields intact.
    let json = r#"{
        "id": "trace-1",
        "kind": "trace",
        "name": "traces",
        "connection": "conn-1",
        "from": {"databaseName": "default", "tableName": "traces"},
        "timestampValueExpression": "Timestamp",
        "defaultTableSelectExpression": "Timestamp, SpanName",
        "durationExpression": "Duration",
        "durationPrecision": 9,
        "parentSpanIdExpression": "ParentSpanId",
        "spanIdExpression": "SpanId",
        "spanKindExpression": "SpanKind",
        "spanNameExpression": "SpanName",
        "traceIdExpression": "TraceId"
    }"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match source {
        ClickStackSource::ClickStackTraceSource(src) => {
            assert_eq!(src.kind, ClickStackTraceSourceKind::Trace);
            assert_eq!(src.name, "traces");
            assert_eq!(src.default_table_select_expression, "Timestamp, SpanName");
            assert_eq!(src.duration_expression, "Duration");
            assert_eq!(src.duration_precision, 9);
            assert_eq!(src.parent_span_id_expression, "ParentSpanId");
            assert_eq!(src.span_id_expression, "SpanId");
            assert_eq!(src.span_kind_expression, "SpanKind");
            assert_eq!(src.span_name_expression, "SpanName");
            assert_eq!(src.trace_id_expression, "TraceId");
        }
        other => panic!("expected trace source, got {other}"),
    }
}

#[test]
fn clickstack_source_dispatches_metric_variant() {
    let json = r#"{
        "id": "metric-1",
        "kind": "metric",
        "name": "metrics",
        "connection": "conn-1",
        "from": {"databaseName": "default", "tableName": "metrics"},
        "timestampValueExpression": "TimeUnix",
        "resourceAttributesExpression": "ResourceAttributes",
        "metricTables": {
            "gauge": "otel_metrics_gauge",
            "histogram": "otel_metrics_histogram",
            "sum": "otel_metrics_sum",
            "summary": "otel_metrics_summary",
            "exponential histogram": "otel_metrics_exponential_histogram"
        }
    }"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match source {
        ClickStackSource::ClickStackMetricSource(src) => {
            assert_eq!(src.kind, ClickStackMetricSourceKind::Metric);
            assert_eq!(src.name, "metrics");
            assert_eq!(src.resource_attributes_expression, "ResourceAttributes");
            assert_eq!(src.metric_tables.gauge, "otel_metrics_gauge");
        }
        other => panic!("expected metric source, got {other}"),
    }
}

#[test]
fn clickstack_source_dispatches_session_variant() {
    let json = r#"{
        "id": "session-1",
        "kind": "session",
        "name": "sessions",
        "connection": "conn-1",
        "from": {"databaseName": "default", "tableName": "sessions"},
        "traceSourceId": "trace-1"
    }"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match source {
        ClickStackSource::ClickStackSessionSource(src) => {
            assert_eq!(src.kind, ClickStackSessionSourceKind::Session);
            assert_eq!(src.name, "sessions");
            assert_eq!(src.trace_source_id, "trace-1");
        }
        other => panic!("expected session source, got {other}"),
    }
}

#[test]
fn clickstack_source_unknown_kind_round_trips() {
    let json = r#"{"kind":"future_kind","name":"x"}"#;
    let source: ClickStackSource = serde_json::from_str(json).unwrap();
    match &source {
        ClickStackSource::Unknown(_) => {}
        other => panic!("expected unknown source, got {other}"),
    }
    let original: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_value(&source).unwrap(), original);
}

#[test]
fn clickstack_source_serializes_as_inner_log_struct() {
    let log = ClickStackLogSource {
        kind: ClickStackLogSourceKind::Log,
        name: "logs".to_string(),
        connection: "conn-1".to_string(),
        default_table_select_expression: "*".to_string(),
        from: ClickStackSourceFrom {
            database_name: "default".to_string(),
            table_name: "logs".to_string(),
        },
        timestamp_value_expression: "ts".to_string(),
        ..Default::default()
    };
    let source = ClickStackSource::ClickStackLogSource(log.clone());
    assert_eq!(
        serde_json::to_value(&source).unwrap(),
        serde_json::to_value(&log).unwrap()
    );
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

#[test]
fn clickstack_alert_num_consecutive_windows_present_and_absent() {
    let json = r#"{
        "id": "alert-1",
        "name": "High CPU",
        "numConsecutiveWindows": 3
    }"#;
    let alert: ClickStackAlertResponse = serde_json::from_str(json).unwrap();
    assert_eq!(alert.num_consecutive_windows, Some(3));
    let v = serde_json::to_value(&alert).unwrap();
    assert_eq!(v["numConsecutiveWindows"], 3);

    // Absent -> None -> dropped by skip_serializing_if.
    let alert = ClickStackAlertResponse::default();
    assert_eq!(alert.num_consecutive_windows, None);
    let v = serde_json::to_value(&alert).unwrap();
    assert!(v.get("numConsecutiveWindows").is_none());
}

#[test]
fn clickstack_create_alert_num_consecutive_windows_round_trip() {
    let v = serde_json::to_value(ClickStackCreateAlertRequest {
        num_consecutive_windows: Some(5),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(v["numConsecutiveWindows"], 5);
    let v = serde_json::to_value(ClickStackCreateAlertRequest::default()).unwrap();
    assert!(v.get("numConsecutiveWindows").is_none());
}

#[test]
fn clickstack_update_alert_num_consecutive_windows_round_trip() {
    let v = serde_json::to_value(ClickStackUpdateAlertRequest {
        num_consecutive_windows: Some(7),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(v["numConsecutiveWindows"], 7);
    let v = serde_json::to_value(ClickStackUpdateAlertRequest::default()).unwrap();
    assert!(v.get("numConsecutiveWindows").is_none());
}

#[test]
fn clickstack_filter_applies_to_source_ids_round_trip() {
    let json = r#"{
        "expression": "level = 'error'",
        "id": "f-1",
        "name": "errors",
        "sourceId": "src-1",
        "type": "sql",
        "appliesToSourceIds": ["src-1", "src-2"]
    }"#;
    let filter: ClickStackFilter = serde_json::from_str(json).unwrap();
    assert_eq!(
        filter.applies_to_source_ids,
        Some(vec!["src-1".to_string(), "src-2".to_string()])
    );
    let v = serde_json::to_value(&filter).unwrap();
    assert_eq!(v["appliesToSourceIds"][0], "src-1");
    assert_eq!(v["appliesToSourceIds"][1], "src-2");

    // Absent -> None -> dropped.
    let filter = ClickStackFilter::default();
    assert_eq!(filter.applies_to_source_ids, None);
    let v = serde_json::to_value(&filter).unwrap();
    assert!(v.get("appliesToSourceIds").is_none());
}

#[test]
fn clickstack_filter_input_applies_to_source_ids_round_trip() {
    let json = r#"{
        "expression": "level = 'error'",
        "name": "errors",
        "sourceId": "src-1",
        "type": "sql",
        "appliesToSourceIds": ["src-9"]
    }"#;
    let filter: ClickStackFilterInput = serde_json::from_str(json).unwrap();
    assert_eq!(
        filter.applies_to_source_ids,
        Some(vec!["src-9".to_string()])
    );
    let v = serde_json::to_value(&filter).unwrap();
    assert_eq!(v["appliesToSourceIds"][0], "src-9");

    let v = serde_json::to_value(ClickStackFilterInput::default()).unwrap();
    assert!(v.get("appliesToSourceIds").is_none());
}

#[test]
fn clickpipe_postgres_table_mapping_partition_by_expr_round_trip() {
    let json = r#"{"partitionByExpr": "toYYYYMM(created_at)"}"#;
    let mapping: ClickPipePostgresPipeTableMapping = serde_json::from_str(json).unwrap();
    assert_eq!(mapping.partition_by_expr, "toYYYYMM(created_at)");
    let v = serde_json::to_value(&mapping).unwrap();
    assert_eq!(v["partitionByExpr"], "toYYYYMM(created_at)");

    // The field is required (non-nullable), so the default is the empty string.
    let mapping: ClickPipePostgresPipeTableMapping = serde_json::from_str("{}").unwrap();
    assert_eq!(mapping.partition_by_expr, "");
}

#[test]
fn clickpipe_patch_remove_table_mapping_partition_by_expr_round_trip() {
    let json = r#"{"partitionByExpr": "toDate(ts)"}"#;
    let mapping: ClickPipePatchPostgresPipeRemoveTableMapping = serde_json::from_str(json).unwrap();
    assert_eq!(mapping.partition_by_expr.as_deref(), Some("toDate(ts)"));
    let v = serde_json::to_value(&mapping).unwrap();
    assert_eq!(v["partitionByExpr"], "toDate(ts)");

    let v = serde_json::to_value(ClickPipePatchPostgresPipeRemoveTableMapping::default()).unwrap();
    assert!(v.get("partitionByExpr").is_none());
}

#[test]
fn activity_type_new_wire_values_deserialize_to_typed_variants() {
    let cases = [
        (
            "organization_member_update_roles",
            ActivityType::Organization_member_update_roles,
        ),
        (
            "organization_saml_connection_create",
            ActivityType::Organization_saml_connection_create,
        ),
        (
            "organization_saml_connection_update",
            ActivityType::Organization_saml_connection_update,
        ),
        (
            "service_update_snapshot_configuration",
            ActivityType::Service_update_snapshot_configuration,
        ),
    ];
    for (wire, expected) in cases {
        let parsed: ActivityType = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
        assert_eq!(parsed, expected, "{wire} should not be Unknown");
        assert_eq!(parsed.to_string(), wire);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }
}

#[test]
fn clickstack_alert_response_state_pending() {
    let parsed: ClickStackAlertResponseState = serde_json::from_str("\"PENDING\"").unwrap();
    assert_eq!(parsed, ClickStackAlertResponseState::PENDING);
    assert_eq!(parsed.to_string(), "PENDING");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), "PENDING");
}

#[test]
fn clickstack_number_tile_color_condition_numeric_variant() {
    // A `gt`/`gte`/`lt`/`lte` operator with a scalar value dispatches to the
    // numeric variant (the first arm of the untagged union).
    let json = r#"{"operator": "gt", "value": 100, "color": "chart-red"}"#;
    let cond: ClickStackNumberTileColorCondition = serde_json::from_str(json).unwrap();
    match cond {
        ClickStackNumberTileColorCondition::ClickStackNumericColorCondition(c) => {
            assert_eq!(c.operator, ClickStackNumericColorConditionOperator::Gt);
            assert_eq!(c.value, 100.0);
            assert_eq!(c.color, ClickStackChartColor::Chart_red);
        }
        other => panic!("expected numeric variant, got {other}"),
    }
}

#[test]
fn clickstack_number_tile_color_condition_between_variant() {
    // The `between` operator carries an inclusive [min, max] array value, so it
    // fails the numeric (scalar) variant and lands on the between variant.
    let json = r#"{"operator": "between", "value": [100, 500], "color": "chart-warning"}"#;
    let cond: ClickStackNumberTileColorCondition = serde_json::from_str(json).unwrap();
    match cond {
        ClickStackNumberTileColorCondition::ClickStackBetweenColorCondition(c) => {
            assert_eq!(c.operator, ClickStackBetweenColorConditionOperator::Between);
            assert_eq!(c.value, vec![100.0, 500.0]);
            assert_eq!(c.color, ClickStackChartColor::Chart_warning);
        }
        other => panic!("expected between variant, got {other}"),
    }
}

#[test]
fn clickstack_number_tile_color_condition_equality_string_value() {
    // A string-valued `eq` fails the numeric and between variants (their values
    // are number/array) and lands on the equality variant.
    let json =
        r#"{"operator": "eq", "value": "healthy", "color": "chart-success", "label": "Healthy"}"#;
    let cond: ClickStackNumberTileColorCondition = serde_json::from_str(json).unwrap();
    match cond {
        ClickStackNumberTileColorCondition::ClickStackEqualityColorCondition(c) => {
            assert_eq!(c.operator, ClickStackEqualityColorConditionOperator::Eq);
            assert_eq!(c.value, serde_json::json!("healthy"));
            assert_eq!(c.color, ClickStackChartColor::Chart_success);
            assert_eq!(c.label.as_deref(), Some("Healthy"));
        }
        other => panic!("expected equality variant, got {other}"),
    }
}

#[test]
fn clickstack_number_tile_color_condition_equality_numeric_value() {
    // A numeric-valued `eq` is structurally identical to a numeric condition; it
    // is the strict operator enum (no `eq` in the numeric operator set) that
    // routes it to the equality variant rather than the numeric one.
    let json = r#"{"operator": "eq", "value": 42, "color": "chart-error"}"#;
    let cond: ClickStackNumberTileColorCondition = serde_json::from_str(json).unwrap();
    match cond {
        ClickStackNumberTileColorCondition::ClickStackEqualityColorCondition(c) => {
            assert_eq!(c.operator, ClickStackEqualityColorConditionOperator::Eq);
            assert_eq!(c.value.as_f64(), Some(42.0));
            assert_eq!(c.color, ClickStackChartColor::Chart_error);
        }
        other => panic!("expected equality variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_categorical_bar_builder_variant() {
    // `displayType: "bar"` must dispatch to the categorical bar variant, not the
    // stacked bar variant (whose discriminator is "stacked_bar").
    let json = r#"{
        "displayType": "bar",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackCategoricalBarChartConfig(
            ClickStackCategoricalBarChartConfig::ClickStackCategoricalBarBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackCategoricalBarBuilderChartConfigDisplaytype::Bar
            );
            assert_eq!(b.select.len(), 1);
        }
        other => panic!("expected categorical bar builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_categorical_bar_raw_sql_variant() {
    // The Raw SQL categorical bar (configType "sql") also dispatches to the
    // categorical bar variant rather than the stacked bar Raw SQL variant.
    let json = r#"{
        "displayType": "bar",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT count() FROM t GROUP BY service"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackCategoricalBarChartConfig(
            ClickStackCategoricalBarChartConfig::ClickStackCategoricalBarRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(r.sql_template, "SELECT count() FROM t GROUP BY service");
            assert_eq!(
                r.display_type,
                ClickStackCategoricalBarRawSqlChartConfigDisplaytype::Bar
            );
        }
        other => panic!("expected categorical bar raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_stacked_bar_not_categorical_bar() {
    // The categorical bar variant is ordered first but its `displayType` is
    // strictly "bar", so a "stacked_bar" payload is rejected by it and falls
    // through to the other builder variants unchanged (i.e. dispatch of the
    // stacked bar payload is not captured by the new variant).
    let json = r#"{
        "displayType": "stacked_bar",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    assert!(
        !matches!(
            cfg,
            ClickStackTileConfig::ClickStackCategoricalBarChartConfig(_)
        ),
        "stacked_bar must not dispatch to the categorical bar variant, got {cfg}"
    );
}

#[test]
fn clickstack_tile_config_event_patterns_variant() {
    // `displayType: "event_patterns"` dispatches to the event-patterns variant;
    // it requires only sourceId (no select), so it is ordered before markdown.
    let json = r#"{
        "displayType": "event_patterns",
        "sourceId": "src-9",
        "where": "level:error",
        "whereLanguage": "lucene"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackEventPatternsChartConfig(e) => {
            assert_eq!(e.source_id, "src-9");
            assert_eq!(
                e.display_type,
                ClickStackEventPatternsChartConfigDisplaytype::Event_patterns
            );
            assert_eq!(e.r#where.as_deref(), Some("level:error"));
            assert_eq!(
                e.where_language,
                Some(ClickStackEventPatternsChartConfigWherelanguage::Lucene)
            );
        }
        other => panic!("expected event patterns variant, got {other}"),
    }
}

#[test]
fn clickstack_chart_color_known_and_unknown_round_trip() {
    // A known palette token maps to its typed variant and back.
    let known: ClickStackChartColor = serde_json::from_str("\"chart-light-blue\"").unwrap();
    assert_eq!(known, ClickStackChartColor::Chart_light_blue);
    assert_eq!(known.to_string(), "chart-light-blue");
    assert_eq!(serde_json::to_value(&known).unwrap(), "chart-light-blue");

    // An unrecognized token round-trips through the Unknown(String) catch-all.
    let unknown: ClickStackChartColor = serde_json::from_str("\"chart-teal\"").unwrap();
    assert_eq!(
        unknown,
        ClickStackChartColor::Unknown("chart-teal".to_string())
    );
    assert_eq!(unknown.to_string(), "chart-teal");
    assert_eq!(serde_json::to_value(&unknown).unwrap(), "chart-teal");
}

#[test]
fn clickstack_background_chart_round_trip() {
    let json = r#"{"type": "area", "color": "chart-blue"}"#;
    let bg: ClickStackBackgroundChart = serde_json::from_str(json).unwrap();
    assert_eq!(bg.r#type, ClickStackBackgroundChartType::Area);
    assert_eq!(bg.color, Some(ClickStackChartColor::Chart_blue));
    let v = serde_json::to_value(&bg).unwrap();
    assert_eq!(v["type"], "area");
    assert_eq!(v["color"], "chart-blue");

    // color is optional and dropped when absent.
    let json = r#"{"type": "line"}"#;
    let bg: ClickStackBackgroundChart = serde_json::from_str(json).unwrap();
    assert_eq!(bg.r#type, ClickStackBackgroundChartType::Line);
    assert_eq!(bg.color, None);
    let v = serde_json::to_value(&bg).unwrap();
    assert!(v.get("color").is_none());
}

#[test]
fn clickstack_number_builder_chart_config_color_fields_round_trip() {
    let json = r#"{
        "displayType": "number",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}],
        "color": "chart-blue",
        "colorRules": [
            {"operator": "gt", "value": 100, "color": "chart-red"}
        ],
        "backgroundChart": {"type": "line", "color": "chart-green"}
    }"#;
    let cfg: ClickStackNumberBuilderChartConfig = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.color, Some(ClickStackChartColor::Chart_blue));
    let rules = cfg.color_rules.as_ref().unwrap();
    assert_eq!(rules.len(), 1);
    match &rules[0] {
        ClickStackNumberTileColorCondition::ClickStackNumericColorCondition(c) => {
            assert_eq!(c.operator, ClickStackNumericColorConditionOperator::Gt);
        }
        other => panic!("expected numeric rule, got {other}"),
    }
    let bg = cfg.background_chart.as_ref().unwrap();
    assert_eq!(bg.r#type, ClickStackBackgroundChartType::Line);

    let v = serde_json::to_value(&cfg).unwrap();
    assert_eq!(v["color"], "chart-blue");
    assert_eq!(v["colorRules"][0]["operator"], "gt");
    assert_eq!(v["backgroundChart"]["type"], "line");

    // The three new fields are optional and dropped when absent.
    let v = serde_json::to_value(ClickStackNumberBuilderChartConfig::default()).unwrap();
    assert!(v.get("color").is_none());
    assert!(v.get("colorRules").is_none());
    assert!(v.get("backgroundChart").is_none());
}

#[test]
fn clickstack_number_raw_sql_chart_config_color_round_trip() {
    let json = r#"{
        "configType": "sql",
        "connectionId": "conn-1",
        "displayType": "number",
        "sqlTemplate": "SELECT count() FROM t",
        "color": "chart-purple"
    }"#;
    let cfg: ClickStackNumberRawSqlChartConfig = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.color, Some(ClickStackChartColor::Chart_purple));
    let v = serde_json::to_value(&cfg).unwrap();
    assert_eq!(v["color"], "chart-purple");

    let v = serde_json::to_value(ClickStackNumberRawSqlChartConfig::default()).unwrap();
    assert!(v.get("color").is_none());
}

#[test]
fn clickstack_on_click_external_round_trip() {
    let json = r#"{"type": "external", "urlTemplate": "https://example.com/{{ServiceName}}"}"#;
    let ext: ClickStackOnClickExternal = serde_json::from_str(json).unwrap();
    assert_eq!(ext.r#type, ClickStackOnClickExternalType::External);
    assert_eq!(ext.url_template, "https://example.com/{{ServiceName}}");
    let v = serde_json::to_value(&ext).unwrap();
    assert_eq!(v["type"], "external");
    assert_eq!(v["urlTemplate"], "https://example.com/{{ServiceName}}");
}

#[test]
fn deserialize_clickstack_on_click_dispatches_external() {
    // An "external" on-click payload has no `target`, so it cannot match the
    // Search or Dashboard variants (both require `target`) and dispatches
    // through the untagged union to the External variant.
    let json = r#"{"type": "external", "urlTemplate": "https://example.com/{{value}}"}"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    match &on_click {
        ClickStackOnClick::ClickStackOnClickExternal(ext) => {
            assert_eq!(ext.r#type, ClickStackOnClickExternalType::External);
            assert_eq!(ext.url_template, "https://example.com/{{value}}");
        }
        other => panic!("expected external variant, got {other}"),
    }
    // Round-trips back to the same wire shape through the union.
    let v = serde_json::to_value(&on_click).unwrap();
    assert_eq!(v["type"], "external");
    assert_eq!(v["urlTemplate"], "https://example.com/{{value}}");
}

#[test]
fn deserialize_clickstack_on_click_dispatches_search() {
    // Regression: adding the External variant must not steal the Search shape.
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
fn deserialize_clickstack_on_click_dashboard_still_parses() {
    // Regression: adding the External variant must not disturb dashboard
    // parsing. As documented on deserialize_clickstack_on_click_dashboard_struct,
    // the untagged parent's first (Search) variant catches the dashboard shape
    // because ClickStackOnClickSearchType has an Unknown(String) catch-all, so
    // we deserialize directly into ClickStackOnClickDashboard. The External
    // variant is inserted after Search/Dashboard, so it changes neither.
    let json = r#"{
        "type": "dashboard",
        "target": {"mode": "template", "template": "{{x}}"}
    }"#;
    let dash: ClickStackOnClickDashboard = serde_json::from_str(json).unwrap();
    assert_eq!(dash.r#type, ClickStackOnClickDashboardType::Dashboard);
}

#[test]
fn deserialize_clickstack_connection_with_null_prefix() {
    let json = r#"{
        "id": "507f1f77bcf86cd799439012",
        "name": "Production ClickHouse",
        "host": "https://clickhouse.example.com:8443",
        "username": "default",
        "hyperdxSettingPrefix": null,
        "isPrometheusEndpoint": false,
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let conn: ClickStackConnection = serde_json::from_str(json).unwrap();
    assert_eq!(conn.id, "507f1f77bcf86cd799439012");
    assert_eq!(conn.name, "Production ClickHouse");
    assert_eq!(conn.host, "https://clickhouse.example.com:8443");
    assert_eq!(conn.username, "default");
    assert_eq!(conn.hyperdx_setting_prefix, None);
    assert_eq!(conn.is_prometheus_endpoint, Some(false));
    assert!(conn.created_at.is_some());
    assert!(conn.updated_at.is_some());
}

#[test]
fn serialize_clickstack_create_connection_request_omits_none() {
    let req = ClickStackCreateConnectionRequest {
        name: "Production ClickHouse".to_string(),
        host: "https://clickhouse.example.com:8443".to_string(),
        username: "default".to_string(),
        password: Some("my-secret-password".to_string()),
        ..Default::default()
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["name"], "Production ClickHouse");
    assert_eq!(v["host"], "https://clickhouse.example.com:8443");
    assert_eq!(v["username"], "default");
    assert_eq!(v["password"], "my-secret-password");
    assert!(
        v.get("hyperdxSettingPrefix").is_none(),
        "hyperdxSettingPrefix must be omitted when None"
    );
    assert!(
        v.get("isPrometheusEndpoint").is_none(),
        "isPrometheusEndpoint must be omitted when None"
    );
}

#[test]
fn deserialize_clickstack_role_with_nested_conditions() {
    let json = r#"{
        "id": "role-1",
        "name": "Deploy Bot",
        "description": "Manages dashboards via Terraform",
        "isPredefined": false,
        "permissions": [
            {
                "action": "read",
                "subject": "dashboard",
                "inverted": false,
                "integration": "mongodb",
                "conditions": {
                    "teamId": "team-1",
                    "tags": ["prod", "eu"]
                }
            }
        ]
    }"#;
    let role: ClickStackRole = serde_json::from_str(json).unwrap();
    assert_eq!(role.id, "role-1");
    assert_eq!(role.name, "Deploy Bot");
    assert!(!role.is_predefined);
    assert_eq!(role.permissions.len(), 1);

    let perm = &role.permissions[0];
    assert_eq!(perm.action, "read");
    assert_eq!(perm.subject, "dashboard");
    assert_eq!(perm.inverted, Some(false));
    assert_eq!(perm.integration, Some("mongodb".to_string()));

    // Free-form conditions land as serde_json::Value with the nested content intact.
    let conditions = perm.conditions.as_ref().unwrap();
    assert_eq!(conditions["teamId"], "team-1");
    assert_eq!(conditions["tags"][0], "prod");
    assert_eq!(conditions["tags"][1], "eu");
}

#[test]
fn clickstack_role_round_trip() {
    let role = ClickStackRole {
        id: "role-1".to_string(),
        name: "Deploy Bot".to_string(),
        is_predefined: false,
        permissions: vec![ClickStackCASLPermission {
            action: "manage".to_string(),
            subject: "all".to_string(),
            conditions: Some(serde_json::json!({ "teamId": "team-1" })),
            ..Default::default()
        }],
        ..Default::default()
    };
    let v = serde_json::to_value(&role).unwrap();
    assert_eq!(v["id"], "role-1");
    assert_eq!(v["isPredefined"], false);
    assert_eq!(v["permissions"][0]["action"], "manage");
    assert_eq!(v["permissions"][0]["subject"], "all");
    assert_eq!(v["permissions"][0]["conditions"]["teamId"], "team-1");

    // Optional fields dropped when None.
    assert!(v.get("description").is_none());
    assert!(v.get("createdAt").is_none());
    assert!(v.get("updatedAt").is_none());

    let back: ClickStackRole = serde_json::from_value(v).unwrap();
    assert_eq!(back, role);
}

#[test]
fn serialize_clickstack_casl_permission_omits_none() {
    let perm = ClickStackCASLPermission {
        action: "read".to_string(),
        subject: "dashboard".to_string(),
        ..Default::default()
    };
    let v = serde_json::to_value(&perm).unwrap();
    assert_eq!(v["action"], "read");
    assert_eq!(v["subject"], "dashboard");
    assert!(v.get("inverted").is_none());
    assert!(v.get("integration").is_none());
    assert!(v.get("conditions").is_none());
}

#[test]
fn serialize_clickstack_create_role_request_omits_none() {
    let req = ClickStackCreateRoleRequest {
        name: "Deploy Bot".to_string(),
        permissions: vec![ClickStackCASLPermission {
            action: "read".to_string(),
            subject: "dashboard".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["name"], "Deploy Bot");
    assert_eq!(v["permissions"][0]["action"], "read");
    assert!(v.get("description").is_none());
}

#[test]
fn serialize_clickstack_update_role_request_omits_none() {
    let req = ClickStackUpdateRoleRequest {
        permissions: vec![ClickStackCASLPermission {
            action: "manage".to_string(),
            subject: "all".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["permissions"][0]["action"], "manage");
    assert!(v.get("name").is_none());
    assert!(v.get("description").is_none());
}

#[test]
fn deserialize_clickstack_saved_search_full_round_trip() {
    let json = r#"{
        "id": "507f1f77bcf86cd799439011",
        "name": "Production Errors",
        "sourceId": "507f1f77bcf86cd799439012",
        "select": "Timestamp, ServiceName, Body",
        "where": "SeverityText:ERROR",
        "whereLanguage": "lucene",
        "orderBy": "Timestamp DESC",
        "tags": ["production", "errors"],
        "filters": [
            {"type": "sql", "condition": "ServiceName IN ('checkout', 'payments')"}
        ],
        "teamId": "507f1f77bcf86cd799439013",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let search: ClickStackSavedSearch = serde_json::from_str(json).unwrap();
    assert_eq!(search.id, "507f1f77bcf86cd799439011");
    assert_eq!(search.name, "Production Errors");
    assert_eq!(search.source_id, "507f1f77bcf86cd799439012");
    assert_eq!(
        search.where_language,
        Some(ClickStackSavedSearchWherelanguage::Lucene)
    );
    let filters = search.filters.clone().unwrap();
    assert_eq!(filters.len(), 1);
    assert_eq!(
        filters[0].r#type,
        Some(ClickStackSavedSearchFilterType::Sql)
    );
    assert_eq!(
        filters[0].condition,
        "ServiceName IN ('checkout', 'payments')"
    );
    assert_eq!(search.team_id, Some("507f1f77bcf86cd799439013".to_string()));

    let v = serde_json::to_value(&search).unwrap();
    assert_eq!(v["whereLanguage"], "lucene");
    assert_eq!(v["filters"][0]["type"], "sql");
    assert_eq!(v["orderBy"], "Timestamp DESC");
    assert_eq!(v["tags"][0], "production");

    let round: ClickStackSavedSearch = serde_json::from_value(v).unwrap();
    assert_eq!(round, search);
}

#[test]
fn serialize_clickstack_saved_search_input_minimal_omits_optionals() {
    let input = ClickStackSavedSearchInput {
        name: "Production Errors".to_string(),
        source_id: "507f1f77bcf86cd799439012".to_string(),
        ..Default::default()
    };
    let v = serde_json::to_value(&input).unwrap();
    assert_eq!(v["name"], "Production Errors");
    assert_eq!(v["sourceId"], "507f1f77bcf86cd799439012");
    assert!(v.get("select").is_none());
    assert!(v.get("where").is_none());
    assert!(v.get("whereLanguage").is_none());
    assert!(v.get("orderBy").is_none());
    assert!(v.get("tags").is_none());
    assert!(v.get("filters").is_none());
}

#[test]
fn clickstack_webhook_input_headers_and_query_params_round_trip() {
    let json = r#"{
        "name": "Production Alerts",
        "service": "incidentio",
        "url": "https://api.incident.io/v2/alert_events/http/xyz",
        "description": "Sends critical alerts",
        "body": "{\"alert\": \"{{title}}\"}",
        "headers": {"Authorization": "Bearer token", "X-Custom": "value"},
        "queryParams": {"source": "clickstack", "env": "prod"}
    }"#;
    let input: ClickStackWebhookInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.name, "Production Alerts");
    assert_eq!(input.service, ClickStackWebhookInputService::Incidentio);
    assert_eq!(
        input.url,
        "https://api.incident.io/v2/alert_events/http/xyz"
    );

    let headers = input.headers.clone().unwrap();
    assert_eq!(
        headers.get("Authorization"),
        Some(&"Bearer token".to_string())
    );
    assert_eq!(headers.get("X-Custom"), Some(&"value".to_string()));
    let query_params = input.query_params.clone().unwrap();
    assert_eq!(query_params.get("source"), Some(&"clickstack".to_string()));
    assert_eq!(query_params.get("env"), Some(&"prod".to_string()));

    // Maps serialize back as JSON objects, and the enum keeps its wire value.
    let v = serde_json::to_value(&input).unwrap();
    assert_eq!(v["service"], "incidentio");
    assert!(v["headers"].is_object());
    assert_eq!(v["headers"]["Authorization"], "Bearer token");
    assert!(v["queryParams"].is_object());
    assert_eq!(v["queryParams"]["source"], "clickstack");

    let back: ClickStackWebhookInput = serde_json::from_value(v).unwrap();
    assert_eq!(back, input);
}

#[test]
fn serialize_clickstack_webhook_input_minimal_omits_optionals() {
    let input = ClickStackWebhookInput {
        name: "Slack Alerts".to_string(),
        service: ClickStackWebhookInputService::Slack,
        url: "https://hooks.slack.com/services/T/B/X".to_string(),
        ..Default::default()
    };
    let v = serde_json::to_value(&input).unwrap();
    assert_eq!(v["name"], "Slack Alerts");
    assert_eq!(v["service"], "slack");
    assert_eq!(v["url"], "https://hooks.slack.com/services/T/B/X");
    assert!(v.get("description").is_none());
    assert!(v.get("body").is_none());
    assert!(v.get("headers").is_none());
    assert!(v.get("queryParams").is_none());
}

#[test]
fn clickstack_validate_dashboard_response_round_trip() {
    let json = r#"{
        "valid": false,
        "errors": [
            {"path": "tiles.0.config", "message": "Required"},
            {"path": "", "message": "Top-level error"}
        ],
        "normalized": {"name": "My Dashboard", "tiles": [{"id": "t1"}]}
    }"#;
    let resp: ClickStackValidateDashboardResponse = serde_json::from_str(json).unwrap();
    assert!(!resp.valid);
    assert_eq!(resp.errors.len(), 2);
    assert_eq!(resp.errors[0].path, "tiles.0.config");
    assert_eq!(resp.errors[0].message, "Required");
    assert_eq!(resp.errors[1].path, "");

    // `normalized` is a free-form Value; its arbitrary payload is preserved.
    let normalized = resp.normalized.as_ref().unwrap();
    assert_eq!(normalized["name"], "My Dashboard");
    assert_eq!(normalized["tiles"][0]["id"], "t1");

    let v = serde_json::to_value(&resp).unwrap();
    assert_eq!(v["valid"], false);
    assert_eq!(v["errors"][0]["path"], "tiles.0.config");
    assert_eq!(v["normalized"]["name"], "My Dashboard");

    let back: ClickStackValidateDashboardResponse = serde_json::from_value(v).unwrap();
    assert_eq!(back, resp);
}

#[test]
fn clickstack_validate_dashboard_response_valid_null_normalized() {
    let json = r#"{"valid": true, "errors": [], "normalized": null}"#;
    let resp: ClickStackValidateDashboardResponse = serde_json::from_str(json).unwrap();
    assert!(resp.valid);
    assert!(resp.errors.is_empty());
    assert_eq!(resp.normalized, None);

    // A required-but-nullable field still serializes (as null), not omitted.
    let v = serde_json::to_value(&resp).unwrap();
    assert!(v.get("normalized").is_some());
    assert!(v["normalized"].is_null());
}

#[test]
fn organization_quota_typed_enums_round_trip() {
    let json = r#"{
        "quotaCode": "replicas-per-warehouse",
        "name": "Replicas per warehouse",
        "description": "Limits each warehouse individually.",
        "scope": "warehouse",
        "value": 20,
        "usage": 3,
        "adjustable": true
    }"#;
    let quota: OrganizationQuota = serde_json::from_str(json).unwrap();
    assert_eq!(
        quota.quota_code,
        OrganizationQuotaQuotacode::Replicas_per_warehouse
    );
    assert_eq!(quota.scope, OrganizationQuotaScope::Warehouse);
    assert_eq!(quota.value, 20);
    assert_eq!(quota.usage, Some(3));
    assert!(quota.adjustable);

    let v = serde_json::to_value(&quota).unwrap();
    assert_eq!(v["quotaCode"], "replicas-per-warehouse");
    assert_eq!(v["scope"], "warehouse");
    assert_eq!(v["value"], 20);
    assert_eq!(v["usage"], 3);

    let back: OrganizationQuota = serde_json::from_value(v).unwrap();
    assert_eq!(back, quota);
}

#[test]
fn organization_quota_usage_optional_omitted() {
    let json = r#"{
        "quotaCode": "services-per-organization",
        "name": "Services per organization",
        "description": "Limits services.",
        "scope": "organization",
        "value": 20,
        "adjustable": false
    }"#;
    let quota: OrganizationQuota = serde_json::from_str(json).unwrap();
    assert_eq!(
        quota.quota_code,
        OrganizationQuotaQuotacode::Services_per_organization
    );
    assert_eq!(quota.scope, OrganizationQuotaScope::Organization);
    assert_eq!(quota.usage, None);

    let v = serde_json::to_value(&quota).unwrap();
    assert!(v.get("usage").is_none(), "usage must be omitted when None");
}

#[test]
fn organization_quota_quota_code_unknown_catch_all() {
    let parsed: OrganizationQuotaQuotacode =
        serde_json::from_str("\"queries-per-second\"").unwrap();
    assert_eq!(
        parsed,
        OrganizationQuotaQuotacode::Unknown("queries-per-second".to_string())
    );
    assert_eq!(parsed.to_string(), "queries-per-second");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), "queries-per-second");
}
