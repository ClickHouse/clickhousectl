use clickhouse_cloud_api::models::*;

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
    assert_eq!(svc.num_replicas, Some(3));
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
        num_replicas: Some(3),
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
        min_replicas: Some(1),
        max_replicas: Some(5),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["autoscalingMode"], "horizontal");
    assert_eq!(json["minReplicas"], 1);
    assert_eq!(json["maxReplicas"], 5);

    // Omitted entirely when unset — mutually exclusive with the vertical
    // scaling fields, so they must not serialize as null/defaults.
    let json = serde_json::to_value(ServicePostRequest::default()).unwrap();
    assert!(json.get("minReplicas").is_none());
    assert!(json.get("maxReplicas").is_none());
    assert!(json.get("autoscalingMode").is_none());
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
fn deserialize_service_endpoint() {
    let json = r#"{
        "protocol": "nativesecure",
        "host": "abc123.clickhouse.cloud",
        "port": 9440
    }"#;
    let ep: ServiceEndpoint = serde_json::from_str(json).unwrap();
    assert_eq!(ep.protocol, Some(ServiceEndpointProtocol::Nativesecure));
    assert_eq!(ep.host.as_deref(), Some("abc123.clickhouse.cloud"));
    assert_eq!(ep.port, Some(9440));
}

#[test]
fn api_integer_fields_round_trip_as_json_integers() {
    let response: ApiResponse<Service> = serde_json::from_value(serde_json::json!({
        "status": 200,
        "result": {
            "numReplicas": 3,
            "minReplicas": 1,
            "maxReplicas": 5,
            "endpoints": [{"port": 9440}]
        }
    }))
    .unwrap();

    let rendered = serde_json::to_string(&response).unwrap();
    assert!(
        !rendered.contains(".0"),
        "unexpected float JSON: {rendered}"
    );
    let json = serde_json::to_value(response).unwrap();
    assert!(json["status"].is_i64());
    assert!(json["result"]["numReplicas"].is_i64());
    assert!(json["result"]["minReplicas"].is_i64());
    assert!(json["result"]["maxReplicas"].is_i64());
    assert!(json["result"]["endpoints"][0]["port"].is_i64());
    assert_eq!(json["status"], 200);
    assert_eq!(json["result"]["numReplicas"], 3);
    assert_eq!(json["result"]["endpoints"][0]["port"], 9440);
}

#[test]
fn api_integer_fields_tolerate_missing_and_null_values() {
    let missing: ApiResponse<Service> = serde_json::from_str("{}").unwrap();
    assert_eq!(
        serde_json::to_value(missing).unwrap(),
        serde_json::json!({})
    );
    let missing_endpoint: ServiceEndpoint = serde_json::from_str("{}").unwrap();
    assert_eq!(missing_endpoint.port, None);

    let nulled: ApiResponse<Service> = serde_json::from_value(serde_json::json!({
        "status": null,
        "result": {
            "numReplicas": null,
            "minReplicas": null,
            "maxReplicas": null,
            "endpoints": [{"port": null}]
        }
    }))
    .unwrap();
    assert_eq!(nulled.status, None);
    let service = nulled.result.as_ref().unwrap();
    assert_eq!(service.num_replicas, None);
    assert_eq!(service.min_replicas, None);
    assert_eq!(service.max_replicas, None);
    assert_eq!(service.endpoints.as_ref().unwrap()[0].port, None);
    assert_eq!(
        serde_json::to_value(nulled).unwrap(),
        serde_json::json!({"result": {"endpoints": [{}]}})
    );
}

#[test]
fn de_facto_integer_fields_accept_integral_float_syntax() {
    let response: ApiResponse<Service> = serde_json::from_value(serde_json::json!({
        "status": 200.0,
        "result": {"endpoints": [{"port": 9440.0}]}
    }))
    .unwrap();
    assert_eq!(response.status, Some(200));
    assert_eq!(
        response
            .result
            .as_ref()
            .unwrap()
            .endpoints
            .as_ref()
            .unwrap()[0]
            .port,
        Some(9440)
    );
    assert_eq!(
        serde_json::to_value(response).unwrap(),
        serde_json::json!({"status": 200, "result": {"endpoints": [{"port": 9440}]}})
    );

    assert!(
        serde_json::from_value::<ApiResponse<Service>>(serde_json::json!({"status": 200.5}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<ServiceEndpoint>(serde_json::json!({"port": 9440.5})).is_err()
    );
    assert!(
        serde_json::from_str::<ApiResponse<Service>>(r#"{"status":9007199254740993.0}"#).is_err()
    );
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
        num_replicas: Some(5),
        min_replicas: None,
        max_replicas: None,
        min_replica_memory_gb: Some(16.0),
        max_replica_memory_gb: Some(64.0),
        idle_scaling: Some(true),
        idle_timeout_minutes: Some(10.0),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["numReplicas"], 5);
    assert_eq!(json["minReplicaMemoryGb"], 16.0);
    assert_eq!(json["maxReplicaMemoryGb"], 64.0);
    assert_eq!(json["idleScaling"], true);
    assert_eq!(json["idleTimeoutMinutes"], 10.0);
}

#[test]
fn serialize_service_scaling_patch_request() {
    let req = ServiceScalingPatchRequest {
        num_replicas: Some(3),
        #[cfg(feature = "deprecated-fields")]
        min_total_memory_gb: Some(24.0),
        #[cfg(feature = "deprecated-fields")]
        max_total_memory_gb: Some(48.0),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["numReplicas"], 3);
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
fn serialize_instance_query_endpoint_post_request() {
    let req = InstanceServiceQueryApiEndpointsPostRequest {
        allowed_origins: Some("https://example.com".to_string()),
        roles: vec![QueryEndpointRole::SqlConsoleReadOnly],
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["allowedOrigins"], "https://example.com");
    assert_eq!(json["roles"], serde_json::json!(["sql_console_read_only"]));
}

#[test]
fn deserialize_query_endpoint_roles_tolerates_unknown_values() {
    let endpoint: ServiceQueryAPIEndpoint = serde_json::from_value(serde_json::json!({
        "roles": ["sql_console_admin", "future_role"]
    }))
    .unwrap();

    assert_eq!(
        endpoint.roles,
        Some(vec![
            QueryEndpointRole::SqlConsoleAdmin,
            QueryEndpointRole::Unknown("future_role".to_string()),
        ])
    );
}

#[test]
fn serialize_servic_private_endpointe_post_request() {
    let req = ServicPrivateEndpointePostRequest {
        id: "vpce-abc".to_string(),
        description: Some("My PE".to_string()),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["id"], "vpce-abc");
    assert_eq!(json["description"], "My PE");
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
fn service_ignores_extra_fields() {
    let json = r#"{"name":"svc","state":"running","futureField":"v2","nested":{"a":1}}"#;
    let svc: Service = serde_json::from_str(json).unwrap();
    assert_eq!(svc.name.as_deref(), Some("svc"));
    assert_eq!(svc.state, Some(ServiceState::Running));
}

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
        duration: Some(UpgradeWindowDuration::SixHours),
        start_hour_utc: Some(UpgradeWindowStartHourUtc::Hour6),
        weekday: Some(2),
    })
    .unwrap();
    assert_eq!(request.start_hour_utc, UpgradeWindowStartHourUtc::Hour6);
    assert_eq!(request.weekday, 2);

    let missing = UpgradeWindowPutRequest::try_from(UpgradeWindow::default()).unwrap_err();
    assert_eq!(missing.fields(), ["startHourUtc", "weekday"]);
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

// ===========================================================================
// Upgrade window (issue #203 drift)
// ===========================================================================

#[test]
fn deserialize_upgrade_window() {
    let json = r#"{
        "weekday": 2,
        "startHourUtc": 6,
        "duration": 6
    }"#;
    let w: UpgradeWindow = serde_json::from_str(json).unwrap();
    assert_eq!(w.weekday, Some(2));
    assert_eq!(w.start_hour_utc, Some(UpgradeWindowStartHourUtc::Hour6));
    assert_eq!(w.duration, Some(UpgradeWindowDuration::SixHours));

    let round_tripped = serde_json::to_value(&w).unwrap();
    assert_eq!(round_tripped["startHourUtc"], 6);
    assert_eq!(round_tripped["weekday"], 2);
    assert_eq!(round_tripped["duration"], 6);
}

#[test]
fn upgrade_window_numeric_enums_round_trip_every_known_value() {
    for (wire, value) in [
        (0, UpgradeWindowStartHourUtc::Hour0),
        (6, UpgradeWindowStartHourUtc::Hour6),
        (12, UpgradeWindowStartHourUtc::Hour12),
        (18, UpgradeWindowStartHourUtc::Hour18),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<UpgradeWindowStartHourUtc>(wire.into()).unwrap(),
            value
        );
    }

    assert_eq!(
        serde_json::to_value(UpgradeWindowDuration::SixHours).unwrap(),
        6
    );
    assert_eq!(
        serde_json::from_value::<UpgradeWindowDuration>(serde_json::json!(6)).unwrap(),
        UpgradeWindowDuration::SixHours
    );
}

#[test]
fn deserialize_upgrade_window_tolerates_unknown_numeric_values() {
    let window: UpgradeWindow = serde_json::from_value(serde_json::json!({
        "startHourUtc": 3,
        "duration": 21600
    }))
    .unwrap();

    assert_eq!(
        window.start_hour_utc,
        Some(UpgradeWindowStartHourUtc::Unknown(3))
    );
    assert_eq!(window.duration, Some(UpgradeWindowDuration::Unknown(21600)));
    assert_eq!(
        serde_json::to_value(window).unwrap(),
        serde_json::json!({"startHourUtc": 3, "duration": 21600})
    );
}

#[test]
fn serialize_upgrade_window_put_request() {
    let req = UpgradeWindowPutRequest {
        weekday: 5,
        start_hour_utc: UpgradeWindowStartHourUtc::Hour18,
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["weekday"], 5);
    assert_eq!(v["startHourUtc"], 18);
    assert!(v.get("start_hour_utc").is_none());
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
fn service_region_ca_central_1() {
    let parsed: ServiceRegion = serde_json::from_str("\"ca-central-1\"").unwrap();
    assert_eq!(parsed, ServiceRegion::Ca_central_1);
    assert_eq!(parsed.to_string(), "ca-central-1");
}

#[test]
fn service_clickhouse_settings_typed_request_preserves_native_values() {
    let settings = serde_json::json!({
        "compatibility": "26.2", "max_query_size": 262146,
        "mark_cache_ram_ratio": "0.5", "signed": -42,
        "unsigned": u64::MAX
    });
    let map: ServiceClickhouseSettingsMap = serde_json::from_value(settings.clone()).unwrap();
    assert_eq!(
        map["compatibility"],
        ServiceClickhouseSettingValue::String("26.2".into())
    );
    assert_eq!(
        map["max_query_size"],
        ServiceClickhouseSettingValue::Integer(262146)
    );
    assert_eq!(
        map["unsigned"],
        ServiceClickhouseSettingValue::UnsignedInteger(u64::MAX)
    );
    let request = ServiceClickhouseSettingsPatchRequest {
        settings: Some(map),
    };
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({"settings": settings})
    );
    let empty = ServiceClickhouseSettingsPatchRequest {
        settings: Some(ServiceClickhouseSettingsMap::new()),
    };
    assert!(serde_json::to_value(empty).is_err());
}

#[test]
fn service_clickhouse_settings_typed_request_rejects_unsupported_value_types() {
    for value in [
        serde_json::json!(true),
        serde_json::Value::Null,
        serde_json::json!(1.5),
        serde_json::json!({"nested": true}),
        serde_json::json!([1]),
    ] {
        assert!(
            serde_json::from_value::<ServiceClickhouseSettingsMap>(
                serde_json::json!({"setting": value})
            )
            .is_err()
        );
    }
}

#[test]
fn service_clickhouse_settings_preserves_dynamic_json_values() {
    let settings = serde_json::json!({
        "string": "26.2", "integer": 262144, "boolean": false, "null": null,
        "array": [1, "two"], "future": {"nested": true}
    });
    let request: ServiceClickhouseSettingsPatchRequest =
        serde_json::from_value(serde_json::json!({"settings": settings})).unwrap();
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({"settings": settings})
    );
    let response: ServiceClickhouseSettingsPatchResponse =
        serde_json::from_value(serde_json::json!({"settings": settings})).unwrap();
    assert_eq!(
        serde_json::to_value(response).unwrap(),
        serde_json::json!({"settings": settings})
    );
    for sparse in [
        serde_json::json!({}),
        serde_json::json!({"settings": null, "warnings": null}),
    ] {
        let response: ServiceClickhouseSettingsPatchResponse =
            serde_json::from_value(sparse).unwrap();
        assert!(response.settings.is_none());
        assert!(response.warnings.is_none());
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({})
        );
    }
    let empty = ServiceClickhouseSettingsPatchRequest {
        settings: Some(std::collections::BTreeMap::<String, serde_json::Value>::new()),
    };
    assert!(serde_json::to_value(empty).is_err());
}

#[test]
fn service_clickhouse_setting_preserves_json_value_types() {
    for value in [
        serde_json::json!(262146),
        serde_json::json!(18446744073709551615_u64),
        serde_json::json!(-42),
        serde_json::json!(1.25),
        serde_json::json!("26.2"),
        serde_json::json!("262146"),
        serde_json::json!(false),
        serde_json::json!({"future": [1, true]}),
    ] {
        let wire = serde_json::json!({"name": "setting", "value": value});
        let setting: ServiceClickhouseSetting = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(setting.value.as_ref(), Some(&value));
        assert_eq!(serde_json::to_value(&setting).unwrap(), wire);
        let list_wire = serde_json::json!({"settings": [wire]});
        let list: ServiceClickhouseSettingsList =
            serde_json::from_value(list_wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(list).unwrap(), list_wire);
    }
}

#[test]
fn service_clickhouse_setting_missing_and_null_values_remain_absent() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({"name": null, "value": null}),
    ] {
        let setting: ServiceClickhouseSetting = serde_json::from_value(wire).unwrap();
        assert_eq!(setting.name, None);
        assert_eq!(setting.value, None);
        assert_eq!(
            serde_json::to_value(setting).unwrap(),
            serde_json::json!({})
        );
    }
}

fn sample_backup_encryption_config() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "restore_key_pairs": [
            {
                "customer_managed_encryption_key": {
                    "aws_kms_key_arn": "arn:aws:kms:us-east-1:111122223333:key/1234abcd-12ab-34cd-56ef-1234567890ab"
                },
                "encrypted_dek": "d3JhcHBlZA==",
                "future_field": null
            }
        ],
        "provider_extension": {"nested": [1, 2.5, true, "x"]}
    })
}

#[test]
fn backup_encryption_config_round_trips_opaque_nested_contents() {
    let file = sample_backup_encryption_config();
    let config: BackupEncryptionConfig = serde_json::from_value(file.clone()).unwrap();
    assert_eq!(config["schema_version"], 1);
    assert_eq!(
        config["restore_key_pairs"][0]["customer_managed_encryption_key"]["aws_kms_key_arn"],
        "arn:aws:kms:us-east-1:111122223333:key/1234abcd-12ab-34cd-56ef-1234567890ab"
    );
    assert_eq!(serde_json::to_value(&config).unwrap(), file);

    // The file is an object; a non-object is not a valid config.
    assert!(serde_json::from_value::<BackupEncryptionConfig>(serde_json::json!([1])).is_err());
}

#[test]
fn service_post_request_backup_encryption_config_serialization() {
    let base = ServicePostRequest {
        name: "restored".to_string(),
        provider: ServicePostRequestProvider::Aws,
        region: ServicePostRequestRegion::Us_east_1,
        backup_id: Some(uuid::Uuid::nil()),
        ..Default::default()
    };
    let json = serde_json::to_value(&base).unwrap();
    assert!(json.get("backupEncryptionConfig").is_none());

    let file = sample_backup_encryption_config();
    let req = ServicePostRequest {
        backup_encryption_config: Some(serde_json::from_value(file.clone()).unwrap()),
        ..base
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["backupEncryptionConfig"], file);
    assert_eq!(json["backupId"], uuid::Uuid::nil().to_string());

    let parsed: ServicePostRequest = serde_json::from_value(json).unwrap();
    assert_eq!(parsed, req);
}

#[test]
fn replica_aware_routing_missing_and_null_fields_are_none() {
    let missing: ReplicaAwareRouting = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, ReplicaAwareRouting::default());
    assert_eq!(
        serde_json::to_value(&missing).unwrap(),
        serde_json::json!({})
    );

    let null: ReplicaAwareRouting = serde_json::from_value(serde_json::json!({
        "httpEnabled": null, "nativeEnabled": null
    }))
    .unwrap();
    assert_eq!(null, ReplicaAwareRouting::default());
}

#[test]
fn serialize_replica_aware_routing_patch_request() {
    assert_eq!(
        serde_json::to_value(ReplicaAwareRoutingPatchRequest::default()).unwrap(),
        serde_json::json!({})
    );
    assert_eq!(
        serde_json::to_value(ReplicaAwareRoutingPatchRequest {
            http_enabled: Some(false),
            native_enabled: None,
        })
        .unwrap(),
        serde_json::json!({"httpEnabled": false})
    );
    assert_eq!(
        serde_json::to_value(ReplicaAwareRoutingPatchRequest {
            http_enabled: Some(true),
            native_enabled: Some(false),
        })
        .unwrap(),
        serde_json::json!({"httpEnabled": true, "nativeEnabled": false})
    );
}
