use clickhouse_cloud_api::models::*;

/// Shared assertion for the discriminated-union `Unknown` catch-all: an
/// unrecognized payload must deserialize into the union's lossless `Unknown`
/// variant (confirmed by `is_unknown`) and re-serialize to the byte-identical
/// JSON object it came from.
fn assert_unknown_variant_round_trips<T>(json: &str, is_unknown: impl FnOnce(&T) -> bool)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let original: serde_json::Value = serde_json::from_str(json).unwrap();
    let parsed: T = serde_json::from_str(json).unwrap();
    assert!(
        is_unknown(&parsed),
        "payload did not deserialize to the Unknown variant"
    );
    let reserialized = serde_json::to_value(&parsed).unwrap();
    assert_eq!(reserialized, original);
    assert!(reserialized.is_object());
}

/// Every `discriminated_union!` enum with a `Default` is reachable from a
/// response that drops a field carrying that union, so its default must be a
/// fixed point of its own `Deserialize`: it has to come back as the same
/// variant. It is not automatic — variants of one union can share the same
/// inline `enum` values for the discriminating field (both alert channels
/// declare `["webhook", "email"]`), so a variant defaulting its discriminator to
/// another variant's value would silently retype the value on the next
/// deserialize. Add new unions with a `Default` impl here.
#[test]
fn discriminated_union_defaults_round_trip_to_the_same_variant() {
    macro_rules! assert_default_round_trips {
        ($($union:ty),+ $(,)?) => {
            $({
                let default = <$union>::default();
                let json = serde_json::to_string(&default).unwrap();
                let parsed: $union = serde_json::from_str(&json).unwrap();
                assert_eq!(
                    parsed,
                    default,
                    "{} default deserialized as another variant from {json}",
                    stringify!($union),
                );
            })+
        };
    }

    assert_default_round_trips!(
        BackupBucket,
        BackupBucketPatchRequest,
        BackupBucketPostRequest,
        BackupBucketProperties,
        ClickStackAlertChannel,
        ClickStackBarChartConfig,
        ClickStackCategoricalBarChartConfig,
        ClickStackDashboardChartSeries,
        ClickStackLineChartConfig,
        ClickStackNumberChartConfig,
        ClickStackOnClick,
        ClickStackOnClickTarget,
        ClickStackPieChartConfig,
        ClickStackSource,
        ClickStackTableChartConfig,
        ClickStackTileConfig,
        ClickStackWebhook,
    );
}

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
    assert_eq!(org.name.as_deref(), Some("My Organization"));
    assert_eq!(
        org.id,
        Some(
            "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
                .parse::<uuid::Uuid>()
                .unwrap()
        )
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
    // Absent response fields are omitted, not emitted as `null`.
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
    assert_eq!(result[0].name.as_deref(), Some("Org 1"));
    assert_eq!(result[1].name.as_deref(), Some("Org 2"));
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
    assert_eq!(svc.name.as_deref(), Some("my-service"));
    assert_eq!(svc.provider, Some(ServiceProvider::Aws));
    assert_eq!(svc.region, Some(ServiceRegion::Us_east_1));
    assert_eq!(svc.state, Some(ServiceState::Running));
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(svc.tier, Some(ServiceTier::Production));
    assert_eq!(svc.num_replicas, Some(3.0));
    assert_eq!(svc.idle_scaling, Some(true));
    assert_eq!(svc.is_primary, Some(true));
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
    assert_eq!(key.name.as_deref(), Some("My API Key"));
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
    assert_eq!(pipe.name.as_deref(), Some("my-pipe"));
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
    assert_eq!(member.name.as_deref(), Some("John Doe"));
    assert_eq!(member.email.as_deref(), Some("john@example.com"));
    #[cfg(feature = "deprecated-fields")]
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
    assert_eq!(inv.email.as_deref(), Some("new@example.com"));
    #[cfg(feature = "deprecated-fields")]
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
    assert_eq!(config.backup_start_time.as_deref(), Some("02:00"));
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
        config.endpoint_service_id.as_deref(),
        Some("vpce-svc-123456")
    );
}

#[test]
fn absent_response_fields_are_omitted_when_serialized() {
    let org = Organization {
        name: Some("Test".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&org).unwrap();
    // Absent means absent: a field the API did not return is omitted from
    // `--json` output rather than emitted as `null`.
    assert_eq!(json, serde_json::json!({ "name": "Test" }));
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
    assert_eq!(ep.host.as_deref(), Some("abc123.clickhouse.cloud"));
    assert_eq!(ep.port, Some(9440.0));
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
    assert_eq!(dash.name.as_deref(), Some("My Dashboard"));
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
    assert_eq!(activity.actor_type, Some(ActivityActortype::Api));
}

#[test]
fn default_response_struct_has_no_values() {
    // A response model's `Default` means "nothing was returned", not a set of
    // fabricated zero values.
    let svc = Service::default();
    assert_eq!(svc.id, None);
    assert_eq!(svc.name, None);
    assert_eq!(svc.provider, None);
    assert_eq!(svc.state, None);
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
    assert_eq!(pg.name.as_deref(), Some("my-postgres"));
}

#[test]
fn unknown_enum_variant_deserializes() {
    // An unknown service state from the API should deserialize into Unknown(String)
    let json = r#"{"state": "brand-new-state"}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(
        svc.state,
        Some(ServiceState::Unknown("brand-new-state".to_string()))
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
    assert_eq!(org.name.as_deref(), Some("Test"));
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
    assert_eq!(entry.autoscaling_mode, Some(AutoscalingMode::Vertical));
    assert_eq!(entry.min_replica_memory_gb, Some(16.0));
    assert_eq!(entry.max_replica_memory_gb, Some(32.0));

    // Writing a fetched entry back goes through the explicit conversion, which
    // resolves the fields the request requires.
    let req = ScalingScheduleEntryRequest::try_from(entry).unwrap();
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
    assert_eq!(org.name.as_deref(), Some("Test"));
}

#[test]
fn service_ignores_extra_fields() {
    let json = r#"{"name":"svc","state":"running","futureField":"v2","nested":{"a":1}}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.name.as_deref(), Some("svc"));
    assert_eq!(svc.state, Some(ServiceState::Running));
}

#[test]
fn clickpipe_ignores_extra_fields() {
    let json = r#"{"name":"pipe","state":"Running","newFeatureFlag":true}"#;
    let pipe: ClickPipe = serde_json::from_str(json).unwrap();
    assert_eq!(pipe.name.as_deref(), Some("pipe"));
    assert_eq!(pipe.state, Some(ClickPipeState::Running));
}

#[test]
fn backup_ignores_extra_fields() {
    let json = r#"{"status":"done","type":"full","compressionRatio":0.85}"#;
    let backup: Backup = serde_json::from_str(json).unwrap();
    assert_eq!(backup.status, Some(BackupStatus::Done));
}

#[test]
fn api_key_ignores_extra_fields() {
    let json = r#"{"name":"key","state":"enabled","rotationPolicy":"weekly"}"#;
    let key: ApiKey = serde_json::from_str(json).unwrap();
    assert_eq!(key.name.as_deref(), Some("key"));
    assert_eq!(key.state, Some(ApiKeyState::Enabled));
}

#[test]
fn member_ignores_extra_fields() {
    let json = r#"{"name":"Alice","role":"admin","department":"eng","mfa":true}"#;
    let m: Member = serde_json::from_str(json).unwrap();
    assert_eq!(m.name.as_deref(), Some("Alice"));
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(m.role, Some(MemberRole::Admin));
}

#[test]
fn invitation_ignores_extra_fields() {
    let json = r#"{"email":"a@b.com","role":"developer","expiresIn":"7d"}"#;
    let inv: Invitation = serde_json::from_str(json).unwrap();
    assert_eq!(inv.email.as_deref(), Some("a@b.com"));
}

#[test]
fn postgres_service_ignores_extra_fields() {
    let json = r#"{"name":"pg","state":"running","maintenanceWindow":"sun-02:00"}"#;
    let pg: PostgresService = serde_json::from_str(json).unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg"));
}

#[test]
fn activity_ignores_extra_fields() {
    let json = r#"{"actorType":"user","sourceIp":"1.2.3.4"}"#;
    let a: Activity = serde_json::from_str(json).unwrap();
    assert_eq!(a.actor_type, Some(ActivityActortype::User));
}

#[test]
fn backup_configuration_ignores_extra_fields() {
    let json = r#"{"backupPeriodInHours":24,"backupRetentionPeriodInHours":168,"compressionEnabled":true}"#;
    let c: BackupConfiguration = serde_json::from_str(json).unwrap();
    assert_eq!(c.backup_period_in_hours, Some(24.0));
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
        Some(
            "11111111-2222-3333-4444-555555555555"
                .parse::<uuid::Uuid>()
                .unwrap()
        )
    );
    // Every response field is `Option<T>`: an omitted key is `None`, not a
    // fabricated zero value.
    assert_eq!(svc.name, None);
    assert_eq!(svc.provider, None);
    assert_eq!(svc.state, None);
    assert_eq!(svc.endpoints, None);
}

#[cfg(feature = "deprecated-fields")]
#[test]
fn service_deserializes_deprecated_fields() {
    // With the `deprecated-fields` feature on, deprecated fields exist on the
    // struct and deserialize normally. Without the feature they are absent from
    // the struct entirely (see `deprecated_fields_absent_by_default`).
    let json = r#"{"tier":"production","minTotalMemoryGb":24,"maxTotalMemoryGb":48}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.min_total_memory_gb, Some(24.0));
    assert_eq!(svc.max_total_memory_gb, Some(48.0));
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
    assert_eq!(svc, Service::default());
    assert_eq!(svc.id, None);
    assert_eq!(svc.name, None);
}

#[test]
fn organization_minimal_response() {
    let org: Organization = serde_json::from_str(r#"{"name":"X"}"#).unwrap();
    assert_eq!(org.name.as_deref(), Some("X"));
    assert_eq!(org.id, None);
    assert_eq!(org.created_at, None);
}

#[test]
fn clickpipe_minimal_response() {
    let pipe: ClickPipe = serde_json::from_str("{}").unwrap();
    assert_eq!(pipe, ClickPipe::default());
    assert_eq!(pipe.id, None);
    assert_eq!(pipe.name, None);
    assert_eq!(pipe.state, None);
}

#[test]
fn postgres_service_minimal_response() {
    let pg: PostgresService =
        serde_json::from_str(r#"{"id":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"}"#).unwrap();
    // Every response field is `Option<T>`: an omitted key is `None`, not a
    // fabricated zero value.
    assert_eq!(pg.name, None);
    assert_eq!(pg.state, None);
}

#[test]
fn postgres_service_response_tolerates_dropped_and_null_fields() {
    // A response field the API stops sending, and one it sends as an explicit
    // `null`, must both land as `None` rather than failing the response. `null`
    // is the case `#[serde(default)]` never covered: it only fills a missing
    // key.
    let dropped: PostgresService = serde_json::from_str("{}").unwrap();
    let nulled: PostgresService = serde_json::from_str(
        r#"{"id":null,"name":null,"state":null,"tags":null,"storageSize":null,"createdAt":null}"#,
    )
    .unwrap();
    assert_eq!(dropped, PostgresService::default());
    assert_eq!(nulled, PostgresService::default());
    assert_eq!(nulled.tags, None);
    assert_eq!(nulled.storage_size, None);
}

#[test]
fn postgres_service_response_omits_absent_fields_when_serialized() {
    // Absent means absent: a response field that was not returned is omitted
    // from `--json` output rather than emitted as `null`.
    let pg = PostgresService {
        name: Some("pg-1".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&pg).unwrap();
    assert_eq!(json, serde_json::json!({ "name": "pg-1" }));
}

#[test]
fn postgres_instance_config_response_converts_back_into_a_request_body() {
    let response = PostgresInstanceConfigResponse {
        pg_config: Some(PgConfigResponse {
            max_connections: Some(serde_json::json!(200)),
            ..Default::default()
        }),
        pg_bouncer_config: Some(PgBouncerConfigResponse {}),
    };
    let request = PostgresInstanceConfig::try_from(response).unwrap();
    assert_eq!(
        request.pg_config.max_connections,
        Some(serde_json::json!(200))
    );
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::json!({ "pgConfig": { "max_connections": 200 }, "pgBouncerConfig": {} })
    );
}

#[test]
fn postgres_instance_config_response_conversion_reports_missing_required_fields() {
    // Both nested objects are required in a write body, so the write-back
    // conversion must fail loudly instead of inventing empty objects.
    let missing_bouncer = PostgresInstanceConfig::try_from(PostgresInstanceConfigResponse {
        pg_config: Some(PgConfigResponse::default()),
        pg_bouncer_config: None,
    })
    .unwrap_err();
    assert_eq!(missing_bouncer.fields(), ["pgBouncerConfig"]);

    let missing_both =
        PostgresInstanceConfig::try_from(PostgresInstanceConfigResponse::default()).unwrap_err();
    assert_eq!(missing_both.fields(), ["pgBouncerConfig", "pgConfig"]);
    assert_eq!(
        missing_both.to_string(),
        "the API response is missing required field(s): pgBouncerConfig, pgConfig"
    );
}

#[test]
fn backup_minimal_response() {
    let b: Backup = serde_json::from_str("{}").unwrap();
    assert_eq!(b.id, None);
    assert_eq!(b.status, None);
    assert_eq!(b.size_in_bytes, None);
}

#[test]
fn api_key_minimal_response() {
    let k: ApiKey = serde_json::from_str(r#"{"name":"k"}"#).unwrap();
    assert_eq!(k.name.as_deref(), Some("k"));
    assert_eq!(k.id, None);
    assert_eq!(k.state, None);
}

#[test]
fn service_response_tolerates_dropped_and_null_fields() {
    // A response field the API stops sending, and one it sends as an explicit
    // `null`, must both land as `None` rather than failing the response. `null`
    // is the case `#[serde(default)]` never covered: it only fills a missing
    // key.
    let dropped: Service = serde_json::from_str("{}").unwrap();
    let nulled: Service = serde_json::from_str(
        r#"{"id":null,"name":null,"state":null,"endpoints":null,"ipAccessList":null,
            "tags":null,"currentScaling":null,"scalingSchedule":null,"numReplicas":null}"#,
    )
    .unwrap();
    assert_eq!(dropped, Service::default());
    assert_eq!(nulled, Service::default());
    assert_eq!(nulled.endpoints, None);
    assert_eq!(nulled.ip_access_list, None);
    assert_eq!(nulled.tags, None);
}

#[test]
fn service_response_omits_absent_fields_when_serialized() {
    // Absent means absent, at every level of the response tree: a field that
    // was not returned is omitted from `--json` output, never emitted as
    // `null`.
    let svc = Service {
        name: Some("svc".to_string()),
        ip_access_list: Some(vec![IpAccessListEntryResponse {
            source: Some("0.0.0.0/0".to_string()),
            description: None,
        }]),
        tags: Some(vec![ResourceTagsV1Response {
            key: Some("env".to_string()),
            value: None,
        }]),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&svc).unwrap(),
        serde_json::json!({
            "name": "svc",
            "ipAccessList": [{ "source": "0.0.0.0/0" }],
            "tags": [{ "key": "env" }],
        })
    );
}

#[test]
fn shared_leaves_stay_strict_on_the_request_side() {
    // `ipAccessListEntry` and `resourceTagsV1` are sent as well as returned, so
    // each splits: the request variant keeps the schema's required fields as
    // `T`, and only the response variant is all-`Option`.
    let entry = IpAccessListEntry {
        source: "0.0.0.0/0".to_string(),
        description: None,
    };
    assert_eq!(
        serde_json::to_value(&entry).unwrap(),
        serde_json::json!({ "source": "0.0.0.0/0" })
    );
    // A request payload missing a required field is rejected, not defaulted.
    assert!(serde_json::from_str::<IpAccessListEntry>("{}").is_err());
    assert!(serde_json::from_str::<ResourceTagsV1>("{}").is_err());
    // The response variants accept the same payload.
    assert_eq!(
        serde_json::from_str::<IpAccessListEntryResponse>("{}").unwrap(),
        IpAccessListEntryResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<ResourceTagsV1Response>("{}").unwrap(),
        ResourceTagsV1Response::default()
    );
}

#[test]
fn infrastructure_responses_tolerate_dropped_and_null_fields() {
    // Reverse private endpoints and quotas are response-only, so every field is
    // `Option<T>`: a dropped key and an explicit `null` both land as `None`.
    // `null` is the case `#[serde(default)]` never covered.
    let dropped: ReversePrivateEndpoint = serde_json::from_str("{}").unwrap();
    let nulled: ReversePrivateEndpoint = serde_json::from_str(
        r#"{"id":null,"description":null,"status":null,"type":null,"dnsNames":null,
            "privateDnsNames":null,"endpointId":null,"serviceId":null,
            "customPrivateDnsMappings":null}"#,
    )
    .unwrap();
    assert_eq!(dropped, ReversePrivateEndpoint::default());
    assert_eq!(nulled, ReversePrivateEndpoint::default());
    // A `null` on a list field is the residual case the previous
    // `#[serde(default)]` policy still failed on.
    assert_eq!(nulled.dns_names, None);
    assert_eq!(nulled.custom_private_dns_mappings, None);

    let quota: OrganizationQuota = serde_json::from_str(r#"{"name":"Services"}"#).unwrap();
    assert_eq!(quota.name.as_deref(), Some("Services"));
    assert_eq!(quota.quota_code, None);
    assert_eq!(quota.value, None);
    assert_eq!(quota.adjustable, None);
}

#[test]
fn reverse_private_endpoint_omits_absent_fields_when_serialized() {
    // Absence stays absent in `--json` output, including inside the nested
    // response variant of the shared `customPrivateDnsMapping` schema.
    let rpe = ReversePrivateEndpoint {
        description: Some("MSK endpoint".to_string()),
        custom_private_dns_mappings: Some(vec![CustomPrivateDnsMappingResponse {
            private_dns_name: None,
        }]),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&rpe).unwrap(),
        serde_json::json!({
            "description": "MSK endpoint",
            "customPrivateDnsMappings": [{}],
        })
    );
}

#[test]
fn infrastructure_shared_leaves_stay_strict_on_the_request_side() {
    // `customPrivateDnsMapping` and `RBACPolicyTags` are sent as well as
    // returned, so each splits: only the `{Name}Response` variant is used by a
    // response type, and neither variant fabricates a value for a missing key.
    let mapping = CustomPrivateDnsMapping {
        private_dns_name: Some("db.internal".to_string()),
    };
    assert_eq!(
        serde_json::to_value(&mapping).unwrap(),
        serde_json::json!({ "privateDnsName": "db.internal" })
    );
    // Both schemas are all-optional upstream, so the request variants accept an
    // empty object too — what the split guarantees is that the response variant
    // can never regain a required field.
    assert_eq!(
        serde_json::from_str::<CustomPrivateDnsMappingResponse>("{}").unwrap(),
        CustomPrivateDnsMappingResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<RBACPolicyTagsResponse>(r#"{"grants":null,"roleV2":null}"#).unwrap(),
        RBACPolicyTagsResponse::default()
    );
    let policy: RBACPolicy = serde_json::from_str(r#"{"tags":{"grants":["select"]}}"#).unwrap();
    assert_eq!(
        policy.tags,
        Some(RBACPolicyTagsResponse {
            grants: Some(vec!["select".to_string()]),
            role_v2: None,
        })
    );
}

#[test]
fn reverse_private_endpoint_request_rejects_a_missing_required_field() {
    // The request side is strict: without `#[serde(default)]` a payload missing
    // `description` or `type` is rejected rather than silently defaulted.
    assert!(serde_json::from_str::<CreateReversePrivateEndpoint>("{}").is_err());
    let body = CreateReversePrivateEndpoint {
        description: "New RPE".to_string(),
        r#type: CreateReversePrivateEndpointType::MSK_MULTI_VPC,
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&body).unwrap(),
        serde_json::json!({ "description": "New RPE", "type": "MSK_MULTI_VPC" })
    );
}

#[test]
fn resource_tag_response_converts_back_into_a_request_tag() {
    let tag = ResourceTagsV1::try_from(ResourceTagsV1Response {
        key: Some("env".to_string()),
        value: Some("dev".to_string()),
    })
    .unwrap();
    assert_eq!(tag.key, "env");
    assert_eq!(tag.value.as_deref(), Some("dev"));

    // A tag is identified by its key, so a keyless response tag cannot be
    // written back.
    let missing = ResourceTagsV1::try_from(ResourceTagsV1Response {
        key: None,
        value: Some("dev".to_string()),
    })
    .unwrap_err();
    assert_eq!(missing.fields(), ["key"]);
}

#[test]
fn scaling_schedule_entry_response_converts_back_into_a_request_entry() {
    let entry = ScalingScheduleEntry {
        name: Some("weekday-peak".to_string()),
        weekdays: Some(vec![1, 2, 3]),
        start_hour_utc: Some(8),
        end_hour_utc: Some(18),
        autoscaling_mode: Some(AutoscalingMode::Vertical),
        min_replica_memory_gb: Some(16.0),
        ..Default::default()
    };
    let request = ScalingScheduleEntryRequest::try_from(entry).unwrap();
    assert_eq!(request.name, "weekday-peak");
    assert_eq!(request.weekdays, vec![1, 2, 3]);
    assert_eq!(request.start_hour_utc, 8);
    assert_eq!(request.end_hour_utc, 18);
    assert_eq!(request.min_replica_memory_gb, Some(16.0));

    // An upsert replaces the whole schedule, so an entry the API returned
    // without its window bounds, weekdays or name cannot be re-sent.
    let missing =
        ScalingScheduleEntryRequest::try_from(ScalingScheduleEntry::default()).unwrap_err();
    assert_eq!(
        missing.fields(),
        ["endHourUtc", "name", "startHourUtc", "weekdays"]
    );
}

#[test]
fn upgrade_window_response_converts_back_into_a_put_body() {
    let request = UpgradeWindowPutRequest::try_from(UpgradeWindow {
        // `duration` is response-only and does not cross over.
        duration: Some(21600),
        start_hour_utc: Some(6),
        weekday: Some(2),
    })
    .unwrap();
    assert_eq!(request.start_hour_utc, 6);
    assert_eq!(request.weekday, 2);

    let missing = UpgradeWindowPutRequest::try_from(UpgradeWindow::default()).unwrap_err();
    assert_eq!(missing.fields(), ["startHourUtc", "weekday"]);
}

#[test]
fn clickstack_dashboard_minimal_response() {
    let d: ClickStackDashboardResponse = serde_json::from_str("{}").unwrap();
    assert_eq!(d.id, None);
    assert_eq!(d.name, None);
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
    assert_eq!(resp.password.as_deref(), Some("gen-pw-123"));
    let service = resp.service.unwrap();
    assert_eq!(service.name.as_deref(), Some("new-svc"));
    assert_eq!(service.state, Some(ServiceState::Provisioning));
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
    assert_eq!(cost.grand_total_chc, Some(35.5));
    assert_eq!(cost.costs.as_deref().map(<[_]>::len), Some(2));
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
    let config: PostgresInstanceConfigResponse = serde_json::from_str(json).unwrap();
    assert_eq!(
        config.pg_config.unwrap().max_connections,
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
    let config: PostgresInstanceConfigResponse = serde_json::from_str(json).unwrap();
    let pg_config = config.pg_config.expect("pgConfig present in the payload");
    assert_eq!(pg_config.max_connections, Some(serde_json::json!("100")));
    assert_eq!(pg_config.random_page_cost, Some(serde_json::json!("1.1")));
    assert_eq!(pg_config.max_worker_processes, Some(serde_json::json!(8)));
    assert_eq!(pg_config.autovacuum_naptime, Some(serde_json::json!("5s")));
    assert_eq!(
        pg_config.autovacuum_vacuum_scale_factor,
        Some(serde_json::json!("0.2"))
    );
    assert_eq!(pg_config.autovacuum_max_workers, Some(serde_json::json!(3)));
}

#[test]
fn deserialize_reverse_private_endpoint() {
    let json = r#"{
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "description": "MSK endpoint",
        "status": "available"
    }"#;
    let rpe: ReversePrivateEndpoint = serde_json::from_str(json).unwrap();
    assert_eq!(rpe.description.as_deref(), Some("MSK endpoint"));
    assert_eq!(
        rpe.status,
        Some(ReversePrivateEndpointStatus::Other("available".to_string()))
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
    assert_eq!(src.brokers.as_deref(), Some("broker1:9092,broker2:9092"));
    assert_eq!(src.topics.as_deref(), Some("my-topic"));
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
    assert_eq!(dest.database.as_deref(), Some("default"));
    assert_eq!(dest.table.as_deref(), Some("events"));
    let columns = dest.columns.expect("columns should populate");
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0].name.as_deref(), Some("id"));
}

#[test]
fn deserialize_clickpipe_scaling() {
    let json = r#"{
        "replicas": 3,
        "concurrency": 2
    }"#;
    let s: ClickPipeScalingResponse = serde_json::from_str(json).unwrap();
    assert_eq!(s.replicas, Some(3));
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(s.concurrency, Some(2));
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
    assert_eq!(w.weekday, Some(2));
    assert_eq!(w.start_hour_utc, Some(6));
    assert_eq!(w.duration, Some(21600));

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
    assert_eq!(src.topic.as_deref(), Some("projects/p/topics/t"));
    assert_eq!(src.project_id.as_deref(), Some("my-project"));
    assert_eq!(
        src.authentication,
        Some(ClickPipePubSubSourceAuthentication::ServiceAccount)
    );
    assert_eq!(src.format, Some(ClickPipePubSubSourceFormat::JSONEachRow));
    assert_eq!(src.seek_type, Some(ClickPipePubSubSourceSeektype::Latest));
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
    assert_eq!(pubsub.topic.as_deref(), Some("projects/p/topics/t"));
    assert_eq!(
        pubsub.format,
        Some(ClickPipePubSubSourceFormat::JSONEachRow)
    );
}

// ===========================================================================
// ClickStack dashboard containers, on-click (issue #203 drift)
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
    assert_eq!(containers[0].id.as_deref(), Some("c-1"));
    assert_eq!(containers[0].collapsed, Some(false));
    let tabs = containers[0].tabs.as_ref().expect("tabs populated");
    assert_eq!(tabs[0].title.as_deref(), Some("Tab 1"));
    let tiles = dash.tiles.expect("tiles should populate");
    assert_eq!(tiles[0].container_id.as_deref(), Some("c-1"));
    assert_eq!(tiles[0].tab_id.as_deref(), Some("t-1"));
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
    // Regression: Search and Dashboard are structurally identical (both require
    // `target` + `type`), so before manual discriminator dispatch a
    // `type: "dashboard"` payload misdispatched to the Search variant. The union
    // now routes on the `type` discriminator.
    let json = r#"{
        "type": "dashboard",
        "target": {"mode": "template", "template": "{{x}}"},
        "whereLanguage": "sql",
        "whereTemplate": "x = {{y}}"
    }"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    let dash = match on_click {
        ClickStackOnClick::ClickStackOnClickDashboard(dash) => dash,
        other => panic!("expected dashboard variant, got {other}"),
    };
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
fn deserialize_clickstack_on_click_target_unknown_shape_round_trips() {
    // A payload matching neither the id nor template variant must land in the
    // lossless Unknown catch-all and re-serialize to the same JSON object rather
    // than erroring on deserialize.
    let json = r#"{
        "mode": "future_mode",
        "foo": 1
    }"#;
    assert_unknown_variant_round_trips(json, |t: &ClickStackOnClickTarget| {
        matches!(t, ClickStackOnClickTarget::Unknown(_))
    });
}

#[test]
fn deserialize_clickstack_on_click_external_round_trip() {
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
    // Regression: a `type: "dashboard"` payload dispatches to the Dashboard
    // variant through the union rather than being greedily absorbed by the
    // structurally identical Search variant.
    let json = r#"{
        "type": "dashboard",
        "target": {"mode": "template", "template": "{{x}}"}
    }"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    match on_click {
        ClickStackOnClick::ClickStackOnClickDashboard(dash) => {
            assert_eq!(dash.r#type, ClickStackOnClickDashboardType::Dashboard);
        }
        other => panic!("expected dashboard variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_on_click_unknown_type_round_trip() {
    // An unrecognized `type` discriminator falls back to the Unknown variant,
    // which stores the raw JSON so it round-trips faithfully.
    let json = r#"{"type":"popover","payload":{"nested":[1,2,3]}}"#;
    let on_click: ClickStackOnClick = serde_json::from_str(json).unwrap();
    match &on_click {
        ClickStackOnClick::Unknown(v) => {
            assert_eq!(v["type"], "popover");
            assert_eq!(v["payload"]["nested"][2], 3);
        }
        other => panic!("expected unknown variant, got {other}"),
    }
    let expected: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_value(&on_click).unwrap(), expected);
}

#[test]
fn clickstack_alert_channel_known_variants_deserialize() {
    let email_json = r#"{"emailRecipients": ["a@b.com"], "type": "email"}"#;
    let email: ClickStackAlertChannel = serde_json::from_str(email_json).unwrap();
    match email {
        ClickStackAlertChannel::ClickStackAlertChannelEmail(v) => {
            assert_eq!(v.email_recipients, vec!["a@b.com".to_string()]);
            assert_eq!(v.r#type, ClickStackAlertChannelEmailType::Email);
        }
        other => panic!("expected email variant, got {other}"),
    }

    let webhook_json = r#"{"webhookId": "wh-1", "type": "webhook"}"#;
    let webhook: ClickStackAlertChannel = serde_json::from_str(webhook_json).unwrap();
    match webhook {
        ClickStackAlertChannel::ClickStackAlertChannelWebhook(v) => {
            assert_eq!(v.webhook_id, "wh-1");
            assert_eq!(v.r#type, ClickStackAlertChannelWebhookType::Webhook);
        }
        other => panic!("expected webhook variant, got {other}"),
    }
}

#[test]
fn clickstack_alert_channel_response_known_type_sparse_payload_is_tolerant() {
    // A recognized `type` dispatches hard to its variant, and the response
    // variant's all-`Option` fields are tolerant: a server that drops
    // `emailRecipients` surfaces `None` rather than failing the response.
    let json = r#"{"type":"email"}"#;
    let channel: ClickStackAlertChannelResponse = serde_json::from_str(json).unwrap();
    match channel {
        ClickStackAlertChannelResponse::ClickStackAlertChannelEmail(v) => {
            assert_eq!(v.r#type, Some(ClickStackAlertChannelEmailType::Email));
            assert_eq!(v.email_recipients, None);
        }
        other => panic!("expected email variant, got {other}"),
    }
    // The request variant stays strict: the same sparse payload is not a valid
    // channel to *send*.
    assert!(serde_json::from_str::<ClickStackAlertChannelEmail>(r#"{"type":"email"}"#).is_err());
}

#[test]
fn clickstack_alert_channel_missing_type_key_is_unknown() {
    // This union has no arm for an absent `type` key, in deliberate contrast to
    // the chart-config unions where absence means the Builder variant, so a
    // payload without the discriminator lands in the lossless Unknown catch-all.
    let json = r#"{"webhookId":"wh-1"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChannel| {
        matches!(c, ClickStackAlertChannel::Unknown(_))
    });
}

#[test]
fn clickstack_alert_channel_unknown_shape_round_trips() {
    // An unrecognized `type` value must land in the lossless Unknown catch-all
    // and re-serialize to the same JSON object rather than erroring on
    // deserialize.
    let json = r#"{
        "type": "future_channel",
        "foo": 1
    }"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChannel| {
        matches!(c, ClickStackAlertChannel::Unknown(_))
    });
}

#[test]
fn clickstack_alert_channel_changed_field_shape_is_unknown() {
    // A recognized `type` whose payload no longer fits the variant — here
    // `emailRecipients` as a string instead of an array — must not fail the
    // response. `list_alerts` returns a Vec, so one such element would otherwise
    // take down the whole call; instead the element lands in Unknown intact.
    let json = r#"{"type":"email","emailRecipients":"a@b.c"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChannel| {
        matches!(c, ClickStackAlertChannel::Unknown(_))
    });
}

#[test]
fn clickstack_alert_channel_default_round_trips_to_the_same_variant() {
    // Both alert-channel variants declare the same `enum: ["webhook", "email"]`
    // for their discriminating `type`, so the email variant's default must name
    // `email`: `channel` is defaulted on the alert response, and a default that
    // named `webhook` would come back from its own union as the webhook variant
    // and could be PUT back as a webhook channel with no `webhookId`.
    let default = ClickStackAlertChannel::default();
    assert!(matches!(
        default,
        ClickStackAlertChannel::ClickStackAlertChannelEmail(_)
    ));
    let json = serde_json::to_string(&default).unwrap();
    assert_eq!(json, r#"{"emailRecipients":[],"type":"email"}"#);
    let round_tripped: ClickStackAlertChannel = serde_json::from_str(&json).unwrap();
    assert_eq!(round_tripped, default);

    // A response that drops `channel` entirely surfaces the absence instead of
    // fabricating a default channel.
    let response: ClickStackAlertResponse = serde_json::from_str(r#"{"name":"my-alert"}"#).unwrap();
    assert_eq!(response.channel, None);
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
    let src: ClickStackLogSourceResponse = serde_json::from_str(json).unwrap();
    let mv = src
        .metadata_materialized_views
        .expect("metadataMaterializedViews should populate");
    assert_eq!(mv.granularity.as_deref(), Some("1 hour"));
    assert_eq!(mv.key_rollup_table.as_deref(), Some("logs_keys_1h"));
    assert_eq!(mv.kv_rollup_table.as_deref(), Some("logs_kv_1h"));
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
fn deserialize_clickstack_trace_source_default_table_select_expression() {
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

    // `defaultTableSelectExpression` is in the spec `required[]` for trace
    // sources, so the request variant types it as `String`. Responses stay
    // tolerant of a server-side field drop through the response variant, where
    // it is `Option<String>`: a missing field lands as `None` instead of
    // failing the whole payload (and, via the `kind`-dispatched
    // `ClickStackSourceResponse` union, the whole list response).
    let missing = r#"{
        "id": "trace-1",
        "kind": "trace",
        "name": "traces",
        "connection": "conn-1",
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
    let src: ClickStackTraceSourceResponse = serde_json::from_str(missing).unwrap();
    assert_eq!(src.default_table_select_expression, None);
    match serde_json::from_str::<ClickStackSourceResponse>(missing).unwrap() {
        ClickStackSourceResponse::ClickStackTraceSource(src) => {
            assert_eq!(src.default_table_select_expression, None);
        }
        other => panic!("expected trace variant, got {other:?}"),
    }
    // The dropped field is absent from the re-serialized payload rather than
    // sent back as `null`.
    let v = serde_json::to_value(&src).unwrap();
    assert!(v.get("defaultTableSelectExpression").is_none());
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
    assert_unknown_variant_round_trips(json, |s: &ClickStackSource| {
        matches!(s, ClickStackSource::Unknown(_))
    });
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
fn clickstack_source_response_dispatches_on_kind_alone() {
    // The response variants are all-`Option`, so every one of them matches any
    // JSON object: under `untagged` shape matching the first arm would swallow
    // all five kinds. Dispatch reads `kind` off the raw JSON instead, so a
    // payload carrying nothing but its discriminator still resolves to the right
    // variant.
    for (kind, expected) in [
        ("log", "ClickStackLogSource"),
        ("trace", "ClickStackTraceSource"),
        ("metric", "ClickStackMetricSource"),
        ("session", "ClickStackSessionSource"),
        ("promql", "ClickStackPromqlSource"),
    ] {
        let json = format!(r#"{{"kind":"{kind}"}}"#);
        let source: ClickStackSourceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(source.to_string(), expected, "wrong variant for {kind}");
    }
}

#[test]
fn clickstack_source_response_treats_dropped_and_null_fields_as_absent() {
    // Both halves of the tolerance contract on one payload: `connection` is sent
    // as an explicit `null` (which `serde(default)` would not have absorbed) and
    // every other field of the schema is dropped outright.
    let json = r#"{"kind":"log","connection":null,"name":"logs"}"#;
    let source: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let ClickStackSourceResponse::ClickStackLogSource(log) = source else {
        panic!("expected the log variant");
    };
    assert_eq!(log.connection, None);
    assert_eq!(log.from, None);
    assert_eq!(log.timestamp_value_expression, None);
    assert_eq!(log.name.as_deref(), Some("logs"));

    // Absent fields are omitted on the way out, not re-emitted as `null`.
    let v = serde_json::to_value(&log).unwrap();
    assert_eq!(v, serde_json::json!({"kind": "log", "name": "logs"}));
}

#[test]
fn clickstack_source_response_unknown_kind_round_trips() {
    let json = r#"{"kind":"future_kind","name":"x"}"#;
    assert_unknown_variant_round_trips(json, |s: &ClickStackSourceResponse| {
        matches!(s, ClickStackSourceResponse::Unknown(_))
    });
}

#[test]
fn clickstack_source_response_nested_objects_are_tolerant() {
    // The nested objects of a source are response variants too, so a dropped
    // field inside `from`, `filterSettings` or `materializedViews` does not fail
    // the source.
    let json = r#"{
        "kind": "log",
        "from": {"databaseName": "default"},
        "filterSettings": {"columns": [{"name": "ServiceName"}]},
        "materializedViews": [{"tableName": "logs_1h", "aggregatedColumns": [{"aggFn": "sum"}]}],
        "querySettings": [{"setting": "max_threads"}]
    }"#;
    let source: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let ClickStackSourceResponse::ClickStackLogSource(log) = source else {
        panic!("expected the log variant");
    };
    let from = log.from.expect("from present");
    assert_eq!(from.database_name.as_deref(), Some("default"));
    assert_eq!(from.table_name, None);
    let filter_settings = log.filter_settings.expect("filterSettings present");
    assert_eq!(filter_settings.table_name, None);
    let columns = filter_settings.columns.expect("columns present");
    assert_eq!(columns[0].name.as_deref(), Some("ServiceName"));
    assert_eq!(columns[0].label, None);
    let views = log.materialized_views.expect("materializedViews present");
    assert_eq!(views[0].min_granularity, None);
    let aggregated = views[0]
        .aggregated_columns
        .as_deref()
        .expect("aggregatedColumns present");
    assert_eq!(aggregated[0].agg_fn.as_deref(), Some("sum"));
    assert_eq!(aggregated[0].mv_column, None);
    let query_settings = log.query_settings.expect("querySettings present");
    assert_eq!(query_settings[0].value, None);
}

#[test]
fn clickstack_source_try_from_response_converts_every_kind() {
    // One spec-complete payload per kind, each carrying the nested objects that
    // kind owns, so every conversion in the source tree is exercised: a source
    // fetched and written back unchanged must produce the JSON it came from.
    let payloads = [
        serde_json::json!({
            "kind": "log",
            "name": "logs",
            "connection": "conn-1",
            "defaultTableSelectExpression": "*",
            "from": {"databaseName": "default", "tableName": "logs"},
            "timestampValueExpression": "ts",
            "filterSettings": {
                "columns": [{"label": "Service", "name": "ServiceName"}],
                "databaseName": "default",
                "tableName": "logs_filters",
            },
            "highlightedRowAttributeExpressions": [{"sqlExpression": "ServiceName"}],
            "materializedViews": [{
                "aggregatedColumns": [{"aggFn": "sum", "mvColumn": "count"}],
                "databaseName": "default",
                "dimensionColumns": "ServiceName",
                "minGranularity": "1 hour",
                "tableName": "logs_1h",
                "timestampColumn": "ts",
            }],
            "metadataMaterializedViews": {
                "granularity": "1 hour",
                "keyRollupTable": "logs_keys_1h",
                "kvRollupTable": "logs_kv_1h",
            },
            "querySettings": [{"setting": "max_threads", "value": "4"}],
        }),
        serde_json::json!({
            "kind": "trace",
            "name": "traces",
            "connection": "conn-1",
            "defaultTableSelectExpression": "*",
            "durationExpression": "Duration",
            "durationPrecision": 9,
            "from": {"databaseName": "default", "tableName": "traces"},
            "parentSpanIdExpression": "ParentSpanId",
            "spanIdExpression": "SpanId",
            "spanKindExpression": "SpanKind",
            "spanNameExpression": "SpanName",
            "timestampValueExpression": "Timestamp",
            "traceIdExpression": "TraceId",
            "metadataMaterializedViews": {
                "granularity": "1 hour",
                "keyRollupTable": "traces_keys_1h",
                "kvRollupTable": "traces_kv_1h",
            },
        }),
        serde_json::json!({
            "kind": "metric",
            "name": "metrics",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "metrics"},
            "metricTables": {
                "exponential histogram": "otel_metrics_exponential_histogram",
                "gauge": "otel_metrics_gauge",
                "histogram": "otel_metrics_histogram",
                "sum": "otel_metrics_sum",
                "summary": "otel_metrics_summary",
            },
            "resourceAttributesExpression": "ResourceAttributes",
            "timestampValueExpression": "TimeUnix",
        }),
        serde_json::json!({
            "kind": "session",
            "name": "sessions",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "sessions"},
            "traceSourceId": "trace-1",
        }),
        serde_json::json!({
            "kind": "promql",
            "name": "prometheus",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "metrics"},
            "timestampValueExpression": "timestamp",
        }),
    ];

    for payload in payloads {
        let response: ClickStackSourceResponse = serde_json::from_value(payload.clone()).unwrap();
        let request = ClickStackSource::try_from(response)
            .unwrap_or_else(|e| panic!("{} should convert: {e}", payload["kind"]));
        assert_eq!(serde_json::to_value(&request).unwrap(), payload);
    }
}

#[test]
fn clickstack_source_try_from_response_converts_a_complete_source() {
    let json = r#"{
        "id": "src-1",
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts",
        "querySettings": [{"setting": "max_threads", "value": "4"}]
    }"#;
    let response: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let request = ClickStackSource::try_from(response).expect("conversion should succeed");
    let ClickStackSource::ClickStackLogSource(log) = &request else {
        panic!("expected the log variant");
    };
    assert_eq!(log.connection, "conn-1");
    assert_eq!(log.from.table_name, "logs");
    assert_eq!(
        log.query_settings.as_deref(),
        Some(
            [ClickStackQuerySetting {
                setting: "max_threads".to_string(),
                value: "4".to_string(),
            }]
            .as_slice()
        )
    );
    // A write-back of an untouched source is byte-identical to what was fetched.
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
}

#[test]
fn clickstack_source_try_from_response_names_every_missing_required_field() {
    let response: ClickStackSourceResponse =
        serde_json::from_str(r#"{"kind":"log","name":"logs"}"#).unwrap();
    let error = ClickStackSource::try_from(response).expect_err("conversion should fail");
    assert_eq!(
        error.fields(),
        [
            "connection",
            "defaultTableSelectExpression",
            "from",
            "timestampValueExpression"
        ]
    );
    assert_eq!(
        error.to_string(),
        "the API response is missing required field(s): connection, \
         defaultTableSelectExpression, from, timestampValueExpression"
    );
}

#[test]
fn clickstack_source_try_from_response_reports_a_nested_missing_field() {
    // A nested object reports its own wire name: `from` is present, so the
    // failure is `tableName` inside it.
    let json = r#"{
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default"},
        "timestampValueExpression": "ts"
    }"#;
    let response: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let error = ClickStackSource::try_from(response).expect_err("conversion should fail");
    assert_eq!(error.fields(), ["tableName"]);

    // The same holds for an element of a nested list.
    let json = r#"{
        "kind": "log",
        "name": "logs",
        "connection": "conn-1",
        "defaultTableSelectExpression": "*",
        "from": {"databaseName": "default", "tableName": "logs"},
        "timestampValueExpression": "ts",
        "querySettings": [{"setting": "max_threads"}]
    }"#;
    let response: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let error = ClickStackSource::try_from(response).expect_err("conversion should fail");
    assert_eq!(error.fields(), ["value"]);
}

#[test]
fn clickstack_source_try_from_response_passes_an_unknown_kind_through() {
    // A source kind this crate does not model must still be writable back: the
    // request union's `Unknown` arm holds the raw payload and serializes it
    // verbatim.
    let json = r#"{"kind":"future_kind","name":"x"}"#;
    let response: ClickStackSourceResponse = serde_json::from_str(json).unwrap();
    let request = ClickStackSource::try_from(response).expect("Unknown converts losslessly");
    assert!(matches!(request, ClickStackSource::Unknown(_)));
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
}

#[test]
fn shared_clickstack_source_types_stay_strict_on_the_request_side() {
    // Sources are sent as well as returned, so each type in the source tree
    // splits: the request variant keeps the schema's required fields as `T` and
    // rejects a payload that omits one, while the response variant accepts any
    // object.
    macro_rules! assert_split_strictness {
        ($($request:ty => $response:ty),+ $(,)?) => {
            $(
                assert!(
                    serde_json::from_str::<$request>("{}").is_err(),
                    "{} accepted a payload missing its required fields",
                    stringify!($request),
                );
                assert_eq!(
                    serde_json::from_str::<$response>("{}").unwrap(),
                    <$response>::default(),
                    "{} rejected an empty payload",
                    stringify!($response),
                );
            )+
        };
    }

    assert_split_strictness!(
        ClickStackAggregatedColumn => ClickStackAggregatedColumnResponse,
        ClickStackCASLPermission => ClickStackCASLPermissionResponse,
        ClickStackFilter => ClickStackFilterResponse,
        ClickStackFilterSettingsColumn => ClickStackFilterSettingsColumnResponse,
        ClickStackHighlightedAttributeExpression => ClickStackHighlightedAttributeExpressionResponse,
        ClickStackLogSource => ClickStackLogSourceResponse,
        ClickStackLogSourceMetadataMaterializedViews => ClickStackLogSourceMetadataMaterializedViewsResponse,
        ClickStackMaterializedView => ClickStackMaterializedViewResponse,
        ClickStackMetricSource => ClickStackMetricSourceResponse,
        ClickStackMetricSourceFrom => ClickStackMetricSourceFromResponse,
        ClickStackMetricTables => ClickStackMetricTablesResponse,
        ClickStackPromqlSource => ClickStackPromqlSourceResponse,
        ClickStackQuerySetting => ClickStackQuerySettingResponse,
        ClickStackSavedFilterValue => ClickStackSavedFilterValueResponse,
        ClickStackSavedSearchFilter => ClickStackSavedSearchFilterResponse,
        ClickStackSessionSource => ClickStackSessionSourceResponse,
        ClickStackSourceFilterSettings => ClickStackSourceFilterSettingsResponse,
        ClickStackSourceFrom => ClickStackSourceFromResponse,
        ClickStackTraceSource => ClickStackTraceSourceResponse,
        ClickStackTraceSourceMetadataMaterializedViews => ClickStackTraceSourceMetadataMaterializedViewsResponse,
    );
}

#[test]
fn clickstack_response_only_models_accept_an_empty_payload() {
    // Connections, roles and saved searches are only ever returned — the create
    // and update bodies are separate schemas — so they are all-`Option` in place
    // instead of splitting, and none of their fields can fail a response.
    assert_eq!(
        serde_json::from_str::<ClickStackConnection>("{}").unwrap(),
        ClickStackConnection::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickStackRole>("{}").unwrap(),
        ClickStackRole::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickStackSavedSearch>("{}").unwrap(),
        ClickStackSavedSearch::default()
    );
    // Their request counterparts stay strict.
    assert!(serde_json::from_str::<ClickStackCreateConnectionRequest>("{}").is_err());
    assert!(serde_json::from_str::<ClickStackCreateRoleRequest>("{}").is_err());
    assert!(serde_json::from_str::<ClickStackSavedSearchInput>("{}").is_err());
}

#[test]
fn shared_clickstack_dashboard_types_stay_strict_on_the_request_side() {
    // Dashboard containers, tiles' chart configs, select items, number formats,
    // color conditions, on-click targets and alert channels are sent as well as
    // returned, so each splits: the request variant keeps the schema's required
    // fields as `T` and rejects a payload that omits one, while the response
    // variant accepts any object.
    macro_rules! assert_split_strictness {
        ($($request:ty => $response:ty),+ $(,)?) => {
            $(
                assert!(
                    serde_json::from_str::<$request>("{}").is_err(),
                    "{} accepted a payload missing its required fields",
                    stringify!($request),
                );
                assert_eq!(
                    serde_json::from_str::<$response>("{}").unwrap(),
                    <$response>::default(),
                    "{} rejected an empty payload",
                    stringify!($response),
                );
            )+
        };
    }

    assert_split_strictness!(
        ClickStackAlertChannelEmail => ClickStackAlertChannelEmailResponse,
        ClickStackAlertChannelWebhook => ClickStackAlertChannelWebhookResponse,
        ClickStackBackgroundChart => ClickStackBackgroundChartResponse,
        ClickStackBarBuilderChartConfig => ClickStackBarBuilderChartConfigResponse,
        ClickStackBarRawSqlChartConfig => ClickStackBarRawSqlChartConfigResponse,
        ClickStackBetweenColorCondition => ClickStackBetweenColorConditionResponse,
        ClickStackCategoricalBarBuilderChartConfig => ClickStackCategoricalBarBuilderChartConfigResponse,
        ClickStackCategoricalBarRawSqlChartConfig => ClickStackCategoricalBarRawSqlChartConfigResponse,
        ClickStackDashboardContainer => ClickStackDashboardContainerResponse,
        ClickStackDashboardContainerTab => ClickStackDashboardContainerTabResponse,
        ClickStackEqualityColorCondition => ClickStackEqualityColorConditionResponse,
        ClickStackEventPatternsChartConfig => ClickStackEventPatternsChartConfigResponse,
        ClickStackHeatmapChartConfig => ClickStackHeatmapChartConfigResponse,
        ClickStackHeatmapSelectItem => ClickStackHeatmapSelectItemResponse,
        ClickStackLineBuilderChartConfig => ClickStackLineBuilderChartConfigResponse,
        ClickStackLineRawSqlChartConfig => ClickStackLineRawSqlChartConfigResponse,
        ClickStackMarkdownChartConfig => ClickStackMarkdownChartConfigResponse,
        ClickStackNumberBuilderChartConfig => ClickStackNumberBuilderChartConfigResponse,
        ClickStackNumberFormat => ClickStackNumberFormatResponse,
        ClickStackNumberRawSqlChartConfig => ClickStackNumberRawSqlChartConfigResponse,
        ClickStackNumericColorCondition => ClickStackNumericColorConditionResponse,
        ClickStackOnClickDashboard => ClickStackOnClickDashboardResponse,
        ClickStackOnClickExternal => ClickStackOnClickExternalResponse,
        ClickStackOnClickFilterTemplate => ClickStackOnClickFilterTemplateResponse,
        ClickStackOnClickSearch => ClickStackOnClickSearchResponse,
        ClickStackOnClickTargetIdVariant => ClickStackOnClickTargetIdVariantResponse,
        ClickStackOnClickTargetTemplateVariant => ClickStackOnClickTargetTemplateVariantResponse,
        ClickStackPieBuilderChartConfig => ClickStackPieBuilderChartConfigResponse,
        ClickStackPieRawSqlChartConfig => ClickStackPieRawSqlChartConfigResponse,
        ClickStackSearchChartConfig => ClickStackSearchChartConfigResponse,
        ClickStackSelectItem => ClickStackSelectItemResponse,
        ClickStackTableBuilderChartConfig => ClickStackTableBuilderChartConfigResponse,
        ClickStackTableRawSqlChartConfig => ClickStackTableRawSqlChartConfigResponse,
    );
}

#[test]
fn clickstack_alert_and_dashboard_response_models_accept_an_empty_payload() {
    // Alerts, dashboards, tiles, validation results and webhooks are only ever
    // returned — their request bodies are separate schemas — so they are
    // all-`Option` in place instead of splitting, and none of their fields can
    // fail a response.
    macro_rules! assert_accepts_empty {
        ($($model:ty),+ $(,)?) => {
            $(
                assert_eq!(
                    serde_json::from_str::<$model>("{}").unwrap(),
                    <$model>::default(),
                    "{} rejected an empty payload",
                    stringify!($model),
                );
            )+
        };
    }

    assert_accepts_empty!(
        ClickStackAlertExecutionError,
        ClickStackAlertResponse,
        ClickStackAlertSilenced,
        ClickStackDashboardResponse,
        ClickStackGenericWebhook,
        ClickStackIncidentIOWebhook,
        ClickStackPagerDutyAPIWebhook,
        ClickStackSlackAPIWebhook,
        ClickStackSlackWebhook,
        ClickStackTileOutput,
        ClickStackValidateDashboardError,
        ClickStackValidateDashboardResponse,
    );
}

#[test]
fn clickstack_dashboard_response_null_fields_deserialize_to_none() {
    // An explicit JSON `null` — not just a missing key — must land as `None`:
    // `Option<T>` absorbs it natively, which `serde(default)` never did.
    let json = r#"{"id":null,"name":null,"tiles":null,"filters":null,"tags":null}"#;
    let dash: ClickStackDashboardResponse = serde_json::from_str(json).unwrap();
    assert_eq!(dash, ClickStackDashboardResponse::default());
    // Absent fields are omitted on serialize, not emitted as `null`.
    assert_eq!(serde_json::to_value(&dash).unwrap(), serde_json::json!({}));
}

#[test]
fn clickstack_tile_config_response_raw_sql_body_without_config_type_falls_to_sub_union_unknown() {
    // Every field of a response builder variant is `Option`, so the builder arm
    // is total and would absorb a raw-SQL payload whose server dropped
    // `configType`, silently retyping it and losing `connectionId` and
    // `sqlTemplate`. The `none unless` guard routes it to Unknown, which keeps
    // the payload verbatim.
    let json = r#"{"displayType":"line","connectionId":"conn-1","sqlTemplate":"SELECT 1"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfigResponse| {
        matches!(
            cfg,
            ClickStackTileConfigResponse::ClickStackLineChartConfig(
                ClickStackLineChartConfigResponse::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_tile_config_response_dispatches_raw_sql_variant() {
    // A `configType: "sql"` payload dispatches to the raw-SQL response variant
    // even when the raw-SQL-required fields are missing: response tolerance is
    // per-field, not per-variant.
    let json = r#"{"displayType":"line","configType":"sql"}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackLineChartConfig(
            ClickStackLineChartConfigResponse::ClickStackLineRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, None);
            assert_eq!(r.sql_template, None);
        }
        other => panic!("expected line raw-SQL variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_response_changed_field_shape_falls_to_sub_union_unknown() {
    // A recognized discriminator whose payload no longer fits the variant —
    // here `sqlTemplate` as a number — must not fail the response: the element
    // lands in the lossless Unknown catch-all and round-trips verbatim.
    let json = r#"{"displayType":"line","configType":"sql","sqlTemplate":5}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfigResponse| {
        matches!(
            cfg,
            ClickStackTileConfigResponse::ClickStackLineChartConfig(
                ClickStackLineChartConfigResponse::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_tile_config_response_unknown_display_type_round_trips() {
    let json = r#"{"displayType":"hologram","select":[]}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfigResponse| {
        matches!(cfg, ClickStackTileConfigResponse::Unknown(_))
    });
}

#[test]
fn clickstack_alert_channel_response_dispatches_and_absorbs_changed_shapes() {
    // Known discriminators dispatch to their response variants.
    let email: ClickStackAlertChannelResponse =
        serde_json::from_str(r#"{"type":"email","emailRecipients":["a@b.c"]}"#).unwrap();
    match email {
        ClickStackAlertChannelResponse::ClickStackAlertChannelEmail(v) => {
            assert_eq!(
                v.email_recipients.as_deref(),
                Some(&["a@b.c".to_string()][..])
            );
        }
        other => panic!("expected email variant, got {other}"),
    }
    // A recognized `type` whose payload no longer fits the variant — here
    // `emailRecipients` as a string — lands in Unknown intact.
    let json = r#"{"type":"email","emailRecipients":"a@b.c"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChannelResponse| {
        matches!(c, ClickStackAlertChannelResponse::Unknown(_))
    });
    // As does an unrecognized `type`.
    let json = r#"{"type":"future_channel","foo":1}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackAlertChannelResponse| {
        matches!(c, ClickStackAlertChannelResponse::Unknown(_))
    });
}

#[test]
fn clickstack_on_click_response_dispatches_on_type_and_mode() {
    let json = r#"{"type":"external","urlTemplate":"https://example.com/{{id}}"}"#;
    let on_click: ClickStackOnClickResponse = serde_json::from_str(json).unwrap();
    match on_click {
        ClickStackOnClickResponse::ClickStackOnClickExternal(e) => {
            assert_eq!(
                e.url_template.as_deref(),
                Some("https://example.com/{{id}}")
            );
        }
        other => panic!("expected external variant, got {other}"),
    }
    // A search on-click whose server dropped `target` still dispatches; the
    // absence is a `None`, not a failure.
    let json = r#"{"type":"search"}"#;
    let on_click: ClickStackOnClickResponse = serde_json::from_str(json).unwrap();
    match on_click {
        ClickStackOnClickResponse::ClickStackOnClickSearch(s) => assert_eq!(s.target, None),
        other => panic!("expected search variant, got {other}"),
    }
    let json = r#"{"mode":"template","template":"{{rowId}}"}"#;
    let target: ClickStackOnClickTargetResponse = serde_json::from_str(json).unwrap();
    match target {
        ClickStackOnClickTargetResponse::ClickStackOnClickTargetTemplateVariant(t) => {
            assert_eq!(t.template.as_deref(), Some("{{rowId}}"));
        }
        other => panic!("expected template variant, got {other}"),
    }
    // Missing discriminators land in Unknown: these unions have no absence arm.
    let json = r#"{"urlTemplate":"https://example.com"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackOnClickResponse| {
        matches!(c, ClickStackOnClickResponse::Unknown(_))
    });
    let json = r#"{"template":"{{rowId}}"}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackOnClickTargetResponse| {
        matches!(c, ClickStackOnClickTargetResponse::Unknown(_))
    });
}

#[test]
fn clickstack_number_tile_color_condition_response_dispatches_on_operator() {
    let json = r#"{"operator":"between","value":[1.0, 2.0]}"#;
    let cond: ClickStackNumberTileColorConditionResponse = serde_json::from_str(json).unwrap();
    match cond {
        ClickStackNumberTileColorConditionResponse::ClickStackBetweenColorCondition(b) => {
            assert_eq!(b.value.as_deref(), Some(&[1.0, 2.0][..]));
            assert_eq!(b.color, None);
        }
        other => panic!("expected between variant, got {other}"),
    }
    let json = r#"{"operator":"someday","value":7}"#;
    assert_unknown_variant_round_trips(json, |c: &ClickStackNumberTileColorConditionResponse| {
        matches!(c, ClickStackNumberTileColorConditionResponse::Unknown(_))
    });
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
    let fields = resp.fields.clone().expect("fields should populate");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name.as_deref(), Some("user_id"));
    assert_eq!(fields[0].r#type.as_deref(), Some("Int64"));
    assert_eq!(fields[0].optional, Some(false));
    assert_eq!(fields[1].optional, Some(true));

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
    // `partitionByExpr` is required and non-nullable, so the request variant
    // types it as `String` and always sends it.
    let mapping = ClickPipePostgresPipeTableMapping {
        partition_by_expr: "toYYYYMM(created_at)".to_string(),
        ..Default::default()
    };
    let v = serde_json::to_value(&mapping).unwrap();
    assert_eq!(v["partitionByExpr"], "toYYYYMM(created_at)");

    // The response variant types it as `Option<String>`: present, dropped, and
    // explicitly `null` all deserialize.
    let mapping: ClickPipePostgresPipeTableMappingResponse =
        serde_json::from_str(r#"{"partitionByExpr": "toYYYYMM(created_at)"}"#).unwrap();
    assert_eq!(
        mapping.partition_by_expr.as_deref(),
        Some("toYYYYMM(created_at)")
    );
    let mapping: ClickPipePostgresPipeTableMappingResponse =
        serde_json::from_str(r#"{"partitionByExpr": null}"#).unwrap();
    assert_eq!(mapping.partition_by_expr, None);
    let mapping: ClickPipePostgresPipeTableMappingResponse = serde_json::from_str("{}").unwrap();
    assert_eq!(mapping.partition_by_expr, None);
}

#[test]
fn clickpipe_response_tolerates_dropped_and_null_fields() {
    // The whole ClickPipe response tree is all-`Option`, so a dropped key and
    // an explicit `null` both land as `None` — at the top level and inside the
    // nested source/destination/scaling/settings shapes.
    let dropped: ClickPipe = serde_json::from_str("{}").unwrap();
    let nulled: ClickPipe = serde_json::from_str(
        r#"{"id":null,"serviceId":null,"name":null,"state":null,"createdAt":null,
            "updatedAt":null,"scaling":null,"settings":null,"source":null,
            "destination":null,"fieldMappings":null}"#,
    )
    .unwrap();
    assert_eq!(dropped, ClickPipe::default());
    assert_eq!(nulled, ClickPipe::default());
    // `null` on a list field is the residual case the previous
    // `#[serde(default)]` policy still failed on.
    assert_eq!(nulled.field_mappings, None);

    let nested: ClickPipe = serde_json::from_str(
        r#"{"source":{"postgres":{"host":"pg.example","settings":null,
            "tableMappings":null}},
            "destination":{"columns":null,"tableDefinition":{"engine":null,
            "sortingKey":null}},
            "scaling":{},"settings":{}}"#,
    )
    .unwrap();
    let postgres = nested
        .source
        .and_then(|source| source.postgres)
        .expect("postgres source should populate");
    assert_eq!(postgres.host.as_deref(), Some("pg.example"));
    assert_eq!(postgres.settings, None);
    assert_eq!(postgres.table_mappings, None);
    let destination = nested.destination.expect("destination should populate");
    assert_eq!(destination.columns, None);
    assert_eq!(
        destination.table_definition,
        Some(ClickPipeDestinationTableDefinitionResponse::default())
    );
    assert_eq!(nested.scaling, Some(ClickPipeScalingResponse::default()));
    assert_eq!(nested.settings, Some(ClickPipeSettingsResponse::default()));
}

#[test]
fn clickpipe_response_omits_absent_fields_when_serialized() {
    // Absence stays absent in `--json` output, including inside the response
    // variants of the shared nested pipe schemas.
    let pipe = ClickPipe {
        name: Some("my-pipe".to_string()),
        scaling: Some(ClickPipeScalingResponse {
            replicas: Some(2),
            ..Default::default()
        }),
        field_mappings: Some(vec![ClickPipeFieldMappingResponse {
            source_field: Some("id".to_string()),
            destination_field: None,
        }]),
        destination: Some(ClickPipeDestination {
            table: Some("events".to_string()),
            table_definition: Some(ClickPipeDestinationTableDefinitionResponse {
                engine: Some(ClickPipeDestinationTableEngineResponse {
                    r#type: Some(ClickPipeDestinationTableEngineType::MergeTree),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&pipe).unwrap(),
        serde_json::json!({
            "name": "my-pipe",
            "scaling": { "replicas": 2 },
            "fieldMappings": [{ "sourceField": "id" }],
            "destination": {
                "table": "events",
                "tableDefinition": { "engine": { "type": "MergeTree" } },
            },
        })
    );
}

#[test]
fn shared_clickpipe_nested_types_stay_strict_on_the_request_side() {
    // The pipe settings, table mappings, destination shapes, field mappings and
    // scaling blocks are sent as well as returned, so each splits: the request
    // variant keeps the schema's required fields as `T`, and only the response
    // variant is all-`Option`.
    let mapping = ClickPipeFieldMapping {
        source_field: "id".to_string(),
        destination_field: "row_id".to_string(),
    };
    assert_eq!(
        serde_json::to_value(&mapping).unwrap(),
        serde_json::json!({ "sourceField": "id", "destinationField": "row_id" })
    );
    // A request payload missing a required field is rejected, not defaulted.
    assert!(serde_json::from_str::<ClickPipeFieldMapping>("{}").is_err());
    assert!(serde_json::from_str::<ClickPipeDestinationColumn>("{}").is_err());
    assert!(serde_json::from_str::<ClickPipeScaling>("{}").is_err());
    assert!(serde_json::from_str::<ClickPipePostgresPipeTableMapping>("{}").is_err());
    // The response variants accept the same payload.
    assert_eq!(
        serde_json::from_str::<ClickPipeFieldMappingResponse>("{}").unwrap(),
        ClickPipeFieldMappingResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickPipeDestinationColumnResponse>("{}").unwrap(),
        ClickPipeDestinationColumnResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickPipeScalingResponse>("{}").unwrap(),
        ClickPipeScalingResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickPipePostgresPipeTableMappingResponse>("{}").unwrap(),
        ClickPipePostgresPipeTableMappingResponse::default()
    );
    // `ClickPipeSettings` is an all-optional schema in both directions, so the
    // split is visible only in the type name the settings endpoints return.
    assert_eq!(
        serde_json::from_str::<ClickPipeSettingsResponse>("{}").unwrap(),
        ClickPipeSettingsResponse::default()
    );
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
    // The manual `operator`-keyed dispatch routes `gt`/`gte`/`lt`/`lte` to the
    // numeric variant.
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
    // The manual dispatch routes the `between` operator, with its inclusive
    // [min, max] array value, to the between variant.
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
    // The manual dispatch routes `eq` to the equality variant, whose value
    // accepts strings as well as numbers.
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
    // A numeric-valued `eq` is structurally identical to a numeric condition;
    // only the `operator`-keyed dispatch routes it to the equality variant
    // rather than the numeric one.
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
fn clickstack_number_tile_color_condition_dispatches_every_operator() {
    // Exhaustive guard over the `operator`-keyed dispatch: every wire value in the
    // discriminated_union! invocation must route to its concrete variant, never the
    // Unknown catch-all. A typo in any operator literal in the macro ("gte"/"lt"/
    // "lte"/"neq"/...) would silently misroute a valid payload to Unknown, and this
    // table would fail on that operator. The seven values are cross-checked against
    // the enum members of the three constituent schemas in the OpenAPI snapshot.
    #[derive(Debug, PartialEq)]
    enum Variant {
        Numeric,
        Between,
        Equality,
    }

    let cases: &[(&str, serde_json::Value, Variant)] = &[
        ("gt", serde_json::json!(100), Variant::Numeric),
        ("gte", serde_json::json!(100), Variant::Numeric),
        ("lt", serde_json::json!(100), Variant::Numeric),
        ("lte", serde_json::json!(100), Variant::Numeric),
        ("between", serde_json::json!([100, 500]), Variant::Between),
        ("eq", serde_json::json!(42), Variant::Equality),
        ("neq", serde_json::json!("healthy"), Variant::Equality),
    ];

    for (operator, value, expected) in cases {
        let json = serde_json::json!({
            "operator": operator,
            "value": value,
            "color": "chart-red",
        });
        let cond: ClickStackNumberTileColorCondition = serde_json::from_value(json)
            .unwrap_or_else(|e| panic!("operator {operator:?} failed to deserialize: {e}"));
        match (&cond, expected) {
            (
                ClickStackNumberTileColorCondition::ClickStackNumericColorCondition(c),
                Variant::Numeric,
            ) => assert_eq!(c.operator.to_string(), *operator),
            (
                ClickStackNumberTileColorCondition::ClickStackBetweenColorCondition(c),
                Variant::Between,
            ) => assert_eq!(c.operator.to_string(), *operator),
            (
                ClickStackNumberTileColorCondition::ClickStackEqualityColorCondition(c),
                Variant::Equality,
            ) => assert_eq!(c.operator.to_string(), *operator),
            (other, _) => {
                panic!("operator {operator:?} expected {expected:?} variant, got {other}")
            }
        }
        assert!(
            !matches!(cond, ClickStackNumberTileColorCondition::Unknown(_)),
            "operator {operator:?} misrouted to the Unknown catch-all"
        );
    }
}

#[test]
fn clickstack_number_tile_color_condition_unknown_operator_round_trip() {
    // An unrecognized `operator` discriminator falls back to the Unknown
    // variant, which now stores the raw JSON object and round-trips faithfully.
    let json = r#"{"operator":"contains","value":"warn","color":"chart-red"}"#;
    let cond: ClickStackNumberTileColorCondition = serde_json::from_str(json).unwrap();
    match &cond {
        ClickStackNumberTileColorCondition::Unknown(v) => {
            assert_eq!(v["operator"], "contains");
            assert_eq!(v["value"], "warn");
        }
        other => panic!("expected unknown variant, got {other}"),
    }
    let expected: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_value(&cond).unwrap(), expected);
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
    // The manual `displayType`-keyed dispatch routes "stacked_bar" to the
    // stacked-bar variant, never to the structurally identical categorical
    // bar variant (whose displayType is "bar").
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
    // `displayType: "event_patterns"` dispatches to the event-patterns variant.
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
fn clickstack_tile_config_line_variant() {
    // `displayType: "line"` must dispatch to the line variant with a typed
    // display_type, not be swallowed as an Unknown displayType by another
    // structurally-identical builder variant.
    let json = r#"{
        "displayType": "line",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackLineChartConfig(
            ClickStackLineChartConfig::ClickStackLineBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackLineBuilderChartConfigDisplaytype::Line
            );
        }
        other => panic!("expected line builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_stacked_bar_builder_variant() {
    // `displayType: "stacked_bar"` must dispatch to the (stacked) bar variant
    // with a typed display_type.
    let json = r#"{
        "displayType": "stacked_bar",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackBarChartConfig(
            ClickStackBarChartConfig::ClickStackBarBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackBarBuilderChartConfigDisplaytype::Stacked_bar
            );
        }
        other => panic!("expected stacked bar builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_table_variant() {
    // `displayType: "table"` must dispatch to the table variant with a typed
    // display_type (the legacy misdispatch parsed it as the first-listed
    // variant with an Unknown display_type).
    let json = r#"{
        "displayType": "table",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackTableChartConfig(
            ClickStackTableChartConfig::ClickStackTableBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackTableBuilderChartConfigDisplaytype::Table
            );
        }
        other => panic!("expected table builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_number_variant() {
    // `displayType: "number"` must dispatch to the number variant with a typed
    // display_type.
    let json = r#"{
        "displayType": "number",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackNumberChartConfig(
            ClickStackNumberChartConfig::ClickStackNumberBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackNumberBuilderChartConfigDisplaytype::Number
            );
        }
        other => panic!("expected number builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_pie_variant() {
    // `displayType: "pie"` must dispatch to the pie variant with a typed
    // display_type.
    let json = r#"{
        "displayType": "pie",
        "sourceId": "src-1",
        "select": [{"aggFn": "count"}]
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackPieChartConfig(
            ClickStackPieChartConfig::ClickStackPieBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, "src-1");
            assert_eq!(
                b.display_type,
                ClickStackPieBuilderChartConfigDisplaytype::Pie
            );
        }
        other => panic!("expected pie builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_search_variant() {
    // `displayType: "search"` must dispatch to the search variant with a typed
    // display_type.
    let json = r#"{
        "displayType": "search",
        "sourceId": "src-1",
        "select": "*",
        "whereLanguage": "lucene"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackSearchChartConfig(s) => {
            assert_eq!(s.source_id, "src-1");
            assert_eq!(s.select, "*");
            assert_eq!(
                s.display_type,
                ClickStackSearchChartConfigDisplaytype::Search
            );
        }
        other => panic!("expected search variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_markdown_variant() {
    // `displayType: "markdown"` must dispatch to the markdown variant with a
    // typed display_type rather than being absorbed by an earlier variant.
    let json = r#"{
        "displayType": "markdown",
        "markdown": "hello world"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackMarkdownChartConfig(m) => {
            assert_eq!(m.markdown.as_deref(), Some("hello world"));
            assert_eq!(
                m.display_type,
                ClickStackMarkdownChartConfigDisplaytype::Markdown
            );
        }
        other => panic!("expected markdown variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_heatmap_variant() {
    // `displayType: "heatmap"` is the only discriminator that reaches the
    // ClickStackHeatmapChartConfig arm; the heatmap-specific `select` shape with
    // `valueExpression` is then parsed by that variant, not used to select it.
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
fn clickstack_tile_config_unknown_display_type_round_trips() {
    // An unrecognized `displayType` falls to the Unknown catch-all, which now
    // stores the raw object and round-trips it faithfully.
    let json = r#"{"displayType":"sankey","sourceId":"src-1"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(cfg, ClickStackTileConfig::Unknown(_))
    });
}

#[test]
fn clickstack_tile_config_response_line_novel_shape_dispatches_to_builder() {
    // A known `displayType` carrying no `configType` dispatches to the builder
    // variant: the novel member is ignored and the builder's all-`Option`
    // response fields surface `None` instead of failing the response.
    let json = r#"{"displayType":"line","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackLineChartConfig(
            ClickStackLineChartConfigResponse::ClickStackLineBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected line builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_response_line_minimal_body_dispatches_to_builder() {
    // The bare discriminator with no `configType` key at all: key absence is the
    // builder discriminator, and every other builder response field is absent.
    let json = r#"{"displayType":"line"}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackLineChartConfig(
            ClickStackLineChartConfigResponse::ClickStackLineBuilderChartConfig(b),
        ) => {
            assert_eq!(
                b,
                ClickStackLineBuilderChartConfigResponse {
                    display_type: Some(ClickStackLineBuilderChartConfigDisplaytype::Line),
                    ..ClickStackLineBuilderChartConfigResponse::default()
                }
            );
        }
        other => panic!("expected line builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_response_non_string_config_type_dispatches_to_builder() {
    // A non-string `configType` is deliberately conflated with an absent one:
    // both dispatch to the builder variant rather than to Unknown.
    let json = r#"{"displayType":"line","configType":123,"sourceId":"src-1"}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackLineChartConfig(
            ClickStackLineChartConfigResponse::ClickStackLineBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id.as_deref(), Some("src-1"));
        }
        other => panic!("expected line builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_line_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the line sub-union's
    // Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"line","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackLineChartConfig(ClickStackLineChartConfig::Unknown(_))
        )
    });
}

#[test]
fn clickstack_tile_config_raw_sql_body_without_config_type_falls_to_sub_union_unknown() {
    // `configType` is spec-required on the Raw SQL configs, so a server that
    // stops sending it must not silently retype the tile: the builder variant is
    // total and would otherwise absorb the body and drop `connectionId` and
    // `sqlTemplate`. The `unless` guard routes it to Unknown, which keeps the
    // payload verbatim.
    let json = r#"{"displayType":"line","connectionId":"conn-1","sqlTemplate":"SELECT 1"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackLineChartConfig(ClickStackLineChartConfig::Unknown(_))
        )
    });

    // Either guard key on its own disqualifies the builder variant, and the
    // guard is wired on every chart-config sub-union, not just the line one.
    let json = r#"{"displayType":"number","connectionId":"conn-1"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackNumberChartConfig(
                ClickStackNumberChartConfig::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_tile_config_changed_field_shape_falls_to_sub_union_unknown() {
    // Field-level tolerance only covers a field the API stops sending. A field
    // whose *shape* changes cannot deserialize into the dispatched variant, so
    // the union hands the payload to its lossless Unknown catch-all rather than
    // failing the whole dashboard response.
    let builder = r#"{"displayType":"line","sourceId":123}"#;
    assert_unknown_variant_round_trips(builder, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackLineChartConfig(ClickStackLineChartConfig::Unknown(_))
        )
    });

    let raw_sql = r#"{"displayType":"line","configType":"sql","sqlTemplate":123}"#;
    assert_unknown_variant_round_trips(raw_sql, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackLineChartConfig(ClickStackLineChartConfig::Unknown(_))
        )
    });
}

#[test]
fn clickstack_tile_config_line_raw_sql_variant() {
    // The Raw SQL line config (configType "sql") dispatches to the line
    // sub-union's Raw SQL variant, not its builder or Unknown variant.
    let json = r#"{
        "displayType": "line",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT count() FROM t GROUP BY toStartOfMinute(ts)"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackLineChartConfig(
            ClickStackLineChartConfig::ClickStackLineRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(
                r.sql_template,
                "SELECT count() FROM t GROUP BY toStartOfMinute(ts)"
            );
            assert_eq!(
                r.display_type,
                ClickStackLineRawSqlChartConfigDisplaytype::Line
            );
        }
        other => panic!("expected line raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_stacked_bar_raw_sql_variant() {
    // The Raw SQL (stacked) bar config (configType "sql") dispatches to the bar
    // sub-union's Raw SQL variant, not its builder or Unknown variant.
    let json = r#"{
        "displayType": "stacked_bar",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT count() FROM t GROUP BY service"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackBarChartConfig(
            ClickStackBarChartConfig::ClickStackBarRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(r.sql_template, "SELECT count() FROM t GROUP BY service");
            assert_eq!(
                r.display_type,
                ClickStackBarRawSqlChartConfigDisplaytype::Stacked_bar
            );
        }
        other => panic!("expected stacked bar raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_table_raw_sql_variant() {
    // The Raw SQL table config (configType "sql") dispatches to the table
    // sub-union's Raw SQL variant, not its builder or Unknown variant.
    let json = r#"{
        "displayType": "table",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT service, count() FROM t GROUP BY service"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackTableChartConfig(
            ClickStackTableChartConfig::ClickStackTableRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(
                r.sql_template,
                "SELECT service, count() FROM t GROUP BY service"
            );
            assert_eq!(
                r.display_type,
                ClickStackTableRawSqlChartConfigDisplaytype::Table
            );
        }
        other => panic!("expected table raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_number_raw_sql_variant() {
    // The Raw SQL number config (configType "sql") dispatches to the number
    // sub-union's Raw SQL variant, not its builder or Unknown variant.
    let json = r#"{
        "displayType": "number",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT count() FROM t"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackNumberChartConfig(
            ClickStackNumberChartConfig::ClickStackNumberRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(r.sql_template, "SELECT count() FROM t");
            assert_eq!(
                r.display_type,
                ClickStackNumberRawSqlChartConfigDisplaytype::Number
            );
        }
        other => panic!("expected number raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_pie_raw_sql_variant() {
    // The Raw SQL pie config (configType "sql") dispatches to the pie
    // sub-union's Raw SQL variant, not its builder or Unknown variant.
    let json = r#"{
        "displayType": "pie",
        "configType": "sql",
        "connectionId": "conn-1",
        "sqlTemplate": "SELECT service, count() FROM t GROUP BY service"
    }"#;
    let cfg: ClickStackTileConfig = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfig::ClickStackPieChartConfig(
            ClickStackPieChartConfig::ClickStackPieRawSqlChartConfig(r),
        ) => {
            assert_eq!(r.connection_id, "conn-1");
            assert_eq!(
                r.sql_template,
                "SELECT service, count() FROM t GROUP BY service"
            );
            assert_eq!(
                r.display_type,
                ClickStackPieRawSqlChartConfigDisplaytype::Pie
            );
        }
        other => panic!("expected pie raw sql variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_response_categorical_bar_novel_shape_dispatches_to_builder() {
    // A "bar" body with no `configType` dispatches to the categorical bar builder
    // variant, whose all-`Option` response fields surface `None` rather than
    // failing the response.
    let json = r#"{"displayType":"bar","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackCategoricalBarChartConfig(
            ClickStackCategoricalBarChartConfigResponse::ClickStackCategoricalBarBuilderChartConfig(
                b,
            ),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected categorical bar builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_categorical_bar_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the categorical bar
    // sub-union's Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"bar","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackCategoricalBarChartConfig(
                ClickStackCategoricalBarChartConfig::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_tile_config_response_stacked_bar_novel_shape_dispatches_to_builder() {
    // A "stacked_bar" body with no `configType` dispatches to the bar builder
    // variant, whose all-`Option` response fields surface `None` rather than
    // failing the response.
    let json = r#"{"displayType":"stacked_bar","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackBarChartConfig(
            ClickStackBarChartConfigResponse::ClickStackBarBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected bar builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_stacked_bar_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the bar sub-union's
    // Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"stacked_bar","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackBarChartConfig(ClickStackBarChartConfig::Unknown(_))
        )
    });
}

#[test]
fn clickstack_tile_config_response_table_novel_shape_dispatches_to_builder() {
    // A "table" body with no `configType` dispatches to the table builder
    // variant, whose all-`Option` response fields surface `None` rather than
    // failing the response.
    let json = r#"{"displayType":"table","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackTableChartConfig(
            ClickStackTableChartConfigResponse::ClickStackTableBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected table builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_table_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the table sub-union's
    // Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"table","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackTableChartConfig(ClickStackTableChartConfig::Unknown(
                _
            ))
        )
    });
}

#[test]
fn clickstack_tile_config_response_number_novel_shape_dispatches_to_builder() {
    // A "number" body with no `configType` dispatches to the number builder
    // variant, whose all-`Option` response fields surface `None` rather than
    // failing the response.
    let json = r#"{"displayType":"number","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackNumberChartConfig(
            ClickStackNumberChartConfigResponse::ClickStackNumberBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected number builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_number_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the number sub-union's
    // Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"number","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackNumberChartConfig(
                ClickStackNumberChartConfig::Unknown(_)
            )
        )
    });
}

#[test]
fn clickstack_tile_config_response_pie_novel_shape_dispatches_to_builder() {
    // A "pie" body with no `configType` dispatches to the pie builder
    // variant, whose all-`Option` response fields surface `None` rather than
    // failing the response.
    let json = r#"{"displayType":"pie","somethingNew":true}"#;
    let cfg: ClickStackTileConfigResponse = serde_json::from_str(json).unwrap();
    match cfg {
        ClickStackTileConfigResponse::ClickStackPieChartConfig(
            ClickStackPieChartConfigResponse::ClickStackPieBuilderChartConfig(b),
        ) => {
            assert_eq!(b.source_id, None);
            assert_eq!(b.select, None);
        }
        other => panic!("expected pie builder variant, got {other}"),
    }
}

#[test]
fn clickstack_tile_config_pie_unrecognized_config_type_falls_to_sub_union_unknown() {
    // An unrecognized *string* `configType` reaches the pie sub-union's
    // Unknown(Value); it round-trips losslessly.
    let json = r#"{"displayType":"pie","configType":"future"}"#;
    assert_unknown_variant_round_trips(json, |cfg: &ClickStackTileConfig| {
        matches!(
            cfg,
            ClickStackTileConfig::ClickStackPieChartConfig(ClickStackPieChartConfig::Unknown(_))
        )
    });
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
    assert_eq!(conn.id.as_deref(), Some("507f1f77bcf86cd799439012"));
    assert_eq!(conn.name.as_deref(), Some("Production ClickHouse"));
    assert_eq!(
        conn.host.as_deref(),
        Some("https://clickhouse.example.com:8443")
    );
    assert_eq!(conn.username.as_deref(), Some("default"));
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
    assert_eq!(role.id.as_deref(), Some("role-1"));
    assert_eq!(role.name.as_deref(), Some("Deploy Bot"));
    assert_eq!(role.is_predefined, Some(false));
    let permissions = role.permissions.as_deref().expect("permissions present");
    assert_eq!(permissions.len(), 1);

    let perm = &permissions[0];
    assert_eq!(perm.action.as_deref(), Some("read"));
    assert_eq!(perm.subject.as_deref(), Some("dashboard"));
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
        id: Some("role-1".to_string()),
        name: Some("Deploy Bot".to_string()),
        is_predefined: Some(false),
        permissions: Some(vec![ClickStackCASLPermissionResponse {
            action: Some("manage".to_string()),
            subject: Some("all".to_string()),
            conditions: Some(serde_json::json!({ "teamId": "team-1" })),
            ..Default::default()
        }]),
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
    assert_eq!(search.id.as_deref(), Some("507f1f77bcf86cd799439011"));
    assert_eq!(search.name.as_deref(), Some("Production Errors"));
    assert_eq!(
        search.source_id.as_deref(),
        Some("507f1f77bcf86cd799439012")
    );
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
        filters[0].condition.as_deref(),
        Some("ServiceName IN ('checkout', 'payments')")
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
fn deserialize_clickstack_webhook_dispatches_slack() {
    let json = r#"{
        "id": "webhook-1",
        "name": "Slack Alerts",
        "service": "slack",
        "url": "https://hooks.slack.com/services/T/B/X",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let w: ClickStackWebhook = serde_json::from_str(json).unwrap();
    match w {
        ClickStackWebhook::ClickStackSlackWebhook(s) => {
            assert_eq!(s.id.as_deref(), Some("webhook-1"));
            assert_eq!(s.name.as_deref(), Some("Slack Alerts"));
        }
        other => panic!("expected Slack webhook variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_webhook_dispatches_incidentio() {
    let json = r#"{
        "id": "webhook-2",
        "name": "Incident Alerts",
        "service": "incidentio",
        "url": "https://api.incident.io/v2/alert_events/http/abc",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let w: ClickStackWebhook = serde_json::from_str(json).unwrap();
    match w {
        ClickStackWebhook::ClickStackIncidentIOWebhook(i) => {
            assert_eq!(i.id.as_deref(), Some("webhook-2"));
            assert_eq!(i.name.as_deref(), Some("Incident Alerts"));
        }
        other => panic!("expected IncidentIO webhook variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_webhook_dispatches_generic_preserves_body() {
    let json = r#"{
        "id": "webhook-3",
        "name": "Generic Alerts",
        "service": "generic",
        "url": "https://example.com/hook",
        "body": "{\"text\": \"{{ message }}\"}",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let w: ClickStackWebhook = serde_json::from_str(json).unwrap();
    match w {
        ClickStackWebhook::ClickStackGenericWebhook(g) => {
            assert_eq!(g.id.as_deref(), Some("webhook-3"));
            assert_eq!(g.body.as_deref(), Some("{\"text\": \"{{ message }}\"}"));
        }
        other => panic!("expected Generic webhook variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_webhook_dispatches_slack_api() {
    let json = r#"{
        "id": "webhook-4",
        "name": "Slack API Alerts",
        "service": "slack_api",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let w: ClickStackWebhook = serde_json::from_str(json).unwrap();
    match w {
        ClickStackWebhook::ClickStackSlackAPIWebhook(s) => {
            assert_eq!(s.id.as_deref(), Some("webhook-4"));
            assert_eq!(s.name.as_deref(), Some("Slack API Alerts"));
        }
        other => panic!("expected SlackAPI webhook variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_webhook_dispatches_pagerduty_api() {
    let json = r#"{
        "id": "webhook-5",
        "name": "PagerDuty Alerts",
        "service": "pagerduty_api",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    let w: ClickStackWebhook = serde_json::from_str(json).unwrap();
    match w {
        ClickStackWebhook::ClickStackPagerDutyAPIWebhook(p) => {
            assert_eq!(p.id.as_deref(), Some("webhook-5"));
            assert_eq!(p.name.as_deref(), Some("PagerDuty Alerts"));
        }
        other => panic!("expected PagerDutyAPI webhook variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_webhook_unknown_service_round_trips() {
    let json = r#"{
        "id": "webhook-6",
        "name": "Future Alerts",
        "service": "future_service",
        "extraField": "kept",
        "createdAt": "2025-01-01T00:00:00.000Z",
        "updatedAt": "2025-06-15T10:30:00.000Z"
    }"#;
    assert_unknown_variant_round_trips(json, |w: &ClickStackWebhook| {
        matches!(w, ClickStackWebhook::Unknown(_))
    });
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
    assert_eq!(resp.valid, Some(false));
    let errors = resp.errors.as_ref().expect("errors should populate");
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].path.as_deref(), Some("tiles.0.config"));
    assert_eq!(errors[0].message.as_deref(), Some("Required"));
    assert_eq!(errors[1].path.as_deref(), Some(""));

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
    assert_eq!(resp.valid, Some(true));
    assert_eq!(resp.errors.as_deref(), Some(&[][..]));
    assert_eq!(resp.normalized, None);

    // An explicit `null` lands as `None` and, like every absent response
    // field, is omitted on the way out rather than re-emitted as `null`.
    let v = serde_json::to_value(&resp).unwrap();
    assert!(
        v.get("normalized").is_none(),
        "absent response fields must be omitted from --json output"
    );
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
        Some(OrganizationQuotaQuotacode::Replicas_per_warehouse)
    );
    assert_eq!(quota.scope, Some(OrganizationQuotaScope::Warehouse));
    assert_eq!(quota.value, Some(20));
    assert_eq!(quota.usage, Some(3));
    assert_eq!(quota.adjustable, Some(true));

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
        Some(OrganizationQuotaQuotacode::Services_per_organization)
    );
    assert_eq!(quota.scope, Some(OrganizationQuotaScope::Organization));
    assert_eq!(quota.usage, None);

    let v = serde_json::to_value(&quota).unwrap();
    assert!(v.get("usage").is_none(), "usage must be omitted when None");
}

#[test]
fn organization_quota_tolerates_explicit_null_fields() {
    // `OrganizationQuota` is response-only, so every field is `Option<T>` and an
    // explicit `null` lands as `None` exactly like a dropped key — the case the
    // superseded `#[serde(default)]` policy never covered.
    let quota: OrganizationQuota = serde_json::from_str(
        r#"{"quotaCode":null,"name":null,"description":null,"scope":null,
            "value":null,"usage":null,"adjustable":null}"#,
    )
    .unwrap();
    assert_eq!(quota, OrganizationQuota::default());

    // Absence is omitted on the way out, never re-emitted as `null`.
    assert_eq!(
        serde_json::to_value(&quota).unwrap(),
        serde_json::json!({}),
        "absent response fields must be omitted from --json output"
    );
}

#[test]
fn scim_request_models_stay_strict() {
    // The spec defines the SCIM schemas but no SCIM path, so the family is in
    // neither direction's tree and stays strict (see the comment above
    // `ScimEnterpriseManager` in models.rs and
    // `scim_models_are_outside_the_response_tree` in spec_coverage_test.rs).
    // Required fields are therefore `T` and a dropped one is a hard error.
    let err = serde_json::from_str::<ScimGroupPostRequest>(r#"{"schemas":[]}"#).unwrap_err();
    assert!(
        err.to_string().contains("displayName"),
        "unexpected error: {err}"
    );

    // Absent optional fields are still omitted rather than sent as `null`.
    let group = ScimGroupPostRequest {
        display_name: "Engineering".to_string(),
        external_id: None,
        members: None,
        schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
    };
    let v = serde_json::to_value(&group).unwrap();
    assert_eq!(v["displayName"], "Engineering");
    assert_eq!(
        v["schemas"][0],
        "urn:ietf:params:scim:schemas:core:2.0:Group"
    );
    assert!(v.get("externalId").is_none());
    assert!(v.get("members").is_none());
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

#[test]
fn deserialize_clickstack_dashboard_chart_series_dispatches_time() {
    // The manual `type`-keyed dispatch routes "time" to the time-series variant.
    let json = r#"{
        "aggFn": "count",
        "groupBy": ["service"],
        "sourceId": "src-1",
        "type": "time",
        "where": "x = 1",
        "whereLanguage": "sql"
    }"#;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::ClickStackTimeChartSeries(s) => {
            assert_eq!(s.r#type, ClickStackTimeChartSeriesType::Time);
            assert_eq!(s.agg_fn, ClickStackTimeChartSeriesAggfn::Count);
        }
        other => panic!("expected time variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_dispatches_table() {
    // Regression: Time and Table share every required field (aggFn, groupBy,
    // sourceId, type, where, whereLanguage), so before manual discriminator
    // dispatch a `type: "table"` payload greedily resolved to the Time variant
    // and silently dropped table-only fields like `sortOrder`. The union now
    // routes on the `type` discriminator and preserves `sortOrder`.
    let json = r#"{
        "aggFn": "count",
        "groupBy": ["service"],
        "sortOrder": "desc",
        "sourceId": "src-2",
        "type": "table",
        "where": "y = 2",
        "whereLanguage": "sql"
    }"#;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::ClickStackTableChartSeries(s) => {
            assert_eq!(s.r#type, ClickStackTableChartSeriesType::Table);
            assert_eq!(
                s.sort_order,
                Some(ClickStackTableChartSeriesSortorder::Desc)
            );
        }
        other => panic!("expected table variant, got {other}"),
    }
    // The table-only `sortOrder` survives the round-trip through the union.
    let v = serde_json::to_value(&series).unwrap();
    assert_eq!(v["type"], "table");
    assert_eq!(v["sortOrder"], "desc");
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_dispatches_number() {
    // The manual `type`-keyed dispatch routes "number" to the number-series variant.
    let json = r#"{
        "aggFn": "count",
        "sourceId": "src-3",
        "type": "number",
        "where": "z = 3",
        "whereLanguage": "sql"
    }"#;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::ClickStackNumberChartSeries(s) => {
            assert_eq!(s.r#type, ClickStackNumberChartSeriesType::Number);
        }
        other => panic!("expected number variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_dispatches_search() {
    // The manual `type`-keyed dispatch routes "search" to the search-series variant.
    let json = r#"{
        "fields": ["message"],
        "sourceId": "src-4",
        "type": "search",
        "where": "w = 4",
        "whereLanguage": "sql"
    }"#;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::ClickStackSearchChartSeries(s) => {
            assert_eq!(s.r#type, ClickStackSearchChartSeriesType::Search);
            assert_eq!(s.fields, vec!["message".to_string()]);
        }
        other => panic!("expected search variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_dispatches_markdown() {
    // The manual `type`-keyed dispatch routes "markdown" to the markdown-series variant.
    let json = r##"{"content": "# Title", "type": "markdown"}"##;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::ClickStackMarkdownChartSeries(s) => {
            assert_eq!(s.r#type, ClickStackMarkdownChartSeriesType::Markdown);
            assert_eq!(s.content, "# Title");
        }
        other => panic!("expected markdown variant, got {other}"),
    }
}

#[test]
fn deserialize_clickstack_dashboard_chart_series_unknown_type_round_trip() {
    // An unrecognized `type` discriminator falls back to the Unknown variant,
    // which stores the raw JSON so it round-trips faithfully.
    let json = r#"{"type":"heatmap","payload":{"nested":[1,2,3]}}"#;
    let series: ClickStackDashboardChartSeries = serde_json::from_str(json).unwrap();
    match &series {
        ClickStackDashboardChartSeries::Unknown(v) => {
            assert_eq!(v["type"], "heatmap");
            assert_eq!(v["payload"]["nested"][2], 3);
        }
        other => panic!("expected unknown variant, got {other}"),
    }
    let expected: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_value(&series).unwrap(), expected);
}
