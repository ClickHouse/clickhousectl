use clickhouse_cloud_api::models::*;

#[test]
fn saved_query_request_is_strict_and_omits_optional_fields() {
    let required = serde_json::json!({
        "name": "orders",
        "sql": "SELECT * FROM orders WHERE id = {id:String}",
        "database": "default"
    });
    let request: PublicSavedQueryRequest = serde_json::from_value(required.clone()).unwrap();
    assert_eq!(request.parameters, None);
    assert_eq!(serde_json::to_value(&request).unwrap(), required);

    for field in ["name", "sql", "database"] {
        let mut missing = required.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<PublicSavedQueryRequest>(missing).is_err(),
            "missing required field {field} must fail"
        );
    }

    let maximal = PublicSavedQueryRequest {
        parameters: Some(std::collections::BTreeMap::from([(
            "id".to_string(),
            "42".to_string(),
        )])),
        ..request
    };
    let mut expected = required;
    expected["parameters"] = serde_json::json!({"id": "42"});
    assert_eq!(serde_json::to_value(maximal).unwrap(), expected);
}

#[test]
fn saved_query_responses_tolerate_missing_and_null_fields() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "id": null,
            "name": null,
            "sql": null,
            "database": null,
            "parameters": null
        }),
    ] {
        let query: PublicSavedQuery = serde_json::from_value(wire).unwrap();
        assert_eq!(query, PublicSavedQuery::default());
        assert_eq!(serde_json::to_value(query).unwrap(), serde_json::json!({}));
    }

    for wire in [
        serde_json::json!({}),
        serde_json::json!({"id": null, "name": null, "database": null}),
    ] {
        let item: PublicSavedQueryListItem = serde_json::from_value(wire).unwrap();
        assert_eq!(item, PublicSavedQueryListItem::default());
        assert_eq!(serde_json::to_value(item).unwrap(), serde_json::json!({}));
    }
}

#[test]
fn saved_query_response_round_trips_every_field_and_ignores_unknown_keys() {
    let wire = serde_json::json!({
        "id": "11111111-2222-3333-8444-555555555555",
        "name": "orders",
        "sql": "SELECT {id:String}",
        "database": "default",
        "parameters": {"id": "42"}
    });
    let mut with_unknown = wire.clone();
    with_unknown["futureField"] = serde_json::json!(true);
    let query: PublicSavedQuery = serde_json::from_value(with_unknown).unwrap();
    assert_eq!(serde_json::to_value(query).unwrap(), wire);

    let item_wire = serde_json::json!({
        "id": "11111111-2222-3333-8444-555555555555",
        "name": "orders",
        "database": "default"
    });
    let item: PublicSavedQueryListItem = serde_json::from_value(item_wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(item).unwrap(), item_wire);
}
