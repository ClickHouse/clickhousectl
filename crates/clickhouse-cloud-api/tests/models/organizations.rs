use clickhouse_cloud_api::models::*;

#[test]
fn deserialize_organization() {
    let json = r#"{
        "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "name": "My Organization",
        "createdAt": "2024-01-15T10:30:00Z",
        "privateEndpoints": [],
        "byocConfig": [],
        "enableCoreDumps": false
    }"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.name.as_deref(), Some("My Organization"));
    assert_eq!(
        org.id,
        Some(
            "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
                .parse::<uuid::Uuid>()
                .unwrap()
        )
    );
    assert_eq!(org.enable_core_dumps, Some(false));
}

#[test]
fn serialize_organization() {
    let org = Organization {
        id: Some("a1b2c3d4-e5f6-7890-abcd-ef1234567890".parse().unwrap()),
        name: Some("Test Org".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&org).unwrap();
    assert_eq!(json["name"], "Test Org");
    assert_eq!(json["id"], "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
    // Absent response fields are omitted, not emitted as `null`.
    assert!(json.get("createdAt").is_none());
    assert!(json.get("enableCoreDumps").is_none());
}

#[test]
fn deserialize_member() {
    let json = r#"{
        "userId": "user-123",
        "name": "John Doe",
        "email": "john@example.com",
        "role": "admin",
        "joinedAt": "2024-01-01T00:00:00Z"
    }"#;
    let member: Member = serde_json::from_str(json).unwrap();
    assert_eq!(member.name.as_deref(), Some("John Doe"));
    assert_eq!(member.email.as_deref(), Some("john@example.com"));
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(member.role, Some(MemberRole::Admin));
}

#[test]
fn deserialize_invitation() {
    let json = r#"{
        "id": "33333333-4444-5555-6666-777777777777",
        "email": "new@example.com",
        "role": "developer",
        "createdAt": "2024-06-01T00:00:00Z"
    }"#;
    let inv: Invitation = serde_json::from_str(json).unwrap();
    assert_eq!(inv.email.as_deref(), Some("new@example.com"));
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(inv.role, Some(InvitationRole::Developer));
}

#[test]
fn absent_response_fields_are_omitted_when_serialized() {
    let org = Organization {
        name: Some("Test".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_value(&org).unwrap();
    // Absent means absent: a field the API did not return is omitted from
    // `--json` output rather than emitted as `null`.
    assert_eq!(json, serde_json::json!({ "name": "Test" }));
}

#[test]
fn serialize_invitation_post_request() {
    let req = InvitationPostRequest {
        email: "alice@example.com".to_string(),
        #[cfg(feature = "deprecated-fields")]
        role: Some(InvitationPostRequestRole::Developer),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["email"], "alice@example.com");
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(json["role"], "developer");
    // By default the deprecated `role` field is gated out and never serialized.
    #[cfg(not(feature = "deprecated-fields"))]
    assert!(json.get("role").is_none());
}

#[cfg(feature = "deprecated-fields")]
#[test]
fn serialize_member_patch_request() {
    let req = MemberPatchRequest {
        role: Some(MemberPatchRequestRole::Admin),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["role"], "admin");
}

#[test]
fn organization_ignores_extra_fields() {
    let json = r#"{"name":"Test","brandNewField":"surprise","anotherNew":42}"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.name.as_deref(), Some("Test"));
}

#[test]
fn member_ignores_extra_fields() {
    let json = r#"{"name":"Alice","role":"admin","department":"eng","mfa":true}"#;
    let m: Member = serde_json::from_str(json).unwrap();
    assert_eq!(m.name.as_deref(), Some("Alice"));
    #[cfg(feature = "deprecated-fields")]
    assert_eq!(m.role, Some(MemberRole::Admin));
}

#[test]
fn invitation_ignores_extra_fields() {
    let json = r#"{"email":"a@b.com","role":"developer","expiresIn":"7d"}"#;
    let inv: Invitation = serde_json::from_str(json).unwrap();
    assert_eq!(inv.email.as_deref(), Some("a@b.com"));
}

#[test]
fn organization_minimal_response() {
    let org: Organization = serde_json::from_str(r#"{"name":"X"}"#).unwrap();
    assert_eq!(org.name.as_deref(), Some("X"));
    assert_eq!(org.id, None);
    assert_eq!(org.created_at, None);
}

#[test]
fn organization_quota_typed_enums_round_trip() {
    let json = r#"{
        "quotaCode": "replicas-per-warehouse",
        "name": "Replicas per warehouse",
        "description": "Limits each warehouse individually.",
        "scope": "warehouse",
        "value": 20,
        "usage": 3,
        "adjustable": true
    }"#;
    let quota: OrganizationQuota = serde_json::from_str(json).unwrap();
    assert_eq!(
        quota.quota_code,
        Some(OrganizationQuotaQuotacode::Replicas_per_warehouse)
    );
    assert_eq!(quota.scope, Some(OrganizationQuotaScope::Warehouse));
    assert_eq!(quota.value, Some(20));
    assert_eq!(quota.usage, Some(3));
    assert_eq!(quota.adjustable, Some(true));

    let v = serde_json::to_value(&quota).unwrap();
    assert_eq!(v["quotaCode"], "replicas-per-warehouse");
    assert_eq!(v["scope"], "warehouse");
    assert_eq!(v["value"], 20);
    assert_eq!(v["usage"], 3);

    let back: OrganizationQuota = serde_json::from_value(v).unwrap();
    assert_eq!(back, quota);
}

#[test]
fn organization_api_key_quota_code_round_trips() {
    let parsed: OrganizationQuotaQuotacode =
        serde_json::from_str("\"api-keys-per-organization\"").unwrap();
    assert_eq!(
        parsed,
        OrganizationQuotaQuotacode::Api_keys_per_organization
    );
    assert_eq!(parsed.to_string(), "api-keys-per-organization");
    assert_eq!(
        serde_json::to_value(parsed).unwrap(),
        "api-keys-per-organization"
    );
}

#[test]
fn organization_quota_usage_optional_omitted() {
    let json = r#"{
        "quotaCode": "services-per-organization",
        "name": "Services per organization",
        "description": "Limits services.",
        "scope": "organization",
        "value": 20,
        "adjustable": false
    }"#;
    let quota: OrganizationQuota = serde_json::from_str(json).unwrap();
    assert_eq!(
        quota.quota_code,
        Some(OrganizationQuotaQuotacode::Services_per_organization)
    );
    assert_eq!(quota.scope, Some(OrganizationQuotaScope::Organization));
    assert_eq!(quota.usage, None);

    let v = serde_json::to_value(&quota).unwrap();
    assert!(v.get("usage").is_none(), "usage must be omitted when None");
}

#[test]
fn organization_quota_tolerates_explicit_null_fields() {
    // `OrganizationQuota` is response-only, so every field is `Option<T>` and an
    // explicit `null` lands as `None` exactly like a dropped key — the case the
    // superseded `#[serde(default)]` policy never covered.
    let quota: OrganizationQuota = serde_json::from_str(
        r#"{"quotaCode":null,"name":null,"description":null,"scope":null,
            "value":null,"usage":null,"adjustable":null}"#,
    )
    .unwrap();
    assert_eq!(quota, OrganizationQuota::default());

    // Absence is omitted on the way out, never re-emitted as `null`.
    assert_eq!(
        serde_json::to_value(&quota).unwrap(),
        serde_json::json!({}),
        "absent response fields must be omitted from --json output"
    );
}

#[test]
fn organization_quota_quota_code_unknown_catch_all() {
    let parsed: OrganizationQuotaQuotacode =
        serde_json::from_str("\"queries-per-second\"").unwrap();
    assert_eq!(
        parsed,
        OrganizationQuotaQuotacode::Unknown("queries-per-second".to_string())
    );
    assert_eq!(parsed.to_string(), "queries-per-second");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), "queries-per-second");
}

#[test]
fn role_update_request_omits_fields_that_are_not_being_changed() {
    let request = RoleUpdateRequest {
        name: Some("auditor".to_string()),
        actors: None,
        policies: None,
    };

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({"name": "auditor"})
    );
}

#[test]
fn organization_capabilities_tolerate_absence_null_and_false() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({"capabilities": null}),
    ] {
        let org: Organization = serde_json::from_value(wire).unwrap();
        assert_eq!(org.capabilities, None);
        assert_eq!(serde_json::to_value(org).unwrap(), serde_json::json!({}));
    }
    for wire in [
        serde_json::json!({}),
        serde_json::json!({"snapshots": null}),
    ] {
        let capabilities: OrganizationCapabilities = serde_json::from_value(wire).unwrap();
        assert_eq!(capabilities.snapshots, None);
        assert_eq!(
            serde_json::to_value(capabilities).unwrap(),
            serde_json::json!({})
        );
    }
    let org: Organization = serde_json::from_value(serde_json::json!({
        "capabilities": {"snapshots": false}
    }))
    .unwrap();
    assert_eq!(org.capabilities.unwrap().snapshots, Some(false));
}

#[test]
fn notification_type_missing_and_null_fields_are_none() {
    let missing: NotificationType = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, NotificationType::default());
    assert_eq!(
        serde_json::to_value(&missing).unwrap(),
        serde_json::json!({})
    );

    let null: NotificationType = serde_json::from_value(serde_json::json!({
        "name": null, "title": null, "description": null, "category": null,
        "severity": null, "resourceType": null, "isConfigurable": null,
        "isAvailable": null, "futureField": 1
    }))
    .unwrap();
    assert_eq!(null, NotificationType::default());
}

#[test]
fn notification_type_severity_round_trips_and_preserves_unknown() {
    for (wire, value) in [
        ("info", NotificationTypeSeverity::Info),
        ("warning", NotificationTypeSeverity::Warning),
        ("critical", NotificationTypeSeverity::Critical),
    ] {
        let parsed: NotificationTypeSeverity =
            serde_json::from_value(serde_json::json!(wire)).unwrap();
        assert_eq!(parsed, value);
        assert_eq!(value.to_string(), wire);
        assert_eq!(
            serde_json::to_value(&value).unwrap(),
            serde_json::json!(wire)
        );
    }
    let unknown: NotificationTypeSeverity =
        serde_json::from_value(serde_json::json!("emergency")).unwrap();
    assert_eq!(
        unknown,
        NotificationTypeSeverity::Unknown("emergency".to_string())
    );
    assert_eq!(unknown.to_string(), "emergency");
}

#[test]
fn prometheus_discovery_labels_keep_named_meta_and_additional_labels() {
    let json = serde_json::json!({
        "__scheme__": "https",
        "__meta_clickhouse_provider": "aws",
        "__meta_clickhouse_region": "us-east-1",
        "clickhouse_discovery_service_name": "analytics",
        "clickhouse_custom_label": "blue"
    });
    // An unknown non-string key is ignored rather than failing the response.
    let mut with_unknown = json.clone();
    with_unknown["future_flag"] = serde_json::json!(true);
    let tolerant: PrometheusDiscoveryLabels = serde_json::from_value(with_unknown).unwrap();
    assert_eq!(serde_json::to_value(&tolerant).unwrap(), json);

    let labels: PrometheusDiscoveryLabels = serde_json::from_value(json.clone()).unwrap();

    assert_eq!(labels.meta_clickhouse_provider.as_deref(), Some("aws"));
    assert_eq!(labels.meta_clickhouse_region.as_deref(), Some("us-east-1"));
    assert_eq!(
        labels.additional_labels,
        Some(std::collections::BTreeMap::from([(
            "clickhouse_custom_label".to_string(),
            "blue".to_string()
        )]))
    );
    // Named labels are not duplicated into the additional map, and the
    // round trip is lossless.
    assert_eq!(serde_json::to_value(&labels).unwrap(), json);
}

#[test]
fn prometheus_discovery_labels_without_additional_labels_serialize_compactly() {
    let labels: PrometheusDiscoveryLabels =
        serde_json::from_str(r#"{"__meta_clickhouse_region":null}"#).unwrap();
    assert_eq!(labels.meta_clickhouse_region, None);
    assert_eq!(
        serde_json::to_value(&labels).unwrap(),
        serde_json::json!({})
    );
}

#[test]
fn notification_tolerates_missing_null_and_future_values() {
    let missing: Notification = serde_json::from_str("{}").unwrap();
    assert_eq!(missing, Notification::default());
    let null: Notification = serde_json::from_value(serde_json::json!({
        "id": null, "type": null, "severity": null, "resourceType": null,
        "resourceId": null, "resourceName": null, "createdAt": null,
        "title": null, "description": null, "future": true
    }))
    .unwrap();
    assert_eq!(null, missing);
    assert_eq!(serde_json::to_value(null).unwrap(), serde_json::json!({}));
    let future: Notification = serde_json::from_value(serde_json::json!({
        "type": "future.type", "severity": "future.severity", "resourceType": "future.resource"
    }))
    .unwrap();
    assert_eq!(future.r#type.as_deref(), Some("future.type"));
    assert_eq!(future.severity.as_deref(), Some("future.severity"));
    assert_eq!(future.resource_type.as_deref(), Some("future.resource"));
}
