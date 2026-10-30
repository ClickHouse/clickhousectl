use clickhouse_cloud_api::models::*;
use wiremock::matchers::{basic_auth, body_json, body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::{ok_empty, ok_json, setup};

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
        availability_zone_suffixes: Some(vec![
            ByocAvailabilityZoneSuffix::A,
            ByocAvailabilityZoneSuffix::B,
        ]),
        display_name: Some("My BYOC".to_string()),
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
async fn create_byoc_infrastructure_sends_byo_vpc_fields_and_tags() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/byocInfrastructure"))
        .and(body_json(serde_json::json!({
            "accountId": "123456789012",
            "regionId": "us-east-1",
            "externalId": "ch-ext",
            "vpcId": "vpc-1",
            "privateSubnetIds": ["subnet-a", "subnet-b"],
            "publicSubnetIds": ["subnet-pub"],
            "tags": {"team": "data", "cost-center": "42"}
        })))
        .respond_with(ok_json(serde_json::json!({"id": "byoc-1"})))
        .expect(1)
        .mount(&s)
        .await;

    let body = ByocInfrastructurePostRequest {
        account_id: "123456789012".to_string(),
        region_id: ByocInfrastructurePostRequestRegionid::Us_east_1,
        external_id: Some("ch-ext".to_string()),
        vpc_id: Some("vpc-1".to_string()),
        private_subnet_ids: Some(vec!["subnet-a".to_string(), "subnet-b".to_string()]),
        public_subnet_ids: Some(vec!["subnet-pub".to_string()]),
        tags: Some(ByocInfrastructureTags::from([
            ("team".to_string(), "data".to_string()),
            ("cost-center".to_string(), "42".to_string()),
        ])),
        ..Default::default()
    };
    let resp = c
        .organization_byoc_infrastructure_create("org-1", &body)
        .await
        .unwrap();
    assert_eq!(resp.result.unwrap().id.as_deref(), Some("byoc-1"));
}

#[tokio::test]
async fn validate_byoc_infrastructure() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/validate"))
        .and(body_json(serde_json::json!({
            "accountId": "123456789012",
            "regionId": "eu-west-1",
            "availabilityZoneSuffixes": ["a", "b", "c"],
            "vpcCidrRange": "10.0.0.0/16",
            "externalId": "ch-ext"
        })))
        .respond_with(ok_json(serde_json::json!({
            "cloudProvider": "aws",
            "allPassed": false,
            "anyPassed": true,
            "supported": true,
            "checks": [
                {
                    "name": "Create EKS cluster",
                    "action": "eks:CreateCluster",
                    "allowed": true,
                    "group": "base"
                },
                {
                    "name": "Create VPC",
                    "action": "ec2:CreateVpc",
                    "allowed": false,
                    "reason": "implicitDeny",
                    "group": "vpc-write"
                }
            ]
        })))
        .expect(1)
        .mount(&s)
        .await;

    let body = ByocInfrastructureValidatePostRequest {
        account_id: "123456789012".to_string(),
        region_id: ByocInfrastructureValidatePostRequestRegionid::Eu_west_1,
        availability_zone_suffixes: Some(vec![
            ByocAvailabilityZoneSuffix::A,
            ByocAvailabilityZoneSuffix::B,
            ByocAvailabilityZoneSuffix::C,
        ]),
        vpc_cidr_range: Some("10.0.0.0/16".to_string()),
        external_id: Some("ch-ext".to_string()),
        ..Default::default()
    };
    let resp = c
        .organization_byoc_infrastructure_validate("org-1", &body)
        .await
        .unwrap();
    let validation = resp.result.unwrap();
    assert_eq!(
        validation.cloud_provider,
        Some(ByocInfrastructureValidationCloudprovider::Aws)
    );
    assert_eq!(validation.all_passed, Some(false));
    assert_eq!(validation.any_passed, Some(true));
    assert_eq!(validation.supported, Some(true));
    let checks = validation.checks.unwrap();
    assert_eq!(checks.len(), 2);
    assert_eq!(checks[0].action.as_deref(), Some("eks:CreateCluster"));
    assert_eq!(checks[0].allowed, Some(true));
    assert_eq!(checks[0].reason, None);
    assert_eq!(checks[1].allowed, Some(false));
    assert_eq!(checks[1].reason.as_deref(), Some("implicitDeny"));
    assert_eq!(checks[1].group.as_deref(), Some("vpc-write"));
}

#[tokio::test]
async fn get_byoc_infrastructure() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/byoc-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "byoc-1",
            "state": "infra-provisioning",
            "accountId": "123456789012",
            "regionId": "us-central1",
            "cloudProvider": "gcp",
            "displayName": "My BYOC",
            "enablePrivateLink": true,
            "enablePrivateLoadBalancer": false,
            "enablePublicLoadBalancer": true,
            "vpcAvailabilityZoneList": ["us-central1-a", "us-central1-b"],
            "isByoVpc": true,
            "byoVpcId": "my-network",
            "byoVpcPrivateSubnetIds": ["my-subnet"],
            "byoVpcPodCidrRangeNames": ["pods"],
            "byoVpcSharedVpcHostProjectId": "host-project",
            "gcpPscSubnetId": "psc-subnet"
        })))
        .expect(1)
        .mount(&s)
        .await;

    let resp = c
        .organization_byoc_infrastructure_get("org-1", "byoc-1")
        .await
        .unwrap();
    let details = resp.result.unwrap();
    assert_eq!(details.id.as_deref(), Some("byoc-1"));
    assert_eq!(
        details.state,
        Some(ByocInfrastructureDetailsState::Infra_provisioning)
    );
    assert_eq!(
        details.region_id,
        Some(ByocInfrastructureDetailsRegionid::Us_central1)
    );
    assert_eq!(
        details.cloud_provider,
        Some(ByocInfrastructureDetailsCloudprovider::Gcp)
    );
    assert_eq!(details.enable_private_link, Some(true));
    assert_eq!(details.enable_private_load_balancer, Some(false));
    assert_eq!(details.is_byo_vpc, Some(true));
    assert_eq!(details.byo_vpc_id.as_deref(), Some("my-network"));
    assert_eq!(
        details.byo_vpc_private_subnet_ids,
        Some(vec!["my-subnet".to_string()])
    );
    assert_eq!(
        details.byo_vpc_shared_vpc_host_project_id.as_deref(),
        Some("host-project")
    );
    assert_eq!(details.gcp_psc_subnet_id.as_deref(), Some("psc-subnet"));
    assert_eq!(details.vpc_cidr_range, None);
}

#[tokio::test]
async fn get_byoc_infrastructure_progress() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/byoc-1/progress",
        ))
        .respond_with(ok_json(serde_json::json!({
            "id": "byoc-1",
            "status": "in_progress",
            "updatedAt": "2026-09-30T12:00:00Z",
            "stages": [
                {
                    "name": "network",
                    "status": "ready",
                    "subStages": [{"name": "vpc", "status": "ready"}]
                },
                {"name": "cluster", "status": "failed", "message": "quota exceeded"}
            ]
        })))
        .expect(1)
        .mount(&s)
        .await;

    let resp = c
        .organization_byoc_infrastructure_progress_get("org-1", "byoc-1")
        .await
        .unwrap();
    let progress = resp.result.unwrap();
    assert_eq!(progress.id.as_deref(), Some("byoc-1"));
    assert_eq!(
        progress.status,
        Some(ByocInfrastructureProgressStatus::InProgress)
    );
    assert_eq!(
        progress.updated_at.map(|t| t.to_rfc3339()),
        Some("2026-09-30T12:00:00+00:00".to_string())
    );
    let stages = progress.stages.unwrap();
    assert_eq!(stages.len(), 2);
    let sub = stages[0].sub_stages.as_ref().unwrap();
    assert_eq!(sub[0].name.as_deref(), Some("vpc"));
    assert_eq!(
        stages[1].status,
        Some(ByocInfrastructureProgressStageStatus::Failed)
    );
    assert_eq!(stages[1].message.as_deref(), Some("quota exceeded"));
}

#[tokio::test]
async fn byoc_infrastructure_get_surfaces_api_errors() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/missing/progress",
        ))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "BYOC infrastructure not found"
        })))
        .mount(&s)
        .await;

    let err = c
        .organization_byoc_infrastructure_progress_get("org-1", "missing")
        .await
        .unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 404);
            assert_eq!(message, "BYOC infrastructure not found");
        }
        other => panic!("unexpected error: {other:?}"),
    }
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
        ..Default::default()
    };
    let resp = c
        .organization_byoc_infrastructure_update("org-1", "byoc-1", &body)
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(config.display_name.as_deref(), Some("Renamed BYOC"));
}

#[tokio::test]
async fn update_byoc_infrastructure_sends_connectivity_and_tags() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/byoc-1"))
        .and(basic_auth("key", "secret"))
        .and(body_json(serde_json::json!({
            "enablePrivateLink": true,
            "enablePrivateLoadBalancer": false,
            "enablePublicLoadBalancer": true,
            "gcpPscSubnetId": "psc-subnet",
            "tags": {"team": "data"}
        })))
        .respond_with(ok_json(serde_json::json!({"id": "byoc-1"})))
        .expect(1)
        .mount(&s)
        .await;

    let body = ByocInfrastructurePatchRequest {
        display_name: None,
        enable_private_link: Some(true),
        enable_private_load_balancer: Some(false),
        enable_public_load_balancer: Some(true),
        gcp_psc_subnet_id: Some("psc-subnet".to_string()),
        tags: Some(ByocInfrastructureTags::from([(
            "team".to_string(),
            "data".to_string(),
        )])),
    };
    let resp = c
        .organization_byoc_infrastructure_update("org-1", "byoc-1", &body)
        .await
        .unwrap();
    assert_eq!(resp.result.unwrap().id.as_deref(), Some("byoc-1"));
}

#[tokio::test]
async fn get_byoc_infrastructure_private_endpoint_config() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/byoc-1/privateEndpointConfig",
        ))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({
            "endpointName": "com.amazonaws.vpce.us-east-1.vpce-svc-123",
            "privateDnsHostname": "byoc-1.private.clickhouse.cloud"
        })))
        .expect(1)
        .mount(&s)
        .await;

    let resp = c
        .organization_byoc_infrastructure_private_endpoint_config_get("org-1", "byoc-1")
        .await
        .unwrap();
    let config = resp.result.unwrap();
    assert_eq!(
        config.endpoint_name.as_deref(),
        Some("com.amazonaws.vpce.us-east-1.vpce-svc-123")
    );
    assert_eq!(
        config.private_dns_hostname.as_deref(),
        Some("byoc-1.private.clickhouse.cloud")
    );
}

#[tokio::test]
async fn get_byoc_infrastructure_tags() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/byoc-1/tags",
        ))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({
            "tags": {"team": "data", "env": "prod"}
        })))
        .expect(1)
        .mount(&s)
        .await;

    let resp = c
        .organization_byoc_infrastructure_tags_get("org-1", "byoc-1")
        .await
        .unwrap();
    let tags = resp.result.unwrap().tags.unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags.get("team").map(String::as_str), Some("data"));
    assert_eq!(tags.get("env").map(String::as_str), Some("prod"));
}

#[tokio::test]
async fn byoc_infrastructure_tags_get_surfaces_api_errors() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/missing/tags",
        ))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "error": "BYOC infrastructure not found"
        })))
        .mount(&s)
        .await;

    let err = c
        .organization_byoc_infrastructure_tags_get("org-1", "missing")
        .await
        .unwrap_err();
    match err {
        clickhouse_cloud_api::Error::Api { status, message } => {
            assert_eq!(status, 404);
            assert_eq!(message, "BYOC infrastructure not found");
        }
        other => panic!("unexpected error: {other:?}"),
    }
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
