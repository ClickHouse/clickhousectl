use clickhouse_cloud_api::models::*;

#[test]
fn jwt_provider_requests_are_strict_and_omit_absent_roles_claim() {
    let post = serde_json::json!({
        "name": "my-oidc-provider",
        "issuer": "https://auth.example.com",
        "audience": "my-clickhouse-service",
        "jwksUrl": "https://auth.example.com/.well-known/jwks.json"
    });
    let request: JwtProviderPostRequest = serde_json::from_value(post.clone()).unwrap();
    assert_eq!(request.roles_claim, None);
    assert_eq!(serde_json::to_value(request).unwrap(), post);
    for field in ["name", "issuer", "audience", "jwksUrl"] {
        let mut missing = post.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<JwtProviderPostRequest>(missing).is_err(),
            "missing required field {field} must fail"
        );
    }

    let mut put = post.clone();
    put.as_object_mut().unwrap().remove("name");
    let request: JwtProviderPutRequest = serde_json::from_value(put.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), put);
    for field in ["issuer", "audience", "jwksUrl"] {
        let mut missing = put.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<JwtProviderPutRequest>(missing).is_err(),
            "missing required field {field} must fail"
        );
    }
}

#[test]
fn jwt_provider_requests_send_roles_claim_when_set() {
    let post = JwtProviderPostRequest {
        roles_claim: Some("clickhouse:roles".into()),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(post).unwrap()["rolesClaim"],
        "clickhouse:roles"
    );
    let put = JwtProviderPutRequest {
        roles_claim: Some("clickhouse:roles".into()),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(put).unwrap()["rolesClaim"],
        "clickhouse:roles"
    );
}

#[test]
fn jwt_provider_response_tolerates_missing_and_null_fields() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "name": null,
            "issuer": null,
            "audience": null,
            "jwksUrl": null,
            "rolesClaim": null
        }),
    ] {
        let provider: JwtProvider = serde_json::from_value(wire).unwrap();
        assert_eq!(provider, JwtProvider::default());
        assert_eq!(
            serde_json::to_value(provider).unwrap(),
            serde_json::json!({})
        );
    }

    let full = serde_json::json!({
        "name": "my-oidc-provider",
        "issuer": "https://auth.example.com",
        "audience": "my-clickhouse-service",
        "jwksUrl": "https://auth.example.com/.well-known/jwks.json",
        "rolesClaim": "clickhouse:roles",
        "futureField": true
    });
    let provider: JwtProvider = serde_json::from_value(full).unwrap();
    assert_eq!(
        provider.jwks_url.as_deref(),
        Some("https://auth.example.com/.well-known/jwks.json")
    );
    assert_eq!(provider.roles_claim.as_deref(), Some("clickhouse:roles"));
}
