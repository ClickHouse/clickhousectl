use clickhouse_cloud_api::models::*;

#[test]
fn whoami_variants_tolerate_missing_and_null_fields() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "actorType": null, "userId": null, "email": null, "name": null,
            "organizations": null
        }),
    ] {
        let user: WhoamiUser = serde_json::from_value(wire).unwrap();
        assert_eq!(user, WhoamiUser::default());
        assert_eq!(serde_json::to_value(user).unwrap(), serde_json::json!({}));
    }
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "actorType": null, "keyId": null, "name": null, "organizationId": null
        }),
    ] {
        let key: WhoamiApiKey = serde_json::from_value(wire).unwrap();
        assert_eq!(key, WhoamiApiKey::default());
        assert_eq!(serde_json::to_value(key).unwrap(), serde_json::json!({}));
    }
    for wire in [
        serde_json::json!({}),
        serde_json::json!({"organizationId": null, "organizationName": null}),
    ] {
        let org: WhoamiOrganization = serde_json::from_value(wire).unwrap();
        assert_eq!(org, WhoamiOrganization::default());
        assert_eq!(serde_json::to_value(org).unwrap(), serde_json::json!({}));
    }
}

#[test]
fn whoami_dispatches_on_actor_type_and_tolerates_sparse_variants() {
    let user: Whoami = serde_json::from_value(serde_json::json!({"actorType": "user"})).unwrap();
    assert_eq!(
        user,
        Whoami::WhoamiUser(WhoamiUser {
            actor_type: Some(WhoamiUserActortype::User),
            ..Default::default()
        })
    );
    assert_eq!(user.to_string(), "WhoamiUser");

    // `name` is shared by both variants; dispatch must follow `actorType`.
    let key: Whoami = serde_json::from_value(serde_json::json!({
        "actorType": "apiKey", "name": "ci key", "organizations": null
    }))
    .unwrap();
    assert_eq!(
        key,
        Whoami::WhoamiApiKey(WhoamiApiKey {
            actor_type: Some(WhoamiApiKeyActortype::ApiKey),
            name: Some("ci key".into()),
            ..Default::default()
        })
    );
    assert_eq!(key.to_string(), "WhoamiApiKey");
}

#[test]
fn whoami_keeps_unknown_absent_or_malformed_actor_types_verbatim() {
    for wire in [
        serde_json::json!({"actorType": "serviceAccount", "name": "sa"}),
        serde_json::json!({"name": "no discriminator"}),
        serde_json::json!({"actorType": null, "keyId": "key-1"}),
        // Recognized discriminator whose payload no longer fits the variant.
        serde_json::json!({"actorType": "user", "organizations": "not-a-list"}),
    ] {
        let whoami: Whoami = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(whoami, Whoami::Unknown(wire.clone()));
        assert_eq!(serde_json::to_value(&whoami).unwrap(), wire);
        assert_eq!(whoami.to_string(), wire.to_string());
    }
}

#[test]
fn whoami_actor_types_preserve_unknown_values() {
    let user: WhoamiUserActortype = serde_json::from_str("\"futureUser\"").unwrap();
    assert_eq!(user, WhoamiUserActortype::Unknown("futureUser".into()));
    assert_eq!(user.to_string(), "futureUser");
    assert_eq!(WhoamiUserActortype::User.to_string(), "user");
    let key: WhoamiApiKeyActortype = serde_json::from_str("\"futureKey\"").unwrap();
    assert_eq!(key, WhoamiApiKeyActortype::Unknown("futureKey".into()));
    assert_eq!(WhoamiApiKeyActortype::ApiKey.to_string(), "apiKey");
}
