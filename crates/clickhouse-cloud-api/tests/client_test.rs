use chrono::Utc;
use clickhouse_cloud_api::{Client, models::*};
use wiremock::matchers::{
    basic_auth, bearer_token, body_json, body_partial_json, method, path, query_param,
};
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

fn created_json(result: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(201).set_body_json(serde_json::json!({
        "status": 201,
        "requestId": "req-test",
        "result": result
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
    assert_eq!(resp.status, Some(200));
    let orgs = resp.result.unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].name.as_deref(), Some("Test Org"));
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
    assert_eq!(org.name.as_deref(), Some("My Org"));
}

#[tokio::test]
async fn get_active_balances_with_pagination() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activeBalances"))
        .and(query_param("limit", "25"))
        .and(query_param("offset", "50"))
        .respond_with(ok_json(serde_json::json!({
            "totalRemainingPrepaidCredits": 12.5,
            "prepaidBalances": [{
                "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                "remainingPrepaidCredits": 12.5,
                "expirationDate": "2027-01-01T00:00:00Z"
            }]
        })))
        .mount(&s)
        .await;

    let balances = c
        .active_balances_get("org-1", Some(25), Some(50))
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(balances.total_remaining_prepaid_credits, Some(12.5));
    assert_eq!(balances.prepaid_balances.unwrap().len(), 1);
}

#[tokio::test]
async fn update_organization() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1"))
        .and(body_partial_json(
            serde_json::json!({"name": "Renamed Org"}),
        ))
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
    assert_eq!(org.name.as_deref(), Some("Renamed Org"));
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
async fn discover_organization_prometheus_targets() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/prometheus/discovery"))
        .and(query_param("filtered_metrics", "false"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                "targets": ["api.clickhouse.cloud"],
                "labels": {
                    "__scheme__": "https",
                    "__metrics_path__": "/v1/organizations/org-1/services/svc-1/prometheus",
                    "__param_filtered_metrics": "false",
                    "clickhouse_org_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                    "clickhouse_service_id": "b1b2c3d4-e5f6-7890-abcd-ef1234567890",
                    "clickhouse_discovery_service_name": "analytics"
                }
            }])),
        )
        .mount(&s)
        .await;

    let groups = c
        .organization_prometheus_discovery_get("org-1", Some("false"))
        .await
        .unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups[0]
            .labels
            .as_ref()
            .and_then(|labels| labels.scheme.as_deref()),
        Some("https")
    );
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
        config.endpoint_service_id.as_deref(),
        Some("com.amazonaws.vpce.us-east-1.vpce-svc-abc")
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
    assert_eq!(activities[0].id.as_deref(), Some("act-1"));
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
    assert_eq!(activity.id.as_deref(), Some("act-1"));
}

// ===========================================================================
// BYOC Infrastructure
// ===========================================================================

#[tokio::test]
async fn create_byoc_infrastructure() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/byocInfrastructure"))
        .and(body_partial_json(serde_json::json!({
            "accountId": "123456789012",
            "availabilityZoneSuffixes": ["a", "b"],
            "displayName": "My BYOC"
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "byoc-1",
            "cloudProvider": "aws",
            "displayName": "My BYOC"
        })))
        .mount(&s)
        .await;

    let body = ByocInfrastructurePostRequest {
        account_id: "123456789012".to_string(),
        availability_zone_suffixes: vec![
            ByocAvailabilityZoneSuffix::A,
            ByocAvailabilityZoneSuffix::B,
        ],
        display_name: "My BYOC".to_string(),
        ..Default::default()
    };
    let resp = c
        .organization_byoc_infrastructure_create("org-1", &body)
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.display_name.as_deref(), Some("My BYOC"));
}

#[tokio::test]
async fn update_byoc_infrastructure() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/byoc-1"))
        .and(body_partial_json(
            serde_json::json!({"displayName": "Renamed BYOC"}),
        ))
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
    assert_eq!(config.display_name.as_deref(), Some("Renamed BYOC"));
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
    assert_eq!(resp.status, Some(200));
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
    assert_eq!(invitations[0].email.as_deref(), Some("alice@example.com"));
}

#[tokio::test]
async fn create_invitation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/invitations"))
        .and(body_partial_json(
            serde_json::json!({"email": "newuser@example.com"}),
        ))
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
        email: "newuser@example.com".to_string(),
        #[cfg(feature = "deprecated-fields")]
        role: Some(InvitationPostRequestRole::Developer),
        ..Default::default()
    };
    let resp = client.invitation_create("org-1", &body).await.unwrap();
    let inv = resp.result.unwrap();
    assert_eq!(inv.email.as_deref(), Some("newuser@example.com"));
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
    assert_eq!(inv.email.as_deref(), Some("bob@example.com"));
    #[cfg(feature = "deprecated-fields")]
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
    assert_eq!(resp.status, Some(200));
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
    assert_eq!(keys[0].name.as_deref(), Some("Production Key"));
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
        name: "New Key".to_string(),
        ..Default::default()
    };
    let resp = c.openapi_key_create("org-1", &body).await.unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.key_id.as_deref(), Some("key-id-abc"));
    assert_eq!(result.key_secret.as_deref(), Some("key-secret-xyz"));
    assert_eq!(
        result.key.as_ref().and_then(|key| key.name.as_deref()),
        Some("New Key")
    );
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
    assert_eq!(key.name.as_deref(), Some("My Key"));
    assert_eq!(key.state, Some(ApiKeyState::Enabled));
    assert_eq!(key.key_suffix.as_deref(), Some("abc"));
}

#[tokio::test]
async fn update_api_key() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/keys/key-1"))
        .and(body_partial_json(
            serde_json::json!({"name": "Renamed Key"}),
        ))
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
    assert_eq!(key.name.as_deref(), Some("Renamed Key"));
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
    assert_eq!(resp.status, Some(200));
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
    #[cfg(feature = "deprecated-fields")]
    {
        assert_eq!(members[0].role, Some(MemberRole::Admin));
        assert_eq!(members[1].role, Some(MemberRole::Developer));
    }
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
    assert_eq!(member.name.as_deref(), Some("Alice"));
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(member.role, Some(MemberRole::Admin));
}

#[tokio::test]
async fn update_member() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/members/user-1"))
        .respond_with(ok_json(serde_json::json!({
            "userId": "user-1",
            "name": "Alice",
            "email": "alice@example.com",
            "role": "admin"
        })))
        .mount(&s)
        .await;

    let body = MemberPatchRequest {
        #[cfg(feature = "deprecated-fields")]
        role: Some(MemberPatchRequestRole::Admin),
        ..Default::default()
    };
    let resp = c.member_update("org-1", "user-1", &body).await.unwrap();
    let member = resp.result.unwrap();
    assert_eq!(member.email.as_deref(), Some("alice@example.com"));
    #[cfg(feature = "deprecated-fields")]
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
    assert_eq!(resp.status, Some(200));
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
        description: "My PE".to_string(),
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
        allowed_origins: "https://example.com".to_string(),
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
        .and(path(
            "/v1/organizations/org-1/services/svc-1/backupConfiguration",
        ))
        .respond_with(ok_json(serde_json::json!({
            "backupPeriodInHours": 24,
            "backupRetentionPeriodInHours": 168,
            "backupStartTime": "02:00"
        })))
        .mount(&s)
        .await;

    let resp = c.backup_configuration_get("org-1", "svc-1").await.unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.backup_period_in_hours, Some(24.0));
    assert_eq!(config.backup_start_time.as_deref(), Some("02:00"));
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
        backup_start_time: Some(Some("03:00".to_string())),
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
        .and(body_partial_json(
            serde_json::json!({"bucketPath": "s3://new-bucket"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "bucketPath": "s3://new-bucket",
            "bucketProvider": "aws_s3",
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        })))
        .mount(&s)
        .await;

    let body =
        BackupBucketPostRequest::AwsBackupBucketPostRequestV1(AwsBackupBucketPostRequestV1 {
            bucket_path: "s3://new-bucket".to_string(),
            ..Default::default()
        });
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
        .and(body_partial_json(
            serde_json::json!({"bucketPath": "s3://updated-bucket"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "bucketPath": "s3://updated-bucket",
            "bucketProvider": "aws_s3",
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        })))
        .mount(&s)
        .await;

    let body =
        BackupBucketPatchRequest::AwsBackupBucketPatchRequestV1(AwsBackupBucketPatchRequestV1 {
            bucket_path: "s3://updated-bucket".to_string(),
            ..Default::default()
        });
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
    assert_eq!(resp.status, Some(200));
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

// ===========================================================================
// ClickStack: Alerts
// ===========================================================================

#[tokio::test]
async fn list_alerts() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts",
        ))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "alert-1",
                "name": "High CPU"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.click_stack_list_alerts("org-1", "svc-1").await.unwrap();
    let alerts = resp.result.unwrap();
    assert_eq!(alerts.len(), 1);
}

#[tokio::test]
async fn create_alert() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts",
        ))
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
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts/alert-1",
        ))
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
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts/alert-1",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "Updated Alert"}),
        ))
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
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/alerts/alert-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_alert("org-1", "svc-1", "alert-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

// ===========================================================================
// ClickStack: Saved Searches
// ===========================================================================

#[tokio::test]
async fn list_saved_searches() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches",
        ))
        .respond_with(ok_json(serde_json::json!([
            {
                "id": "search-1",
                "name": "Production Errors",
                "sourceId": "source-1"
            }
        ])))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_list_saved_searches("org-1", "svc-1")
        .await
        .unwrap();
    let searches = resp.result.unwrap();
    assert_eq!(searches.len(), 1);
    assert_eq!(searches[0].source_id.as_deref(), Some("source-1"));
}

#[tokio::test]
async fn create_saved_search() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "Production Errors",
            "sourceId": "source-1"
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "search-1",
            "name": "Production Errors",
            "sourceId": "source-1"
        })))
        .mount(&s)
        .await;

    let body = ClickStackSavedSearchInput {
        name: "Production Errors".to_string(),
        source_id: "source-1".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_create_saved_search("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert_eq!(resp.result.unwrap().id.as_deref(), Some("search-1"));
}

#[tokio::test]
async fn get_saved_search() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches/search-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "search-1",
            "name": "Production Errors",
            "sourceId": "source-1"
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_get_saved_search("org-1", "svc-1", "search-1")
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_saved_search() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches/search-1",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "Updated Search",
            "sourceId": "source-1"
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "search-1",
            "name": "Updated Search",
            "sourceId": "source-1"
        })))
        .mount(&s)
        .await;

    let body = ClickStackSavedSearchInput {
        name: "Updated Search".to_string(),
        source_id: "source-1".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_update_saved_search("org-1", "svc-1", "search-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn delete_saved_search() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/saved-searches/search-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_saved_search("org-1", "svc-1", "search-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

// ===========================================================================
// ClickStack: Dashboards
// ===========================================================================

#[tokio::test]
async fn list_dashboards() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards",
        ))
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
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "New Dashboard", "tiles": []}),
        ))
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
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards/dash-1",
        ))
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
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards/dash-1",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "Updated Dashboard", "tiles": []}),
        ))
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
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards/dash-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_dashboard("org-1", "svc-1", "dash-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

// ===========================================================================
// ClickStack: Sources & Webhooks
// ===========================================================================

#[tokio::test]
async fn list_sources() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources",
        ))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c.click_stack_list_sources("org-1", "svc-1").await.unwrap();
    let sources = resp.result.unwrap();
    assert_eq!(sources.len(), 0);
}

#[tokio::test]
async fn create_source() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources",
        ))
        .and(body_partial_json(
            serde_json::json!({"kind": "promql", "name": "Prometheus Metrics"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "source-1",
            "kind": "promql",
            "name": "Prometheus Metrics",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "metrics"},
            "timestampValueExpression": "timestamp"
        })))
        .mount(&s)
        .await;

    let body = ClickStackSource::ClickStackPromqlSource(ClickStackPromqlSource {
        name: "Prometheus Metrics".to_string(),
        kind: ClickStackPromqlSourceKind::Promql,
        connection: "conn-1".to_string(),
        from: ClickStackSourceFrom {
            database_name: "default".to_string(),
            table_name: "metrics".to_string(),
        },
        timestamp_value_expression: "timestamp".to_string(),
        ..Default::default()
    });
    let resp = c
        .click_stack_create_source("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn get_source() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources/source-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "source-1",
            "kind": "promql",
            "name": "My Source",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "metrics"},
            "timestampValueExpression": "timestamp"
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_get_source("org-1", "svc-1", "source-1")
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_source() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources/source-1",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "Updated Source"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "source-1",
            "kind": "promql",
            "name": "Updated Source",
            "connection": "conn-1",
            "from": {"databaseName": "default", "tableName": "metrics"},
            "timestampValueExpression": "timestamp"
        })))
        .mount(&s)
        .await;

    let body = ClickStackSource::ClickStackPromqlSource(ClickStackPromqlSource {
        name: "Updated Source".to_string(),
        kind: ClickStackPromqlSourceKind::Promql,
        connection: "conn-1".to_string(),
        from: ClickStackSourceFrom {
            database_name: "default".to_string(),
            table_name: "metrics".to_string(),
        },
        timestamp_value_expression: "timestamp".to_string(),
        ..Default::default()
    });
    let resp = c
        .click_stack_update_source("org-1", "svc-1", "source-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn delete_source() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/sources/source-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_source("org-1", "svc-1", "source-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

#[tokio::test]
async fn list_webhooks() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks",
        ))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c.click_stack_list_webhooks("org-1", "svc-1").await.unwrap();
    let webhooks = resp.result.unwrap();
    assert_eq!(webhooks.len(), 0);
}

// ===========================================================================
// UDFs
// ===========================================================================

#[tokio::test]
async fn delete_udf() {
    let (s, c) = setup().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_delete("org-1", "my_udf").await.unwrap().status,
        Some(200)
    );
}

#[tokio::test]
async fn detach_udf() {
    let (s, c) = setup().await;
    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_detach("org-1", "my_udf", "svc-1")
            .await
            .unwrap()
            .status,
        Some(200)
    );
}

#[tokio::test]
async fn delete_udf_version() {
    let (s, c) = setup().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/versions/7"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_version_delete("org-1", "my_udf", 7)
            .await
            .unwrap()
            .status,
        Some(200)
    );
}

#[tokio::test]
async fn list_udfs_encodes_pagination() {
    let (s, c) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs"))
        .and(query_param("cursor", "next-page"))
        .and(query_param("limit", "25"))
        .respond_with(ok_json(serde_json::json!({"items": []})))
        .mount(&s)
        .await;

    assert!(
        c.udf_list("org-1", Some("next-page"), Some(25))
            .await
            .unwrap()
            .result
            .unwrap()
            .items
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn get_udf() {
    let (s, c) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(ok_json(serde_json::json!({"functionName": "my_udf"})))
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_get("org-1", "my_udf")
            .await
            .unwrap()
            .result
            .unwrap()
            .function_name
            .as_deref(),
        Some("my_udf")
    );
}

#[tokio::test]
async fn list_udf_attachments_encodes_pagination() {
    let (s, c) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/attachments"))
        .and(query_param("cursor", "next-page"))
        .and(query_param("limit", "25"))
        .respond_with(ok_json(serde_json::json!({"items": []})))
        .mount(&s)
        .await;

    assert!(
        c.udf_attachment_list("org-1", "my_udf", Some("next-page"), Some(25))
            .await
            .unwrap()
            .result
            .unwrap()
            .items
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn get_udf_attachment() {
    let (s, c) = setup().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ok_json(serde_json::json!({"serviceId": "svc-1"})))
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_attachment_get("org-1", "my_udf", "svc-1")
            .await
            .unwrap()
            .result
            .unwrap()
            .service_id
            .as_deref(),
        Some("svc-1")
    );
}

#[tokio::test]
async fn list_udf_versions_encodes_pagination() {
    let (s, c) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
        .and(query_param("cursor", "next-page"))
        .and(query_param("limit", "25"))
        .respond_with(ok_json(serde_json::json!({"items": []})))
        .mount(&s)
        .await;

    assert!(
        c.udf_version_list("org-1", "my_udf", Some("next-page"), Some(25))
            .await
            .unwrap()
            .result
            .unwrap()
            .items
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn create_udf_upload_session() {
    let (s, c) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfUploads/url"))
        .respond_with(created_json(serde_json::json!({"uploadId": "upload-1"})))
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_upload_session_create("org-1")
            .await
            .unwrap()
            .result
            .unwrap()
            .upload_id
            .as_deref(),
        Some("upload-1")
    );
}

#[tokio::test]
async fn create_udf_encodes_request_body() {
    let (s, c) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .and(body_partial_json(serde_json::json!({
            "arguments": [],
            "functionName": "my_udf",
            "returnType": "String",
            "runtime": "python3.11",
            "type": "executable",
            "uploadId": "upload-1", "deterministic": false, "memoryLimitMib": 256
        })))
        .respond_with(created_json(serde_json::json!({"functionName": "my_udf"})))
        .mount(&s)
        .await;

    let body = UdfCreateRequest::UdfCreateRequestV1(UdfCreateRequestV1 {
        deterministic: Some(false),
        memory_limit_mib: Some(256),
        arguments: vec![],
        function_name: "my_udf".to_string(),
        return_type: "String".to_string(),
        runtime: UdfRuntime::Python3_11,
        r#type: UdfCreateRequestV1Type::Executable,
        upload_id: "upload-1".to_string(),
        ..Default::default()
    });
    assert_eq!(
        c.udf_create("org-1", &body)
            .await
            .unwrap()
            .result
            .unwrap()
            .function_name
            .as_deref(),
        Some("my_udf")
    );
}

#[tokio::test]
async fn create_udf_version_encodes_request_body() {
    let (s, c) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
        .and(body_partial_json(serde_json::json!({
            "arguments": [],
            "returnType": "String",
            "runtime": "python3.11",
            "type": "executable",
            "uploadId": "upload-1", "deterministic": false, "memoryLimitMib": 256
        })))
        .respond_with(created_json(serde_json::json!({"functionName": "my_udf"})))
        .mount(&s)
        .await;

    let body = UdfVersionCreateRequest::UdfVersionCreateRequestV1(UdfVersionCreateRequestV1 {
        deterministic: Some(false),
        memory_limit_mib: Some(256),
        arguments: vec![],
        return_type: "String".to_string(),
        runtime: UdfRuntime::Python3_11,
        r#type: UdfVersionCreateRequestV1Type::Executable,
        upload_id: "upload-1".to_string(),
        ..Default::default()
    });
    assert_eq!(
        c.udf_version_create("org-1", "my_udf", &body)
            .await
            .unwrap()
            .result
            .unwrap()
            .function_name
            .as_deref(),
        Some("my_udf")
    );
}

#[tokio::test]
async fn attach_udf_encodes_optional_version() {
    let (s, c) = setup().await;
    let attachment_path = "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1";
    let attachment = serde_json::json!({"functionName": "my_udf", "serviceId": "svc-1"});

    Mock::given(method("PUT"))
        .and(path(attachment_path))
        .and(body_json(serde_json::json!({})))
        .respond_with(ok_json(attachment.clone()))
        .mount(&s)
        .await;
    Mock::given(method("PUT"))
        .and(path(attachment_path))
        .and(body_json(serde_json::json!({"version": 7})))
        .respond_with(ok_json(attachment))
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_attach("org-1", "my_udf", "svc-1", None)
            .await
            .unwrap()
            .result
            .unwrap()
            .service_id
            .as_deref(),
        Some("svc-1")
    );
    assert_eq!(
        c.udf_attach("org-1", "my_udf", "svc-1", Some(7))
            .await
            .unwrap()
            .result
            .unwrap()
            .function_name
            .as_deref(),
        Some("my_udf")
    );
}

// ===========================================================================
// ClickStack: Roles
// ===========================================================================

#[tokio::test]
async fn list_roles() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/roles",
        ))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let resp = c.click_stack_list_roles("org-1", "svc-1").await.unwrap();
    let roles = resp.result.unwrap();
    assert_eq!(roles.len(), 0);
}

#[tokio::test]
async fn create_role() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/roles",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "Deploy Bot",
            "permissions": [{
                "action": "read",
                "subject": "dashboard",
                "conditions": { "teamId": "team-1" }
            }]
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "role-1",
            "name": "Deploy Bot",
            "isPredefined": false,
            "permissions": [{
                "action": "read",
                "subject": "dashboard",
                "conditions": { "teamId": "team-1" }
            }]
        })))
        .mount(&s)
        .await;

    let body = ClickStackCreateRoleRequest {
        name: "Deploy Bot".to_string(),
        permissions: vec![ClickStackCASLPermission {
            action: "read".to_string(),
            subject: "dashboard".to_string(),
            conditions: Some(serde_json::json!({ "teamId": "team-1" })),
            ..Default::default()
        }],
        ..Default::default()
    };
    let resp = c
        .click_stack_create_role("org-1", "svc-1", &body)
        .await
        .unwrap();
    let role = resp.result.unwrap();
    assert_eq!(role.id.as_deref(), Some("role-1"));
    assert_eq!(role.name.as_deref(), Some("Deploy Bot"));
    assert_eq!(role.is_predefined, Some(false));
}

#[tokio::test]
async fn get_role() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/roles/role-1",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "role-1",
            "name": "Read Only",
            "isPredefined": true,
            "permissions": [{ "action": "read", "subject": "dashboard" }]
        })))
        .mount(&s)
        .await;

    let resp = c
        .click_stack_get_role("org-1", "svc-1", "role-1")
        .await
        .unwrap();
    let role = resp.result.unwrap();
    assert_eq!(role.id.as_deref(), Some("role-1"));
    assert_eq!(role.is_predefined, Some(true));
}

#[tokio::test]
async fn update_role() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/roles/role-1",
        ))
        .and(body_partial_json(serde_json::json!({
            "permissions": [{ "action": "manage", "subject": "all" }]
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "role-1",
            "name": "Deploy Bot",
            "isPredefined": false,
            "permissions": [{ "action": "manage", "subject": "all" }]
        })))
        .mount(&s)
        .await;

    let body = ClickStackUpdateRoleRequest {
        permissions: vec![ClickStackCASLPermission {
            action: "manage".to_string(),
            subject: "all".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let resp = c
        .click_stack_update_role("org-1", "svc-1", "role-1", &body)
        .await
        .unwrap();
    let role = resp.result.unwrap();
    let permissions = role.permissions.unwrap_or_default();
    assert_eq!(permissions.len(), 1);
    assert_eq!(permissions[0].action.as_deref(), Some("manage"));
}

#[tokio::test]
async fn delete_role() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/roles/role-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_role("org-1", "svc-1", "role-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

// ===========================================================================
// ClickStack: Webhooks & Dashboard validation
// ===========================================================================

#[tokio::test]
async fn create_webhook() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "Production Alerts",
            "service": "slack",
            "url": "https://hooks.slack.com/services/T/B/X"
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "webhook-1",
            "name": "Production Alerts",
            "service": "slack",
            "url": "https://hooks.slack.com/services/T/B/X",
            "createdAt": "2025-01-01T00:00:00.000Z",
            "updatedAt": "2025-06-15T10:30:00.000Z"
        })))
        .mount(&s)
        .await;

    let body = ClickStackWebhookInput {
        name: "Production Alerts".to_string(),
        service: ClickStackWebhookInputService::Slack,
        url: "https://hooks.slack.com/services/T/B/X".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_create_webhook("org-1", "svc-1", &body)
        .await
        .unwrap();
    assert!(resp.result.is_some());
}

#[tokio::test]
async fn update_webhook() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks/webhook-1",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "Updated Alerts"
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "webhook-1",
            "name": "Updated Alerts",
            "service": "slack",
            "url": "https://hooks.slack.com/services/T/B/X",
            "createdAt": "2025-01-01T00:00:00.000Z",
            "updatedAt": "2025-06-15T10:30:00.000Z"
        })))
        .mount(&s)
        .await;

    let body = ClickStackWebhookInput {
        name: "Updated Alerts".to_string(),
        service: ClickStackWebhookInputService::Slack,
        url: "https://hooks.slack.com/services/T/B/X".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_update_webhook("org-1", "svc-1", "webhook-1", &body)
        .await
        .unwrap();
    // The response union resolves to a concrete Slack webhook variant.
    match resp.result.unwrap() {
        ClickStackWebhook::ClickStackSlackWebhook(w) => {
            assert_eq!(w.id.as_deref(), Some("webhook-1"));
            assert_eq!(w.name.as_deref(), Some("Updated Alerts"));
        }
        other => panic!("expected Slack webhook variant, got {other}"),
    }
}

#[tokio::test]
async fn create_webhook_incidentio_response() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "webhook-2",
            "name": "Incident Alerts",
            "service": "incidentio",
            "url": "https://api.incident.io/v2/alert_events/http/abc",
            "createdAt": "2025-01-01T00:00:00.000Z",
            "updatedAt": "2025-06-15T10:30:00.000Z"
        })))
        .mount(&s)
        .await;

    let body = ClickStackWebhookInput {
        name: "Incident Alerts".to_string(),
        service: ClickStackWebhookInputService::Incidentio,
        url: "https://api.incident.io/v2/alert_events/http/abc".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_create_webhook("org-1", "svc-1", &body)
        .await
        .unwrap();
    // The response union resolves to a concrete IncidentIO webhook variant
    // rather than greedily matching the structurally-identical Slack variant.
    match resp.result.unwrap() {
        ClickStackWebhook::ClickStackIncidentIOWebhook(w) => {
            assert_eq!(w.id.as_deref(), Some("webhook-2"));
            assert_eq!(w.name.as_deref(), Some("Incident Alerts"));
        }
        other => panic!("expected IncidentIO webhook variant, got {other}"),
    }
}

#[tokio::test]
async fn update_webhook_generic_response() {
    let (s, c) = setup().await;

    Mock::given(method("PUT"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks/webhook-3",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "webhook-3",
            "name": "Generic Alerts",
            "service": "generic",
            "url": "https://example.com/hook",
            "body": "{\"text\": \"{{ message }}\"}",
            "createdAt": "2025-01-01T00:00:00.000Z",
            "updatedAt": "2025-06-15T10:30:00.000Z"
        })))
        .mount(&s)
        .await;

    let body = ClickStackWebhookInput {
        name: "Generic Alerts".to_string(),
        service: ClickStackWebhookInputService::Generic,
        url: "https://example.com/hook".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_update_webhook("org-1", "svc-1", "webhook-3", &body)
        .await
        .unwrap();
    // The response union resolves to the Generic variant and preserves its
    // optional `body` field, which the greedy Slack match would have discarded.
    match resp.result.unwrap() {
        ClickStackWebhook::ClickStackGenericWebhook(w) => {
            assert_eq!(w.id.as_deref(), Some("webhook-3"));
            assert_eq!(w.body.as_deref(), Some("{\"text\": \"{{ message }}\"}"));
        }
        other => panic!("expected Generic webhook variant, got {other}"),
    }
}

#[tokio::test]
async fn delete_webhook() {
    let (s, c) = setup().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/webhooks/webhook-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    let resp = c
        .click_stack_delete_webhook("org-1", "svc-1", "webhook-1")
        .await
        .unwrap();
    assert_eq!(resp.status, Some(200));
}

#[tokio::test]
async fn validate_dashboard() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/services/svc-1/clickstack/dashboards/validate",
        ))
        .and(body_partial_json(serde_json::json!({
            "name": "My Dashboard"
        })))
        .respond_with(ok_json(serde_json::json!({
            "valid": false,
            "errors": [
                {"path": "tiles.0.config", "message": "Required"}
            ],
            "normalized": null
        })))
        .mount(&s)
        .await;

    let body = ClickStackCreateDashboardRequest {
        name: "My Dashboard".to_string(),
        ..Default::default()
    };
    let resp = c
        .click_stack_validate_dashboard("org-1", "svc-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.valid, Some(false));
    let errors = result.errors.as_ref().expect("errors should populate");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path.as_deref(), Some("tiles.0.config"));
    assert_eq!(result.normalized, None);
}

// ===========================================================================
// Organization quotas
// ===========================================================================

#[tokio::test]
async fn list_quotas() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/quotas"))
        .respond_with(ok_json(serde_json::json!([
            {
                "quotaCode": "services-per-organization",
                "name": "Services per organization",
                "description": "Limits services.",
                "scope": "organization",
                "value": 20,
                "usage": 3,
                "adjustable": true
            }
        ])))
        .mount(&s)
        .await;

    let resp = c.organization_quotas_get_list("org-1").await.unwrap();
    let quotas = resp.result.unwrap();
    assert_eq!(quotas.len(), 1);
    assert_eq!(
        quotas[0].quota_code,
        Some(OrganizationQuotaQuotacode::Services_per_organization)
    );
    assert_eq!(quotas[0].scope, Some(OrganizationQuotaScope::Organization));
    assert_eq!(quotas[0].usage, Some(3));
}

#[tokio::test]
async fn get_quota() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/quotas/replicas-per-warehouse",
        ))
        .respond_with(ok_json(serde_json::json!({
            "quotaCode": "replicas-per-warehouse",
            "name": "Replicas per warehouse",
            "description": "Limits each warehouse individually.",
            "scope": "warehouse",
            "value": 20,
            "adjustable": true
        })))
        .mount(&s)
        .await;

    let resp = c
        .organization_quota_get("org-1", "replicas-per-warehouse")
        .await
        .unwrap();
    let quota = resp.result.unwrap();
    assert_eq!(
        quota.quota_code,
        Some(OrganizationQuotaQuotacode::Replicas_per_warehouse)
    );
    assert_eq!(quota.scope, Some(OrganizationQuotaScope::Warehouse));
    assert_eq!(quota.usage, None);
}

// ===========================================================================
// PostgreSQL Services
// ===========================================================================

#[tokio::test]
async fn list_postgres_logs_with_filters() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/logs"))
        .and(query_param("from_date", "2026-08-01T00:00:00Z"))
        .and(query_param("to_date", "2026-08-02T00:00:00Z"))
        .and(query_param("body_contains", "checkpoint"))
        .and(query_param("severity", "LOG"))
        .and(query_param("sort_order", "asc"))
        .and(query_param("limit", "100"))
        .and(query_param("offset", "20"))
        .respond_with(ok_json(serde_json::json!([{
            "timestamp": "2026-08-01T12:00:00Z",
            "severity": "LOG",
            "body": "checkpoint complete"
        }])))
        .mount(&s)
        .await;

    let logs = c
        .postgres_logs_get_list(
            "org-1",
            "pg-1",
            "2026-08-01T00:00:00Z",
            "2026-08-02T00:00:00Z",
            Some("checkpoint"),
            Some("LOG"),
            Some(&PostgresLogsGetListSortorder::Asc),
            Some(100),
            Some(20),
        )
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].body.as_deref(), Some("checkpoint complete"));
}

#[tokio::test]
async fn list_slow_query_patterns_with_typed_sorting() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/slowQueryPatterns",
        ))
        .and(query_param("from_date", "2026-08-01T00:00:00Z"))
        .and(query_param("to_date", "2026-08-02T00:00:00Z"))
        .and(query_param("db_name", "analytics"))
        .and(query_param("db_user", "reporter"))
        .and(query_param("db_operation", "SELECT"))
        .and(query_param("app", "dashboard"))
        .and(query_param("sort_by", "p95_duration"))
        .and(query_param("sort_order", "asc"))
        .and(query_param("limit", "100"))
        .and(query_param("offset", "20"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    let patterns = c
        .slow_query_patterns_get_list(
            "org-1",
            "pg-1",
            "2026-08-01T00:00:00Z",
            "2026-08-02T00:00:00Z",
            Some("analytics"),
            Some("reporter"),
            Some("SELECT"),
            Some("dashboard"),
            Some(&SlowQueryPatternsGetListSortby::P95_duration),
            Some(&SlowQueryPatternsGetListSortorder::Asc),
            Some(100),
            Some(20),
        )
        .await
        .unwrap()
        .result
        .unwrap();
    assert!(patterns.is_empty());
}

#[tokio::test]
async fn create_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/postgres"))
        .and(body_partial_json(serde_json::json!({"name": "pg-svc", "provider": "aws", "region": "us-east-1", "size": "c6gd.large", "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}})))
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
        size: PgSize::C6gd_large,
        pg_bouncer_config: Some(PgBouncerConfig::from([
            ("default_pool_size".into(), "16".into()),
            ("future_parameter".into(), "on".into()),
        ])),
        ..Default::default()
    };
    let resp = c.postgres_service_create("org-1", &body).await.unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg-svc"));
    assert_eq!(pg.password.as_deref(), Some("generated-pw"));
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
    assert_eq!(services[0].name.as_deref(), Some("pg-1"));
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
    assert_eq!(pg.name.as_deref(), Some("pg-1"));
    assert_eq!(
        pg.connection_string.as_deref(),
        Some("postgres://user@host/db")
    );
}

#[tokio::test]
async fn update_postgres_service() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1"))
        .and(body_partial_json(serde_json::json!({"size": "c6gd.large"})))
        .respond_with(ok_json(serde_json::json!({
            "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "name": "pg-1"
        })))
        .mount(&s)
        .await;

    let body = PostgresServicePatchRequest {
        size: Some(PgSize::C6gd_large),
        ..Default::default()
    };
    let resp = c
        .postgres_service_patch("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg-1"));
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
    assert_eq!(resp.status, Some(200));
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
        command: PostgresServiceSetStateCommand::Restart,
    };
    let resp = c
        .postgres_service_patch_state("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg-1"));
}

#[tokio::test]
async fn set_postgres_password() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/password"))
        .and(body_partial_json(
            serde_json::json!({"password": "new-pg-password"}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "password": "new-pg-password"
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceSetPassword {
        password: "new-pg-password".to_string(),
    };
    let resp = c
        .postgres_service_set_password("org-1", "pg-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(result.password.as_deref(), Some("new-pg-password"));
}

#[tokio::test]
async fn get_postgres_certs() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/caCertificates"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "-----BEGIN CERTIFICATE-----\nMIIC...\n-----END CERTIFICATE-----\n",
            ),
        )
        .mount(&s)
        .await;

    let resp = c.postgres_service_certs_get("org-1", "pg-1").await.unwrap();
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
            "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}
        })))
        .mount(&s)
        .await;

    let resp = c
        .postgres_instance_config_get("org-1", "pg-1")
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(
        config.pg_bouncer_config.as_ref().unwrap()["default_pool_size"],
        "16"
    );
    assert_eq!(
        config.pg_bouncer_config.as_ref().unwrap()["future_parameter"],
        "on"
    );
    assert_eq!(
        config
            .pg_config
            .expect("pgConfig in response")
            .max_connections,
        Some(serde_json::json!(100))
    );
}

#[tokio::test]
async fn replace_postgres_config() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .and(body_partial_json(
            serde_json::json!({"pgConfig": {"max_connections": 200}, "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "message": "Configuration updated",
            "pgConfig": { "max_connections": 200 },
            "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}
        })))
        .mount(&s)
        .await;

    let body = PostgresInstanceConfig {
        pg_config: PgConfig {
            max_connections: Some(serde_json::json!(200)),
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig::from([
            ("default_pool_size".into(), "16".into()),
            ("future_parameter".into(), "on".into()),
        ]),
    };
    let resp = c
        .postgres_instance_config_post("org-1", "pg-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(
        result.pg_bouncer_config.as_ref().unwrap()["default_pool_size"],
        "16"
    );
    assert_eq!(
        result.pg_bouncer_config.as_ref().unwrap()["future_parameter"],
        "on"
    );
    assert_eq!(result.message, Some("Configuration updated".to_string()));
    assert_eq!(
        result
            .pg_config
            .expect("pgConfig in response")
            .max_connections,
        Some(serde_json::json!(200))
    );
}

#[tokio::test]
async fn patch_postgres_config() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/config"))
        .and(body_partial_json(
            serde_json::json!({"pgConfig": {"max_connections": 150}, "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "message": "OK",
            "pgConfig": { "max_connections": 150 },
            "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}
        })))
        .mount(&s)
        .await;

    let body = PostgresInstanceConfig {
        pg_config: PgConfig {
            max_connections: Some(serde_json::json!(150)),
            ..Default::default()
        },
        pg_bouncer_config: PgBouncerConfig::from([
            ("default_pool_size".into(), "16".into()),
            ("future_parameter".into(), "on".into()),
        ]),
    };
    let resp = c
        .postgres_instance_config_patch("org-1", "pg-1", &body)
        .await
        .unwrap();
    let result = resp.result.unwrap();
    assert_eq!(
        result.pg_bouncer_config.as_ref().unwrap()["default_pool_size"],
        "16"
    );
    assert_eq!(
        result.pg_bouncer_config.as_ref().unwrap()["future_parameter"],
        "on"
    );
    assert_eq!(
        result
            .pg_config
            .expect("pgConfig in response")
            .max_connections,
        Some(serde_json::json!(150))
    );
}

#[tokio::test]
async fn create_postgres_read_replica() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/readReplica"))
        .and(body_partial_json(
            serde_json::json!({"name": "pg-1-replica", "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
            "name": "pg-1-replica",
            "isPrimary": false
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceReadReplicaRequest {
        name: "pg-1-replica".to_string(),
        pg_bouncer_config: Some(PgBouncerConfig::from([
            ("default_pool_size".into(), "16".into()),
            ("future_parameter".into(), "on".into()),
        ])),
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
        .and(path(
            "/v1/organizations/org-1/postgres/pg-1/restoredService",
        ))
        .and(body_partial_json(
            serde_json::json!({"name": "pg-1-restored", "pgBouncerConfig": {"default_pool_size": "16", "future_parameter": "on"}}),
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "cccccccc-dddd-eeee-ffff-000000000000",
            "name": "pg-1-restored",
            "state": "creating"
        })))
        .mount(&s)
        .await;

    let body = PostgresServiceRestoreRequest {
        name: "pg-1-restored".to_string(),
        pg_bouncer_config: Some(PgBouncerConfig::from([
            ("default_pool_size".into(), "16".into()),
            ("future_parameter".into(), "on".into()),
        ])),
        restore_target: Utc::now(),
        ..Default::default()
    };
    let resp = c
        .postgres_instance_restore("org-1", "pg-1", &body)
        .await
        .unwrap();
    let pg = resp.result.unwrap();
    assert_eq!(pg.name.as_deref(), Some("pg-1-restored"));
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
    assert_eq!(orgs[0].name.as_deref(), Some("Bearer Org"));
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
    client.set_bearer_token("refreshed-token").unwrap();
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
    let client = Client::with_http_client(http, mock_server.uri(), "custom-key", "custom-secret");
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
fn set_bearer_token_errors_on_basic_auth() {
    let mut client = Client::new("key", "secret");
    let err = client.set_bearer_token("token").unwrap_err();
    assert!(
        err.to_string().contains("auth mismatch"),
        "unexpected error: {err}"
    );
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
        name: "test".to_string(),
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

    let err = c.instance_get("org-1", "nonexistent").await.unwrap_err();
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
        .respond_with(ResponseTemplate::new(502).set_body_string("Bad Gateway"))
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
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "status": 403,
            "error": "Metrics access denied"
        })))
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
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "Postgres service not found"
        })))
        .mount(&s)
        .await;

    let err = c
        .postgres_service_certs_get("org-1", "pg-1")
        .await
        .unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 404);
            assert_eq!(message, "Postgres service not found");
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
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"status": 200, "result":"#))
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
        .respond_with(ResponseTemplate::new(200).set_body_string("cpu 42\nmemory 1024\n"))
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
        .respond_with(ResponseTemplate::new(200).set_body_string("# HELP metric\nmetric 1\n"))
        .mount(&s)
        .await;

    let resp = c.organization_prometheus_get("org-1", None).await.unwrap();
    assert!(resp.contains("metric"));
}

#[tokio::test]
async fn postgres_instance_prometheus_get_returns_metrics() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/pg-1/prometheus"))
        .respond_with(ResponseTemplate::new(200).set_body_string("# HELP pg_metric\npg_metric 7\n"))
        .mount(&s)
        .await;

    let resp = c
        .postgres_instance_prometheus_get("org-1", "pg-1")
        .await
        .unwrap();
    assert!(resp.contains("pg_metric"));
}

#[tokio::test]
async fn postgres_org_prometheus_get_returns_metrics() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/postgres/prometheus"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("# HELP pg_org_metric\npg_org_metric 3\n"),
        )
        .mount(&s)
        .await;

    let resp = c.postgres_org_prometheus_get("org-1").await.unwrap();
    assert!(resp.contains("pg_org_metric"));
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

#[tokio::test]
async fn credit_balances_get_includes_trial_and_prepaid_balances() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/creditBalances"))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({
            "totalRemainingCredits": 12.5,
            "balances": [{"type": "trial", "remainingCredits": 2.5}, {"type": "prepaid", "remainingCredits": 10.0}]
        })))
        .expect(1).mount(&server).await;
    let result = client
        .credit_balances_get("org")
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(result.total_remaining_credits, Some(12.5));
    let balances = result.balances.unwrap();
    assert_eq!(balances[0].r#type, Some(CreditBalanceType::Trial));
    assert_eq!(balances[1].r#type, Some(CreditBalanceType::Prepaid));
}

#[tokio::test]
async fn service_profiles_list_encodes_region_and_optional_byoc() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org/serviceProfiles"))
        .and(basic_auth("key", "secret"))
        .and(query_param("region_id", "us-east-1"))
        .respond_with(ok_json(
            serde_json::json!([{"profile": "v1-standard-byoc-4", "cpuCores": 4, "memoryGi": 16}]),
        ))
        .expect(2)
        .mount(&server)
        .await;
    for byoc in [None, Some("byoc +/id")] {
        let result = client
            .service_profiles_list("org", "us-east-1", byoc)
            .await
            .unwrap()
            .result
            .unwrap();
        assert_eq!(result[0].profile.as_deref(), Some("v1-standard-byoc-4"));
        assert_eq!(result[0].cpu_cores, Some(4.0));
        assert_eq!(result[0].memory_gi, Some(16.0));
    }
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests[0]
            .url
            .query_pairs()
            .any(|(key, _)| key == "byoc_id")
    );
    assert!(
        requests[1]
            .url
            .query_pairs()
            .any(|(key, value)| key == "byoc_id" && value == "byoc +/id")
    );
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

#[tokio::test]
async fn new_discovery_operations_preserve_api_errors() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(serde_json::json!({"error": "forbidden"})),
        )
        .expect(3)
        .mount(&server)
        .await;
    let errors = [
        client.credit_balances_get("org").await.unwrap_err(),
        client
            .service_profiles_list("org", "region", None)
            .await
            .unwrap_err(),
        client
            .click_pipes_service_context_get("org", "service")
            .await
            .unwrap_err(),
    ];
    for error in errors {
        assert!(
            matches!(error, clickhouse_cloud_api::Error::Api { status: 403, message } if message == "forbidden")
        );
    }
}

#[tokio::test]
async fn update_api_key_sends_omitted_timestamp_and_null_expiry() {
    let timestamp = chrono::DateTime::parse_from_rfc3339("2030-01-02T03:04:05Z")
        .unwrap()
        .with_timezone(&Utc);
    for (expire_at, wire) in [
        (None, serde_json::json!({"name": "retained"})),
        (
            Some(Some(timestamp)),
            serde_json::json!({"name": "retained", "expireAt": "2030-01-02T03:04:05Z"}),
        ),
        (
            Some(None),
            serde_json::json!({"name": "retained", "expireAt": null}),
        ),
    ] {
        let (server, client) = setup().await;
        Mock::given(method("PATCH"))
            .and(path("/v1/organizations/org-1/keys/key-1"))
            .and(body_json(wire))
            .respond_with(ok_json(serde_json::json!({"name": "retained"})))
            .expect(1)
            .mount(&server)
            .await;
        let request = ApiKeyPatchRequest {
            name: Some("retained".into()),
            expire_at,
            ..Default::default()
        };
        let response = client
            .openapi_key_update("org-1", "key-1", &request)
            .await
            .unwrap();
        assert_eq!(response.result.unwrap().name.as_deref(), Some("retained"));
    }
}
