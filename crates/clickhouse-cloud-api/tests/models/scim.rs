use clickhouse_cloud_api::models::*;

#[test]
fn scim_user_enterprise_extension_uses_the_exact_wire_name() {
    let user = ScimUser {
        enterprise_user: Some(ScimEnterpriseUser::default()),
        ..Default::default()
    };
    let json = serde_json::to_value(&user).unwrap();
    let extension = "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User";
    assert!(json.get(extension).is_some());
    assert_eq!(
        serde_json::from_value::<ScimUser>(json)
            .unwrap()
            .enterprise_user,
        user.enterprise_user
    );
}

#[test]
fn scim_request_models_stay_strict() {
    // The spec defines the SCIM schemas but no SCIM path, so the family is in
    // neither direction's tree and stays strict (see the comment above
    // `ScimEnterpriseManager` in models.rs and
    // `scim_models_are_outside_the_response_tree` in spec_coverage_test.rs).
    // Required fields are therefore `T` and a dropped one is a hard error.
    let err = serde_json::from_str::<ScimGroupPostRequest>(r#"{"schemas":[]}"#).unwrap_err();
    assert!(
        err.to_string().contains("displayName"),
        "unexpected error: {err}"
    );

    // Absent optional fields are still omitted rather than sent as `null`.
    let group = ScimGroupPostRequest {
        display_name: "Engineering".to_string(),
        external_id: None,
        id: None,
        members: None,
        meta: None,
        schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
    };
    let v = serde_json::to_value(&group).unwrap();
    assert_eq!(v["displayName"], "Engineering");
    assert_eq!(
        v["schemas"][0],
        "urn:ietf:params:scim:schemas:core:2.0:Group"
    );
    assert!(v.get("externalId").is_none());
    assert!(v.get("members").is_none());
    assert!(v.get("id").is_none());
    assert!(v.get("meta").is_none());
}

#[test]
fn scim_request_echoed_id_and_meta_round_trip() {
    let created = "2026-01-02T03:04:05Z";
    let modified = "2026-01-03T03:04:05Z";
    let patch: ScimPatchOp = serde_json::from_value(serde_json::json!({
        "schemas": ["urn:ietf:params:scim:api:messages:2.0:PatchOp"],
        "Operations": [],
        "id": "user-1",
        "meta": {"resourceType": "User", "created": created, "lastModified": modified}
    }))
    .unwrap();
    assert_eq!(patch.id.as_deref(), Some("user-1"));
    let Some(ScimPatchOpMeta::ScimUserMeta(meta)) = &patch.meta else {
        panic!("expected user meta, got {:?}", patch.meta);
    };
    assert_eq!(meta.resource_type, "User");
    assert_eq!(
        serde_json::to_value(&patch).unwrap()["meta"]["resourceType"],
        "User"
    );

    let group_meta = serde_json::json!({
        "resourceType": "Group", "created": created, "lastModified": modified
    });
    let parsed: ScimPatchOpMeta = serde_json::from_value(group_meta.clone()).unwrap();
    assert!(matches!(parsed, ScimPatchOpMeta::ScimGroupMeta(_)));
    let other = serde_json::json!({"resourceType": "Device"});
    let parsed: ScimPatchOpMeta = serde_json::from_value(other.clone()).unwrap();
    assert_eq!(parsed, ScimPatchOpMeta::Unknown(other));

    let group: ScimGroupPostRequest = serde_json::from_value(serde_json::json!({
        "schemas": [], "displayName": "Engineering", "id": "group-1", "meta": group_meta
    }))
    .unwrap();
    assert_eq!(group.id.as_deref(), Some("group-1"));
    assert_eq!(
        group.meta.map(|meta| meta.resource_type).as_deref(),
        Some("Group")
    );

    let user = ScimUserPostRequest {
        user_name: "ada".to_string(),
        ..Default::default()
    };
    let v = serde_json::to_value(&user).unwrap();
    assert!(v.get("id").is_none());
    assert!(v.get("meta").is_none());
    let v = serde_json::to_value(ScimPatchOp::default()).unwrap();
    assert!(v.get("id").is_none());
    assert!(v.get("meta").is_none());
}
