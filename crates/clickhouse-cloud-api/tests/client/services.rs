use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{basic_auth, body_json, body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{ok_empty, ok_json, setup};

// ===========================================================================
// Services (instances)
// ===========================================================================

#[tokio::test]
async fn list_services() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-123/services"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": [
                {
                    "id": "11111111-2222-3333-4444-555555555555",
                    "name": "svc-1",
                    "provider": "aws",
                    "region": "us-east-1",
                    "state": "running",
                    "tier": "production"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client.instance_get_list("org-123", &[]).await.unwrap();
    let services = resp.result.unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].name.as_deref(), Some("svc-1"));
    assert_eq!(services[0].provider, Some(ServiceProvider::Aws));
    assert_eq!(services[0].state, Some(ServiceState::Running));
}

#[tokio::test]
async fn create_service() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-123/services"))
        .and(body_partial_json(
            serde_json::json!({"name": "new-service", "provider": "aws", "region": "us-east-1"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "service": {
                    "id": "22222222-3333-4444-5555-666666666666",
                    "name": "new-service",
                    "provider": "aws",
                    "region": "us-east-1",
                    "state": "provisioning",
                    "tier": "production"
                },
                "password": "generated-password-123"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let body = ServicePostRequest {
        name: "new-service".to_string(),
        provider: ServicePostRequestProvider::Aws,
        region: ServicePostRequestRegion::Us_east_1,
        #[cfg(feature = "deprecated-fields")]
        tier: Some(ServicePostRequestTier::Production),
        ..Default::default()
    };
    let resp = client.instance_create("org-123", &body).await.unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.password.as_deref(), Some("generated-password-123"));
}

#[tokio::test]
async fn get_service_details() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "id": "11111111-2222-3333-4444-555555555555",
                "name": "prod-service",
                "provider": "gcp",
                "region": "us-east1",
                "state": "running",
                "tier": "production",
                "numReplicas": 3,
                "endpoints": [
                    {
                        "protocol": "https",
                        "host": "abc.clickhouse.cloud",
                        "port": 8443
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client.instance_get("org-1", "svc-1").await.unwrap();
    let svc = resp.result.unwrap();
    assert_eq!(svc.name.as_deref(), Some("prod-service"));
    assert_eq!(svc.provider, Some(ServiceProvider::Gcp));
    assert_eq!(svc.num_replicas, Some(3));
}

#[tokio::test]
async fn update_service() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .and(body_partial_json(
            serde_json::json!({"name": "renamed-svc"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "11111111-2222-3333-4444-555555555555",
            "name": "renamed-svc",
            "state": "running"
        })))
        .mount(&s)
        .await;

    let body = ServicePatchRequest {
        name: Some("renamed-svc".to_string()),
        ..Default::default()
    };
    let resp = c.instance_update("org-1", "svc-1", &body).await.unwrap();
    let svc = resp.result.unwrap();
    assert_eq!(svc.name.as_deref(), Some("renamed-svc"));
}

#[tokio::test]
async fn delete_service() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-123/services/svc-456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "req-del-123"
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client.instance_delete("org-123", "svc-456").await.unwrap();
    assert_eq!(resp.status, Some(200));
}

#[tokio::test]
async fn update_service_state() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-123/services/svc-456/state"))
        .and(body_partial_json(serde_json::json!({"command": "stop"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "id": "11111111-2222-3333-4444-555555555555",
                "name": "my-svc",
                "state": "stopping"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let body = ServiceStatePatchRequest {
        command: Some(ServiceStatePatchRequestCommand::Stop),
    };
    let resp = client
        .instance_state_update("org-123", "svc-456", &body)
        .await
        .unwrap();
    let svc = resp.result.unwrap();
    assert_eq!(svc.state, Some(ServiceState::Stopping));
}

#[tokio::test]
async fn update_service_password() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/password"))
        .and(body_partial_json(serde_json::json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "password": "new-password-abc"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let body = ServicePasswordPatchRequest {
        ..Default::default()
    };
    let resp = client
        .instance_password_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.password.as_deref(), Some("new-password-abc"));
}

// ===========================================================================
// Service sub-resources: scaling, private endpoints, query endpoints, prometheus
// ===========================================================================

#[tokio::test]
async fn update_replica_scaling() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/replicaScaling"))
        .and(body_partial_json(serde_json::json!({"numReplicas": 5, "minReplicaMemoryGb": 16.0, "maxReplicaMemoryGb": 64.0})))
        .respond_with(ok_json(serde_json::json!({
            "id": "11111111-2222-3333-4444-555555555555",
            "name": "svc-1",
            "numReplicas": 5,
            "minReplicaMemoryGb": 16,
            "maxReplicaMemoryGb": 64
        })))
        .mount(&s)
        .await;

    let body = ServiceReplicaScalingPatchRequest {
        num_replicas: Some(5),
        min_replica_memory_gb: Some(16.0),
        max_replica_memory_gb: Some(64.0),
        ..Default::default()
    };
    let resp = c
        .instance_replica_scaling_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.num_replicas, Some(5));
}

#[tokio::test]
#[allow(deprecated)]
async fn update_scaling_deprecated() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/scaling"))
        .and(body_partial_json(serde_json::json!({"numReplicas": 3})))
        .respond_with(ok_json(serde_json::json!({
            "id": "11111111-2222-3333-4444-555555555555",
            "name": "svc-1",
            "numReplicas": 3
        })))
        .mount(&s)
        .await;

    let body = ServiceScalingPatchRequest {
        num_replicas: Some(3),
        ..Default::default()
    };
    let resp = c
        .instance_scaling_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    let svc = resp.result.unwrap();
    assert_eq!(svc.num_replicas, Some(3));
}

#[tokio::test]
async fn create_private_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/privateEndpoint",
        ))
        .and(body_partial_json(
            serde_json::json!({"id": "vpce-abc", "description": "My PE"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "pe-1",
            "description": "My PE"
        })))
        .mount(&s)
        .await;

    let body = ServicPrivateEndpointePostRequest {
        id: "vpce-abc".to_string(),
        description: Some("My PE".to_string()),
    };
    let resp = c
        .instance_private_endpoint_create("org-1", "svc-1", &body)
        .await
        .unwrap();
    let pe = resp.result.unwrap();
    assert_eq!(pe.description.as_deref(), Some("My PE"));
}

#[tokio::test]
async fn get_private_endpoint_config_for_service() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/privateEndpointConfig",
        ))
        .respond_with(ok_json(serde_json::json!({
            "endpointServiceId": "vpce-svc-abc",
            "privateDnsHostname": "svc-1.private.clickhouse.cloud"
        })))
        .mount(&s)
        .await;

    let resp = c
        .instance_private_endpoint_config_get("org-1", "svc-1")
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.endpoint_service_id.as_deref(), Some("vpce-svc-abc"));
    assert_eq!(
        config.private_dns_hostname.as_deref(),
        Some("svc-1.private.clickhouse.cloud")
    );
}

#[tokio::test]
async fn get_service_prometheus_metrics() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/prometheus"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("# HELP svc_metric\nsvc_metric 100\n"),
        )
        .mount(&s)
        .await;

    let resp = c
        .instance_prometheus_get("org-1", "svc-1", None)
        .await
        .unwrap();
    assert!(resp.contains("svc_metric"));
}

#[tokio::test]
async fn get_service_prometheus_with_filter() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/prometheus"))
        .and(query_param("filtered_metrics", "cpu,memory"))
        .respond_with(ResponseTemplate::new(200).set_body_string("cpu 42\nmemory 1024\n"))
        .mount(&s)
        .await;

    let resp = c
        .instance_prometheus_get("org-1", "svc-1", Some("cpu,memory"))
        .await
        .unwrap();
    assert!(resp.contains("cpu 42"));
}

#[tokio::test]
async fn get_query_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/serviceQueryEndpoint",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "qe-1",
            "allowedOrigins": "*",
            "roles": ["sql_console_admin"]
        })))
        .mount(&s)
        .await;

    let resp = c
        .instance_query_endpoint_get("org-1", "svc-1")
        .await
        .unwrap();
    let qe = resp.result.unwrap();
    assert_eq!(qe.id.as_deref(), Some("qe-1"));
    assert_eq!(qe.allowed_origins.as_deref(), Some("*"));
    assert_eq!(qe.roles, Some(vec![QueryEndpointRole::SqlConsoleAdmin]));
}

#[tokio::test]
async fn upsert_query_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/serviceQueryEndpoint",
        ))
        .and(body_partial_json(serde_json::json!({
            "allowedOrigins": "https://example.com",
            "roles": ["sql_console_read_only"]
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "qe-1",
            "allowedOrigins": "https://example.com",
            "roles": ["sql_console_read_only"]
        })))
        .mount(&s)
        .await;

    let body = InstanceServiceQueryApiEndpointsPostRequest {
        allowed_origins: Some("https://example.com".to_string()),
        roles: vec![QueryEndpointRole::SqlConsoleReadOnly],
        ..Default::default()
    };
    let resp = c
        .instance_query_endpoint_upsert("org-1", "svc-1", &body)
        .await
        .unwrap();
    let qe = resp.result.unwrap();
    assert_eq!(qe.allowed_origins.as_deref(), Some("https://example.com"));
}

#[tokio::test]
async fn delete_query_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/serviceQueryEndpoint",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .instance_query_endpoint_delete("org-1", "svc-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

// ===========================================================================
// Empty collection responses
// ===========================================================================

#[tokio::test]
async fn list_services_returns_empty_vec() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c.instance_get_list("org-1", &[]).await.unwrap();
    let services = resp.result.unwrap();
    assert!(services.is_empty());
}

// ===========================================================================
// Query parameter coverage
// ===========================================================================

#[tokio::test]
async fn list_services_with_filters() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services"))
        .and(query_param("filter", "state=running"))
        .respond_with(ok_json(serde_json::json!([
            {"id": "11111111-2222-3333-4444-555555555555", "name": "svc-1", "state": "running"}
        ])))
        .mount(&s)
        .await;

    let resp = c
        .instance_get_list("org-1", &["state=running"])
        .await
        .unwrap();
    let services = resp.result.unwrap();
    assert_eq!(services.len(), 1);
}

#[tokio::test]
async fn list_services_with_multiple_filters() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services"))
        .and(query_param("filter", "state=running"))
        .and(query_param("filter", "tier=production"))
        .respond_with(ok_json(serde_json::json!([
            {"id": "11111111-2222-3333-4444-555555555555", "name": "svc-1", "state": "running"}
        ])))
        .mount(&s)
        .await;

    let resp = c
        .instance_get_list("org-1", &["state=running", "tier=production"])
        .await
        .unwrap();
    let services = resp.result.unwrap();
    assert_eq!(services.len(), 1);
}

#[tokio::test]
async fn scaling_schedule_delete_succeeds() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/scalingSchedule",
        ))
        .respond_with(ok_json(serde_json::json!({
            "status": 200,
            "requestId": "00000000-0000-0000-0000-000000000000"
        })))
        .mount(&s)
        .await;

    let resp = c.scaling_schedule_delete("org-1", "svc-1").await.unwrap();
    assert_eq!(resp.status, Some(200));
}

// ===========================================================================
// Service: Upgrade Window
// ===========================================================================

#[tokio::test]
async fn upgrade_window_get_returns_window() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/upgradeWindow"))
        .respond_with(ok_json(serde_json::json!({
            "weekday": 2,
            "startHourUtc": 6,
            "duration": 6
        })))
        .mount(&s)
        .await;

    let resp = c.upgrade_window_get("org-1", "svc-1").await.unwrap();
    let window = resp.result.unwrap();
    assert_eq!(window.weekday, Some(2));
    assert_eq!(
        window.start_hour_utc,
        Some(UpgradeWindowStartHourUtc::Hour6)
    );
    assert_eq!(window.duration, Some(UpgradeWindowDuration::SixHours));
}

#[tokio::test]
async fn upgrade_window_update_sends_body() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path("/v1/organizations/org-1/services/svc-1/upgradeWindow"))
        .and(body_partial_json(serde_json::json!({
            "weekday": 2,
            "startHourUtc": 6
        })))
        .respond_with(ok_json(serde_json::json!({
            "weekday": 2,
            "startHourUtc": 6,
            "duration": 6
        })))
        .mount(&s)
        .await;

    let body = UpgradeWindowPutRequest {
        weekday: 2,
        start_hour_utc: UpgradeWindowStartHourUtc::Hour6,
    };
    let resp = c
        .upgrade_window_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    let window = resp.result.unwrap();
    assert_eq!(window.weekday, Some(2));
    assert_eq!(
        window.start_hour_utc,
        Some(UpgradeWindowStartHourUtc::Hour6)
    );
}

#[tokio::test]
async fn upgrade_window_delete_succeeds() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/svc-1/upgradeWindow"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.upgrade_window_delete("org-1", "svc-1").await.unwrap();
    assert_eq!(resp.status, Some(200));
}

#[tokio::test]
async fn service_profiles_list_encodes_optional_region_and_byoc() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/serviceProfiles"))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(
            serde_json::json!([{"profile": "v1-standard-byoc-4", "cpuCores": 4, "memoryGi": 16}]),
        ))
        .expect(4)
        .mount(&server)
        .await;
    let region = "region +/=&?雪";
    let byoc = "byoc +/=&?雪";
    for (region_id, byoc_id) in [
        (None, None),
        (Some(region), None),
        (None, Some(byoc)),
        (Some(region), Some(byoc)),
    ] {
        let result = client
            .service_profiles_list("org", region_id, byoc_id)
            .await
            .unwrap()
            .result
            .unwrap();
        assert_eq!(result[0].profile.as_deref(), Some("v1-standard-byoc-4"));
        assert_eq!(result[0].cpu_cores, Some(4.0));
        assert_eq!(result[0].memory_gi, Some(16.0));
    }
    let requests = server.received_requests().await.unwrap();
    let queries: Vec<Vec<(String, String)>> = requests
        .iter()
        .map(|request| request.url.query_pairs().into_owned().collect())
        .collect();
    assert_eq!(
        queries,
        vec![
            vec![],
            vec![("region_id".into(), region.into())],
            vec![("byoc_id".into(), byoc.into())],
            vec![
                ("region_id".into(), region.into()),
                ("byoc_id".into(), byoc.into())
            ],
        ]
    );
}

#[tokio::test]
async fn service_clickhouse_settings_update_sends_and_receives_objects() {
    let (server, client) = setup().await;
    let settings = serde_json::json!({"compatibility": "26.2", "max_query_size": 262144});
    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickhouseSettings",
        ))
        .and(basic_auth("key", "secret"))
        .and(body_json(serde_json::json!({"settings": settings})))
        .respond_with(ok_json(
            serde_json::json!({"settings": settings, "warnings": []}),
        ))
        .expect(3)
        .mount(&server)
        .await;
    let body: ServiceClickhouseSettingsPatchRequest =
        serde_json::from_value(serde_json::json!({"settings": settings})).unwrap();
    let response = client
        .service_clickhouse_settings_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(response.result.unwrap().settings.unwrap()).unwrap(),
        settings
    );
    let legacy = ServiceClickhouseSettingsPatchRequest {
        settings: Some(settings.to_string()),
    };
    client
        .service_clickhouse_settings_update("org-1", "svc-1", &legacy)
        .await
        .unwrap();
    let typed = ServiceClickhouseSettingsPatchRequest {
        settings: Some(serde_json::from_value::<ServiceClickhouseSettingsMap>(settings).unwrap()),
    };
    client
        .service_clickhouse_settings_update("org-1", "svc-1", &typed)
        .await
        .unwrap();
}

#[tokio::test]
async fn service_clickhouse_settings_update_rejects_invalid_legacy_input_before_http() {
    let (server, client) = setup().await;
    for invalid in ["{broken", "{}", "[]", "null", "42", r#""string""#] {
        let body = ServiceClickhouseSettingsPatchRequest {
            settings: Some(invalid.to_string()),
        };
        assert!(
            client
                .service_clickhouse_settings_update("org-1", "svc-1", &body)
                .await
                .is_err()
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_clickhouse_setting_reads_preserve_numeric_and_string_values() {
    let (server, client) = setup().await;
    let collection = "/v1/organizations/org-1/services/svc-1/clickhouseSettings";
    let settings = serde_json::json!([
        {"name": "max_query_size", "value": 262146},
        {"name": "compatibility", "value": "26.2"},
        {"name": "future_bool", "value": false}
    ]);
    for setting in settings.as_array().unwrap() {
        let name = setting["name"].as_str().unwrap();
        Mock::given(method("GET"))
            .and(path(format!("{collection}/{name}")))
            .and(basic_auth("key", "secret"))
            .respond_with(ok_json(setting.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let response = client
            .service_clickhouse_setting_get("org-1", "svc-1", name)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(response.result.unwrap()).unwrap(),
            *setting
        );
    }
    Mock::given(method("GET"))
        .and(path(collection))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({"settings": settings})))
        .expect(1)
        .mount(&server)
        .await;
    let response = client
        .service_clickhouse_settings_list_get("org-1", "svc-1")
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(response.result.unwrap()).unwrap(),
        serde_json::json!({"settings": settings})
    );
}

// ---------------------------------------------------------------------------
// Replica aware routing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_replica_aware_routing() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/replicaAwareRouting",
        ))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({
            "httpEnabled": false,
            "nativeEnabled": true
        })))
        .expect(1)
        .mount(&s)
        .await;

    let resp = c.replica_aware_routing_get("org-1", "svc-1").await.unwrap();
    let routing = resp.result.unwrap();
    assert_eq!(routing.http_enabled, Some(false));
    assert_eq!(routing.native_enabled, Some(true));
}

#[tokio::test]
async fn update_replica_aware_routing_sends_only_set_fields() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/replicaAwareRouting",
        ))
        .and(basic_auth("key", "secret"))
        .and(body_json(serde_json::json!({"httpEnabled": true})))
        .respond_with(ok_json(serde_json::json!({
            "httpEnabled": true,
            "nativeEnabled": false
        })))
        .expect(1)
        .mount(&s)
        .await;

    let body = ReplicaAwareRoutingPatchRequest {
        http_enabled: Some(true),
        native_enabled: None,
    };
    let resp = c
        .replica_aware_routing_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    let routing = resp.result.unwrap();
    assert_eq!(routing.http_enabled, Some(true));
    assert_eq!(routing.native_enabled, Some(false));
}

#[tokio::test]
async fn replica_aware_routing_update_surfaces_api_errors() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/replicaAwareRouting",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": "Replica aware routing is not enabled for this organization"
        })))
        .mount(&s)
        .await;

    let body = ReplicaAwareRoutingPatchRequest {
        http_enabled: None,
        native_enabled: Some(true),
    };
    let err = c
        .replica_aware_routing_update("org-1", "svc-1", &body)
        .await
        .unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 400);
            assert_eq!(
                message,
                "Replica aware routing is not enabled for this organization"
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
