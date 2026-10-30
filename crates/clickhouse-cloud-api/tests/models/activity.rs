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
