use clickhouse_cloud_api::models::*;

#[test]
fn deserialize_api_key() {
    let json = r#"{
        "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        "name": "My API Key",
        "state": "enabled",
        "roles": ["admin"],
        "createdAt": "2024-01-01T00:00:00Z",
        "expireAt": "2025-01-01T00:00:00Z"
    }"#;
    let key: ApiKey = serde_json::from_str(json).unwrap();
    assert_eq!(key.name.as_deref(), Some("My API Key"));
    assert_eq!(key.state, Some(ApiKeyState::Enabled));
}

#[test]
fn serialize_api_key_post_request() {
    let req = ApiKeyPostRequest {
        name: "test-key".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "test-key");
}

#[test]
fn api_key_ignores_extra_fields() {
    let json = r#"{"name":"key","state":"enabled","rotationPolicy":"weekly"}"#;
    let key: ApiKey = serde_json::from_str(json).unwrap();
    assert_eq!(key.name.as_deref(), Some("key"));
    assert_eq!(key.state, Some(ApiKeyState::Enabled));
}

#[test]
fn api_key_minimal_response() {
    let k: ApiKey = serde_json::from_str(r#"{"name":"k"}"#).unwrap();
    assert_eq!(k.name.as_deref(), Some("k"));
    assert_eq!(k.id, None);
    assert_eq!(k.state, None);
}

#[test]
fn api_key_patch_expiry_preserves_omit_set_and_clear() {
    let timestamp = chrono::DateTime::parse_from_rfc3339("2030-01-02T03:04:05Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    for (expire_at, wire) in [
        (None, serde_json::json!({})),
        (
            Some(Some(timestamp)),
            serde_json::json!({"expireAt": "2030-01-02T03:04:05Z"}),
        ),
        (Some(None), serde_json::json!({"expireAt": null})),
    ] {
        let request = ApiKeyPatchRequest {
            expire_at,
            ..Default::default()
        };
        assert_eq!(serde_json::to_value(&request).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<ApiKeyPatchRequest>(wire).unwrap(),
            request
        );
    }
}

#[test]
fn api_key_patch_deserialization_preserves_other_fields_and_ignores_unknowns() {
    let wire = serde_json::json!({
        "expireAt": null,
        "assignedRoleIds": ["aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"],
        "ipAccessList": [{"source": "10.0.0.0/8", "description": "private"}],
        "name": "renamed",
        "state": "disabled"
    });
    let mut extended = wire.clone();
    extended["newServerField"] = serde_json::json!({"nested": true});
    let request: ApiKeyPatchRequest = serde_json::from_value(extended).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), wire);
    let nulls: ApiKeyPatchRequest = serde_json::from_value(serde_json::json!({
        "assignedRoleIds": null, "ipAccessList": null, "name": null, "state": null
    }))
    .unwrap();
    assert_eq!(nulls, ApiKeyPatchRequest::default());
}

#[test]
fn api_key_patch_deserialization_rejects_invalid_recognized_fields() {
    for wire in [
        serde_json::json!({"expireAt": "tomorrow"}),
        serde_json::json!({"expireAt": 42}),
        serde_json::json!({"assignedRoleIds": ["invalid"]}),
        serde_json::json!({"ipAccessList": [{}]}),
        serde_json::json!({"name": false}),
        serde_json::json!({"state": {}}),
        serde_json::json!(null),
        serde_json::json!([]),
    ] {
        assert!(
            serde_json::from_value::<ApiKeyPatchRequest>(wire.clone()).is_err(),
            "accepted {wire}"
        );
    }
}

#[cfg(feature = "deprecated-fields")]
#[test]
fn api_key_patch_deserialization_preserves_deprecated_roles() {
    let wire = serde_json::json!({"expireAt": null, "roles": ["admin"]});
    let request: ApiKeyPatchRequest = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), wire);
}
