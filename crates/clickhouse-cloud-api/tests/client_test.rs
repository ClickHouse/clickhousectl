use chrono::Utc;
use clickhouse_cloud_api::{models::*, Client};
use wiremock::matchers::{basic_auth, bearer_token, body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn ok_json(result: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "req-test",
        "result": result
    }))
}

fn ok_empty() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "req-test"
    }))
}

async fn setup() -> (MockServer, Client) {
    let s = MockServer::start().await;
    let c = Client::with_base_url(s.uri(), "key", "secret");
    (s, c)
}

// ===========================================================================
// Organizations
// ===========================================================================

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
async fn update_organization() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1"))
        .and(body_partial_json(serde_json::json!({"name": "Renamed Org"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
            "name": "Renamed Org"
        })))
        .mount(&s)
        .await;

    let body = OrganizationPatchRequest {
        name: Some("Renamed Org".to_string()),
        ..Default::default()
    };
    let resp = c.organization_update("org-1", &body).await.unwrap();
    let org = resp.result.unwrap();
    assert_eq!(org.name, Some("Renamed Org".to_string()));
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
#[allow(deprecated)]
async fn get_private_endpoint_config() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/privateEndpointConfig"))
        .and(query_param("cloud_provider", "aws"))
        .and(query_param("region_id", "us-east-1"))
        .respond_with(ok_json(serde_json::json!({
            "endpointServiceId": "com.amazonaws.vpce.us-east-1.vpce-svc-abc"
        })))
        .mount(&s)
        .await;

    let resp = c
        .organization_private_endpoint_config_get_list("org-1", "aws", "us-east-1")
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(
        config.endpoint_service_id,
        Some("com.amazonaws.vpce.us-east-1.vpce-svc-abc".to_string())
    );
}

// ===========================================================================
// Activities
// ===========================================================================

#[tokio::test]
async fn list_activities() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activities"))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "act-1",
                "type": "SERVICE_START",
                "actorType": "user",
                "organizationId": "org-1",
                "serviceId": "svc-1"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.activity_get_list("org-1", None, None).await.unwrap();
    let activities = resp.result.unwrap();
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].id, Some("act-1".to_string()));
}

#[tokio::test]
async fn list_activities_with_date_filter() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activities"))
        .and(query_param("from_date", "2024-06-01"))
        .and(query_param("to_date", "2024-06-30"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c
        .activity_get_list("org-1", Some("2024-06-01"), Some("2024-06-30"))
        .await
        .unwrap();
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[tokio::test]
async fn get_activity() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activities/act-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "act-1",
            "type": "SERVICE_STOP",
            "actorType": "api"
        })))
        .mount(&s)
        .await;

    let resp = c.activity_get("org-1", "act-1").await.unwrap();
    let activity = resp.result.unwrap();
    assert_eq!(activity.id, Some("act-1".to_string()));
}

// ===========================================================================
// BYOC Infrastructure
// ===========================================================================

#[tokio::test]
async fn create_byoc_infrastructure() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/byocInfrastructure"))
        .and(body_partial_json(serde_json::json!({"accountId": "123456789012", "displayName": "My BYOC"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "byoc-1",
            "cloudProvider": "aws",
            "displayName": "My BYOC"
        })))
        .mount(&s)
        .await;

    let body = ByocInfrastructurePostRequest {
        account_id: Some("123456789012".to_string()),
        display_name: Some("My BYOC".to_string()),
        ..Default::default()
    };
    let resp = c
        .organization_byoc_infrastructure_create("org-1", &body)
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.display_name, Some("My BYOC".to_string()));
}

#[tokio::test]
async fn update_byoc_infrastructure() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/byoc-1"))
        .and(body_partial_json(serde_json::json!({"displayName": "Renamed BYOC"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "byoc-1",
            "displayName": "Renamed BYOC"
        })))
        .mount(&s)
        .await;

    let body = ByocInfrastructurePatchRequest {
        display_name: Some("Renamed BYOC".to_string()),
    };
    let resp = c
        .organization_byoc_infrastructure_update("org-1", "byoc-1", &body)
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.display_name, Some("Renamed BYOC".to_string()));
}

#[tokio::test]
async fn delete_byoc_infrastructure() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/byoc-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .organization_byoc_infrastructure_delete("org-1", "byoc-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200.0));
}

// ===========================================================================
// Invitations
// ===========================================================================

#[tokio::test]
async fn list_invitations() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/invitations"))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "email": "alice@example.com",
                "role": "developer"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.invitation_get_list("org-1").await.unwrap();
    let invitations = resp.result.unwrap();
    assert_eq!(invitations.len(), 1);
    assert_eq!(invitations[0].email, Some("alice@example.com".to_string()));
}

#[tokio::test]
async fn create_invitation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/invitations"))
        .and(body_partial_json(serde_json::json!({"email": "newuser@example.com", "role": "developer"})))
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
async fn get_invitation() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/invitations/inv-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
            "email": "bob@example.com",
            "role": "admin"
        })))
        .mount(&s)
        .await;

    let resp = c.invitation_get("org-1", "inv-1").await.unwrap();
    let inv = resp.result.unwrap();
    assert_eq!(inv.email, Some("bob@example.com".to_string()));
    assert_eq!(inv.role, Some(InvitationRole::Admin));
}

#[tokio::test]
async fn delete_invitation() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/invitations/inv-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.invitation_delete("org-1", "inv-1").await.unwrap();
    assert_eq!(resp.status, Some(200.0));
}

// ===========================================================================
// API Keys
// ===========================================================================

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
async fn create_api_key() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/keys"))
        .and(body_partial_json(serde_json::json!({"name": "New Key"})))
        .respond_with(ok_json(serde_json::json!({
            "key": {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "name": "New Key",
                "state": "enabled"
            },
            "keyId": "key-id-abc",
            "keySecret": "key-secret-xyz"
        })))
        .mount(&s)
        .await;

    let body = ApiKeyPostRequest {
        name: Some("New Key".to_string()),
        ..Default::default()
    };
    let resp = c.openapi_key_create("org-1", &body).await.unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.key_id, Some("key-id-abc".to_string()));
    assert_eq!(result.key_secret, Some("key-secret-xyz".to_string()));
    assert_eq!(result.key.unwrap().name, Some("New Key".to_string()));
}

#[tokio::test]
async fn get_api_key() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/keys/key-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "My Key",
            "state": "enabled",
            "keySuffix": "abc"
        })))
        .mount(&s)
        .await;

    let resp = c.openapi_key_get("org-1", "key-1").await.unwrap();
    let key = resp.result.unwrap();
    assert_eq!(key.name, Some("My Key".to_string()));
    assert_eq!(key.state, Some(ApiKeyState::Enabled));
    assert_eq!(key.key_suffix, Some("abc".to_string()));
}

#[tokio::test]
async fn update_api_key() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/keys/key-1"))
        .and(body_partial_json(serde_json::json!({"name": "Renamed Key"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "Renamed Key",
            "state": "enabled"
        })))
        .mount(&s)
        .await;

    let body = ApiKeyPatchRequest {
        name: Some("Renamed Key".to_string()),
        ..Default::default()
    };
    let resp = c.openapi_key_update("org-1", "key-1", &body).await.unwrap();
    let key = resp.result.unwrap();
    assert_eq!(key.name, Some("Renamed Key".to_string()));
}

#[tokio::test]
async fn delete_api_key() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/keys/key-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.openapi_key_delete("org-1", "key-1").await.unwrap();
    assert_eq!(resp.status, Some(200.0));
}

// ===========================================================================
// Members
// ===========================================================================

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
async fn get_member() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/members/user-1"))
        .respond_with(ok_json(serde_json::json!({
            "userId": "user-1",
            "name": "Alice",
            "email": "alice@example.com",
            "role": "admin"
        })))
        .mount(&s)
        .await;

    let resp = c.member_get("org-1", "user-1").await.unwrap();
    let member = resp.result.unwrap();
    assert_eq!(member.name, Some("Alice".to_string()));
    assert_eq!(member.role, Some(MemberRole::Admin));
}

#[tokio::test]
async fn update_member() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/members/user-1"))
        .and(body_partial_json(serde_json::json!({"role": "admin"})))
        .respond_with(ok_json(serde_json::json!({
            "userId": "user-1",
            "name": "Alice",
            "email": "alice@example.com",
            "role": "admin"
        })))
        .mount(&s)
        .await;

    let body = MemberPatchRequest {
        role: Some(MemberPatchRequestRole::Admin),
        ..Default::default()
    };
    let resp = c.member_update("org-1", "user-1", &body).await.unwrap();
    let member = resp.result.unwrap();
    assert_eq!(member.role, Some(MemberRole::Admin));
}

#[tokio::test]
async fn delete_member() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/members/user-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.member_delete("org-1", "user-1").await.unwrap();
    assert_eq!(resp.status, Some(200.0));
}

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
    assert_eq!(services[0].name, Some("svc-1".to_string()));
    assert_eq!(services[0].provider, Some(ServiceProvider::Aws));
    assert_eq!(services[0].state, Some(ServiceState::Running));
}

#[tokio::test]
async fn create_service() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-123/services"))
        .and(body_partial_json(serde_json::json!({"name": "new-service", "provider": "aws", "region": "us-east-1", "tier": "production"})))
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
async fn update_service() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1"))
        .and(body_partial_json(serde_json::json!({"name": "renamed-svc"})))
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
    assert_eq!(svc.name, Some("renamed-svc".to_string()));
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
    assert_eq!(result.password, Some("new-password-abc".to_string()));
}

// ===========================================================================
// Service sub-resources: scaling, private endpoints, query endpoints, prometheus
// ===========================================================================

#[tokio::test]
async fn update_replica_scaling() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/replicaScaling"))
        .and(body_partial_json(serde_json::json!({"numReplicas": 5.0, "minReplicaMemoryGb": 16.0, "maxReplicaMemoryGb": 64.0})))
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
        num_replicas: Some(5.0),
        min_replica_memory_gb: Some(16.0),
        max_replica_memory_gb: Some(64.0),
        ..Default::default()
    };
    let resp = c
        .instance_replica_scaling_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.num_replicas, Some(5.0));
}

#[tokio::test]
#[allow(deprecated)]
async fn update_scaling_deprecated() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/scaling"))
        .and(body_partial_json(serde_json::json!({"numReplicas": 3.0})))
        .respond_with(ok_json(serde_json::json!({
            "id": "11111111-2222-3333-4444-555555555555",
            "name": "svc-1",
            "numReplicas": 3
        })))
        .mount(&s)
        .await;

    let body = ServiceScalingPatchRequest {
        num_replicas: Some(3.0),
        ..Default::default()
    };
    let resp = c
        .instance_scaling_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    let svc = resp.result.unwrap();
    assert_eq!(svc.num_replicas, Some(3.0));
}

#[tokio::test]
async fn create_private_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services/svc-1/privateEndpoint"))
        .and(body_partial_json(serde_json::json!({"id": "vpce-abc", "description": "My PE"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "pe-1",
            "description": "My PE"
        })))
        .mount(&s)
        .await;

    let body = ServicPrivateEndpointePostRequest {
        id: Some("vpce-abc".to_string()),
        description: Some("My PE".to_string()),
    };
    let resp = c
        .instance_private_endpoint_create("org-1", "svc-1", &body)
        .await
        .unwrap();
    let pe = resp.result.unwrap();
    assert_eq!(pe.description, Some("My PE".to_string()));
}

#[tokio::test]
async fn get_private_endpoint_config_for_service() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/privateEndpointConfig"))
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
    assert_eq!(config.endpoint_service_id, Some("vpce-svc-abc".to_string()));
    assert_eq!(
        config.private_dns_hostname,
        Some("svc-1.private.clickhouse.cloud".to_string())
    );
}

#[tokio::test]
async fn get_service_prometheus_metrics() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/prometheus"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("# HELP svc_metric\nsvc_metric 100\n"),
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("cpu 42\nmemory 1024\n"),
        )
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
        .and(path("/v1/organizations/org-1/services/svc-1/serviceQueryEndpoint"))
        .respond_with(ok_json(serde_json::json!({
            "id": "qe-1",
            "allowedOrigins": "*",
            "roles": ["admin"]
        })))
        .mount(&s)
        .await;

    let resp = c
        .instance_query_endpoint_get("org-1", "svc-1")
        .await
        .unwrap();
    let qe = resp.result.unwrap();
    assert_eq!(qe.id, Some("qe-1".to_string()));
    assert_eq!(qe.allowed_origins, Some("*".to_string()));
}

#[tokio::test]
async fn upsert_query_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services/svc-1/serviceQueryEndpoint"))
        .and(body_partial_json(serde_json::json!({"allowedOrigins": "https://example.com", "roles": ["reader"]})))
        .respond_with(ok_json(serde_json::json!({
            "id": "qe-1",
            "allowedOrigins": "https://example.com",
            "roles": ["reader"]
        })))
        .mount(&s)
        .await;

    let body = InstanceServiceQueryApiEndpointsPostRequest {
        allowed_origins: Some("https://example.com".to_string()),
        roles: Some(vec!["reader".to_string()]),
        ..Default::default()
    };
    let resp = c
        .instance_query_endpoint_upsert("org-1", "svc-1", &body)
        .await
        .unwrap();
    let qe = resp.result.unwrap();
    assert_eq!(
        qe.allowed_origins,
        Some("https://example.com".to_string())
    );
}

#[tokio::test]
async fn delete_query_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/svc-1/serviceQueryEndpoint"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .instance_query_endpoint_delete("org-1", "svc-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200.0));
}

// ===========================================================================
// Backups & Backup Configuration
// ===========================================================================

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
async fn get_single_backup() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/backups/bak-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "status": "done",
            "sizeInBytes": 2048,
            "type": "full"
        })))
        .mount(&s)
        .await;

    let resp = c.backup_get("org-1", "svc-1", "bak-1").await.unwrap();
    let backup = resp.result.unwrap();
    assert_eq!(backup.status, Some(BackupStatus::Done));
    assert_eq!(backup.size_in_bytes, Some(2048.0));
}

#[tokio::test]
async fn get_backup_configuration() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/backupConfiguration"))
        .respond_with(ok_json(serde_json::json!({
            "backupPeriodInHours": 24,
            "backupRetentionPeriodInHours": 168,
            "backupStartTime": "02:00"
        })))
        .mount(&s)
        .await;

    let resp = c
        .backup_configuration_get("org-1", "svc-1")
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.backup_period_in_hours, Some(24.0));
    assert_eq!(config.backup_start_time, Some("02:00".to_string()));
}

#[tokio::test]
async fn update_backup_configuration() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/backupConfiguration",
        ))
        .and(body_partial_json(serde_json::json!({"backupPeriodInHours": 12.0, "backupRetentionPeriodInHours": 336.0, "backupStartTime": "03:00"})))
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

// ===========================================================================
// Backup Buckets
// ===========================================================================

#[tokio::test]
async fn get_backup_bucket() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/backupBucket"))
        .respond_with(ok_json(serde_json::json!({
            "bucketPath": "s3://my-backup-bucket/prefix",
            "bucketProvider": "aws_s3",
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "roleArn": "arn:aws:iam::123:role/backup-role"
        })))
        .mount(&s)
        .await;

    let resp = c.backup_bucket_get("org-1", "svc-1").await.unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn create_backup_bucket() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services/svc-1/backupBucket"))
        .and(body_partial_json(serde_json::json!({"bucketPath": "s3://new-bucket"})))
        .respond_with(ok_json(serde_json::json!({
            "bucketPath": "s3://new-bucket",
            "bucketProvider": "aws_s3",
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        })))
        .mount(&s)
        .await;

    let body = BackupBucketPostRequest::AwsBackupBucketPostRequestV1(
        AwsBackupBucketPostRequestV1 {
            bucket_path: Some("s3://new-bucket".to_string()),
            ..Default::default()
        },
    );
    let resp = c
        .backup_bucket_create("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_backup_bucket() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/backupBucket"))
        .and(body_partial_json(serde_json::json!({"bucketPath": "s3://updated-bucket"})))
        .respond_with(ok_json(serde_json::json!({
            "bucketPath": "s3://updated-bucket",
            "bucketProvider": "aws_s3",
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        })))
        .mount(&s)
        .await;

    let body = BackupBucketPatchRequest::AwsBackupBucketPatchRequestV1(
        AwsBackupBucketPatchRequestV1 {
            bucket_path: Some("s3://updated-bucket".to_string()),
            ..Default::default()
        },
    );
    let resp = c
        .backup_bucket_update("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn delete_backup_bucket() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/svc-1/backupBucket"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.backup_bucket_delete("org-1", "svc-1").await.unwrap();
    assert_eq!(resp.status, Some(200.0));
}

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
    assert_eq!(pipes[0].name, Some("kafka-pipe".to_string()));
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
        name: Some("new-pipe".to_string()),
        ..Default::default()
    };
    let resp = c
        .click_pipe_create("org-1", "svc-1", &body)
        .await
        .unwrap();
    let pipe = resp.result.unwrap();
    assert_eq!(pipe.name, Some("new-pipe".to_string()));
}

#[tokio::test]
async fn get_click_pipe() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "my-pipe",
            "state": "Running"
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_pipe_get("org-1", "svc-1", "pipe-1")
        .await
        .unwrap();
    let pipe = resp.result.unwrap();
    assert_eq!(pipe.name, Some("my-pipe".to_string()));
}

#[tokio::test]
async fn update_click_pipe() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1"))
        .and(body_partial_json(serde_json::json!({"name": "renamed-pipe"})))
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
    assert_eq!(pipe.name, Some("renamed-pipe".to_string()));
}

#[tokio::test]
async fn delete_click_pipe() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_pipe_delete("org-1", "svc-1", "pipe-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200.0));
}

#[tokio::test]
async fn update_click_pipe_state() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1/state"))
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
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1/scaling"))
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
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1/settings"))
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
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1/settings"))
        .and(body_partial_json(serde_json::json!({})))
        .respond_with(ok_json(serde_json::json!({})))
        .mount(&s)
        .await;

    let body = ClickPipeSettingsPutRequest {
        ..Default::default()
    };
    let resp = c
        .click_pipe_settings_update("org-1", "svc-1", "pipe-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

// ===========================================================================
// ClickPipes CDC Scaling
// ===========================================================================

#[tokio::test]
async fn get_cdc_scaling() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipesCdcScaling"))
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
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipesCdcScaling"))
        .and(body_partial_json(serde_json::json!({"replicaCpuMillicores": 4000, "replicaMemoryGb": 16.0})))
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
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipesReversePrivateEndpoints"))
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
    assert_eq!(endpoints[0].description, Some("MSK endpoint".to_string()));
}

#[tokio::test]
async fn create_reverse_private_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipesReversePrivateEndpoints"))
        .and(body_partial_json(serde_json::json!({"description": "New RPE"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "description": "New RPE",
            "status": "creating"
        })))
        .mount(&s)
        .await;

    let body = CreateReversePrivateEndpoint {
        description: Some("New RPE".to_string()),
        ..Default::default()
    };
    let resp = c
        .click_pipe_reverse_private_endpoint_create("org-1", "svc-1", &body)
        .await
        .unwrap();
    let rpe = resp.result.unwrap();
    assert_eq!(rpe.description, Some("New RPE".to_string()));
}

#[tokio::test]
async fn get_reverse_private_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipesReversePrivateEndpoints/rpe-1"))
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
    assert_eq!(rpe.description, Some("My RPE".to_string()));
}

#[tokio::test]
async fn delete_reverse_private_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipesReversePrivateEndpoints/rpe-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_pipe_reverse_private_endpoint_delete("org-1", "svc-1", "rpe-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200.0));
}

// ===========================================================================
// ClickStack: Alerts
// ===========================================================================

#[tokio::test]
async fn list_alerts() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/alerts"))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "alert-1",
                "name": "High CPU"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_list_alerts("org-1", "svc-1")
        .await
        .unwrap();
    let alerts = resp.result.unwrap();
    assert_eq!(alerts.len(), 1);
}

#[tokio::test]
async fn create_alert() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/alerts"))
        .and(body_partial_json(serde_json::json!({"name": "New Alert"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "alert-1",
            "name": "New Alert"
        })))
        .mount(&s)
        .await;

    let body = ClickStackCreateAlertRequest {
        name: Some("New Alert".to_string()),
        ..Default::default()
    };
    let resp = c
        .click_stack_create_alert("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn get_alert() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/alerts/alert-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "alert-1",
            "name": "My Alert"
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_get_alert("org-1", "svc-1", "alert-1")
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_alert() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/alerts/alert-1"))
        .and(body_partial_json(serde_json::json!({"name": "Updated Alert"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "alert-1",
            "name": "Updated Alert"
        })))
        .mount(&s)
        .await;

    let body = ClickStackUpdateAlertRequest {
        name: Some("Updated Alert".to_string()),
        ..Default::default()
    };
    let resp = c
        .click_stack_update_alert("org-1", "svc-1", "alert-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn delete_alert() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/alerts/alert-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_alert("org-1", "svc-1", "alert-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200.0));
}

// ===========================================================================
// ClickStack: Dashboards
// ===========================================================================

#[tokio::test]
async fn list_dashboards() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/dashboards"))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "dash-1",
                "name": "Overview"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_list_dashboards("org-1", "svc-1")
        .await
        .unwrap();
    let dashboards = resp.result.unwrap();
    assert_eq!(dashboards.len(), 1);
}

#[tokio::test]
async fn create_dashboard() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/dashboards"))
        .and(body_partial_json(serde_json::json!({"name": "New Dashboard", "tiles": []})))
        .respond_with(ok_json(serde_json::json!({
            "id": "dash-new",
            "name": "New Dashboard"
        })))
        .mount(&s)
        .await;

    let body = ClickStackCreateDashboardRequest {
        name: "New Dashboard".to_string(),
        tiles: vec![],
        ..Default::default()
    };
    let resp = c
        .click_stack_create_dashboard("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn get_dashboard() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/dashboards/dash-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "dash-1",
            "name": "My Dashboard"
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_get_dashboard("org-1", "svc-1", "dash-1")
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_dashboard() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/dashboards/dash-1"))
        .and(body_partial_json(serde_json::json!({"name": "Updated Dashboard", "tiles": []})))
        .respond_with(ok_json(serde_json::json!({
            "id": "dash-1",
            "name": "Updated Dashboard"
        })))
        .mount(&s)
        .await;

    let body = ClickStackUpdateDashboardRequest {
        name: "Updated Dashboard".to_string(),
        tiles: vec![],
        ..Default::default()
    };
    let resp = c
        .click_stack_update_dashboard("org-1", "svc-1", "dash-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn delete_dashboard() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/dashboards/dash-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_dashboard("org-1", "svc-1", "dash-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200.0));
}

// ===========================================================================
// ClickStack: Sources & Webhooks
// ===========================================================================

#[tokio::test]
async fn list_sources() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/sources"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_list_sources("org-1", "svc-1")
        .await
        .unwrap();
    let sources = resp.result.unwrap();
    assert_eq!(sources.len(), 0);
}

#[tokio::test]
async fn list_webhooks() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickstack/webhooks"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_list_webhooks("org-1", "svc-1")
        .await
        .unwrap();
    let webhooks = resp.result.unwrap();
    assert_eq!(webhooks.len(), 0);
}

// ===========================================================================
// PostgreSQL Services
// ===========================================================================

#[tokio::test]
async fn create_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/postgres"))
        .and(body_partial_json(serde_json::json!({"name": "pg-svc", "provider": "aws", "region": "us-east-1", "size": "c6gd.medium", "storageSize": 100})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "pg-svc",
            "state": "creating",
            "provider": "aws",
            "region": "us-east-1",
            "password": "generated-pw"
        })))
        .mount(&s)
        .await;

    let body = PostgresServicePostRequest {
        name: "pg-svc".to_string(),
        provider: PgProvider::Aws,
        region: "us-east-1".to_string(),
        size: PgSize::C6gd_medium,
        storage_size: 100,
        ..Default::default()
    };
    let resp = c
        .postgres_service_create("org-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name, Some("pg-svc".to_string()));
    assert_eq!(pg.password, Some("generated-pw".to_string()));
}

#[tokio::test]
async fn list_postgres_services() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres"))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                "name": "pg-1",
                "state": "running"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.postgres_service_get_list("org-1").await.unwrap();
    let services = resp.result.unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].name, Some("pg-1".to_string()));
}

#[tokio::test]
async fn get_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "pg-1",
            "state": "running",
            "connectionString": "postgres://user@host/db"
        })))
        .mount(&s)
        .await;

    let resp = c.postgres_service_get("org-1", "pg-1").await.unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name, Some("pg-1".to_string()));
    assert_eq!(
        pg.connection_string,
        Some("postgres://user@host/db".to_string())
    );
}

#[tokio::test]
async fn update_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1"))
        .and(body_partial_json(serde_json::json!({"name": "pg-renamed"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "pg-renamed"
        })))
        .mount(&s)
        .await;

    let body = PostgresServicePatchRequest {
        name: Some("pg-renamed".to_string()),
        ..Default::default()
    };
    let resp = c
        .postgres_service_patch("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name, Some("pg-renamed".to_string()));
}

#[tokio::test]
async fn delete_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/postgres/pg-1"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c.postgres_service_delete("org-1", "pg-1").await.unwrap();
    assert_eq!(resp.status, Some(200.0));
}

#[tokio::test]
async fn update_postgres_service_state() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/state"))
        .and(body_partial_json(serde_json::json!({"command": "restart"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "pg-1",
            "state": "restarting"
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceSetState {
        command: Some(PostgresServiceSetStateCommand::Restart),
    };
    let resp = c
        .postgres_service_patch_state("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name, Some("pg-1".to_string()));
}

#[tokio::test]
async fn set_postgres_password() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/password"))
        .and(body_partial_json(serde_json::json!({"password": "new-pg-password"})))
        .respond_with(ok_json(serde_json::json!({
            "password": "new-pg-password"
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceSetPassword {
        password: Some("new-pg-password".to_string()),
    };
    let resp = c
        .postgres_service_set_password("org-1", "pg-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.password, Some("new-pg-password".to_string()));
}

#[tokio::test]
async fn get_postgres_certs() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/caCertificates"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("-----BEGIN CERTIFICATE-----\nMIIC...\n-----END CERTIFICATE-----\n"),
        )
        .mount(&s)
        .await;

    let resp = c
        .postgres_service_certs_get("org-1", "pg-1")
        .await
        .unwrap();
    assert!(resp.contains("BEGIN CERTIFICATE"));
}

#[tokio::test]
async fn get_postgres_config() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .respond_with(ok_json(serde_json::json!({
            "pgConfig": {
                "max_connections": 100
            },
            "pgBouncerConfig": {}
        })))
        .mount(&s)
        .await;

    let resp = c
        .postgres_instance_config_get("org-1", "pg-1")
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.pg_config.max_connections, Some(100));
}

#[tokio::test]
async fn replace_postgres_config() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .and(body_partial_json(serde_json::json!({"pgConfig": {"max_connections": 200}, "pgBouncerConfig": {}})))
        .respond_with(ok_json(serde_json::json!({
            "message": "Configuration updated",
            "pgConfig": { "max_connections": 200 },
            "pgBouncerConfig": {}
        })))
        .mount(&s)
        .await;

    let body = PostgresInstanceConfig {
        pg_config: PgConfig {
            max_connections: Some(200),
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig {},
    };
    let resp = c
        .postgres_instance_config_post("org-1", "pg-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.message, Some("Configuration updated".to_string()));
    assert_eq!(result.pg_config.max_connections, Some(200));
}

#[tokio::test]
async fn patch_postgres_config() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .and(body_partial_json(serde_json::json!({"pgConfig": {"max_connections": 150}, "pgBouncerConfig": {}})))
        .respond_with(ok_json(serde_json::json!({
            "message": "OK",
            "pgConfig": { "max_connections": 150 },
            "pgBouncerConfig": {}
        })))
        .mount(&s)
        .await;

    let body = PostgresInstanceConfig {
        pg_config: PgConfig {
            max_connections: Some(150),
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig {},
    };
    let resp = c
        .postgres_instance_config_patch("org-1", "pg-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.pg_config.max_connections, Some(150));
}

#[tokio::test]
async fn create_postgres_read_replica() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/readReplica"))
        .and(body_partial_json(serde_json::json!({"name": "pg-1-replica"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
            "name": "pg-1-replica",
            "isPrimary": false
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceReadReplicaRequest {
        name: "pg-1-replica".to_string(),
        ..Default::default()
    };
    let resp = c
        .postgres_instance_create_read_replica("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.is_primary, Some(false));
}

#[tokio::test]
async fn restore_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/restoredService"))
        .and(body_partial_json(serde_json::json!({"name": "pg-1-restored"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "cccccccc-dddd-eeee-ffff-000000000000",
            "name": "pg-1-restored",
            "state": "creating"
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceRestoreRequest {
        name: "pg-1-restored".to_string(),
        restore_target: Utc::now(),
        ..Default::default()
    };
    let resp = c
        .postgres_instance_restore("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name, Some("pg-1-restored".to_string()));
}

// ===========================================================================
// Authentication
// ===========================================================================

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

#[tokio::test]
async fn with_http_client_basic_auth() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(basic_auth("custom-key", "custom-secret"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&mock_server)
        .await;

    let http = reqwest::Client::new();
    let client =
        Client::with_http_client(http, mock_server.uri(), "custom-key", "custom-secret");
    let resp = client.organization_get_list().await.unwrap();
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[tokio::test]
async fn with_http_client_bearer_auth() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .and(bearer_token("custom-bearer"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&mock_server)
        .await;

    let http = reqwest::Client::new();
    let client = Client::with_http_client_bearer(http, mock_server.uri(), "custom-bearer");
    let resp = client.organization_get_list().await.unwrap();
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[test]
#[should_panic(expected = "set_bearer_token called on a Basic-auth client")]
fn set_bearer_token_panics_on_basic_auth() {
    let mut client = Client::new("key", "secret");
    client.set_bearer_token("token");
}

// ===========================================================================
// Error handling
// ===========================================================================

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
async fn api_error_403_forbidden() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "status": 403,
            "error": "Forbidden: insufficient permissions"
        })))
        .mount(&s)
        .await;

    let body = ServicePostRequest {
        name: Some("test".to_string()),
        ..Default::default()
    };
    let err = c.instance_create("org-1", &body).await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 403);
            assert!(message.contains("Forbidden"));
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_404_not_found() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "Service not found"
        })))
        .mount(&s)
        .await;

    let err = c
        .instance_get("org-1", "nonexistent")
        .await
        .unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 404);
            assert_eq!(message, "Service not found");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_500_server_error() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "status": 500,
            "error": "Internal server error"
        })))
        .mount(&s)
        .await;

    let err = c.organization_get_list().await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 500);
            assert_eq!(message, "Internal server error");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_non_json_body() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(
            ResponseTemplate::new(502).set_body_string("Bad Gateway"),
        )
        .mount(&s)
        .await;

    let err = c.organization_get_list().await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 502);
            // Falls back to raw body text when JSON parsing fails
            assert_eq!(message, "Bad Gateway");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_on_prometheus_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "status": 403,
                "error": "Metrics access denied"
            })),
        )
        .mount(&s)
        .await;

    let err = c
        .organization_prometheus_get("org-1", None)
        .await
        .unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 403);
            assert_eq!(message, "Metrics access denied");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_on_postgres_certs_endpoint() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/caCertificates"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
        .mount(&s)
        .await;

    let err = c
        .postgres_service_certs_get("org-1", "pg-1")
        .await
        .unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, .. } => {
            assert_eq!(status, 404);
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

// ===========================================================================
// Malformed responses (Error::Json coverage)
// ===========================================================================

#[tokio::test]
async fn malformed_json_success_response() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not json"))
        .mount(&s)
        .await;

    let err = c.organization_get_list().await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Json(_) => {} // expected
        other => panic!("Expected Json error, got: {:?}", other),
    }
}

#[tokio::test]
async fn truncated_json_success_response() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"status": 200, "result":"#),
        )
        .mount(&s)
        .await;

    let err = c.organization_get_list().await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Json(_) => {} // expected
        other => panic!("Expected Json error, got: {:?}", other),
    }
}

// ===========================================================================
// Additional HTTP status codes
// ===========================================================================

#[tokio::test]
async fn api_error_429_rate_limited() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "status": 429,
            "error": "Rate limit exceeded"
        })))
        .mount(&s)
        .await;

    let err = c.organization_get_list().await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 429);
            assert_eq!(message, "Rate limit exceeded");
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn api_error_422_validation() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/services"))
        .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
            "status": 422,
            "error": "Validation failed: name is required"
        })))
        .mount(&s)
        .await;

    let body = ServicePostRequest::default();
    let err = c.instance_create("org-1", &body).await.unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 422);
            assert!(message.contains("Validation failed"));
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }
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

#[tokio::test]
async fn list_organizations_returns_empty_vec() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c.organization_get_list().await.unwrap();
    let orgs = resp.result.unwrap();
    assert!(orgs.is_empty());
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
async fn usage_cost_with_filters() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/usageCost"))
        .and(query_param("from_date", "2024-01-01"))
        .and(query_param("to_date", "2024-01-31"))
        .and(query_param("filter", "service_id=svc-1"))
        .respond_with(ok_json(serde_json::json!({
            "costs": [],
            "grandTotalCHC": 10.0
        })))
        .mount(&s)
        .await;

    let resp = c
        .usage_cost_get("org-1", "2024-01-01", "2024-01-31", &["service_id=svc-1"])
        .await
        .unwrap();
    let cost = resp.result.unwrap();
    assert_eq!(cost.grand_total_chc, Some(10.0));
}

#[tokio::test]
async fn activity_list_with_only_from_date() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activities"))
        .and(query_param("from_date", "2024-06-01"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c
        .activity_get_list("org-1", Some("2024-06-01"), None)
        .await
        .unwrap();
    let activities = resp.result.unwrap();
    assert!(activities.is_empty());
}

#[tokio::test]
async fn organization_prometheus_with_filtered_metrics() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus"))
        .and(query_param("filtered_metrics", "cpu,memory"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("cpu 42\nmemory 1024\n"),
        )
        .mount(&s)
        .await;

    let resp = c
        .organization_prometheus_get("org-1", Some("cpu,memory"))
        .await
        .unwrap();
    assert!(resp.contains("cpu 42"));
}

#[tokio::test]
async fn organization_prometheus_without_filter() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("# HELP metric\nmetric 1\n"),
        )
        .mount(&s)
        .await;

    let resp = c
        .organization_prometheus_get("org-1", None)
        .await
        .unwrap();
    assert!(resp.contains("metric"));
}

// ===========================================================================
// Base URL handling
// ===========================================================================

#[tokio::test]
async fn base_url_trailing_slash_stripped() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&mock_server)
        .await;

    let url_with_slash = format!("{}/", mock_server.uri());
    let client = Client::with_base_url(url_with_slash, "key", "secret");
    let resp = client.organization_get_list().await.unwrap();
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[tokio::test]
async fn default_base_url_is_production() {
    // Client::new() uses https://api.clickhouse.cloud -- we can't hit it,
    // but we can verify the client is constructable without panicking.
    let _client = Client::new("key", "secret");
}
