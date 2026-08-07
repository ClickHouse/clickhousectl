use clickhouse_cloud_api as api;

fn assert_same_type<T>(_: T, _: T) {}

#[test]
fn extracted_models_keep_root_and_models_paths() {
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
        api::ClickStackChartColor::default(),
        api::models::ClickStackChartColor::default(),
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
    assert_same_type(api::RBACRole::default(), api::models::RBACRole::default());
    assert_same_type(api::ScimUser::default(), api::models::ScimUser::default());
    assert_same_type(
        api::ApiResponse::<()>::default(),
        api::models::ApiResponse::<()>::default(),
    );
    assert_same_type(api::Service::default(), api::models::Service::default());
    assert_same_type(api::Udf::default(), api::models::Udf::default());
}
