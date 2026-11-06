use clickhouse_cloud_api::models::*;

#[test]
fn clickpipe_new_request_fields_preserve_omission_and_wire_names() {
    let mut request = ClickPipePostRequest::default();
    assert!(
        serde_json::to_value(&request)
            .unwrap()
            .get("startPaused")
            .is_none()
    );
    request.start_paused = Some(true);
    assert_eq!(serde_json::to_value(&request).unwrap()["startPaused"], true);

    let mut table = ClickPipeDestinationTableDefinition::default();
    assert!(serde_json::to_value(&table).unwrap().get("ttl").is_none());
    table.ttl = "event_time + INTERVAL 30 DAY".to_string();
    let serialized = serde_json::to_value(&table).unwrap();
    assert_eq!(serialized["ttl"], table.ttl);

    let mut settings = ClickPipeMongoDBPipeSettings::default();
    assert!(
        serde_json::to_value(&settings)
            .unwrap()
            .get("initialLoadParallelism")
            .is_none()
    );
    settings.initial_load_parallelism = Some(4);
    let serialized = serde_json::to_value(&settings).unwrap();
    assert_eq!(serialized["initialLoadParallelism"], 4);
    assert_eq!(
        serde_json::from_value::<ClickPipeMongoDBPipeSettings>(serialized).unwrap(),
        settings
    );
}

#[test]
fn clickpipe_new_response_fields_tolerate_missing_and_null() {
    for value in [serde_json::json!({}), serde_json::json!({"ttl": null})] {
        let response: ClickPipeDestinationTableDefinitionResponse =
            serde_json::from_value(value).unwrap();
        assert_eq!(response.ttl, None);
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({})
        );
    }
    for value in [
        serde_json::json!({}),
        serde_json::json!({"initialLoadParallelism": null}),
    ] {
        let response: ClickPipeMongoDBPipeSettingsResponse = serde_json::from_value(value).unwrap();
        assert_eq!(response.initial_load_parallelism, None);
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({})
        );
    }
    let table: ClickPipeDestinationTableDefinitionResponse =
        serde_json::from_value(serde_json::json!({"ttl": "event_time + INTERVAL 30 DAY"})).unwrap();
    assert_eq!(table.ttl.as_deref(), Some("event_time + INTERVAL 30 DAY"));
    let settings: ClickPipeMongoDBPipeSettingsResponse =
        serde_json::from_value(serde_json::json!({"initialLoadParallelism": 4})).unwrap();
    assert_eq!(settings.initial_load_parallelism, Some(4));
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
fn deserialize_clickpipe_settings() {
    let json = r#"{
        "streaming_max_insert_wait_ms": 5000,
        "object_storage_concurrency": null,
        "clickhouse_max_threads": 4,
        "kafka_read_committed": true
    }"#;
    let settings: ClickPipeSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.streaming_max_insert_wait_ms, Some(5000));
    assert_eq!(settings.object_storage_concurrency, None);
    assert_eq!(settings.clickhouse_max_threads, Some(4));
    assert_eq!(settings.kafka_read_committed, Some(true));
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
        "Degraded",
    ];
    for s in states {
        let parsed: ClickPipeState = serde_json::from_str(&format!(r#""{s}""#)).unwrap();
        let serialized = serde_json::to_string(&parsed).unwrap();
        assert_eq!(serialized, format!(r#""{s}""#));
    }
}

#[test]
fn postgres_table_mapping_table_engines_round_trip_and_match_values() {
    // The CLI validates `--table-mapping-json`'s `tableEngine` against
    // `VALUES`, so the const must stay equal to the wire values the enum
    // actually accepts.
    assert_eq!(
        ClickPipePostgresPipeTableMappingTableengine::VALUES,
        &["MergeTree", "ReplacingMergeTree", "Null"]
    );
    for value in ClickPipePostgresPipeTableMappingTableengine::VALUES {
        let parsed: ClickPipePostgresPipeTableMappingTableengine =
            serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(
                parsed,
                ClickPipePostgresPipeTableMappingTableengine::Unknown(_)
            ),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
        assert_eq!(
            serde_json::to_string(&parsed).unwrap(),
            format!(r#""{value}""#)
        );
    }
}

#[test]
fn database_table_mapping_table_engines_round_trip_and_match_values() {
    macro_rules! assert_values {
        ($engine:ty) => {{
            assert_eq!(
                <$engine>::VALUES,
                &["MergeTree", "ReplacingMergeTree", "Null"]
            );
            for value in <$engine>::VALUES {
                let parsed: $engine = serde_json::from_str(&format!(r#""{value}""#)).unwrap();
                assert_eq!(parsed.to_string(), *value);
                assert_eq!(
                    serde_json::to_string(&parsed).unwrap(),
                    format!(r#""{value}""#)
                );
            }
        }};
    }

    assert_values!(ClickPipeMySQLPipeTableMappingTableengine);
    assert_values!(ClickPipeMongoDBPipeTableMappingTableengine);
    assert_values!(ClickPipeBigQueryPipeTableMappingTableengine);
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
fn serialize_clickpipe_mutate_postgres_source_omits_absent_credentials() {
    // IAM_ROLE authentication has no username or password: `iamRole` is the
    // whole credential, so the `credentials` object must not reach the wire.
    let iam_role = ClickPipeMutatePostgresSource {
        authentication: ClickPipeMutatePostgresSourceAuthentication::IAM_ROLE,
        credentials: None,
        iam_role: Some("arn:aws:iam::123456789012:role/clickpipe".to_string()),
        host: "postgres.example".to_string(),
        port: 5432,
        database: "source-db".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_value(&iam_role).unwrap();
    assert!(
        json.get("credentials").is_none(),
        "credentials must be omitted, got {json}"
    );
    assert_eq!(json["iamRole"], "arn:aws:iam::123456789012:role/clickpipe");
    assert_eq!(json["authentication"], "IAM_ROLE");

    // Basic auth still sends the pair verbatim.
    let basic = ClickPipeMutatePostgresSource {
        authentication: ClickPipeMutatePostgresSourceAuthentication::Basic,
        credentials: Some(PLAIN {
            username: "user".to_string(),
            password: "secret".to_string(),
        }),
        ..iam_role.clone()
    };
    let json = serde_json::to_value(&basic).unwrap();
    assert_eq!(json["credentials"]["username"], "user");
    assert_eq!(json["credentials"]["password"], "secret");
}

#[test]
fn clickpipe_ignores_extra_fields() {
    let json = r#"{"name":"pipe","state":"Running","newFeatureFlag":true}"#;
    let pipe: ClickPipe = serde_json::from_str(json).unwrap();
    assert_eq!(pipe.name.as_deref(), Some("pipe"));
    assert_eq!(pipe.state, Some(ClickPipeState::Running));
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
fn schema_discovery_response_tolerates_dropped_and_null_meta() {
    let dropped: ClickPipeSchemaDiscoveryResponse = serde_json::from_str("{}").unwrap();
    let nulled: ClickPipeSchemaDiscoveryResponse =
        serde_json::from_str(r#"{"fields":null,"meta":null}"#).unwrap();

    assert_eq!(dropped, ClickPipeSchemaDiscoveryResponse::default());
    assert_eq!(nulled, ClickPipeSchemaDiscoveryResponse::default());
    assert_eq!(serde_json::to_value(nulled).unwrap(), serde_json::json!({}));
}

#[test]
fn reverse_private_endpoint_dns_targets_and_mappings_tolerate_dropped_and_null_fields() {
    // Both nested schemas are response-only, so every field is `Option<T>`.
    assert_eq!(
        serde_json::from_str::<ReversePrivateEndpointDnsTarget>("{}").unwrap(),
        ReversePrivateEndpointDnsTarget::default()
    );
    assert_eq!(
        serde_json::from_str::<ReversePrivateEndpointDnsTarget>(
            r#"{"id":null,"kind":null,"internalDnsName":null}"#
        )
        .unwrap(),
        ReversePrivateEndpointDnsTarget::default()
    );
    assert_eq!(
        serde_json::from_str::<ReversePrivateEndpointPrivateDnsMapping>("{}").unwrap(),
        ReversePrivateEndpointPrivateDnsMapping::default()
    );
    assert_eq!(
        serde_json::from_str::<ReversePrivateEndpointPrivateDnsMapping>(
            r#"{"privateDnsName":null,"internalDnsName":null}"#
        )
        .unwrap(),
        ReversePrivateEndpointPrivateDnsMapping::default()
    );
    assert_eq!(
        serde_json::from_str::<CustomPrivateDnsMappingResponse>(
            r#"{"privateDnsName":null,"targetId":null}"#
        )
        .unwrap(),
        CustomPrivateDnsMappingResponse::default()
    );

    let rpe: ReversePrivateEndpoint = serde_json::from_str(
        r#"{
            "customPrivateDnsMappings":[{"privateDnsName":"db.example.com","targetId":"rcfg-1"}],
            "dnsTargets":[{"id":"rcfg-1","kind":"RESOURCE_CONFIGURATION","internalDnsName":"a.on.aws"},
                          {"id":"rcfg-2","kind":"FUTURE_KIND"}],
            "privateDnsMappings":[{"privateDnsName":"my.example.com","internalDnsName":"b.on.aws"}]
        }"#,
    )
    .unwrap();
    assert_eq!(
        rpe.custom_private_dns_mappings.unwrap()[0]
            .target_id
            .as_deref(),
        Some("rcfg-1")
    );
    assert_eq!(
        rpe.dns_targets,
        Some(vec![
            ReversePrivateEndpointDnsTarget {
                id: Some("rcfg-1".to_string()),
                internal_dns_name: Some("a.on.aws".to_string()),
                kind: Some(ReversePrivateEndpointDnsTargetKind::RESOURCE_CONFIGURATION),
            },
            ReversePrivateEndpointDnsTarget {
                id: Some("rcfg-2".to_string()),
                internal_dns_name: None,
                kind: Some(ReversePrivateEndpointDnsTargetKind::Unknown(
                    "FUTURE_KIND".to_string()
                )),
            },
        ])
    );
    assert_eq!(
        rpe.private_dns_mappings,
        Some(vec![ReversePrivateEndpointPrivateDnsMapping {
            private_dns_name: Some("my.example.com".to_string()),
            internal_dns_name: Some("b.on.aws".to_string()),
        }])
    );
}

#[test]
fn reverse_private_endpoint_omits_absent_fields_when_serialized() {
    // Absence stays absent in `--json` output, including inside the nested
    // response variant of the shared `customPrivateDnsMapping` schema.
    let rpe = ReversePrivateEndpoint {
        description: Some("MSK endpoint".to_string()),
        custom_private_dns_mappings: Some(vec![CustomPrivateDnsMappingResponse {
            private_dns_name: None,
            target_id: None,
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
    let ClickPipePostPubSubSource::ClickPipePostPubSubServiceAccountSource(src) = src else {
        panic!("expected service-account source")
    };
    assert_eq!(src.topic, "projects/p/topics/t");
    assert_eq!(src.seek_type, ClickPipePostPubSubSourceSeektype::Earliest);
    assert_eq!(
        src.service_account_key.service_account_file,
        "/path/to/key.json"
    );
}

#[test]
fn serialize_clickpipe_patch_pubsub_source_omits_absent_credentials() {
    let empty = serde_json::to_value(ClickPipePatchPubSubSource::default()).unwrap();
    assert_eq!(empty, serde_json::json!({}));

    let ack_deadline_only = serde_json::to_value(ClickPipePatchPubSubSource {
        ack_deadline: Some(30),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(ack_deadline_only, serde_json::json!({"ackDeadline": 30}));
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
            object_storage: None,
            pubsub: None,
        },
    };
    let v = serde_json::to_value(&req).unwrap();
    assert!(v["source"]["kafka"].is_object());
    assert!(v["source"].get("kinesis").is_none());
    assert!(v["source"].get("objectStorage").is_none());
    assert!(v["source"].get("pubsub").is_none());
}

// A Kafka broker with no authentication has no spec enum value; absence of the
// `authentication` key is the wire representation, so `None` must be omitted
// rather than serialized as a mechanism or as `null`.
#[test]
fn click_pipe_post_kafka_source_omits_absent_authentication() {
    let no_auth = ClickPipePostKafkaSource {
        brokers: "broker1:9092".to_string(),
        topics: "events".to_string(),
        authentication: None,
        credentials: serde_json::Value::Null,
        ..Default::default()
    };
    let v = serde_json::to_value(&no_auth).unwrap();
    assert!(
        v.get("authentication").is_none(),
        "absent authentication must be omitted, got {v}"
    );
    assert_eq!(v["brokers"], "broker1:9092");

    let plain = ClickPipePostKafkaSource {
        authentication: Some(ClickPipePostKafkaSourceAuthentication::PLAIN),
        credentials: serde_json::json!({"username": "u", "password": "p"}),
        ..Default::default()
    };
    let v = serde_json::to_value(&plain).unwrap();
    assert_eq!(v["authentication"], "PLAIN");
    assert_eq!(v["credentials"]["username"], "u");
}

// `kafka_read_committed` is only supported for Kafka pipes, so a settings PUT
// for any other source must leave the key off the wire entirely — not send
// `false` and not send `null`.
#[test]
fn click_pipe_settings_put_request_omits_absent_kafka_read_committed() {
    let non_kafka = ClickPipeSettingsPutRequest {
        object_storage_max_file_count: Some(200),
        kafka_read_committed: None,
        ..Default::default()
    };
    let v = serde_json::to_value(&non_kafka).unwrap();
    assert!(
        v.get("kafka_read_committed").is_none(),
        "absent kafka_read_committed must be omitted, got {v}"
    );
    assert_eq!(
        v,
        serde_json::json!({ "object_storage_max_file_count": 200 })
    );

    let kafka = ClickPipeSettingsPutRequest {
        streaming_max_insert_wait_ms: Some(1000),
        kafka_read_committed: Some(true),
        ..Default::default()
    };
    let v = serde_json::to_value(&kafka).unwrap();
    assert_eq!(v["kafka_read_committed"], true);
    assert_eq!(v["streaming_max_insert_wait_ms"], 1000);
}

#[test]
fn click_pipe_schema_discovery_request_supports_new_sources() {
    let object_storage = ClickPipeSchemaDiscoveryRequest {
        source: ClickPipeSchemaDiscoverySource {
            object_storage: Some(ClickPipePostObjectStorageSource::default()),
            ..Default::default()
        },
    };
    let pubsub = ClickPipeSchemaDiscoveryRequest {
        source: ClickPipeSchemaDiscoverySource {
            pubsub: Some(ClickPipePostPubSubSource::default()),
            ..Default::default()
        },
    };

    assert!(serde_json::to_value(object_storage).unwrap()["source"]["objectStorage"].is_object());
    assert!(serde_json::to_value(pubsub).unwrap()["source"]["pubsub"].is_object());
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
    // Source-conditional create values stay absent unless the caller supplies
    // them, while explicit false remains distinguishable from omission.
    assert_eq!(
        serde_json::from_str::<ClickPipeSettings>("{}").unwrap(),
        ClickPipeSettings::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickPipePostSource>("{}").unwrap(),
        ClickPipePostSource::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickPipeSettingsPutRequest>("{}").unwrap(),
        ClickPipeSettingsPutRequest::default()
    );
    assert_eq!(
        serde_json::from_str::<ClickPipeSettingsResponse>("{}").unwrap(),
        ClickPipeSettingsResponse::default()
    );
}

#[test]
fn clickpipe_create_optional_controls_omit_absence_and_preserve_false() {
    let source = ClickPipePostSource::default();
    let settings = ClickPipeSettings::default();
    assert_eq!(
        serde_json::to_value(&source).unwrap(),
        serde_json::json!({})
    );
    assert_eq!(
        serde_json::to_value(&settings).unwrap(),
        serde_json::json!({})
    );

    let source = ClickPipePostSource {
        validate_samples: Some(false),
        ..Default::default()
    };
    let settings = ClickPipeSettings {
        kafka_read_committed: Some(false),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&source).unwrap(),
        serde_json::json!({ "validateSamples": false })
    );
    assert_eq!(
        serde_json::to_value(&settings).unwrap(),
        serde_json::json!({ "kafka_read_committed": false })
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
fn reverse_private_endpoint_create_enums_round_trip_and_match_values() {
    // The CLI validates `reverse-private-endpoint create --type` and
    // `--msk-authentication` against `VALUES`, so each const must stay equal
    // to the wire values its enum actually accepts.
    assert_eq!(
        CreateReversePrivateEndpointType::VALUES,
        &[
            "VPC_ENDPOINT_SERVICE",
            "VPC_RESOURCE",
            "MSK_MULTI_VPC",
            "GCP_PSC_SERVICE_ATTACHMENT",
        ]
    );
    for value in CreateReversePrivateEndpointType::VALUES {
        let parsed: CreateReversePrivateEndpointType =
            serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(parsed, CreateReversePrivateEndpointType::Unknown(_)),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
        assert_eq!(
            serde_json::to_string(&parsed).unwrap(),
            format!(r#""{value}""#)
        );
    }

    assert_eq!(
        CreateReversePrivateEndpointMskauthentication::VALUES,
        &["SASL_IAM", "SASL_SCRAM"]
    );
    for value in CreateReversePrivateEndpointMskauthentication::VALUES {
        let parsed: CreateReversePrivateEndpointMskauthentication =
            serde_json::from_str(&format!(r#""{value}""#)).unwrap();
        assert!(
            !matches!(
                parsed,
                CreateReversePrivateEndpointMskauthentication::Unknown(_)
            ),
            "{value} fell through to the catch-all"
        );
        assert_eq!(parsed.to_string(), *value);
        assert_eq!(
            serde_json::to_string(&parsed).unwrap(),
            format!(r#""{value}""#)
        );
    }
}

/// The endpoint's own model is a response type: a key the API drops and a key
/// it sends as `null` both have to land as `None`, and neither is emitted back.
#[test]
fn reverse_private_endpoint_tolerates_missing_and_null_fields() {
    let sparse: ReversePrivateEndpoint = serde_json::from_str(r#"{"description":"warehouse"}"#)
        .expect("a response with one key must deserialize");
    assert_eq!(sparse.description.as_deref(), Some("warehouse"));
    assert_eq!(sparse.dns_names, None);
    assert_eq!(sparse.status, None);
    assert_eq!(sparse.id, None);

    let explicit_nulls: ReversePrivateEndpoint = serde_json::from_str(
        r#"{"description":null,"dnsNames":null,"status":null,"id":null,"type":null}"#,
    )
    .expect("explicit nulls must deserialize");
    assert_eq!(explicit_nulls, ReversePrivateEndpoint::default());
    assert_eq!(
        serde_json::to_string(&explicit_nulls).unwrap(),
        "{}",
        "absent fields must be omitted, never serialized as null"
    );
}

#[test]
fn clickpipes_workload_identity_sources_round_trip_without_credentials() {
    let bigquery = serde_json::json!({
        "authentication": "SERVICE_ACCOUNT_WORKLOAD_IDENTITY", "projectId": "project",
        "snapshotStagingPath": "gs://bucket/staging", "settings": {"replicationMode": "snapshot"},
        "tableMappings": [{"sourceDatasetName": "dataset", "sourceTable": "source", "targetTable": "target"}]
    });
    let parsed: ClickPipeMutateBigQuerySource = serde_json::from_value(bigquery.clone()).unwrap();
    assert!(matches!(
        parsed,
        ClickPipeMutateBigQuerySource::ClickPipePostBigQueryWorkloadIdentitySource(_)
    ));
    assert_eq!(serde_json::to_value(parsed).unwrap(), bigquery);
    let pubsub = serde_json::json!({
        "authentication": "SERVICE_ACCOUNT_WORKLOAD_IDENTITY", "projectId": "project",
        "topic": "topic", "format": "JSONEachRow", "seekType": "earliest"
    });
    let parsed: ClickPipePostPubSubSource = serde_json::from_value(pubsub.clone()).unwrap();
    assert!(matches!(
        parsed,
        ClickPipePostPubSubSource::ClickPipePostPubSubWorkloadIdentitySource(_)
    ));
    assert_eq!(serde_json::to_value(parsed).unwrap(), pubsub);
    assert!(
        serde_json::from_value::<ClickPipePostBigQueryWorkloadIdentitySource>(
            serde_json::json!({})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ClickPipePostPubSubWorkloadIdentitySource>(serde_json::json!({}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<ClickPipePostBigQueryServiceAccountSource>(serde_json::json!({}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<ClickPipePostPubSubServiceAccountSource>(serde_json::json!({}))
            .is_err()
    );
}

#[test]
fn bigquery_service_account_allows_omitted_authentication() {
    let json = serde_json::json!({
        "snapshotStagingPath": "gs://bucket/staging", "settings": {"replicationMode": "snapshot"},
        "tableMappings": [], "credentials": {"serviceAccountFile": "encoded-key"}
    });
    let parsed: ClickPipeMutateBigQuerySource = serde_json::from_value(json.clone()).unwrap();
    assert!(matches!(
        parsed,
        ClickPipeMutateBigQuerySource::ClickPipePostBigQueryServiceAccountSource(_)
    ));
    assert_eq!(serde_json::to_value(parsed).unwrap(), json);
}

#[test]
fn kafka_inline_protobuf_schema_and_schema_registry_are_independent_options() {
    let json_source = ClickPipePostKafkaSource {
        format: ClickPipePostKafkaSourceFormat::JSONEachRow,
        ..Default::default()
    };
    assert!(
        serde_json::to_value(json_source)
            .unwrap()
            .get("protobufSchema")
            .is_none()
    );
    let inline = ClickPipePostKafkaSource {
        format: ClickPipePostKafkaSourceFormat::Protobuf,
        protobuf_schema: Some("c3ludGF4".into()),
        ..Default::default()
    };
    let wire = serde_json::to_value(inline).unwrap();
    assert_eq!(wire["protobufSchema"], "c3ludGF4");
    assert!(wire.get("schemaRegistry").is_none());
    let registry = ClickPipePostKafkaSource {
        format: ClickPipePostKafkaSourceFormat::Protobuf,
        schema_registry: Some(ClickPipeMutateKafkaSchemaRegistry::default()),
        ..Default::default()
    };
    let wire = serde_json::to_value(registry).unwrap();
    assert!(wire.get("protobufSchema").is_none());
    assert!(wire.get("schemaRegistry").is_some());
}

#[test]
fn kafka_patch_source_preserves_partial_and_explicit_list_updates() {
    let partial: ClickPipePatchKafkaSource = serde_json::from_value(serde_json::json!({
        "caCertificate": "new-ca"
    }))
    .unwrap();
    assert!(partial.credentials.is_none());
    assert!(partial.reverse_private_endpoint_ids.is_none());
    assert_eq!(
        serde_json::to_value(partial).unwrap(),
        serde_json::json!({"caCertificate": "new-ca"})
    );

    let cleared: ClickPipePatchKafkaSource = serde_json::from_value(serde_json::json!({
        "reversePrivateEndpointIds": []
    }))
    .unwrap();
    assert_eq!(cleared.reverse_private_endpoint_ids, Some(vec![]));
    assert_eq!(
        serde_json::to_value(cleared).unwrap(),
        serde_json::json!({"reversePrivateEndpointIds": []})
    );

    let nulls: ClickPipePatchKafkaSource = serde_json::from_value(serde_json::json!({
        "credentials": null,
        "reversePrivateEndpointIds": null
    }))
    .unwrap();
    assert_eq!(nulls, ClickPipePatchKafkaSource::default());
    assert_eq!(serde_json::to_value(nulls).unwrap(), serde_json::json!({}));
}

#[test]
fn clickpipe_patch_nested_objects_preserve_omission_and_explicit_empty_values() {
    let partial = serde_json::json!({
        "source": {
            "postgres": {"host": "postgres.example.com"}
        }
    });
    let request: ClickPipePatchRequest = serde_json::from_value(partial.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), partial);

    let explicit = serde_json::json!({
        "destination": {"columns": []},
        "source": {
            "postgres": {
                "tableMappingsToAdd": [],
                "tableMappingsToRemove": []
            },
            "validateSamples": false
        }
    });
    let request: ClickPipePatchRequest = serde_json::from_value(explicit.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), explicit);

    let nulls: ClickPipePatchRequest = serde_json::from_value(serde_json::json!({
        "source": {
            "postgres": {
                "credentials": null,
                "settings": null,
                "tableMappingsToAdd": null,
                "tableMappingsToRemove": null
            },
            "validateSamples": null
        }
    }))
    .unwrap();
    assert_eq!(
        serde_json::to_value(nulls).unwrap(),
        serde_json::json!({"source": {"postgres": {}}})
    );
}

#[test]
fn kinesis_protobuf_schema_round_trips_and_other_formats_omit_it() {
    let json_source = ClickPipePostKinesisSource::default();
    let wire = serde_json::to_value(&json_source).unwrap();
    assert!(wire.get("protobufSchema").is_none());
    assert_eq!(
        serde_json::from_value::<ClickPipePostKinesisSource>(wire).unwrap(),
        json_source
    );

    let protobuf = ClickPipePostKinesisSource {
        format: ClickPipePostKinesisSourceFormat::Protobuf,
        protobuf_schema: Some("c3ludGF4".into()),
        ..Default::default()
    };
    let wire = serde_json::to_value(&protobuf).unwrap();
    assert_eq!(wire["format"], "Protobuf");
    assert_eq!(wire["protobufSchema"], "c3ludGF4");
    assert_eq!(
        serde_json::from_value::<ClickPipePostKinesisSource>(wire).unwrap(),
        protobuf
    );
    assert_eq!(protobuf.format.to_string(), "Protobuf");

    let response: ClickPipeKinesisSource =
        serde_json::from_value(serde_json::json!({"format": "Protobuf"})).unwrap();
    assert_eq!(
        response.format,
        Some(ClickPipeKinesisSourceFormat::Protobuf)
    );
    assert_eq!(response.format.unwrap().to_string(), "Protobuf");
    assert!(
        matches!(serde_json::from_str::<ClickPipeKinesisSourceFormat>("\"FutureFormat\"").unwrap(), ClickPipeKinesisSourceFormat::Unknown(value) if value == "FutureFormat")
    );
}

#[test]
fn kinesis_registry_request_is_strict_and_response_is_tolerant() {
    let full = serde_json::json!({
        "type": "glue", "glueRegion": "us-east-1", "glueRegistryName": "events"
    });
    let request: ClickPipeKinesisSchemaRegistry = serde_json::from_value(full.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), full);
    for field in ["type", "glueRegion", "glueRegistryName"] {
        let mut missing = full.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ClickPipeKinesisSchemaRegistry>(missing).is_err());
        let mut null = full.clone();
        null[field] = serde_json::Value::Null;
        assert!(serde_json::from_value::<ClickPipeKinesisSchemaRegistry>(null).is_err());
    }
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "type": null, "glueRegion": null, "glueRegistryName": null, "glueRoleArn": null
        }),
    ] {
        let response: ClickPipeKinesisSchemaRegistryResponse =
            serde_json::from_value(wire).unwrap();
        assert_eq!(response, ClickPipeKinesisSchemaRegistryResponse::default());
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({})
        );
    }
}

#[test]
fn kinesis_registry_conversion_reports_nested_missing_fields_and_preserves_unknown_type() {
    let response: ClickPipeKinesisSchemaRegistryResponse =
        serde_json::from_value(serde_json::json!({
            "type": "future", "glueRegion": "us-east-1", "glueRoleArn": "arn:aws:iam::123:role/Glue"
        }))
        .unwrap();
    let error = ClickPipeKinesisSchemaRegistry::try_from(response.clone()).unwrap_err();
    assert_eq!(error.fields(), &["glueRegistryName"]);
    let mut complete = response;
    complete.glue_registry_name = Some("events".into());
    let request = ClickPipeKinesisSchemaRegistry::try_from(complete).unwrap();
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({
            "type": "future", "glueRegion": "us-east-1", "glueRegistryName": "events",
            "glueRoleArn": "arn:aws:iam::123:role/Glue"
        })
    );
    assert_eq!(
        ClickPipeKinesisSchemaRegistryType::Unknown("future".into()).to_string(),
        "future"
    );
}

#[test]
fn kafka_schema_registry_response_dispatches_legacy_confluent_and_glue() {
    // Payloads written before Glue support carry no `type`; they stay Confluent.
    for wire in [
        serde_json::json!({
            "url": "https://registry.example", "authentication": "PLAIN",
            "caCertificate": "CA"
        }),
        serde_json::json!({
            "type": "confluent", "url": "https://registry.example",
            "authentication": "PLAIN", "caCertificate": "CA"
        }),
    ] {
        let registry: ClickPipeKafkaSchemaRegistry = serde_json::from_value(wire.clone()).unwrap();
        let ClickPipeKafkaSchemaRegistry::ClickPipeKafkaConfluentSchemaRegistry(confluent) =
            &registry
        else {
            panic!("expected Confluent, got {registry:?}");
        };
        assert_eq!(confluent.url.as_deref(), Some("https://registry.example"));
        assert_eq!(
            confluent.authentication,
            Some(ClickPipeKafkaConfluentSchemaRegistryAuthentication::PLAIN)
        );
        assert_eq!(confluent.ca_certificate.as_deref(), Some("CA"));
        assert_eq!(serde_json::to_value(&registry).unwrap(), wire);
    }

    let glue = serde_json::json!({
        "type": "glue", "glueRegion": "us-east-1", "glueRegistryName": "events",
        "glueRoleArn": "arn:aws:iam::123:role/Glue"
    });
    let registry: ClickPipeKafkaSchemaRegistry = serde_json::from_value(glue.clone()).unwrap();
    let ClickPipeKafkaSchemaRegistry::ClickPipeKafkaGlueSchemaRegistryResponse(response) =
        &registry
    else {
        panic!("expected Glue, got {registry:?}");
    };
    assert_eq!(
        response.r#type,
        Some(ClickPipeKafkaGlueSchemaRegistryType::Glue)
    );
    assert_eq!(response.glue_region.as_deref(), Some("us-east-1"));
    assert_eq!(response.glue_registry_name.as_deref(), Some("events"));
    assert_eq!(serde_json::to_value(&registry).unwrap(), glue);

    // A Glue payload that lost its discriminator, or a future registry type,
    // is kept verbatim instead of being misread as Confluent.
    for wire in [
        serde_json::json!({"glueRegion": "us-east-1", "glueRegistryName": "events"}),
        serde_json::json!({"type": "future", "url": "https://registry.example"}),
    ] {
        let registry: ClickPipeKafkaSchemaRegistry = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(registry, ClickPipeKafkaSchemaRegistry::Unknown(wire));
    }

    let default = serde_json::to_value(ClickPipeKafkaSchemaRegistry::default()).unwrap();
    assert_eq!(default, serde_json::json!({}));
    assert_eq!(
        serde_json::from_value::<ClickPipeKafkaSchemaRegistry>(default).unwrap(),
        ClickPipeKafkaSchemaRegistry::default()
    );
}

#[test]
fn kafka_schema_registry_variants_tolerate_missing_and_null_fields() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "type": null, "url": null, "authentication": null, "caCertificate": null
        }),
    ] {
        let confluent: ClickPipeKafkaConfluentSchemaRegistry =
            serde_json::from_value(wire).unwrap();
        assert_eq!(confluent, ClickPipeKafkaConfluentSchemaRegistry::default());
        assert_eq!(
            serde_json::to_value(confluent).unwrap(),
            serde_json::json!({})
        );
    }
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "type": null, "glueRegion": null, "glueRegistryName": null, "glueRoleArn": null
        }),
    ] {
        let glue: ClickPipeKafkaGlueSchemaRegistryResponse = serde_json::from_value(wire).unwrap();
        assert_eq!(glue, ClickPipeKafkaGlueSchemaRegistryResponse::default());
        assert_eq!(serde_json::to_value(glue).unwrap(), serde_json::json!({}));
    }
}

#[test]
fn kafka_source_response_schema_registry_may_be_missing_or_null() {
    for wire in [
        serde_json::json!({"brokers": "broker:9092"}),
        serde_json::json!({"brokers": "broker:9092", "schemaRegistry": null}),
    ] {
        let source: ClickPipeKafkaSource = serde_json::from_value(wire).unwrap();
        assert_eq!(source.schema_registry, None);
        assert!(
            serde_json::to_value(source)
                .unwrap()
                .get("schemaRegistry")
                .is_none()
        );
    }
    let source: ClickPipeKafkaSource = serde_json::from_value(serde_json::json!({
        "schemaRegistry": {"type": "glue", "glueRegion": "eu-west-1", "glueRegistryName": "r"}
    }))
    .unwrap();
    assert!(matches!(
        source.schema_registry,
        Some(ClickPipeKafkaSchemaRegistry::ClickPipeKafkaGlueSchemaRegistryResponse(_))
    ));
}

#[test]
fn kafka_schema_registry_requests_serialize_each_variant_wire_shape() {
    let confluent: ClickPipeMutateKafkaSchemaRegistry =
        ClickPipeMutateKafkaConfluentSchemaRegistry {
            r#type: None,
            url: "https://registry.example".into(),
            authentication: ClickPipeMutateKafkaConfluentSchemaRegistryAuthentication::PLAIN,
            credentials: ClickPipeKafkaSchemaRegistryCredentials {
                username: "user".into(),
                password: "secret".into(),
            },
            ca_certificate: None,
        }
        .into();
    assert_eq!(
        serde_json::to_value(&confluent).unwrap(),
        serde_json::json!({
            "url": "https://registry.example", "authentication": "PLAIN",
            "credentials": {"username": "user", "password": "secret"}
        })
    );
    let ClickPipeMutateKafkaSchemaRegistry::ClickPipeMutateKafkaConfluentSchemaRegistry(
        mut explicit,
    ) = confluent
    else {
        unreachable!()
    };
    explicit.r#type = Some(ClickPipeMutateKafkaConfluentSchemaRegistryType::Confluent);
    explicit.ca_certificate = Some("CA".into());
    let explicit = ClickPipeMutateKafkaSchemaRegistry::from(explicit);
    let wire = serde_json::json!({
        "type": "confluent", "url": "https://registry.example", "authentication": "PLAIN",
        "credentials": {"username": "user", "password": "secret"}, "caCertificate": "CA"
    });
    assert_eq!(serde_json::to_value(&explicit).unwrap(), wire);
    assert_eq!(
        serde_json::from_value::<ClickPipeMutateKafkaSchemaRegistry>(wire).unwrap(),
        explicit
    );

    let glue: ClickPipeMutateKafkaSchemaRegistry = ClickPipeKafkaGlueSchemaRegistry {
        r#type: ClickPipeKafkaGlueSchemaRegistryType::Glue,
        glue_region: "us-east-1".into(),
        glue_registry_name: "events".into(),
        glue_role_arn: None,
    }
    .into();
    let wire = serde_json::json!({
        "type": "glue", "glueRegion": "us-east-1", "glueRegistryName": "events"
    });
    assert_eq!(serde_json::to_value(&glue).unwrap(), wire);
    assert_eq!(
        serde_json::from_value::<ClickPipeMutateKafkaSchemaRegistry>(wire).unwrap(),
        glue
    );
    let source = ClickPipePostKafkaSource {
        schema_registry: Some(glue),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(source).unwrap()["schemaRegistry"]["type"],
        "glue"
    );
}

#[test]
fn kafka_schema_registry_request_variants_are_strict() {
    let confluent = serde_json::json!({
        "url": "https://registry.example", "authentication": "PLAIN",
        "credentials": {"username": "user", "password": "secret"}
    });
    serde_json::from_value::<ClickPipeMutateKafkaConfluentSchemaRegistry>(confluent.clone())
        .unwrap();
    for field in ["url", "authentication", "credentials"] {
        let mut missing = confluent.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<ClickPipeMutateKafkaConfluentSchemaRegistry>(missing).is_err()
        );
    }
    let glue = serde_json::json!({
        "type": "glue", "glueRegion": "us-east-1", "glueRegistryName": "events"
    });
    for field in ["type", "glueRegion", "glueRegistryName"] {
        let mut missing = glue.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ClickPipeKafkaGlueSchemaRegistry>(missing).is_err());
        let mut null = glue.clone();
        null[field] = serde_json::Value::Null;
        assert!(serde_json::from_value::<ClickPipeKafkaGlueSchemaRegistry>(null).is_err());
    }
}

#[test]
fn kafka_glue_registry_conversion_reports_missing_fields() {
    let response: ClickPipeKafkaGlueSchemaRegistryResponse =
        serde_json::from_value(serde_json::json!({
            "glueRegion": "us-east-1", "glueRoleArn": "arn:aws:iam::123:role/Glue"
        }))
        .unwrap();
    let error = ClickPipeKafkaGlueSchemaRegistry::try_from(response.clone()).unwrap_err();
    assert_eq!(error.fields(), &["type", "glueRegistryName"]);
    let mut complete = response;
    complete.r#type = Some(ClickPipeKafkaGlueSchemaRegistryType::Glue);
    complete.glue_registry_name = Some("events".into());
    let request = ClickPipeKafkaGlueSchemaRegistry::try_from(complete).unwrap();
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({
            "type": "glue", "glueRegion": "us-east-1", "glueRegistryName": "events",
            "glueRoleArn": "arn:aws:iam::123:role/Glue"
        })
    );
    assert_eq!(
        ClickPipeKafkaGlueSchemaRegistryType::Glue.to_string(),
        "glue"
    );
    assert_eq!(
        ClickPipeKafkaConfluentSchemaRegistryType::Confluent.to_string(),
        "confluent"
    );
}

#[test]
fn kafka_tombstone_mode_is_lossless_and_optional() {
    let request = ClickPipePostKafkaSource::default();
    assert!(
        serde_json::to_value(request)
            .unwrap()
            .get("tombstoneMode")
            .is_none()
    );
    let response: ClickPipeKafkaSource = serde_json::from_value(serde_json::json!({
        "tombstoneMode": "future"
    }))
    .unwrap();
    assert!(
        matches!(response.tombstone_mode, Some(ClickPipeKafkaSourceTombstonemode::Unknown(ref value)) if value == "future")
    );
    let request = ClickPipePostKafkaSource {
        tombstone_mode: Some(ClickPipePostKafkaSourceTombstonemode::Delete),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(request).unwrap()["tombstoneMode"],
        "delete"
    );
    assert_eq!(
        ClickPipePostKafkaSourceTombstonemode::Delete.to_string(),
        "delete"
    );
    for wire in [
        serde_json::json!({}),
        serde_json::json!({"tombstoneMode": null}),
    ] {
        let response: ClickPipeKafkaSource = serde_json::from_value(wire).unwrap();
        assert_eq!(response.tombstone_mode, None);
        assert!(
            serde_json::to_value(response)
                .unwrap()
                .get("tombstoneMode")
                .is_none()
        );
    }
}

#[test]
fn kinesis_sources_omit_missing_or_null_registry() {
    let request = ClickPipePostKinesisSource::default();
    assert!(
        serde_json::to_value(request)
            .unwrap()
            .get("schemaRegistry")
            .is_none()
    );
    for wire in [
        serde_json::json!({}),
        serde_json::json!({"schemaRegistry": null}),
    ] {
        let response: ClickPipeKinesisSource = serde_json::from_value(wire).unwrap();
        assert_eq!(response.schema_registry, None);
        assert!(
            serde_json::to_value(response)
                .unwrap()
                .get("schemaRegistry")
                .is_none()
        );
    }
}

#[test]
fn kafka_tombstone_mode_soft_delete_round_trips() {
    let parsed: ClickPipeKafkaSourceTombstonemode =
        serde_json::from_str(r#""soft_delete""#).unwrap();
    assert_eq!(parsed, ClickPipeKafkaSourceTombstonemode::Soft_delete);
    assert_eq!(parsed.to_string(), "soft_delete");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), "soft_delete");

    let parsed: ClickPipePostKafkaSourceTombstonemode =
        serde_json::from_str(r#""soft_delete""#).unwrap();
    assert_eq!(parsed, ClickPipePostKafkaSourceTombstonemode::Soft_delete);
    assert_eq!(parsed.to_string(), "soft_delete");
    let request = ClickPipePostKafkaSource {
        tombstone_mode: Some(ClickPipePostKafkaSourceTombstonemode::Soft_delete),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(request).unwrap()["tombstoneMode"],
        "soft_delete"
    );
}

#[test]
fn database_clickpipe_sources_accept_workload_identity_and_cloud_sql_mysql() {
    macro_rules! assert_round_trip {
        ($ty:ty, $variant:expr, $wire:literal) => {{
            let parsed: $ty = serde_json::from_str(concat!('"', $wire, '"')).unwrap();
            assert_eq!(parsed, $variant);
            assert_eq!(parsed.to_string(), $wire);
            assert_eq!(
                serde_json::to_string(&parsed).unwrap(),
                concat!('"', $wire, '"')
            );
        }};
    }

    assert_round_trip!(
        ClickPipeMutateMySQLSourceAuthentication,
        ClickPipeMutateMySQLSourceAuthentication::ServiceAccountWorkloadIdentity,
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"
    );
    assert_round_trip!(
        ClickPipeMySQLSourceAuthentication,
        ClickPipeMySQLSourceAuthentication::ServiceAccountWorkloadIdentity,
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"
    );
    assert_round_trip!(
        ClickPipePatchMySQLSourceAuthentication,
        ClickPipePatchMySQLSourceAuthentication::ServiceAccountWorkloadIdentity,
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"
    );
    assert_round_trip!(
        ClickPipeMutatePostgresSourceAuthentication,
        ClickPipeMutatePostgresSourceAuthentication::ServiceAccountWorkloadIdentity,
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"
    );
    assert_round_trip!(
        ClickPipePostgresSourceAuthentication,
        ClickPipePostgresSourceAuthentication::ServiceAccountWorkloadIdentity,
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"
    );
    assert_round_trip!(
        ClickPipeMutateMySQLSourceType,
        ClickPipeMutateMySQLSourceType::Cloudsqlmysql,
        "cloudsqlmysql"
    );
    assert_round_trip!(
        ClickPipeMySQLSourceType,
        ClickPipeMySQLSourceType::Cloudsqlmysql,
        "cloudsqlmysql"
    );
    assert!(
        ClickPipeMutateMySQLSourceAuthentication::VALUES
            .contains(&"SERVICE_ACCOUNT_WORKLOAD_IDENTITY")
    );
    assert!(ClickPipeMutateMySQLSourceType::VALUES.contains(&"cloudsqlmysql"));
}

#[test]
fn mongodb_structured_ingestion_request_is_strict_and_omits_unset_fields() {
    let minimal = ClickPipeMongoDBPipeTableMapping {
        source_database_name: "mydb".to_string(),
        source_collection: "users".to_string(),
        target_table: "mydb_users".to_string(),
        structured_ingestion: Some(ClickPipeMongoDBStructuredIngestion {
            enabled: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&minimal).unwrap(),
        serde_json::json!({
            "sourceDatabaseName": "mydb",
            "sourceCollection": "users",
            "targetTable": "mydb_users",
            "structuredIngestion": { "enabled": true }
        })
    );

    let maximal: ClickPipeMongoDBStructuredIngestion = serde_json::from_value(serde_json::json!({
        "enabled": true,
        "dropUnexpectedValues": false,
        "columns": [{ "name": "amount", "type": "Float64" }],
        "excludedColumns": ["internal_notes"],
        "sortingKeys": ["_id", "created_at"],
        "partitionByExpr": "toYYYYMM(created_at)"
    }))
    .unwrap();
    assert_eq!(
        maximal.columns,
        Some(vec![ClickPipeMongoDBPipeColumn {
            name: "amount".to_string(),
            r#type: "Float64".to_string(),
        }])
    );
    assert_eq!(
        maximal.partition_by_expr.as_deref(),
        Some("toYYYYMM(created_at)")
    );

    // `enabled` and both column fields are required in request position.
    assert!(serde_json::from_str::<ClickPipeMongoDBStructuredIngestion>("{}").is_err());
    assert!(serde_json::from_str::<ClickPipeMongoDBPipeColumn>(r#"{"name":"amount"}"#).is_err());
}

#[test]
fn mongodb_structured_ingestion_response_tolerates_absent_and_null_fields() {
    let mapping: ClickPipeMongoDBPipeTableMappingResponse = serde_json::from_str(
        r#"{"sourceDatabaseName":"mydb","structuredIngestion":{"enabled":null,"columns":[{"name":null}],"sortingKeys":null}}"#,
    )
    .unwrap();
    let ingestion = mapping.structured_ingestion.unwrap();
    assert_eq!(ingestion.enabled, None);
    assert_eq!(
        ingestion.columns,
        Some(vec![ClickPipeMongoDBPipeColumnResponse::default()])
    );
    assert_eq!(
        serde_json::from_str::<ClickPipeMongoDBStructuredIngestionResponse>("{}").unwrap(),
        ClickPipeMongoDBStructuredIngestionResponse::default()
    );
    assert_eq!(
        serde_json::to_value(ClickPipeMongoDBStructuredIngestionResponse::default()).unwrap(),
        serde_json::json!({})
    );
}

#[test]
fn mongodb_table_config_request_is_strict_and_round_trips() {
    let mapping = ClickPipeMongoDBPipeTableMapping {
        source_database_name: "mydb".to_string(),
        source_collection: "users".to_string(),
        target_table: "mydb_users".to_string(),
        mongo_config: Some(ClickPipeMongoDBTableConfig {
            persist_deleted_documents: true,
        }),
        ..Default::default()
    };
    let wire = serde_json::json!({
        "sourceDatabaseName": "mydb",
        "sourceCollection": "users",
        "targetTable": "mydb_users",
        "mongoConfig": { "persistDeletedDocuments": true }
    });
    assert_eq!(serde_json::to_value(&mapping).unwrap(), wire);
    assert_eq!(
        serde_json::from_value::<ClickPipeMongoDBPipeTableMapping>(wire).unwrap(),
        mapping
    );

    // `persistDeletedDocuments` is required in request position, so an
    // explicit `false` is sent and an empty config is rejected.
    assert_eq!(
        serde_json::to_value(ClickPipeMongoDBTableConfig::default()).unwrap(),
        serde_json::json!({ "persistDeletedDocuments": false })
    );
    assert!(serde_json::from_str::<ClickPipeMongoDBTableConfig>("{}").is_err());

    // The mapping itself omits an unset `mongoConfig`.
    let unset = ClickPipeMongoDBPipeTableMapping {
        mongo_config: None,
        ..mapping
    };
    assert!(
        serde_json::to_value(&unset)
            .unwrap()
            .get("mongoConfig")
            .is_none()
    );
}

#[test]
fn mongodb_table_config_response_tolerates_absent_and_null_fields() {
    for raw in [
        r#"{"sourceDatabaseName":"mydb"}"#,
        r#"{"sourceDatabaseName":"mydb","mongoConfig":null}"#,
    ] {
        let mapping: ClickPipeMongoDBPipeTableMappingResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(mapping.mongo_config, None);
    }
    for raw in [
        r#"{"mongoConfig":{}}"#,
        r#"{"mongoConfig":{"persistDeletedDocuments":null}}"#,
    ] {
        let mapping: ClickPipeMongoDBPipeTableMappingResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(
            mapping.mongo_config,
            Some(ClickPipeMongoDBTableConfigResponse::default())
        );
    }
    let mapping: ClickPipeMongoDBPipeTableMappingResponse =
        serde_json::from_str(r#"{"mongoConfig":{"persistDeletedDocuments":false}}"#).unwrap();
    assert_eq!(
        mapping.mongo_config.unwrap().persist_deleted_documents,
        Some(false)
    );
    assert_eq!(
        serde_json::to_value(ClickPipeMongoDBTableConfigResponse::default()).unwrap(),
        serde_json::json!({})
    );
}

#[test]
fn bigquery_cdc_fields_round_trip_and_write_back() {
    let settings = serde_json::json!({
        "replicationMode": "cdc", "replicationMethod": "query_based",
        "syncIntervalSeconds": 60, "queryCdcPullSyncParallelism": 4,
        "syncDelaySeconds": 0, "pullWindowSizeSeconds": 86400
    });
    let response: ClickPipeBigQueryPipeSettingsResponse =
        serde_json::from_value(settings.clone()).unwrap();
    let request = ClickPipeBigQueryPipeSettings::try_from(response).unwrap();
    assert_eq!(request.replication_method.as_deref(), Some("query_based"));
    assert_eq!(request.sync_interval_seconds, Some(60));
    assert_eq!(request.query_cdc_pull_sync_parallelism, Some(4));
    assert_eq!(request.sync_delay_seconds, Some(0));
    assert_eq!(request.pull_window_size_seconds, Some(86400));
    assert_eq!(serde_json::to_value(&request).unwrap(), settings);
    assert_eq!(
        serde_json::from_value::<ClickPipeBigQueryPipeSettings>(settings).unwrap(),
        request
    );

    let mapping = serde_json::json!({
        "sourceDatasetName": "sales", "sourceTable": "orders", "targetTable": "orders_raw",
        "queryCdcWatermarkColumn": "updated_at", "eventsFunction": "future_function"
    });
    let response: ClickPipeBigQueryPipeTableMappingResponse =
        serde_json::from_value(mapping.clone()).unwrap();
    let request = ClickPipeBigQueryPipeTableMapping::try_from(response).unwrap();
    assert_eq!(
        request.query_cdc_watermark_column.as_deref(),
        Some("updated_at")
    );
    assert_eq!(request.events_function.as_deref(), Some("future_function"));
    assert_eq!(serde_json::to_value(request).unwrap(), mapping);
}

#[test]
fn bigquery_cdc_response_fields_tolerate_absence_and_null() {
    for value in [
        serde_json::json!({}),
        serde_json::json!({
            "replicationMode": null, "replicationMethod": null, "syncIntervalSeconds": null,
            "queryCdcPullSyncParallelism": null, "syncDelaySeconds": null, "pullWindowSizeSeconds": null
        }),
    ] {
        let response: ClickPipeBigQueryPipeSettingsResponse =
            serde_json::from_value(value).unwrap();
        assert_eq!(response, ClickPipeBigQueryPipeSettingsResponse::default());
        assert_eq!(
            serde_json::to_value(&response).unwrap(),
            serde_json::json!({})
        );
        assert_eq!(
            ClickPipeBigQueryPipeSettings::try_from(response)
                .unwrap_err()
                .fields(),
            &["replicationMode"]
        );
    }
    for value in [
        serde_json::json!({}),
        serde_json::json!({
            "eventsFunction": null, "queryCdcWatermarkColumn": null
        }),
    ] {
        let response: ClickPipeBigQueryPipeTableMappingResponse =
            serde_json::from_value(value).unwrap();
        assert_eq!(
            response,
            ClickPipeBigQueryPipeTableMappingResponse::default()
        );
        assert_eq!(
            serde_json::to_value(&response).unwrap(),
            serde_json::json!({})
        );
        assert_eq!(
            ClickPipeBigQueryPipeTableMapping::try_from(response)
                .unwrap_err()
                .fields(),
            &["sourceDatasetName", "sourceTable", "targetTable"]
        );
    }
    assert!(
        serde_json::from_value::<ClickPipeBigQueryPipeSettings>(serde_json::json!({})).is_err()
    );
    assert!(
        serde_json::from_value::<ClickPipeBigQueryPipeTableMapping>(serde_json::json!({})).is_err()
    );
}

#[test]
fn bigquery_sync_cron_round_trips_writes_back_and_tolerates_absence() {
    let settings = serde_json::json!({
        "replicationMode": "cdc", "replicationMethod": "query_based",
        "syncCron": "CRON_TZ=Europe/London 0 * * * *"
    });
    let response: ClickPipeBigQueryPipeSettingsResponse =
        serde_json::from_value(settings.clone()).unwrap();
    assert_eq!(
        response.sync_cron.as_deref(),
        Some("CRON_TZ=Europe/London 0 * * * *")
    );
    let request = ClickPipeBigQueryPipeSettings::try_from(response).unwrap();
    assert_eq!(
        request.sync_cron.as_deref(),
        Some("CRON_TZ=Europe/London 0 * * * *")
    );
    assert_eq!(request.sync_interval_seconds, None);
    assert_eq!(serde_json::to_value(&request).unwrap(), settings);

    let unset = ClickPipeBigQueryPipeSettings {
        replication_mode: ClickPipeBigQueryPipeSettingsReplicationmode::Snapshot,
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&unset).unwrap(),
        serde_json::json!({"replicationMode": "snapshot"})
    );

    for value in [
        serde_json::json!({"replicationMode": "cdc"}),
        serde_json::json!({"replicationMode": "cdc", "syncCron": null}),
    ] {
        let response: ClickPipeBigQueryPipeSettingsResponse =
            serde_json::from_value(value).unwrap();
        assert_eq!(response.sync_cron, None);
        let request = ClickPipeBigQueryPipeSettings::try_from(response).unwrap();
        assert_eq!(request.sync_cron, None);
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({"replicationMode": "cdc"})
        );
    }
}

#[test]
fn bigquery_replication_modes_cover_cdc_and_preserve_future_values() {
    for (wire, expected) in [
        (
            "snapshot",
            ClickPipeBigQueryPipeSettingsReplicationmode::Snapshot,
        ),
        ("cdc", ClickPipeBigQueryPipeSettingsReplicationmode::Cdc),
        (
            "cdc_only",
            ClickPipeBigQueryPipeSettingsReplicationmode::CdcOnly,
        ),
        (
            "future",
            ClickPipeBigQueryPipeSettingsReplicationmode::Unknown("future".into()),
        ),
    ] {
        let parsed: ClickPipeBigQueryPipeSettingsReplicationmode =
            serde_json::from_value(serde_json::json!(wire)).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.to_string(), wire);
        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            serde_json::json!(wire)
        );
    }
    assert_eq!(
        ClickPipeBigQueryPipeSettingsReplicationmode::VALUES,
        &["snapshot", "cdc", "cdc_only"]
    );
}
