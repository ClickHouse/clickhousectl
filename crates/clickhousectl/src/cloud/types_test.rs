use super::types::DeleteResponse;

// Contract policy for this suite:
// - mirror GA request/response schemas and query surfaces only
// - exclude beta endpoints, deprecated endpoints, and deprecated request fields
// - allow deprecated or BYOC fields in responses when they appear in GA schemas
// - keep BYOC request/command surface out of scope until explicitly added

// ── Response deserialization tests ──────────────────────────────────

#[test]
fn test_delete_response_deserialize() {
    let json = serde_json::json!({
        "status": 200,
        "requestId": "0182edf5-8c5b-4586-a6f8-78452320e4b1"
    });
    let response: DeleteResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.status, 200.0);
    assert_eq!(response.request_id, "0182edf5-8c5b-4586-a6f8-78452320e4b1");
}

#[test]
fn test_delete_response_serialize() {
    let response = DeleteResponse {
        status: 200.0,
        request_id: "0182edf5-8c5b-4586-a6f8-78452320e4b1".to_string(),
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["status"], 200.0);
    assert_eq!(json["requestId"], "0182edf5-8c5b-4586-a6f8-78452320e4b1");
    assert!(json.get("request_id").is_none());
}

// ── Activity tests (library types) ─────────────────────────────────

#[test]
fn test_activity_deserialize() {
    use clickhouse_cloud_api::models::Activity;
    let json = serde_json::json!({
        "id": "act-1",
        "type": "service_create",
        "actorType": "user",
        "actorId": "user-5",
        "createdAt": "2024-07-01T08:00:00Z",
        "actorDetails": "Alice Smith",
        "actorIpAddress": "1.2.3.4",
        "organizationId": "org-1",
        "serviceId": "svc-1",
        "userAgent": "clickhousectl/0.1.0"
    });
    let act: Activity = serde_json::from_value(json).unwrap();
    assert_eq!(act.id.as_deref(), Some("act-1"));
    assert_eq!(act.r#type.as_ref().unwrap().to_string(), "service_create");
    assert_eq!(act.actor_type.as_ref().unwrap().to_string(), "user");
    assert_eq!(act.actor_id.as_deref(), Some("user-5"));
    assert!(act.created_at.is_some());
    assert_eq!(act.actor_details.as_deref(), Some("Alice Smith"));
    assert_eq!(act.actor_ip_address.as_deref(), Some("1.2.3.4"));
    assert_eq!(act.organization_id.as_deref(), Some("org-1"));
    assert_eq!(act.service_id.as_deref(), Some("svc-1"));
    assert_eq!(act.user_agent.as_deref(), Some("clickhousectl/0.1.0"));
}

#[test]
fn test_activity_key_update() {
    use clickhouse_cloud_api::models::Activity;
    let json = serde_json::json!({
        "id": "act-2",
        "type": "openapi_key_update",
        "actorType": "api",
        "targetKeyId": "key-1",
        "keyUpdateType": "state-changed"
    });
    let act: Activity = serde_json::from_value(json).unwrap();
    assert_eq!(act.r#type.as_ref().unwrap().to_string(), "openapi_key_update");
    assert_eq!(act.target_key_id.as_deref(), Some("key-1"));
    assert_eq!(act.key_update_type.as_ref().unwrap().to_string(), "state-changed");
}

#[test]
fn test_activity_minimal() {
    use clickhouse_cloud_api::models::Activity;
    let json = serde_json::json!({
        "id": "act-3",
        "type": "user_login"
    });
    let act: Activity = serde_json::from_value(json).unwrap();
    assert_eq!(act.id.as_deref(), Some("act-3"));
    assert_eq!(act.r#type.as_ref().unwrap().to_string(), "user_login");
    assert!(act.actor_type.is_none());
    assert!(act.actor_details.is_none());
    assert!(act.service_id.is_none());
}

// ── Comprehensive enum value tests (from OpenAPI spec) ──────────────

#[test]
fn test_activity_type_values() {
    use clickhouse_cloud_api::models::Activity;
    let types = [
        "create_organization",
        "organization_update_name",
        "transfer_service_in",
        "transfer_service_out",
        "save_payment_method",
        "marketplace_subscription",
        "migrate_marketplace_billing_details_in",
        "migrate_marketplace_billing_details_out",
        "organization_update_tier",
        "organization_invite_create",
        "organization_invite_delete",
        "organization_member_join",
        "organization_member_add",
        "organization_member_leave",
        "organization_member_delete",
        "organization_member_update_role",
        "organization_member_update_mfa_method",
        "user_login",
        "user_login_failed",
        "user_logout",
        "key_create",
        "key_delete",
        "openapi_key_update",
        "service_create",
        "service_start",
        "service_stop",
        "service_awaken",
        "service_idle",
        "service_running",
        "service_partially_running",
        "service_delete",
        "service_update_name",
        "service_update_ip_access_list",
        "service_update_autoscaling_memory",
        "service_update_autoscaling_idling",
        "service_update_password",
        "service_update_autoscaling_replicas",
        "service_update_max_allowable_replicas",
        "service_update_backup_configuration",
        "service_restore_backup",
        "service_update_release_channel",
        "service_update_gpt_usage_consent",
        "service_update_private_endpoints",
        "service_import_to_organization",
        "service_export_from_organization",
        "service_maintenance_start",
        "service_maintenance_end",
        "service_update_core_dump",
        "backup_delete",
    ];
    for t in &types {
        let json = serde_json::json!({"id": "act-1", "type": t});
        let act: Activity = serde_json::from_value(json).unwrap();
        assert_eq!(act.r#type.as_ref().unwrap().to_string(), *t);
    }
}

#[test]
fn test_activity_actor_type_values() {
    use clickhouse_cloud_api::models::Activity;
    for actor_type in &["user", "support", "system", "api"] {
        let json =
            serde_json::json!({"id": "act-1", "type": "user_login", "actorType": actor_type});
        let act: Activity = serde_json::from_value(json).unwrap();
        assert_eq!(act.actor_type.as_ref().unwrap().to_string(), *actor_type);
    }
}

#[test]
fn test_activity_key_update_type_values() {
    use clickhouse_cloud_api::models::Activity;
    let update_types = [
        "created",
        "deleted",
        "name-changed",
        "role-changed",
        "state-changed",
        "date-changed",
        "ip-access-list-changed",
        "org-role-changed",
        "default-service-role-changed",
        "service-role-changed",
        "roles-v2-changed",
    ];
    for t in &update_types {
        let json = serde_json::json!({
            "id": "act-1",
            "type": "openapi_key_update",
            "keyUpdateType": t
        });
        let act: Activity = serde_json::from_value(json).unwrap();
        assert_eq!(act.key_update_type.as_ref().unwrap().to_string(), *t);
    }
}
