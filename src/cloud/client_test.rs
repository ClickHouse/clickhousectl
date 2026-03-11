use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Match, Mock, MockServer, ResponseTemplate};

use super::client::CloudClient;
use super::types::*;

fn test_client(server: &MockServer) -> CloudClient {
    CloudClient::test_client(&server.uri())
}

fn auth_header_matcher() -> impl Match {
    // "test_key:test_secret" base64 encoded
    header("Authorization", "Basic dGVzdF9rZXk6dGVzdF9zZWNyZXQ=")
}

// ── Organization endpoints ──────────────────────────────────────────

#[tokio::test]
async fn test_list_organizations() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                {"id": "org-1", "name": "My Org", "createdAt": "2024-01-01T00:00:00Z"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let orgs = client.list_organizations().await.unwrap();

    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].id, "org-1");
    assert_eq!(orgs[0].name, "My Org");
    assert_eq!(orgs[0].created_at.as_deref(), Some("2024-01-01T00:00:00Z"));
}

#[tokio::test]
async fn test_get_organization() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "org-1", "name": "My Org", "createdAt": "2024-01-01T00:00:00Z"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let org = client.get_organization("org-1").await.unwrap();

    assert_eq!(org.id, "org-1");
    assert_eq!(org.name, "My Org");
}

// ── Service endpoints ───────────────────────────────────────────────

#[tokio::test]
async fn test_list_services() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/services"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{
                "id": "svc-1",
                "name": "my-service",
                "provider": "aws",
                "region": "us-east-1",
                "state": "running",
                "tier": "production",
                "idleScaling": true,
                "idleTimeoutMinutes": 10,
                "endpoints": [{"protocol": "https", "host": "abc.clickhouse.cloud", "port": 8443}],
                "minReplicaMemoryGb": 24,
                "maxReplicaMemoryGb": 48,
                "numReplicas": 3
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let services = client.list_services("org-1").await.unwrap();

    assert_eq!(services.len(), 1);
    assert_eq!(services[0].id, "svc-1");
    assert_eq!(services[0].name, "my-service");
    assert_eq!(services[0].state, "running");
    assert_eq!(services[0].endpoints.as_ref().unwrap()[0].port, 8443);
}

#[tokio::test]
async fn test_get_service() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/services/svc-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "svc-1",
                "name": "my-service",
                "provider": "aws",
                "region": "us-east-1",
                "state": "running"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let svc = client.get_service("org-1", "svc-1").await.unwrap();

    assert_eq!(svc.id, "svc-1");
    assert_eq!(svc.name, "my-service");
}

#[tokio::test]
async fn test_create_service() {
    let server = MockServer::start().await;

    let request = CreateServiceRequest {
        name: "new-svc".to_string(),
        provider: "aws".to_string(),
        region: "us-east-1".to_string(),
        ..Default::default()
    };

    Mock::given(method("POST"))
        .and(path("/organizations/org-1/services"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({
            "name": "new-svc",
            "provider": "aws",
            "region": "us-east-1"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "service": {
                    "id": "svc-new",
                    "name": "new-svc",
                    "provider": "aws",
                    "region": "us-east-1",
                    "state": "provisioning"
                },
                "password": "s3cret!"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let resp = client.create_service("org-1", &request).await.unwrap();

    assert_eq!(resp.service.id, "svc-new");
    assert_eq!(resp.service.state, "provisioning");
    assert_eq!(resp.password, "s3cret!");
}

#[tokio::test]
async fn test_delete_service() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/organizations/org-1/services/svc-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    client.delete_service("org-1", "svc-1").await.unwrap();
}

#[tokio::test]
async fn test_change_service_state() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/organizations/org-1/services/svc-1/state"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"command": "stop"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "svc-1",
                "name": "my-service",
                "provider": "aws",
                "region": "us-east-1",
                "state": "stopping"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let svc = client
        .change_service_state("org-1", "svc-1", "stop")
        .await
        .unwrap();

    assert_eq!(svc.state, "stopping");
}

// ── Backup endpoints ────────────────────────────────────────────────

#[tokio::test]
async fn test_list_backups() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/services/svc-1/backups"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{
                "id": "backup-1",
                "serviceId": "svc-1",
                "status": "completed",
                "createdAt": "2024-01-01T00:00:00Z",
                "finishedAt": "2024-01-01T00:05:00Z",
                "sizeInBytes": 1048576
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let backups = client.list_backups("org-1", "svc-1").await.unwrap();

    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].id, "backup-1");
    assert_eq!(backups[0].status, "completed");
    assert_eq!(backups[0].size_in_bytes, Some(1048576));
}

#[tokio::test]
async fn test_get_backup() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/services/svc-1/backups/backup-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "backup-1",
                "serviceId": "svc-1",
                "status": "completed",
                "createdAt": "2024-01-01T00:00:00Z",
                "finishedAt": "2024-01-01T00:05:00Z",
                "sizeInBytes": 2097152
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let backup = client
        .get_backup("org-1", "svc-1", "backup-1")
        .await
        .unwrap();

    assert_eq!(backup.id, "backup-1");
    assert_eq!(backup.size_in_bytes, Some(2097152));
}

// ── Helper endpoints ────────────────────────────────────────────────

#[tokio::test]
async fn test_get_default_org_id() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                {"id": "org-default", "name": "Default Org"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let org_id = client.get_default_org_id().await.unwrap();
    assert_eq!(org_id, "org-default");
}

#[tokio::test]
async fn test_get_default_org_id_empty() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let err = client.get_default_org_id().await.unwrap_err();
    assert!(err.message.contains("No organization found"));
}

// ── Phase 2: Service update/scale/password/query-endpoint/private-endpoint ──

#[tokio::test]
async fn test_update_service() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/organizations/org-1/services/svc-1"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"name": "renamed"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "svc-1",
                "name": "renamed",
                "provider": "aws",
                "region": "us-east-1",
                "state": "running"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = super::types::UpdateServiceRequest {
        name: Some("renamed".to_string()),
        ..Default::default()
    };
    let svc = client.update_service("org-1", "svc-1", &req).await.unwrap();
    assert_eq!(svc.name, "renamed");
}

#[tokio::test]
async fn test_update_replica_scaling() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/organizations/org-1/services/svc-1/replicaScaling"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({
            "minReplicaMemoryGb": 24,
            "maxReplicaMemoryGb": 48,
            "numReplicas": 3
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "svc-1",
                "name": "my-service",
                "provider": "aws",
                "region": "us-east-1",
                "state": "running",
                "minReplicaMemoryGb": 24,
                "maxReplicaMemoryGb": 48,
                "numReplicas": 3
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = super::types::ReplicaScalingRequest {
        min_replica_memory_gb: Some(24),
        max_replica_memory_gb: Some(48),
        num_replicas: Some(3),
        idle_scaling: None,
        idle_timeout_minutes: None,
    };
    let svc = client
        .update_replica_scaling("org-1", "svc-1", &req)
        .await
        .unwrap();
    assert_eq!(svc.min_replica_memory_gb, Some(24));
    assert_eq!(svc.num_replicas, Some(3));
}

#[tokio::test]
async fn test_reset_password() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/organizations/org-1/services/svc-1/password"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"password": "new-secret-pw"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let resp = client.reset_password("org-1", "svc-1").await.unwrap();
    assert_eq!(resp.password, "new-secret-pw");
}

#[tokio::test]
async fn test_get_query_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/services/svc-1/serviceQueryEndpoint"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"openApiEnabled": true, "roles": ["admin"]}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let ep = client.get_query_endpoint("org-1", "svc-1").await.unwrap();
    assert_eq!(ep.open_api_enabled, Some(true));
    assert_eq!(ep.roles.as_ref().unwrap(), &["admin"]);
}

#[tokio::test]
async fn test_create_query_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organizations/org-1/services/svc-1/serviceQueryEndpoint"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"openApiEnabled": true, "roles": ["reader"]}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = super::types::CreateQueryEndpointRequest {
        roles: Some(vec!["reader".to_string()]),
        open_api_enabled: Some(true),
    };
    let ep = client
        .create_query_endpoint("org-1", "svc-1", &req)
        .await
        .unwrap();
    assert_eq!(ep.open_api_enabled, Some(true));
}

#[tokio::test]
async fn test_delete_query_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/organizations/org-1/services/svc-1/serviceQueryEndpoint"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    client
        .delete_query_endpoint("org-1", "svc-1")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_create_private_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organizations/org-1/services/svc-1/privateEndpoint"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({
            "id": "vpce-123",
            "description": "My VPC endpoint"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "vpce-123",
                "description": "My VPC endpoint",
                "cloudProvider": "aws",
                "region": "us-east-1"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = super::types::CreatePrivateEndpointRequest {
        id: "vpce-123".to_string(),
        description: Some("My VPC endpoint".to_string()),
    };
    let ep = client
        .create_private_endpoint("org-1", "svc-1", &req)
        .await
        .unwrap();
    assert_eq!(ep.id.as_deref(), Some("vpce-123"));
    assert_eq!(ep.cloud_provider.as_deref(), Some("aws"));
}

#[tokio::test]
async fn test_list_services_filtered() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/services"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [{
                "id": "svc-filtered",
                "name": "filtered-svc",
                "provider": "aws",
                "region": "us-east-1",
                "state": "running"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let services = client
        .list_services_filtered("org-1", &["tag:env=prod".to_string()])
        .await
        .unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].id, "svc-filtered");
}

// ── Error handling ──────────────────────────────────────────────────

#[tokio::test]
async fn test_api_error_response() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/bad-id"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": {"code": "NOT_FOUND", "message": "Organization not found"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let err = client.get_organization("bad-id").await.unwrap_err();
    assert_eq!(err.message, "Organization not found");
}

// ── Phase 3: Org update, prometheus, usage ──────────────────────────

#[tokio::test]
async fn test_update_organization() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/organizations/org-1"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"name": "New Name"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "org-1", "name": "New Name", "createdAt": "2024-01-01T00:00:00Z"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = UpdateOrgRequest {
        name: Some("New Name".to_string()),
    };
    let org = client.update_organization("org-1", &req).await.unwrap();
    assert_eq!(org.id, "org-1");
    assert_eq!(org.name, "New Name");
}

#[tokio::test]
async fn test_get_org_prometheus() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/prometheus"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"host": "prom.example.com", "port": "9090", "protocol": "https"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let prom = client.get_org_prometheus("org-1").await.unwrap();
    assert_eq!(prom.host.as_deref(), Some("prom.example.com"));
    assert_eq!(prom.port.as_deref(), Some("9090"));
    assert_eq!(prom.protocol.as_deref(), Some("https"));
}

#[tokio::test]
async fn test_get_org_usage() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/usageCost"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "totalCost": 123.45,
                "currency": "USD",
                "usageDetails": [
                    {"serviceName": "svc-a", "serviceId": "svc-1", "cost": 100.0, "unit": "credits"}
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let usage = client.get_org_usage("org-1").await.unwrap();
    assert_eq!(usage.total_cost, Some(123.45));
    assert_eq!(usage.currency.as_deref(), Some("USD"));
    let details = usage.usage_details.unwrap();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].service_name.as_deref(), Some("svc-a"));
    assert_eq!(details[0].cost, Some(100.0));
}

// ── Phase 4: Members ────────────────────────────────────────────────

#[tokio::test]
async fn test_list_members() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/members"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                {"userId": "user-1", "email": "alice@example.com", "role": "admin"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let members = client.list_members("org-1").await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].user_id, "user-1");
    assert_eq!(members[0].email, "alice@example.com");
    assert_eq!(members[0].role, "admin");
}

#[tokio::test]
async fn test_get_member() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/members/user-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"userId": "user-1", "email": "alice@example.com", "role": "admin"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let member = client.get_member("org-1", "user-1").await.unwrap();
    assert_eq!(member.user_id, "user-1");
    assert_eq!(member.role, "admin");
}

#[tokio::test]
async fn test_update_member() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/organizations/org-1/members/user-1"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"role": "admin"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"userId": "user-1", "email": "alice@example.com", "role": "admin"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = UpdateMemberRequest {
        role: "admin".to_string(),
    };
    let member = client.update_member("org-1", "user-1", &req).await.unwrap();
    assert_eq!(member.role, "admin");
}

#[tokio::test]
async fn test_delete_member() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/organizations/org-1/members/user-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    client.delete_member("org-1", "user-1").await.unwrap();
}

// ── Phase 4: Invitations ────────────────────────────────────────────

#[tokio::test]
async fn test_list_invitations() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/invitations"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                {"id": "inv-1", "email": "bob@example.com", "role": "developer"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let invitations = client.list_invitations("org-1").await.unwrap();
    assert_eq!(invitations.len(), 1);
    assert_eq!(invitations[0].id, "inv-1");
    assert_eq!(invitations[0].email, "bob@example.com");
    assert_eq!(invitations[0].role, "developer");
}

#[tokio::test]
async fn test_create_invitation() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organizations/org-1/invitations"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"email": "a@b.com", "role": "developer"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "inv-new", "email": "a@b.com", "role": "developer"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = CreateInvitationRequest {
        email: "a@b.com".to_string(),
        role: "developer".to_string(),
    };
    let inv = client.create_invitation("org-1", &req).await.unwrap();
    assert_eq!(inv.id, "inv-new");
    assert_eq!(inv.email, "a@b.com");
    assert_eq!(inv.role, "developer");
}

#[tokio::test]
async fn test_get_invitation() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/invitations/inv-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "inv-1", "email": "bob@example.com", "role": "developer"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let inv = client.get_invitation("org-1", "inv-1").await.unwrap();
    assert_eq!(inv.id, "inv-1");
    assert_eq!(inv.email, "bob@example.com");
}

#[tokio::test]
async fn test_delete_invitation() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/organizations/org-1/invitations/inv-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    client.delete_invitation("org-1", "inv-1").await.unwrap();
}

// ── Phase 5: API Keys ──────────────────────────────────────────────

#[tokio::test]
async fn test_list_api_keys() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/keys"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                {"id": "key-1", "name": "my-key", "state": "active"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let keys = client.list_api_keys("org-1").await.unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].id, "key-1");
    assert_eq!(keys[0].name, "my-key");
    assert_eq!(keys[0].state, "active");
}

#[tokio::test]
async fn test_create_api_key() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organizations/org-1/keys"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"name": "new-key"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "apiKey": {"id": "key-new", "name": "new-key", "state": "active"},
                "keyId": "key-new",
                "keySecret": "super-secret"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = CreateApiKeyRequest {
        name: "new-key".to_string(),
        roles: None,
        expires_at: None,
    };
    let resp = client.create_api_key("org-1", &req).await.unwrap();
    assert_eq!(resp.api_key.id, "key-new");
    assert_eq!(resp.key_id, "key-new");
    assert_eq!(resp.key_secret, "super-secret");
}

#[tokio::test]
async fn test_get_api_key() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/keys/key-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "key-1", "name": "my-key", "state": "active"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let key = client.get_api_key("org-1", "key-1").await.unwrap();
    assert_eq!(key.id, "key-1");
    assert_eq!(key.name, "my-key");
}

#[tokio::test]
async fn test_update_api_key() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/organizations/org-1/keys/key-1"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"name": "renamed-key"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "key-1", "name": "renamed-key", "state": "active"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = UpdateApiKeyRequest {
        name: Some("renamed-key".to_string()),
        ..Default::default()
    };
    let key = client.update_api_key("org-1", "key-1", &req).await.unwrap();
    assert_eq!(key.name, "renamed-key");
}

#[tokio::test]
async fn test_delete_api_key() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/organizations/org-1/keys/key-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    client.delete_api_key("org-1", "key-1").await.unwrap();
}

// ── Phase 6: Activities ─────────────────────────────────────────────

#[tokio::test]
async fn test_list_activities() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/activities"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                {"id": "act-1", "activityType": "service_start", "status": "completed"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let activities = client.list_activities("org-1").await.unwrap();
    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].id, "act-1");
    assert_eq!(activities[0].activity_type, "service_start");
    assert_eq!(activities[0].status, "completed");
}

#[tokio::test]
async fn test_get_activity() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/activities/act-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "act-1", "activityType": "service_start", "status": "completed"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let activity = client.get_activity("org-1", "act-1").await.unwrap();
    assert_eq!(activity.id, "act-1");
    assert_eq!(activity.activity_type, "service_start");
}

// ── Phase 6: BYOC ──────────────────────────────────────────────────

#[tokio::test]
async fn test_create_byoc() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organizations/org-1/byocInfrastructure"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"provider": "aws", "region": "us-east-1"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "byoc-1", "provider": "aws", "region": "us-east-1", "state": "provisioning"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = CreateByocRequest {
        provider: "aws".to_string(),
        region: "us-east-1".to_string(),
        vpc_id: None,
    };
    let byoc = client.create_byoc("org-1", &req).await.unwrap();
    assert_eq!(byoc.id.as_deref(), Some("byoc-1"));
    assert_eq!(byoc.provider, "aws");
    assert_eq!(byoc.state.as_deref(), Some("provisioning"));
}

#[tokio::test]
async fn test_update_byoc() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/organizations/org-1/byocInfrastructure/byoc-1"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"state": "active"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "byoc-1", "provider": "aws", "region": "us-east-1", "state": "active"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = UpdateByocRequest {
        state: Some("active".to_string()),
        ..Default::default()
    };
    let byoc = client.update_byoc("org-1", "byoc-1", &req).await.unwrap();
    assert_eq!(byoc.state.as_deref(), Some("active"));
}

#[tokio::test]
async fn test_delete_byoc() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/organizations/org-1/byocInfrastructure/byoc-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    client.delete_byoc("org-1", "byoc-1").await.unwrap();
}

// ── Phase 6: Backup Bucket ─────────────────────────────────────────

#[tokio::test]
async fn test_list_backup_buckets() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/services/svc-1/backupBucket"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [
                {"id": "bucket-1", "bucketName": "my-backup-bucket", "provider": "aws", "region": "us-east-1"}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let buckets = client.list_backup_buckets("org-1", "svc-1").await.unwrap();
    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0].id.as_deref(), Some("bucket-1"));
    assert_eq!(buckets[0].bucket_name, "my-backup-bucket");
}

#[tokio::test]
async fn test_create_backup_bucket() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organizations/org-1/services/svc-1/backupBucket"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"bucketName": "new-bucket"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "bucket-new", "bucketName": "new-bucket", "state": "creating"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = CreateBackupBucketRequest {
        bucket_name: "new-bucket".to_string(),
        bucket_path: None,
    };
    let bucket = client.create_backup_bucket("org-1", "svc-1", &req).await.unwrap();
    assert_eq!(bucket.id.as_deref(), Some("bucket-new"));
    assert_eq!(bucket.bucket_name, "new-bucket");
}

#[tokio::test]
async fn test_update_backup_bucket() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/organizations/org-1/services/svc-1/backupBucket/bucket-1"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"bucketPath": "/new/path"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"id": "bucket-1", "bucketName": "my-bucket", "bucketPath": "/new/path"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = UpdateBackupBucketRequest {
        bucket_path: Some("/new/path".to_string()),
        ..Default::default()
    };
    let bucket = client.update_backup_bucket("org-1", "svc-1", "bucket-1", &req).await.unwrap();
    assert_eq!(bucket.bucket_path.as_deref(), Some("/new/path"));
}

#[tokio::test]
async fn test_delete_backup_bucket() {
    let server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/organizations/org-1/services/svc-1/backupBucket/bucket-1"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    client.delete_backup_bucket("org-1", "svc-1", "bucket-1").await.unwrap();
}

// ── Phase 6: Backup Config ─────────────────────────────────────────

#[tokio::test]
async fn test_get_backup_config() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/services/svc-1/backupConfiguration"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"schedule": "0 0 * * *", "retentionPeriodDays": 30}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let config = client.get_backup_config("org-1", "svc-1").await.unwrap();
    assert_eq!(config.schedule.as_deref(), Some("0 0 * * *"));
    assert_eq!(config.retention_period_days, Some(30));
}

#[tokio::test]
async fn test_update_backup_config() {
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .and(path("/organizations/org-1/services/svc-1/backupConfiguration"))
        .and(auth_header_matcher())
        .and(body_json(serde_json::json!({"retentionPeriodDays": 14})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"schedule": "0 0 * * *", "retentionPeriodDays": 14}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let req = UpdateBackupConfigRequest {
        retention_period_days: Some(14),
        ..Default::default()
    };
    let config = client.update_backup_config("org-1", "svc-1", &req).await.unwrap();
    assert_eq!(config.retention_period_days, Some(14));
}

// ── Phase 6: Service Prometheus ─────────────────────────────────────

#[tokio::test]
async fn test_get_service_prometheus() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/organizations/org-1/services/svc-1/prometheus"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"host": "prom-svc.example.com", "port": 9090, "protocol": "https"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let prom = client.get_service_prometheus("org-1", "svc-1").await.unwrap();
    assert_eq!(prom.host.as_deref(), Some("prom-svc.example.com"));
    assert_eq!(prom.port, Some(9090));
    assert_eq!(prom.protocol.as_deref(), Some("https"));
}

#[tokio::test]
async fn test_setup_service_prometheus() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/organizations/org-1/services/svc-1/prometheus"))
        .and(auth_header_matcher())
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"host": "prom-svc.example.com", "port": 9090, "protocol": "https"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = test_client(&server);
    let prom = client.setup_service_prometheus("org-1", "svc-1").await.unwrap();
    assert_eq!(prom.host.as_deref(), Some("prom-svc.example.com"));
    assert_eq!(prom.port, Some(9090));
}
