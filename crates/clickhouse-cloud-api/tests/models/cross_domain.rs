use clickhouse_cloud_api::models::*;

use crate::assert_unknown_variant_round_trips;

/// Every `discriminated_union!` enum with a `Default` is reachable from a
/// response that drops a field carrying that union, so its default must be a
/// fixed point of its own `Deserialize`: it has to come back as the same
/// variant. It is not automatic — variants of one union can share the same
/// inline `enum` values for the discriminating field (both alert channels
/// declare `["webhook", "email"]`), so a variant defaulting its discriminator to
/// another variant's value would silently retype the value on the next
/// deserialize. The covered list is enforced structurally, not by convention:
/// it must equal the set of hand-written `impl Default for` blocks in the model
/// module tree (via the analyzer's `model_types_with_manual_default_impl`), so a
/// new union gaining a `Default` without a list entry fails this test.
#[test]
fn discriminated_union_defaults_round_trip_to_the_same_variant() {
    let mut covered: Vec<&str> = Vec::new();

    macro_rules! assert_default_round_trips {
        ($($union:ty),+ $(,)?) => {
            $({
                covered.push(stringify!($union));
                let default = <$union>::default();
                let json = serde_json::to_string(&default).unwrap();
                let parsed: $union = serde_json::from_str(&json).unwrap();
                assert_eq!(
                    parsed,
                    default,
                    "{} default deserialized as another variant from {json}",
                    stringify!($union),
                );
            })+
        };
    }

    assert_default_round_trips!(
        BackupBucket,
        BackupBucketPatchRequest,
        BackupBucketPostRequest,
        BackupBucketProperties,
        ClickPipeBigQuerySource,
        ClickPipeKafkaSchemaRegistry,
        ClickPipeMutateBigQuerySource,
        ClickPipeMutateKafkaSchemaRegistry,
        ClickPipePostPubSubSource,
        ClickStackSavedFilterValue,
        ClickStackSavedFilterValueResponse,
        ClickStackAlertBarChartConfig,
        ClickStackAlertChannel,
        ClickStackAlertChartConfig,
        ClickStackAlertLineChartConfig,
        ClickStackAlertNumberChartConfig,
        ClickStackAlertStackedLineChartConfig,
        ClickStackBarChartConfig,
        ClickStackCategoricalBarChartConfig,
        ClickStackDashboardChartSeries,
        ClickStackFilter,
        ClickStackFilterInput,
        ClickStackHeatmapChartConfig,
        ClickStackLineChartConfig,
        ClickStackNumberChartConfig,
        ClickStackOnClick,
        ClickStackOnClickTarget,
        ClickStackPieChartConfig,
        ClickStackSource,
        ClickStackStackedLineChartConfig,
        ClickStackTableChartConfig,
        ClickStackTileConfig,
        ClickStackWebhook,
    );

    covered.sort_unstable();
    let manual_default_impls = clickhouse_openapi_analyzer::model_types_with_manual_default_impl(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")),
    )
    .unwrap();
    assert_eq!(
        covered,
        manual_default_impls
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        "the covered list must equal the manual `impl Default for` blocks in the model tree"
    );
}

/// In the default build the deprecated request fields don't exist on the
/// struct, so callers can't set them and they never reach the wire.
#[cfg(not(feature = "deprecated-fields"))]
#[test]
fn deprecated_request_fields_absent_by_default() {
    let member = MemberPatchRequest {
        assigned_role_ids: Some(vec!["admin".to_string()]),
    };
    assert!(serde_json::to_value(&member).unwrap().get("role").is_none());

    let invitation = InvitationPostRequest {
        email: "alice@example.com".to_string(),
        assigned_role_ids: Some(vec!["admin".to_string()]),
    };
    assert!(
        serde_json::to_value(&invitation)
            .unwrap()
            .get("role")
            .is_none()
    );

    let scaling = ServiceScalingPatchRequest {
        num_replicas: Some(3),
        ..Default::default()
    };
    let scaling = serde_json::to_value(&scaling).unwrap();
    assert!(scaling.get("minTotalMemoryGb").is_none());
    assert!(scaling.get("maxTotalMemoryGb").is_none());
}

/// In the default build (no `deprecated-fields` feature) deprecated response
/// fields don't exist on the struct, so they can't be read and never appear in
/// serialized output. Deserializing a payload that contains them simply ignores
/// the extra keys.
#[cfg(not(feature = "deprecated-fields"))]
#[test]
fn deprecated_fields_absent_by_default() {
    let svc: Service = serde_json::from_str(
        r#"{"name":"svc","tier":"production","minTotalMemoryGb":24,"maxTotalMemoryGb":48}"#,
    )
    .unwrap();
    let v = serde_json::to_value(&svc).unwrap();
    assert!(v.get("tier").is_none());
    assert!(v.get("minTotalMemoryGb").is_none());
    assert!(v.get("maxTotalMemoryGb").is_none());

    let m: Member = serde_json::from_str(r#"{"name":"Alice","role":"admin"}"#).unwrap();
    assert!(serde_json::to_value(&m).unwrap().get("role").is_none());
}

#[test]
fn infrastructure_responses_tolerate_dropped_and_null_fields() {
    // Reverse private endpoints and quotas are response-only, so every field is
    // `Option<T>`: a dropped key and an explicit `null` both land as `None`.
    // `null` is the case `#[serde(default)]` never covered.
    let dropped: ReversePrivateEndpoint = serde_json::from_str("{}").unwrap();
    let nulled: ReversePrivateEndpoint = serde_json::from_str(
        r#"{"id":null,"description":null,"status":null,"type":null,"dnsNames":null,
            "privateDnsNames":null,"endpointId":null,"serviceId":null,
            "customPrivateDnsMappings":null,"dnsTargets":null,
            "privateDnsMappings":null}"#,
    )
    .unwrap();
    assert_eq!(dropped, ReversePrivateEndpoint::default());
    assert_eq!(nulled, ReversePrivateEndpoint::default());
    // A `null` on a list field is the residual case the previous
    // `#[serde(default)]` policy still failed on.
    assert_eq!(nulled.dns_names, None);
    assert_eq!(nulled.custom_private_dns_mappings, None);
    assert_eq!(nulled.dns_targets, None);
    assert_eq!(nulled.private_dns_mappings, None);

    let quota: OrganizationQuota = serde_json::from_str(r#"{"name":"Services"}"#).unwrap();
    assert_eq!(quota.name.as_deref(), Some("Services"));
    assert_eq!(quota.quota_code, None);
    assert_eq!(quota.value, None);
    assert_eq!(quota.adjustable, None);
}

#[test]
fn infrastructure_shared_leaves_stay_strict_on_the_request_side() {
    // `customPrivateDnsMapping` and `RBACPolicyTags` are sent as well as
    // returned, so each splits: only the `{Name}Response` variant is used by a
    // response type, and neither variant fabricates a value for a missing key.
    let mapping = CustomPrivateDnsMapping {
        private_dns_name: Some("db.internal".to_string()),
        target_id: None,
    };
    assert_eq!(
        serde_json::to_value(&mapping).unwrap(),
        serde_json::json!({ "privateDnsName": "db.internal" })
    );
    let targeted = CustomPrivateDnsMapping {
        private_dns_name: Some("db.internal".to_string()),
        target_id: Some("rcfg-097648d8068504966".to_string()),
    };
    assert_eq!(
        serde_json::to_value(&targeted).unwrap(),
        serde_json::json!({
            "privateDnsName": "db.internal",
            "targetId": "rcfg-097648d8068504966",
        })
    );
    // Both schemas are all-optional upstream, so the request variants accept an
    // empty object too — what the split guarantees is that the response variant
    // can never regain a required field.
    assert_eq!(
        serde_json::from_str::<CustomPrivateDnsMappingResponse>("{}").unwrap(),
        CustomPrivateDnsMappingResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<RBACPolicyTagsResponse>(r#"{"grants":null,"roleV2":null}"#).unwrap(),
        RBACPolicyTagsResponse::default()
    );
    let policy: RBACPolicy = serde_json::from_str(r#"{"tags":{"grants":["select"]}}"#).unwrap();
    assert_eq!(
        policy.tags,
        Some(RBACPolicyTagsResponse {
            grants: Some(vec!["select".to_string()]),
            role_v2: None,
        })
    );
}

#[test]
fn new_response_models_tolerate_absent_and_null_fields() {
    let balances: ActiveBalances =
        serde_json::from_str(r#"{"totalRemainingPrepaidCredits":null,"prepaidBalances":null}"#)
            .unwrap();
    let balance: ActiveBalance = serde_json::from_str(
        r#"{"id":null,"remainingPrepaidCredits":null,"totalAmount":null,"amountSpent":null,"startDate":null,"expirationDate":null}"#,
    )
    .unwrap();
    let labels: PrometheusDiscoveryLabels = serde_json::from_str(
        r#"{"__scheme__":null,"__metrics_path__":null,"__param_filtered_metrics":null,"clickhouse_org_id":null,"clickhouse_service_id":null,"clickhouse_discovery_service_name":null}"#,
    )
    .unwrap();
    let group: PrometheusDiscoveryTargetGroup =
        serde_json::from_str(r#"{"targets":null,"labels":null}"#).unwrap();
    let log: PostgresLogEntry =
        serde_json::from_str(r#"{"timestamp":null,"severity":null,"body":null}"#).unwrap();

    assert_eq!(balances, ActiveBalances::default());
    assert_eq!(balance, ActiveBalance::default());
    assert_eq!(labels, PrometheusDiscoveryLabels::default());
    assert_eq!(group, PrometheusDiscoveryTargetGroup::default());
    assert_eq!(log, PostgresLogEntry::default());
    assert_eq!(
        serde_json::to_value(balances).unwrap(),
        serde_json::json!({})
    );
    assert_eq!(serde_json::to_value(log).unwrap(), serde_json::json!({}));
}

#[test]
fn new_response_models_preserve_missing_and_null_fields() {
    macro_rules! tolerant {
        ($model:ty, [$($field:literal),+]) => {{
            let empty: $model = serde_json::from_value(serde_json::json!({})).unwrap();
            let nulls: $model = serde_json::from_value(serde_json::json!({$($field: null),+})).unwrap();
            assert_eq!(empty, nulls);
            assert_eq!(serde_json::to_value(empty).unwrap(), serde_json::json!({}));
        }};
    }
    tolerant!(
        CreditBalance,
        [
            "id",
            "type",
            "remainingCredits",
            "totalAmount",
            "amountSpent",
            "startDate",
            "expirationDate"
        ]
    );
    tolerant!(CreditBalances, ["totalRemainingCredits", "balances"]);
    tolerant!(ServiceProfile, ["profile", "cpuCores", "memoryGi"]);
    tolerant!(
        ClickPipeBigQueryServiceAccountSource,
        [
            "snapshotStagingPath",
            "settings",
            "tableMappings",
            "authentication",
            "projectId"
        ]
    );
    tolerant!(
        ClickPipeBigQueryWorkloadIdentitySource,
        [
            "snapshotStagingPath",
            "settings",
            "tableMappings",
            "authentication",
            "projectId"
        ]
    );
    tolerant!(
        ClickPipesGcpWorkloadIdentityContext,
        ["supported", "ready", "principal"]
    );
    tolerant!(ClickPipesServiceContext, ["gcpWorkloadIdentity"]);
    tolerant!(
        ClickStackFormulaResponse,
        ["expression", "alias", "numberFormat"]
    );
    tolerant!(ClickStackSqlSavedFilterValueResponse, ["type", "condition"]);
    tolerant!(
        ClickStackVariableSavedFilterValueResponse,
        ["type", "name", "values"]
    );
}

#[test]
fn new_source_and_saved_filter_unions_preserve_unknown_payloads() {
    assert_unknown_variant_round_trips::<ClickPipeMutateBigQuerySource>(
        r#"{"authentication":"FUTURE","newField":1}"#,
        |v| matches!(v, ClickPipeMutateBigQuerySource::Unknown(_)),
    );
    assert_unknown_variant_round_trips::<ClickPipeBigQuerySource>(
        r#"{"authentication":"FUTURE","newField":1}"#,
        |v| matches!(v, ClickPipeBigQuerySource::Unknown(_)),
    );
    assert_unknown_variant_round_trips::<ClickPipePostPubSubSource>(
        r#"{"authentication":"FUTURE","newField":1}"#,
        |v| matches!(v, ClickPipePostPubSubSource::Unknown(_)),
    );
    assert_unknown_variant_round_trips::<ClickStackSavedFilterValueResponse>(
        r#"{"type":"future","values":["keep"]}"#,
        |v| matches!(v, ClickStackSavedFilterValueResponse::Unknown(_)),
    );
    assert_unknown_variant_round_trips::<ClickStackSavedFilterValueResponse>(
        r#"{"name":"service","values":["keep"]}"#,
        |v| matches!(v, ClickStackSavedFilterValueResponse::Unknown(_)),
    );
    assert_unknown_variant_round_trips::<ClickPipeBigQuerySource>(
        r#"{"authentication":"SERVICE_ACCOUNT","settings":"new-shape"}"#,
        |v| matches!(v, ClickPipeBigQuerySource::Unknown(_)),
    );
}
