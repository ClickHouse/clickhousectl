use clickhouse_cloud_api::models::*;
use wiremock::Mock;
use wiremock::matchers::{basic_auth, body_partial_json, method, path};

use crate::{ok_empty, ok_json, setup};

// ===========================================================================
// ClickPipes
// ===========================================================================

#[tokio::test]
async fn list_click_pipes() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes"))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "name": "kafka-pipe",
                "state": "Running"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.click_pipe_get_list("org-1", "svc-1").await.unwrap();
    let pipes = resp.result.unwrap();
    assert_eq!(pipes.len(), 1);
    assert_eq!(pipes[0].name.as_deref(), Some("kafka-pipe"));
}

/// Mirror the shape the live API actually returns for a Kafka pipe — including
/// `reversePrivateEndpointIds: null`, which the spec declares as a required
/// array but the server happily emits as null when unset. Response fields are
/// `Option<T>`, so `null` lands as `None` rather than failing with
/// `invalid type: null, expected a sequence`.
#[tokio::test]
async fn list_click_pipes_with_null_array_fields() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes"))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "serviceId": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
                "name": "kafka-pipe",
                "state": "Running",
                "scaling": {
                    "replicas": 1,
                    "replicaCpuMillicores": 125,
                    "replicaMemoryGb": 0.5
                },
                "source": {
                    "kafka": {
                        "type": "confluent",
                        "format": "JSONEachRow",
                        "brokers": "broker.example:9092",
                        "topics": "events",
                        "consumerGroup": "clickpipes-aaaaaaaa",
                        "authentication": "PLAIN",
                        "reversePrivateEndpointIds": null
                    }
                },
                "destination": {
                    "database": "default",
                    "table": "events",
                    "managedTable": true,
                    "tableDefinition": {
                        "engine": {"type": "MergeTree"},
                        "sortingKey": ["id"]
                    },
                    "columns": []
                },
                "fieldMappings": [],
                "settings": {},
                "createdAt": "2026-05-13T17:47:28.132987Z",
                "updatedAt": "2026-05-13T17:47:28.132987Z"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.click_pipe_get_list("org-1", "svc-1").await.unwrap();
    let pipes = resp.result.unwrap();
    assert_eq!(pipes.len(), 1);
    assert_eq!(pipes[0].name.as_deref(), Some("kafka-pipe"));
    let kafka = pipes[0]
        .source
        .as_ref()
        .and_then(|source| source.kafka.as_ref())
        .expect("kafka source present");
    assert_eq!(kafka.reverse_private_endpoint_ids, None);
}

#[tokio::test]
async fn create_click_pipe() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes"))
        .and(body_partial_json(serde_json::json!({"name": "new-pipe"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "new-pipe",
            "state": "Provisioning"
        })))
        .mount(&s)
        .await;

    let body = ClickPipePostRequest {
        name: "new-pipe".to_string(),
        ..Default::default()
    };
    let resp = c.click_pipe_create("org-1", "svc-1", &body).await.unwrap();
    let pipe = resp.result.unwrap();
    assert_eq!(pipe.name.as_deref(), Some("new-pipe"));
}

#[tokio::test]
async fn create_kinesis_click_pipe_sends_glue_registry() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes"))
        .and(body_partial_json(serde_json::json!({
            "source": {"kinesis": {"schemaRegistry": {
                "type": "glue", "glueRegion": "us-east-1", "glueRegistryName": "events"
            }}}
        })))
        .respond_with(ok_json(serde_json::json!({"name": "kinesis-pipe"})))
        .expect(1)
        .mount(&server)
        .await;
    let request = ClickPipePostRequest {
        name: "kinesis-pipe".into(),
        source: ClickPipePostSource {
            kinesis: Some(ClickPipePostKinesisSource {
                schema_registry: Some(ClickPipeKinesisSchemaRegistry {
                    r#type: ClickPipeKinesisSchemaRegistryType::Glue,
                    glue_region: "us-east-1".into(),
                    glue_registry_name: "events".into(),
                    glue_role_arn: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let response = client
        .click_pipe_create("org-1", "svc-1", &request)
        .await
        .unwrap();
    assert_eq!(
        response.result.unwrap().name.as_deref(),
        Some("kinesis-pipe")
    );
}

#[tokio::test]
async fn get_click_pipe_preserves_nullable_kinesis_registry() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "source": {"kinesis": {"schemaRegistry": {
                "type": "glue", "glueRegion": "us-east-1", "glueRegistryName": null
            }}}
        })))
        .mount(&server)
        .await;
    let pipe = client
        .click_pipe_get("org-1", "svc-1", "pipe-1")
        .await
        .unwrap()
        .result
        .unwrap();
    let registry = pipe
        .source
        .unwrap()
        .kinesis
        .unwrap()
        .schema_registry
        .unwrap();
    assert_eq!(registry.glue_region.as_deref(), Some("us-east-1"));
    assert_eq!(registry.glue_registry_name, None);
    assert_eq!(
        ClickPipeKinesisSchemaRegistry::try_from(registry)
            .unwrap_err()
            .fields(),
        &["glueRegistryName"]
    );
}

#[tokio::test]
async fn create_click_pipe_sends_start_paused_and_table_ttl() {
    let (server, client) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes"))
        .and(body_partial_json(serde_json::json!({
            "startPaused": true,
            "destination": {"tableDefinition": {"ttl": "event_time + INTERVAL 30 DAY"}}
        })))
        .respond_with(ok_json(serde_json::json!({"state": "Stopped"})))
        .expect(1)
        .mount(&server)
        .await;
    let request = ClickPipePostRequest {
        start_paused: Some(true),
        destination: ClickPipeMutateDestination {
            table_definition: Some(ClickPipeDestinationTableDefinition {
                ttl: "event_time + INTERVAL 30 DAY".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let response = client
        .click_pipe_create("org-1", "svc-1", &request)
        .await
        .unwrap();
    assert_eq!(
        response.result.unwrap().state,
        Some(ClickPipeState::Stopped)
    );
}

#[tokio::test]
async fn get_click_pipe() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "my-pipe",
            "state": "Running"
        })))
        .mount(&s)
        .await;

    let resp = c.click_pipe_get("org-1", "svc-1", "pipe-1").await.unwrap();
    let pipe = resp.result.unwrap();
    assert_eq!(pipe.name.as_deref(), Some("my-pipe"));
}

#[tokio::test]
async fn update_click_pipe() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "renamed-pipe"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "renamed-pipe",
            "state": "Running"
        })))
        .mount(&s)
        .await;

    let body = ClickPipePatchRequest {
        name: Some("renamed-pipe".to_string()),
        ..Default::default()
    };
    let resp = c
        .click_pipe_update("org-1", "svc-1", "pipe-1", &body)
        .await
        .unwrap();
    let pipe = resp.result.unwrap();
    assert_eq!(pipe.name.as_deref(), Some("renamed-pipe"));
}

#[tokio::test]
async fn delete_click_pipe() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_pipe_delete("org-1", "svc-1", "pipe-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

#[tokio::test]
async fn update_click_pipe_state() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1/state",
        ))
        .and(body_partial_json(serde_json::json!({"command": "stop"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "my-pipe",
            "state": "Stopped"
        })))
        .mount(&s)
        .await;

    let body = ClickPipeStatePatchRequest {
        command: Some(ClickPipeStatePatchRequestCommand::Stop),
    };
    let resp = c
        .click_pipe_state_update("org-1", "svc-1", "pipe-1", &body)
        .await
        .unwrap();
    let pipe = resp.result.unwrap();
    assert_eq!(pipe.state, Some(ClickPipeState::Stopped));
}

#[tokio::test]
async fn update_click_pipe_scaling() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1/scaling",
        ))
        .and(body_partial_json(serde_json::json!({})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "my-pipe",
            "state": "Running"
        })))
        .mount(&s)
        .await;

    let body = ClickPipeScalingPatchRequest {
        ..Default::default()
    };
    let resp = c
        .click_pipe_scaling_update("org-1", "svc-1", "pipe-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn get_click_pipe_settings() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1/settings",
        ))
        .respond_with(ok_json(serde_json::json!({})))
        .mount(&s)
        .await;

    let resp = c
        .click_pipe_settings_get("org-1", "svc-1", "pipe-1")
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_click_pipe_settings() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1/settings",
        ))
        .and(body_partial_json(serde_json::json!({})))
        .respond_with(ok_json(serde_json::json!({})))
        .mount(&s)
        .await;

    let body = ClickPipeSettingsPutRequest {
        kafka_read_committed: Some(true),
        ..Default::default()
    };
    let resp = c
        .click_pipe_settings_update("org-1", "svc-1", "pipe-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

// ===========================================================================
// ClickPipes Schema Discovery (Beta)
// ===========================================================================

#[tokio::test]
async fn click_pipe_schema_discovery_kafka() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipes/schemaDiscovery",
        ))
        .and(body_partial_json(serde_json::json!({
            "source": {"kafka": {"brokers": "broker1:9092"}}
        })))
        .respond_with(ok_json(serde_json::json!({
            "fields": [
                {"name": "user_id", "type": "Int64", "optional": false},
                {"name": "event", "type": "String", "optional": true}
            ]
        })))
        .mount(&s)
        .await;

    let body = ClickPipeSchemaDiscoveryRequest {
        source: ClickPipeSchemaDiscoverySource {
            kafka: Some(ClickPipePostKafkaSource {
                brokers: "broker1:9092".to_string(),
                ..Default::default()
            }),
            kinesis: None,
            object_storage: None,
            pubsub: None,
        },
    };
    let resp = c
        .click_pipe_schema_discovery("org-1", "svc-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    let fields = result.fields.expect("fields should populate");
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name.as_deref(), Some("user_id"));
    assert_eq!(fields[0].r#type.as_deref(), Some("Int64"));
    assert_eq!(fields[1].optional, Some(true));
}

// ===========================================================================
// ClickPipes CDC Scaling
// ===========================================================================

#[tokio::test]
async fn get_cdc_scaling() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipesCdcScaling",
        ))
        .respond_with(ok_json(serde_json::json!({
            "replicaCpuMillicores": 2000,
            "replicaMemoryGb": 8.0
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_pipe_cdc_scaling_get("org-1", "svc-1")
        .await
        .unwrap();
    let scaling = resp.result.unwrap();
    assert_eq!(scaling.replica_cpu_millicores, Some(2000));
    assert_eq!(scaling.replica_memory_gb, Some(8.0));
}

#[tokio::test]
async fn update_cdc_scaling() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipesCdcScaling",
        ))
        .and(body_partial_json(
            serde_json::json!({"replicaCpuMillicores": 4000, "replicaMemoryGb": 16.0}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "replicaCpuMillicores": 4000,
            "replicaMemoryGb": 16.0
        })))
        .mount(&s)
        .await;

    let body = ClickPipesCdcScalingPatchRequest {
        replica_cpu_millicores: Some(4000),
        replica_memory_gb: Some(16.0),
    };
    let resp = c
        .click_pipe_cdc_scaling_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    let scaling = resp.result.unwrap();
    assert_eq!(scaling.replica_cpu_millicores, Some(4000));
}

// ===========================================================================
// ClickPipes Reverse Private Endpoints
// ===========================================================================

#[tokio::test]
async fn list_reverse_private_endpoints() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipesReversePrivateEndpoints",
        ))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "description": "MSK endpoint",
                "status": "available"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c
        .click_pipe_reverse_private_endpoint_get_list("org-1", "svc-1")
        .await
        .unwrap();
    let endpoints = resp.result.unwrap();
    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].description.as_deref(), Some("MSK endpoint"));
}

#[tokio::test]
async fn create_reverse_private_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipesReversePrivateEndpoints",
        ))
        .and(body_partial_json(
            serde_json::json!({"description": "New RPE"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "description": "New RPE",
            "status": "creating"
        })))
        .mount(&s)
        .await;

    let body = CreateReversePrivateEndpoint {
        description: "New RPE".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_pipe_reverse_private_endpoint_create("org-1", "svc-1", &body)
        .await
        .unwrap();
    let rpe = resp.result.unwrap();
    assert_eq!(rpe.description.as_deref(), Some("New RPE"));
}

#[tokio::test]
async fn get_reverse_private_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipesReversePrivateEndpoints/rpe-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "description": "My RPE",
            "status": "available"
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_pipe_reverse_private_endpoint_get("org-1", "svc-1", "rpe-1")
        .await
        .unwrap();
    let rpe = resp.result.unwrap();
    assert_eq!(rpe.description.as_deref(), Some("My RPE"));
}

#[tokio::test]
async fn delete_reverse_private_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickpipesReversePrivateEndpoints/rpe-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_pipe_reverse_private_endpoint_delete("org-1", "svc-1", "rpe-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

#[tokio::test]
async fn clickpipes_context_returns_workload_identity_and_tolerates_omissions() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/services/service/clickpipes/context"))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({"gcpWorkloadIdentity": {
            "supported": true, "ready": null, "principal": "clickpipes@example.iam.gserviceaccount.com"
        }})))
        .expect(1).mount(&server).await;
    let result = client
        .click_pipes_service_context_get("org", "service")
        .await
        .unwrap()
        .result
        .unwrap();
    let identity = result.gcp_workload_identity.unwrap();
    assert_eq!(identity.supported, Some(true));
    assert_eq!(identity.ready, None);
    assert_eq!(
        identity.principal.as_deref(),
        Some("clickpipes@example.iam.gserviceaccount.com")
    );
}
