use clickhouse_cloud_api::{models::*, Client};
use wiremock::matchers::{basic_auth, bearer_token, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_organizations() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(basic_auth("test-key", "test-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "req-123",
            "result": [
                {
                    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                    "name": "Test Org"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "test-key", "test-secret");
    let resp = client.organization_get_list().await.unwrap();
    assert_eq!(resp.status, Some(200.0));
    let orgs = resp.result.unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].name, Some("Test Org".to_string()));
}

#[tokio::test]
async fn get_organization() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-123"))
        .and(basic_auth("my-key", "my-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                "name": "My Org",
                "createdAt": "2024-01-01T00:00:00Z"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "my-key", "my-secret");
    let resp = client.organization_get("org-123").await.unwrap();
    let org = resp.result.unwrap();
    assert_eq!(org.name, Some("My Org".to_string()));
}

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
    assert_eq!(services[0].name, Some("svc-1".to_string()));
    assert_eq!(services[0].provider, Some(ServiceProvider::Aws));
    assert_eq!(services[0].state, Some(ServiceState::Running));
}

#[tokio::test]
async fn create_service() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-123/services"))
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
        name: Some("new-service".to_string()),
        provider: Some(ServicePostRequestProvider::Aws),
        region: Some(ServicePostRequestRegion::Us_east_1),
        tier: Some(ServicePostRequestTier::Production),
        ..Default::default()
    };
    let resp = client.instance_create("org-123", &body).await.unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.password, Some("generated-password-123".to_string()));
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
    assert_eq!(resp.status, Some(200.0));
}

#[tokio::test]
async fn update_service_state() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-123/services/svc-456/state"))
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
async fn api_error_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "status": 401,
            "error": "Invalid credentials",
            "requestId": "req-err-123"
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "bad-key", "bad-secret");
    let err = client.organization_get_list().await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 401);
            assert_eq!(message, "Invalid credentials");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn list_backups() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/backups"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": [
                {
                    "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                    "status": "done",
                    "serviceId": "svc-1",
                    "startedAt": "2024-06-01T00:00:00Z",
                    "finishedAt": "2024-06-01T00:05:00Z",
                    "sizeInBytes": 1024,
                    "durationInSeconds": 300,
                    "type": "full"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client.backup_get_list("org-1", "svc-1").await.unwrap();
    let backups = resp.result.unwrap();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].status, Some(BackupStatus::Done));
    assert_eq!(backups[0].r#type, Some(BackupType::Full));
}

#[tokio::test]
async fn list_members() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": [
                {
                    "userId": "user-1",
                    "name": "Alice",
                    "email": "alice@example.com",
                    "role": "admin"
                },
                {
                    "userId": "user-2",
                    "name": "Bob",
                    "email": "bob@example.com",
                    "role": "developer"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client.member_get_list("org-1").await.unwrap();
    let members = resp.result.unwrap();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].role, Some(MemberRole::Admin));
    assert_eq!(members[1].role, Some(MemberRole::Developer));
}

#[tokio::test]
async fn list_api_keys() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": [
                {
                    "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                    "name": "Production Key",
                    "state": "enabled"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client.openapi_key_get_list("org-1").await.unwrap();
    let keys = resp.result.unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].name, Some("Production Key".to_string()));
}

#[tokio::test]
async fn get_usage_cost_with_query_params() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/usageCost"))
        .and(query_param("from_date", "2024-01-01"))
        .and(query_param("to_date", "2024-01-31"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "costs": [],
                "grandTotalCHC": 50.25
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client
        .usage_cost_get("org-1", "2024-01-01", "2024-01-31", &[])
        .await
        .unwrap();
    let cost = resp.result.unwrap();
    assert_eq!(cost.grand_total_chc, Some(50.25));
}

#[tokio::test]
async fn get_prometheus_metrics() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("# HELP ch_metric A metric\nch_metric{service=\"svc-1\"} 42\n"),
        )
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let resp = client
        .organization_prometheus_get("org-1", None)
        .await
        .unwrap();
    assert!(resp.contains("ch_metric"));
}

#[tokio::test]
async fn create_invitation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/invitations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "id": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
                "email": "newuser@example.com",
                "role": "developer"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let body = InvitationPostRequest {
        email: Some("newuser@example.com".to_string()),
        role: Some(InvitationPostRequestRole::Developer),
        ..Default::default()
    };
    let resp = client.invitation_create("org-1", &body).await.unwrap();
    let inv = resp.result.unwrap();
    assert_eq!(inv.email, Some("newuser@example.com".to_string()));
}

#[tokio::test]
async fn update_backup_configuration() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/backupConfiguration",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {
                "backupPeriodInHours": 12,
                "backupRetentionPeriodInHours": 336,
                "backupStartTime": "03:00"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_base_url(mock_server.uri(), "key", "secret");
    let body = BackupConfigurationPatchRequest {
        backup_period_in_hours: Some(12.0),
        backup_retention_period_in_hours: Some(336.0),
        backup_start_time: Some("03:00".to_string()),
    };
    let resp = client
        .backup_configuration_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.backup_period_in_hours, Some(12.0));
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
    assert_eq!(svc.name, Some("prod-service".to_string()));
    assert_eq!(svc.provider, Some(ServiceProvider::Gcp));
    assert_eq!(svc.num_replicas, Some(3.0));
}

#[tokio::test]
async fn update_service_password() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/password"))
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
    assert_eq!(result.password, Some("new-password-abc".to_string()));
}

#[tokio::test]
async fn bearer_auth_sends_token() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(bearer_token("my-oauth-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": [
                {
                    "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                    "name": "Bearer Org"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::with_bearer_token(mock_server.uri(), "my-oauth-token");
    let resp = client.organization_get_list().await.unwrap();
    let orgs = resp.result.unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].name, Some("Bearer Org".to_string()));
}

#[tokio::test]
async fn set_bearer_token_updates_token() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(bearer_token("refreshed-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": []
        })))
        .mount(&mock_server)
        .await;

    let mut client = Client::with_bearer_token(mock_server.uri(), "old-token");
    client.set_bearer_token("refreshed-token");
    let resp = client.organization_get_list().await.unwrap();
    assert_eq!(resp.result.unwrap().len(), 0);
}
