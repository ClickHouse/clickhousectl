use clickhouse_cloud_api as api;

#[test]
fn snapshot_and_udf_output_types_are_available_through_both_public_paths() {
    let snapshot: api::Snapshot = api::models::Snapshot::default();
    assert_eq!(snapshot, api::Snapshot::default());
    let argument: api::UdfArgumentOutput = api::models::UdfArgumentOutput::default();
    let compatible: api::UdfArgumentResponse = argument;
    assert_eq!(compatible, api::models::UdfArgumentResponse::default());
}

fn assert_same_type<T>(_: T, _: T) {}

#[cfg(feature = "deprecated-fields")]
#[test]
fn deprecated_api_key_roles_preserve_string_source_and_wire_compatibility() {
    let roles: Vec<String> = vec!["admin".to_string(), "future_role".to_string()];
    let response = api::ApiKey {
        roles: Some(roles.clone()),
        ..Default::default()
    };
    let patch = api::ApiKeyPatchRequest {
        roles: Some(roles.clone()),
        ..Default::default()
    };
    let post = api::ApiKeyPostRequest {
        roles: Some(roles),
        ..Default::default()
    };
    let response_wire = serde_json::to_value(&response).unwrap();
    let patch_wire = serde_json::to_value(&patch).unwrap();
    let post_wire = serde_json::to_value(&post).unwrap();
    for wire in [&response_wire, &patch_wire, &post_wire] {
        assert_eq!(wire["roles"], serde_json::json!(["admin", "future_role"]));
    }
    assert_eq!(
        serde_json::from_value::<api::ApiKey>(response_wire).unwrap(),
        response
    );
    assert_eq!(
        serde_json::from_value::<api::ApiKeyPatchRequest>(patch_wire).unwrap(),
        patch
    );
    assert_eq!(
        serde_json::from_value::<api::ApiKeyPostRequest>(post_wire).unwrap(),
        post
    );
}

#[test]
fn extracted_models_keep_root_and_models_paths() {
    assert_same_type(
        api::SnapshotConfiguration::default(),
        api::models::SnapshotConfiguration::default(),
    );
    assert_same_type(
        api::SnapshotConfigurationPatchRequest::default(),
        api::models::SnapshotConfigurationPatchRequest::default(),
    );
    assert_same_type(
        api::UdfAttachResponse424::default(),
        api::models::UdfAttachResponse424::default(),
    );
    assert_same_type(
        api::UdfAttachErrorCode::ServiceIdle,
        api::models::UdfAttachErrorCode::ServiceIdle,
    );
    assert_same_type(
        api::ActiveBalances::default(),
        api::models::ActiveBalances::default(),
    );
    assert_same_type(api::Activity::default(), api::models::Activity::default());
    assert_same_type(api::ApiKey::default(), api::models::ApiKey::default());
    assert_same_type(
        api::BackupBucket::default(),
        api::models::BackupBucket::default(),
    );
    assert_same_type(
        api::ByocConfig::default(),
        api::models::ByocConfig::default(),
    );
    assert_same_type(
        api::ByocAvailabilityZoneSuffix::default(),
        api::models::ByocAvailabilityZoneSuffix::default(),
    );
    assert_same_type(
        api::ClickStackChartColor::default(),
        api::models::ClickStackChartColor::default(),
    );
    assert_same_type(
        api::ClickStackDashboardResponse::default(),
        api::models::ClickStackDashboardResponse::default(),
    );
    assert_same_type(api::ClickPipe::default(), api::models::ClickPipe::default());
    assert_same_type(
        api::ReversePrivateEndpoint::default(),
        api::models::ReversePrivateEndpoint::default(),
    );
    assert_same_type(api::PLAIN::default(), api::models::PLAIN::default());
    assert_same_type(
        api::Invitation::default(),
        api::models::Invitation::default(),
    );
    assert_same_type(api::Member::default(), api::models::Member::default());
    assert_same_type(
        api::OrganizationPrivateEndpoint::default(),
        api::models::OrganizationPrivateEndpoint::default(),
    );
    assert_same_type(
        api::Organization::default(),
        api::models::Organization::default(),
    );
    assert_same_type(
        api::OrganizationQuota::default(),
        api::models::OrganizationQuota::default(),
    );
    assert_same_type(
        api::PostgresInstanceConfig::default(),
        api::models::PostgresInstanceConfig::default(),
    );
    assert_same_type(
        api::PostgresLogEntry::default(),
        api::models::PostgresLogEntry::default(),
    );
    assert_same_type(
        api::PostgresLogsGetListSortorder::default(),
        api::models::PostgresLogsGetListSortorder::default(),
    );
    assert_same_type(
        api::SlowQueryPatternsGetListSortby::default(),
        api::models::SlowQueryPatternsGetListSortby::default(),
    );
    assert_same_type(
        api::SlowQueryPatternsGetListSortorder::default(),
        api::models::SlowQueryPatternsGetListSortorder::default(),
    );
    assert_same_type(
        api::PrometheusDiscoveryTargetGroup::default(),
        api::models::PrometheusDiscoveryTargetGroup::default(),
    );
    assert_same_type(api::RBACRole::default(), api::models::RBACRole::default());
    assert_same_type(api::ScimUser::default(), api::models::ScimUser::default());
    assert_same_type(
        api::ApiResponse::<()>::default(),
        api::models::ApiResponse::<()>::default(),
    );
    assert_same_type(api::Service::default(), api::models::Service::default());
    assert_same_type(
        api::QueryEndpointRole::default(),
        api::models::QueryEndpointRole::default(),
    );
    assert_same_type(
        api::UpgradeWindowDuration::default(),
        api::models::UpgradeWindowDuration::default(),
    );
    assert_same_type(
        api::UpgradeWindowStartHourUtc::default(),
        api::models::UpgradeWindowStartHourUtc::default(),
    );
    assert_same_type(api::Udf::default(), api::models::Udf::default());
}
