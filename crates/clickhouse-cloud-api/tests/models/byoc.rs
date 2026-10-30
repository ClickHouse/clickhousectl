use clickhouse_cloud_api::models::*;

#[test]
fn serialize_byoc_infrastructure_post_request() {
    let req = ByocInfrastructurePostRequest {
        account_id: "123456789012".to_string(),
        availability_zone_suffixes: Some(vec![
            ByocAvailabilityZoneSuffix::A,
            ByocAvailabilityZoneSuffix::B,
            ByocAvailabilityZoneSuffix::C,
            ByocAvailabilityZoneSuffix::D,
            ByocAvailabilityZoneSuffix::E,
            ByocAvailabilityZoneSuffix::F,
        ]),
        display_name: Some("My BYOC".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["accountId"], "123456789012");
    assert_eq!(
        json["availabilityZoneSuffixes"],
        serde_json::json!(["a", "b", "c", "d", "e", "f"])
    );
    assert_eq!(json["displayName"], "My BYOC");
}

#[test]
fn byoc_infrastructure_post_request_requires_only_account_and_region() {
    // Compile-time strictness: accountId and regionId are the spec's only required fields.
    let req = ByocInfrastructurePostRequest {
        account_id: "123456789012".to_string(),
        region_id: ByocInfrastructurePostRequestRegionid::Eastus,
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
    assert_eq!(
        serde_json::to_value(&req).unwrap(),
        serde_json::json!({"accountId": "123456789012", "regionId": "eastus"})
    );
}

#[test]
fn byoc_infrastructure_post_request_serializes_every_optional_field() {
    let req = ByocInfrastructurePostRequest {
        account_id: "project-1".to_string(),
        region_id: ByocInfrastructurePostRequestRegionid::Us_central1,
        availability_zone_suffixes: Some(vec![ByocAvailabilityZoneSuffix::A]),
        display_name: Some("gcp".to_string()),
        external_id: Some("ext".to_string()),
        gcp_pod_cidr_range_names: Some(vec!["pods".to_string()]),
        gcp_shared_vpc_host_project_id: Some("host".to_string()),
        private_subnet_ids: Some(vec!["private".to_string()]),
        public_subnet_ids: Some(vec!["public".to_string()]),
        service_principal_client_id: Some("sp".to_string()),
        tags: Some(ByocInfrastructureTags::from([(
            "env".to_string(),
            "prod".to_string(),
        )])),
        tenant_id: Some("tenant".to_string()),
        vpc_cidr_range: Some("10.0.0.0/16".to_string()),
        vpc_id: Some("network".to_string()),
    };
    assert_eq!(
        serde_json::to_value(&req).unwrap(),
        serde_json::json!({
            "accountId": "project-1",
            "regionId": "us-central1",
            "availabilityZoneSuffixes": ["a"],
            "displayName": "gcp",
            "externalId": "ext",
            "gcpPodCidrRangeNames": ["pods"],
            "gcpSharedVpcHostProjectId": "host",
            "privateSubnetIds": ["private"],
            "publicSubnetIds": ["public"],
            "servicePrincipalClientId": "sp",
            "tags": {"env": "prod"},
            "tenantId": "tenant",
            "vpcCidrRange": "10.0.0.0/16",
            "vpcId": "network"
        })
    );
}

#[test]
fn byoc_infrastructure_validate_post_request_is_strict_on_account_and_region() {
    let req = ByocInfrastructureValidatePostRequest {
        account_id: "sub-1".to_string(),
        region_id: ByocInfrastructureValidatePostRequestRegionid::Westus3,
        availability_zone_suffixes: None,
        external_id: None,
        gcp_pod_cidr_range_names: None,
        gcp_shared_vpc_host_project_id: None,
        private_subnet_ids: None,
        public_subnet_ids: None,
        service_principal_client_id: Some("sp".to_string()),
        tags: None,
        tenant_id: Some("tenant".to_string()),
        vpc_cidr_range: None,
        vpc_id: None,
    };
    assert_eq!(
        serde_json::to_value(&req).unwrap(),
        serde_json::json!({
            "accountId": "sub-1",
            "regionId": "westus3",
            "servicePrincipalClientId": "sp",
            "tenantId": "tenant"
        })
    );
}

#[test]
fn byoc_infrastructure_tags_preserve_dynamic_keys() {
    let tags: ByocInfrastructureTags =
        serde_json::from_str(r#"{"team":"data","kubernetes.io/cluster":"owned","":"empty"}"#)
            .unwrap();
    assert_eq!(tags.len(), 3);
    assert_eq!(tags["team"], "data");
    assert_eq!(tags["kubernetes.io/cluster"], "owned");
    assert_eq!(tags[""], "empty");
    assert_eq!(
        serde_json::to_value(&tags).unwrap(),
        serde_json::json!({"team": "data", "kubernetes.io/cluster": "owned", "": "empty"})
    );
}

#[test]
fn byoc_infrastructure_details_missing_and_null_fields_are_none() {
    let missing: ByocInfrastructureDetails = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, ByocInfrastructureDetails::default());
    assert_eq!(
        serde_json::to_value(&missing).unwrap(),
        serde_json::json!({})
    );

    let null: ByocInfrastructureDetails = serde_json::from_value(serde_json::json!({
        "id": null, "state": null, "accountId": null, "regionId": null,
        "cloudProvider": null, "displayName": null, "enablePrivateLink": null,
        "enablePrivateLoadBalancer": null, "enablePublicLoadBalancer": null,
        "vpcCidrRange": null, "vpcAvailabilityZoneList": null, "isByoVpc": null,
        "byoVpcId": null, "byoVpcPrivateSubnetIds": null, "byoVpcPodCidrRangeNames": null,
        "byoVpcSharedVpcHostProjectId": null, "gcpPscSubnetId": null
    }))
    .unwrap();
    assert_eq!(null, ByocInfrastructureDetails::default());
}

#[test]
fn byoc_infrastructure_details_enums_preserve_unknown_values() {
    let details: ByocInfrastructureDetails = serde_json::from_value(serde_json::json!({
        "state": "infra-hibernating",
        "regionId": "mars-north-1",
        "cloudProvider": "oci"
    }))
    .unwrap();
    assert_eq!(
        details.state,
        Some(ByocInfrastructureDetailsState::Unknown(
            "infra-hibernating".to_string()
        ))
    );
    assert_eq!(
        details.region_id,
        Some(ByocInfrastructureDetailsRegionid::Unknown(
            "mars-north-1".to_string()
        ))
    );
    assert_eq!(
        details.cloud_provider,
        Some(ByocInfrastructureDetailsCloudprovider::Unknown(
            "oci".to_string()
        ))
    );
}

#[test]
fn byoc_infrastructure_progress_missing_and_null_fields_are_none() {
    let missing: ByocInfrastructureProgress = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, ByocInfrastructureProgress::default());
    assert_eq!(
        serde_json::to_value(&missing).unwrap(),
        serde_json::json!({})
    );

    let null: ByocInfrastructureProgress = serde_json::from_value(serde_json::json!({
        "id": null, "status": null, "updatedAt": null, "stages": null
    }))
    .unwrap();
    assert_eq!(null, ByocInfrastructureProgress::default());
}

#[test]
fn byoc_infrastructure_progress_stage_missing_and_null_fields_are_none() {
    let missing: ByocInfrastructureProgressStage = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, ByocInfrastructureProgressStage::default());
    assert_eq!(
        serde_json::to_value(&missing).unwrap(),
        serde_json::json!({})
    );

    let null: ByocInfrastructureProgressStage = serde_json::from_value(serde_json::json!({
        "name": null, "status": null, "updatedAt": null, "message": null, "subStages": null
    }))
    .unwrap();
    assert_eq!(null, ByocInfrastructureProgressStage::default());
}

#[test]
fn byoc_infrastructure_progress_stages_nest_recursively() {
    let progress: ByocInfrastructureProgress = serde_json::from_value(serde_json::json!({
        "id": "byoc-1",
        "status": "waiting",
        "stages": [{
            "name": "network",
            "status": "in_progress",
            "subStages": [{
                "name": "vpc",
                "status": "ready",
                "updatedAt": "2026-09-30T10:00:00Z",
                "subStages": [{
                    "name": "subnet-a",
                    "status": "not_started",
                    "message": "queued"
                }]
            }]
        }]
    }))
    .unwrap();
    assert_eq!(
        progress.status,
        Some(ByocInfrastructureProgressStatus::Waiting)
    );
    let network = &progress.stages.as_ref().unwrap()[0];
    assert_eq!(
        network.status,
        Some(ByocInfrastructureProgressStageStatus::InProgress)
    );
    let vpc = &network.sub_stages.as_ref().unwrap()[0];
    assert_eq!(vpc.name.as_deref(), Some("vpc"));
    assert!(vpc.updated_at.is_some());
    let leaf = &vpc.sub_stages.as_ref().unwrap()[0];
    assert_eq!(leaf.name.as_deref(), Some("subnet-a"));
    assert_eq!(
        leaf.status,
        Some(ByocInfrastructureProgressStageStatus::NotStarted)
    );
    assert_eq!(leaf.message.as_deref(), Some("queued"));
    assert_eq!(leaf.sub_stages, None);

    let reserialized = serde_json::to_value(&progress).unwrap();
    assert_eq!(
        reserialized["stages"][0]["subStages"][0]["subStages"][0]["status"],
        "not_started"
    );
    assert!(
        reserialized["stages"][0]["subStages"][0]["subStages"][0]
            .get("subStages")
            .is_none()
    );
}

#[test]
fn byoc_infrastructure_progress_status_preserves_unknown_values() {
    let status: ByocInfrastructureProgressStatus =
        serde_json::from_str(r#""rolling_back""#).unwrap();
    assert_eq!(
        status,
        ByocInfrastructureProgressStatus::Unknown("rolling_back".to_string())
    );
    assert_eq!(status.to_string(), "rolling_back");
    assert_eq!(
        ByocInfrastructureProgressStageStatus::NotStarted.to_string(),
        "not_started"
    );
}

#[test]
fn byoc_infrastructure_validation_missing_and_null_fields_are_none() {
    let missing: ByocInfrastructureValidation = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, ByocInfrastructureValidation::default());
    assert_eq!(
        serde_json::to_value(&missing).unwrap(),
        serde_json::json!({})
    );

    let null: ByocInfrastructureValidation = serde_json::from_value(serde_json::json!({
        "cloudProvider": null, "allPassed": null, "anyPassed": null,
        "supported": null, "checks": null
    }))
    .unwrap();
    assert_eq!(null, ByocInfrastructureValidation::default());
}

#[test]
fn byoc_infrastructure_validation_check_missing_and_null_fields_are_none() {
    let missing: ByocInfrastructureValidationCheck = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, ByocInfrastructureValidationCheck::default());
    assert_eq!(
        serde_json::to_value(&missing).unwrap(),
        serde_json::json!({})
    );

    let null: ByocInfrastructureValidationCheck = serde_json::from_value(serde_json::json!({
        "name": null, "action": null, "allowed": null, "reason": null, "group": null
    }))
    .unwrap();
    assert_eq!(null, ByocInfrastructureValidationCheck::default());
}

#[test]
fn byoc_availability_zone_suffix_preserves_unknown_values() {
    let suffix: ByocAvailabilityZoneSuffix = serde_json::from_str(r#""future""#).unwrap();
    assert_eq!(
        suffix,
        ByocAvailabilityZoneSuffix::Unknown("future".to_string())
    );
    assert_eq!(serde_json::to_string(&suffix).unwrap(), r#""future""#);
}

#[test]
fn serialize_byoc_infrastructure_patch_request() {
    let req = ByocInfrastructurePatchRequest {
        display_name: Some("Renamed".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json, serde_json::json!({"displayName": "Renamed"}));
}

#[test]
fn serialize_byoc_infrastructure_patch_request_full() {
    let req = ByocInfrastructurePatchRequest {
        display_name: Some("Renamed".to_string()),
        enable_private_link: Some(true),
        enable_private_load_balancer: Some(true),
        enable_public_load_balancer: Some(false),
        gcp_psc_subnet_id: Some("psc-subnet".to_string()),
        tags: Some(ByocInfrastructureTags::from([(
            "team".to_string(),
            "data".to_string(),
        )])),
    };
    assert_eq!(
        serde_json::to_value(&req).unwrap(),
        serde_json::json!({
            "displayName": "Renamed",
            "enablePrivateLink": true,
            "enablePrivateLoadBalancer": true,
            "enablePublicLoadBalancer": false,
            "gcpPscSubnetId": "psc-subnet",
            "tags": {"team": "data"}
        })
    );
    assert_eq!(
        serde_json::to_value(ByocInfrastructurePatchRequest::default()).unwrap(),
        serde_json::json!({})
    );
}

#[test]
fn byoc_infrastructure_private_endpoint_config_missing_and_null_fields_are_none() {
    let missing: ByocInfrastructurePrivateEndpointConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, ByocInfrastructurePrivateEndpointConfig::default());
    assert_eq!(
        serde_json::to_value(&missing).unwrap(),
        serde_json::json!({})
    );

    let null: ByocInfrastructurePrivateEndpointConfig = serde_json::from_value(serde_json::json!({
        "endpointName": null, "privateDnsHostname": null
    }))
    .unwrap();
    assert_eq!(null, ByocInfrastructurePrivateEndpointConfig::default());
}

#[test]
fn byoc_infrastructure_tags_response_missing_and_null_fields_are_none() {
    let missing: ByocInfrastructureTagsResponse = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, ByocInfrastructureTagsResponse::default());
    assert_eq!(
        serde_json::to_value(&missing).unwrap(),
        serde_json::json!({})
    );

    let null: ByocInfrastructureTagsResponse =
        serde_json::from_value(serde_json::json!({"tags": null})).unwrap();
    assert_eq!(null, ByocInfrastructureTagsResponse::default());

    let empty: ByocInfrastructureTagsResponse =
        serde_json::from_value(serde_json::json!({"tags": {}})).unwrap();
    assert_eq!(empty.tags, Some(ByocInfrastructureTags::new()));
}

#[test]
fn byoc_config_regionid_ca_central_1() {
    let parsed: ByocConfigRegionid = serde_json::from_str("\"ca-central-1\"").unwrap();
    assert_eq!(parsed, ByocConfigRegionid::Ca_central_1);
    assert_eq!(parsed.to_string(), "ca-central-1");
}

#[test]
fn byoc_config_state_new_values_round_trip() {
    let cases = [
        ("infra-degraded", ByocConfigState::Infra_degraded),
        ("infra-terminating", ByocConfigState::Infra_terminating),
        ("infra-upgrading", ByocConfigState::Infra_upgrading),
    ];
    for (wire, expected) in cases {
        let parsed: ByocConfigState = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
        assert_eq!(parsed, expected, "{wire} should not be Unknown");
        assert_eq!(parsed.to_string(), wire);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }
}

#[test]
fn byoc_config_account_id_deserializes_and_tolerates_absence() {
    let config: ByocConfig = serde_json::from_value(serde_json::json!({
        "accountId": "123456789012",
        "state": "infra-ready"
    }))
    .unwrap();
    assert_eq!(config.account_id.as_deref(), Some("123456789012"));
    assert_eq!(
        serde_json::to_value(&config).unwrap()["accountId"],
        "123456789012"
    );

    for wire in [
        serde_json::json!({}),
        serde_json::json!({ "accountId": null }),
    ] {
        let config: ByocConfig = serde_json::from_value(wire).unwrap();
        assert_eq!(config.account_id, None);
        assert!(
            serde_json::to_value(&config)
                .unwrap()
                .get("accountId")
                .is_none()
        );
    }
}

#[cfg(not(feature = "deprecated-fields"))]
#[test]
fn byoc_config_deprecated_account_name_is_hidden_by_default() {
    let config: ByocConfig = serde_json::from_value(serde_json::json!({
        "accountId": "123456789012",
        "accountName": "123456789012"
    }))
    .unwrap();
    assert!(
        serde_json::to_value(&config)
            .unwrap()
            .get("accountName")
            .is_none()
    );
}
