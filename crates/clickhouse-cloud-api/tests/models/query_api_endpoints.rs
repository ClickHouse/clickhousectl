use clickhouse_cloud_api::models::*;

#[test]
fn query_api_endpoint_request_is_strict_and_omits_optional_fields() {
    let required = serde_json::json!({
        "name": "orders",
        "sql": "SELECT * FROM orders WHERE id = {id:String}",
        "database": "default",
        "apiKeyIds": ["11111111-2222-3333-8444-555555555555"],
        "roles": ["query_api"]
    });
    let request: PublicQueryApiEndpointRequest = serde_json::from_value(required.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), required);

    for field in ["name", "sql", "database", "apiKeyIds", "roles"] {
        let mut missing = required.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<PublicQueryApiEndpointRequest>(missing).is_err(),
            "missing required field {field} must fail"
        );
    }
}

#[test]
fn query_api_endpoint_responses_tolerate_missing_and_null_fields() {
    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "id": null,
            "name": null,
            "sql": null,
            "database": null,
            "parameters": null,
            "apiKeyIds": null,
            "roles": null,
            "allowedOrigins": null,
            "url": null,
            "ownerType": null
        }),
    ] {
        let endpoint: PublicQueryApiEndpoint = serde_json::from_value(wire).unwrap();
        assert_eq!(endpoint, PublicQueryApiEndpoint::default());
        assert_eq!(
            serde_json::to_value(endpoint).unwrap(),
            serde_json::json!({})
        );
    }

    for wire in [
        serde_json::json!({}),
        serde_json::json!({
            "id": null,
            "name": null,
            "database": null,
            "apiKeyIds": null,
            "roles": null,
            "allowedOrigins": null,
            "url": null,
            "ownerType": null
        }),
    ] {
        let item: PublicQueryApiEndpointListItem = serde_json::from_value(wire).unwrap();
        assert_eq!(item, PublicQueryApiEndpointListItem::default());
        assert_eq!(serde_json::to_value(item).unwrap(), serde_json::json!({}));
    }

    for wire in [
        serde_json::json!({}),
        serde_json::json!({"items": null, "pagination": null}),
    ] {
        let list: QueryApiEndpointListResponse = serde_json::from_value(wire).unwrap();
        assert_eq!(list, QueryApiEndpointListResponse::default());
        assert_eq!(serde_json::to_value(list).unwrap(), serde_json::json!({}));
    }
}

#[test]
fn query_api_endpoint_list_response_preserves_array_entries_and_envelope_pagination() {
    for items in [serde_json::json!([]), serde_json::json!([{}])] {
        let response: ApiResponse<Vec<PublicQueryApiEndpointListItem>> =
            serde_json::from_value(serde_json::json!({
                "result": items, "limit": 1, "totalCount": 2,
                "nextCursor": "opaque +/=&? page", "futureEnvelopeField": true
            }))
            .unwrap();
        assert_eq!(
            serde_json::to_value(response.result.unwrap()).unwrap(),
            items
        );
        assert_eq!(response.limit, Some(1));
        assert_eq!(response.total_count, Some(2));
        assert_eq!(response.next_cursor.as_deref(), Some("opaque +/=&? page"));
    }
    for wire in [serde_json::json!({}), serde_json::json!({"result": null})] {
        let response: ApiResponse<Vec<PublicQueryApiEndpointListItem>> =
            serde_json::from_value(wire).unwrap();
        assert!(response.result.is_none());
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({})
        );
    }
}

#[test]
fn query_api_endpoint_owner_types_preserve_unknown_values() {
    let owner: PublicQueryApiEndpointOwnertype = serde_json::from_str("\"futureOwner\"").unwrap();
    assert_eq!(
        owner,
        PublicQueryApiEndpointOwnertype::Unknown("futureOwner".into())
    );
    assert_eq!(owner.to_string(), "futureOwner");
    assert_eq!(serde_json::to_string(&owner).unwrap(), "\"futureOwner\"");

    let list_owner: PublicQueryApiEndpointListItemOwnertype =
        serde_json::from_str("\"futureOwner\"").unwrap();
    assert_eq!(
        list_owner,
        PublicQueryApiEndpointListItemOwnertype::Unknown("futureOwner".into())
    );
    assert_eq!(list_owner.to_string(), "futureOwner");
    assert_eq!(
        serde_json::to_string(&list_owner).unwrap(),
        "\"futureOwner\""
    );
}
