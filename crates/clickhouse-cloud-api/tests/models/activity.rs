use clickhouse_cloud_api::models::*;

#[test]
fn deserialize_activity() {
    let json = r#"{
        "actorType": "api",
        "actorId": "actor-123",
        "createdAt": "2024-06-01T00:00:00Z"
    }"#;
    let activity: Activity = serde_json::from_str(json).unwrap();
    assert_eq!(activity.actor_type, Some(ActivityActortype::Api));
}

#[test]
fn activity_ignores_extra_fields() {
    let json = r#"{"actorType":"user","sourceIp":"1.2.3.4"}"#;
    let a: Activity = serde_json::from_str(json).unwrap();
    assert_eq!(a.actor_type, Some(ActivityActortype::User));
}

#[test]
fn activity_type_new_wire_values_deserialize_to_typed_variants() {
    let cases = [
        (
            "organization_member_update_roles",
            ActivityType::Organization_member_update_roles,
        ),
        (
            "organization_saml_connection_create",
            ActivityType::Organization_saml_connection_create,
        ),
        (
            "organization_saml_connection_update",
            ActivityType::Organization_saml_connection_update,
        ),
        (
            "service_update_snapshot_configuration",
            ActivityType::Service_update_snapshot_configuration,
        ),
        ("backup_bucket_create", ActivityType::Backup_bucket_create),
        ("role_update", ActivityType::Role_update),
        ("service_mcp_enabled", ActivityType::Service_mcp_enabled),
        ("udf_create", ActivityType::Udf_create),
        (
            "service_update_deletion_protection",
            ActivityType::Service_update_deletion_protection,
        ),
    ];
    for (wire, expected) in cases {
        let parsed: ActivityType = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
        assert_eq!(parsed, expected, "{wire} should not be Unknown");
        assert_eq!(parsed.to_string(), wire);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }
}

#[test]
fn activity_type_approved_domain_and_org_update_values_round_trip() {
    let cases = [
        (
            "organization_approved_domain_auto_invite",
            ActivityType::Organization_approved_domain_auto_invite,
        ),
        (
            "organization_approved_domain_auto_join",
            ActivityType::Organization_approved_domain_auto_join,
        ),
        (
            "organization_approved_domain_delete",
            ActivityType::Organization_approved_domain_delete,
        ),
        (
            "organization_approved_domain_update",
            ActivityType::Organization_approved_domain_update,
        ),
        (
            "organization_approved_domain_verify",
            ActivityType::Organization_approved_domain_verify,
        ),
        (
            "organization_update_public_preview",
            ActivityType::Organization_update_public_preview,
        ),
        (
            "byoc_infrastructure_update_customer_authorized_access",
            ActivityType::Byoc_infrastructure_update_customer_authorized_access,
        ),
        (
            "organization_update_saml_query_ownership_migration",
            ActivityType::Organization_update_saml_query_ownership_migration,
        ),
    ];
    for (wire, expected) in cases {
        let parsed: ActivityType = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
        assert_eq!(parsed, expected, "{wire} should not be Unknown");
        assert_eq!(parsed.to_string(), wire);
        assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
    }
}

#[test]
fn activity_type_postgres_action_round_trips() {
    let parsed: ActivityType = serde_json::from_str("\"postgres_action\"").unwrap();
    assert_eq!(parsed, ActivityType::Postgres_action);
    assert_eq!(parsed.to_string(), "postgres_action");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), "postgres_action");
}

/// The spec now types `Activity.type` as an open string ("treat unknown values
/// as valid"), so any value outside the known list must still deserialize.
#[test]
fn activity_type_accepts_unlisted_values() {
    let activity: Activity =
        serde_json::from_value(serde_json::json!({"type": "some_future_activity"})).unwrap();
    assert_eq!(
        activity.r#type,
        Some(ActivityType::Unknown("some_future_activity".to_string()))
    );
    assert_eq!(
        serde_json::to_value(&activity).unwrap(),
        serde_json::json!({"type": "some_future_activity"})
    );
}

#[test]
fn activity_deletion_protection_bypassed_round_trips_and_tolerates_absence() {
    let activity: Activity = serde_json::from_value(serde_json::json!({
        "type": "service_delete",
        "deletionProtectionBypassed": true
    }))
    .unwrap();
    assert_eq!(activity.deletion_protection_bypassed, Some(true));
    assert_eq!(
        serde_json::to_value(&activity).unwrap()["deletionProtectionBypassed"],
        serde_json::json!(true)
    );

    let absent: Activity =
        serde_json::from_str(r#"{"type":"service_delete","deletionProtectionBypassed":null}"#)
            .unwrap();
    assert_eq!(absent.deletion_protection_bypassed, None);
    assert!(
        serde_json::to_value(&absent)
            .unwrap()
            .get("deletionProtectionBypassed")
            .is_none()
    );
}
