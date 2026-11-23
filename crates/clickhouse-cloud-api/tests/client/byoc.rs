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
        access_model: None,
        restricted_facts: None,
        account_id: "123456789012".to_string(),
        availability_zone_suffixes: Some(vec![
            ByocAvailabilityZoneSuffix::A,
            ByocAvailabilityZoneSuffix::B,
        ]),
        display_name: Some("My BYOC".to_string()),
        external_id: None,
        gcp_pod_cidr_range_names: None,
        gcp_shared_vpc_host_project_id: None,
        private_subnet_ids: None,
        public_subnet_ids: None,
        region_id: ByocInfrastructurePostRequestRegionid::Ap_northeast_1,
        service_principal_client_id: None,
        tags: None,
        tenant_id: None,
        vpc_cidr_range: None,
        vpc_id: None,
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
        access_model: None,
        restricted_facts: None,
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
        availability_zone_suffixes: None,
        display_name: None,
        gcp_pod_cidr_range_names: None,
        gcp_shared_vpc_host_project_id: None,
        service_principal_client_id: None,
        tenant_id: None,
        vpc_cidr_range: None,
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
        access_model: None,
        restricted_facts: None,
        account_id: "123456789012".to_string(),
        region_id: ByocInfrastructureValidatePostRequestRegionid::Eu_west_1,
        availability_zone_suffixes: Some(vec![
            ByocAvailabilityZoneSuffix::A,
            ByocAvailabilityZoneSuffix::B,
            ByocAvailabilityZoneSuffix::C,
        ]),
        vpc_cidr_range: Some("10.0.0.0/16".to_string()),
        external_id: Some("ch-ext".to_string()),
        gcp_pod_cidr_range_names: None,
        gcp_shared_vpc_host_project_id: None,
        private_subnet_ids: None,
        public_subnet_ids: None,
        service_principal_client_id: None,
        tags: None,
        tenant_id: None,
        vpc_id: None,
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
        progress.updated_at.map(|t| t.to_string()),
        Some("2026-09-30T12:00:00Z".to_string())
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

// ===========================================================================
// BYOC customer-authorized access
// ===========================================================================

#[tokio::test]
async fn get_byoc_customer_authorized_access() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/infra-1/customerAuthorizedAccess",
        ))
        .and(basic_auth("key", "secret"))
        .respond_with(ok_json(serde_json::json!({
            "mode": "window",
            "startTimestamp": "2026-11-09T10:00:00Z",
            "durationSeconds": 3600,
            "allowed": true,
            "windowEndTimestamp": "2026-11-09T11:00:00Z"
        })))
        .expect(1)
        .mount(&s)
        .await;

    let access = c
        .organization_byoc_infrastructure_customer_authorized_access_get("org-1", "infra-1")
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(access.mode.as_deref(), Some("window"));
    assert_eq!(access.duration_seconds, Some(3600));
    assert_eq!(access.allowed, Some(true));
    assert!(access.window_end_timestamp.is_some());
}

#[tokio::test]
async fn update_byoc_customer_authorized_access_sends_exact_body() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/infra-1/customerAuthorizedAccess",
        ))
        .and(basic_auth("key", "secret"))
        .and(body_json(serde_json::json!({
            "mode": "window",
            "startTimestamp": "2026-11-09T10:00:00Z",
            "durationSeconds": 7200
        })))
        .respond_with(ok_json(serde_json::json!({
            "mode": "window",
            "startTimestamp": "2026-11-09T10:00:00Z",
            "durationSeconds": 7200,
            "allowed": false
        })))
        .expect(1)
        .mount(&s)
        .await;

    let body = ByocCustomerAuthorizedAccessPatchRequest {
        mode: Some("window".into()),
        start_timestamp: Some("2026-11-09T10:00:00Z".parse().unwrap()),
        duration_seconds: Some(7200),
    };
    let access = c
        .organization_byoc_infrastructure_customer_authorized_access_update(
            "org-1", "infra-1", &body,
        )
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(access.allowed, Some(false));
    assert_eq!(access.window_end_timestamp, None);
}

#[tokio::test]
async fn byoc_customer_authorized_access_surfaces_api_errors() {
    let (s, c) = setup().await;

    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/infra-1/customerAuthorizedAccess",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "error": "durationSeconds is required"
        })))
        .mount(&s)
        .await;

    let body = ByocCustomerAuthorizedAccessPatchRequest {
        mode: Some("window".into()),
        ..Default::default()
    };
    let err = c
        .organization_byoc_infrastructure_customer_authorized_access_update(
            "org-1", "infra-1", &body,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, clickhouse_cloud_api::Error::Api { status: 400, message } if message == "durationSeconds is required")
    );
}

// ===========================================================================
// BYOC enrollment tokens
// ===========================================================================

#[tokio::test]
async fn byoc_enrollment_token_create_sends_ttl_and_decodes_the_token() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/infra-1/enrollmentToken",
        ))
        .and(body_json(serde_json::json!({"ttlSeconds": 7200})))
        .respond_with(ok_json(serde_json::json!({
            "token": "enroll-secret",
            "expiresAt": "2026-11-23T14:00:00Z"
        })))
        .expect(1)
        .mount(&s)
        .await;

    let body = ByocInfrastructureEnrollmentTokenPostRequest {
        ttl_seconds: Some(7200),
    };
    let token = c
        .organization_byoc_infrastructure_enrollment_token_create("org-1", "infra-1", &body)
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(token.token.as_deref(), Some("enroll-secret"));
    assert_eq!(
        token.expires_at,
        Some("2026-11-23T14:00:00Z".parse().unwrap())
    );
}

#[tokio::test]
async fn byoc_enrollment_token_create_default_body_leaves_the_ttl_to_the_server() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/infra-1/enrollmentToken",
        ))
        .and(body_json(serde_json::json!({})))
        .respond_with(ok_json(serde_json::json!({"token": "t"})))
        .expect(1)
        .mount(&s)
        .await;

    let resp = c
        .organization_byoc_infrastructure_enrollment_token_create(
            "org-1",
            "infra-1",
            &ByocInfrastructureEnrollmentTokenPostRequest::default(),
        )
        .await
        .unwrap();
    assert_eq!(resp.result.unwrap().token.as_deref(), Some("t"));
}

#[tokio::test]
async fn byoc_enrollment_token_create_surfaces_409_before_roles_are_admitted() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org-1/byocInfrastructure/infra-1/enrollmentToken",
        ))
        .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "status": 409,
            "error": "roles not admitted yet"
        })))
        .mount(&s)
        .await;

    let err = c
        .organization_byoc_infrastructure_enrollment_token_create(
            "org-1",
            "infra-1",
            &ByocInfrastructureEnrollmentTokenPostRequest::default(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, clickhouse_cloud_api::Error::Api { status: 409, message } if message == "roles not admitted yet")
    );
}

#[tokio::test]
async fn create_restricted_byoc_infrastructure_sends_access_model_and_facts() {
    let (s, c) = setup().await;

    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/byocInfrastructure"))
        .and(body_json(serde_json::json!({
            "accountId": "123456789012",
            "regionId": "us-east-1",
            "accessModel": "restricted",
            "restrictedFacts": {
                "clusterName": "eks-1",
                "imageMode": "bundle",
                "imageRegistry": "123456789012.dkr.ecr.us-east-1.amazonaws.com/ch",
                "dataBucket": "data",
                "backupBucket": "backups",
                "storageRoleArn": "arn:aws:iam::123456789012:role/storage"
            }
        })))
        .respond_with(ok_json(serde_json::json!({
            "id": "byoc-1",
            "accessModel": "restricted"
        })))
        .expect(1)
        .mount(&s)
        .await;

    let body = ByocInfrastructurePostRequest {
        access_model: Some("restricted".to_string()),
        restricted_facts: Some(ByocRestrictedFacts {
            image_registry: Some("123456789012.dkr.ecr.us-east-1.amazonaws.com/ch".to_string()),
            ..restricted_facts("bundle")
        }),
        account_id: "123456789012".to_string(),
        region_id: ByocInfrastructurePostRequestRegionid::Us_east_1,
        availability_zone_suffixes: None,
        display_name: None,
        external_id: None,
        gcp_pod_cidr_range_names: None,
        gcp_shared_vpc_host_project_id: None,
        private_subnet_ids: None,
        public_subnet_ids: None,
        service_principal_client_id: None,
        tags: None,
        tenant_id: None,
        vpc_cidr_range: None,
        vpc_id: None,
    };
    let config = c
        .organization_byoc_infrastructure_create("org-1", &body)
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(config.access_model.as_deref(), Some("restricted"));
}

#[tokio::test]
async fn get_restricted_byoc_infrastructure_decodes_facts_and_connector() {
    let (s, c) = setup().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/byocInfrastructure/infra-1"))
        .respond_with(ok_json(serde_json::json!({
            "id": "infra-1",
            "accessModel": "restricted",
            "restrictedFacts": {"clusterName": "eks-1", "imageMode": "direct"},
            "connector": {
                "platformState": "waiting",
                "components": [{"name": "scraper", "state": "healthy"}]
            }
        })))
        .mount(&s)
        .await;

    let details = c
        .organization_byoc_infrastructure_get("org-1", "infra-1")
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(details.access_model.as_deref(), Some("restricted"));
    assert_eq!(
        details
            .restricted_facts
            .as_ref()
            .and_then(|facts| facts.cluster_name.as_deref()),
        Some("eks-1")
    );
    let connector = details.connector.unwrap();
    assert_eq!(connector.platform_state.as_deref(), Some("waiting"));
    assert_eq!(
        connector.components.unwrap()[0].name.as_deref(),
        Some("scraper")
    );
}

fn restricted_facts(image_mode: &str) -> ByocRestrictedFacts {
    ByocRestrictedFacts {
        availability_zones: None,
        backup_bucket: "backups".to_string(),
        cluster_name: "eks-1".to_string(),
        data_bucket: "data".to_string(),
        enable_autoscaler: None,
        enable_monitoring: None,
        enable_snapshot_controller: None,
        executor_role_arn: None,
        image_mode: image_mode.to_string(),
        image_registry: None,
        namespace_prefix: None,
        node_role_arns: None,
        platform_node_group: None,
        scraper_endpoint: None,
        storage_role_arn: "arn:aws:iam::123456789012:role/storage".to_string(),
    }
}
