use clickhouse_cloud_api::models::*;

#[test]
fn deserialize_api_response_with_org_list() {
    let json = r#"{
        "status": 200,
        "requestId": "req-uuid-123",
        "result": [
            {
                "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
                "name": "Org 1"
            },
            {
                "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
                "name": "Org 2"
            }
        ]
    }"#;
    let resp: ApiResponse<Vec<Organization>> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, Some(200));
    assert_eq!(resp.request_id, Some("req-uuid-123".to_string()));
    let result = resp.result.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name.as_deref(), Some("Org 1"));
    assert_eq!(result[1].name.as_deref(), Some("Org 2"));
}

#[test]
fn deserialize_api_response_error() {
    let json = r#"{
        "status": 401,
        "error": "Unauthorized",
        "requestId": "req-uuid-456"
    }"#;
    let resp: ApiResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, Some(401));
    assert_eq!(resp.error, Some("Unauthorized".to_string()));
    assert!(resp.result.is_none());
}

// ===========================================================================
// ApiResponse envelope edge cases
// ===========================================================================

#[test]
fn api_response_result_explicitly_null() {
    let json = r#"{"status": 200, "requestId": "req-1", "result": null}"#;
    let resp: ApiResponse<Vec<Organization>> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, Some(200));
    assert!(resp.result.is_none());
}

#[test]
fn api_response_missing_status() {
    let json = r#"{"result": []}"#;
    let resp: ApiResponse<Vec<Organization>> = serde_json::from_str(json).unwrap();
    assert!(resp.status.is_none());
    assert!(resp.request_id.is_none());
    assert_eq!(resp.result.unwrap().len(), 0);
}

#[test]
fn api_response_extra_fields_ignored() {
    let json = r#"{
        "status": 200,
        "requestId": "req-1",
        "result": {"name": "Test"},
        "extraField": true,
        "anotherField": 42,
        "nestedExtra": {"a": 1}
    }"#;
    let resp: ApiResponse<Organization> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.status, Some(200));
    let org = resp.result.unwrap();
    assert_eq!(org.name.as_deref(), Some("Test"));
}

#[test]
fn api_response_empty_object() {
    let json = r#"{}"#;
    let resp: ApiResponse<Organization> = serde_json::from_str(json).unwrap();
    assert!(resp.status.is_none());
    assert!(resp.request_id.is_none());
    assert!(resp.result.is_none());
    assert!(resp.error.is_none());
}

#[test]
fn service_response_omits_absent_fields_when_serialized() {
    // Absent means absent, at every level of the response tree: a field that
    // was not returned is omitted from `--json` output, never emitted as
    // `null`.
    let svc = Service {
        name: Some("svc".to_string()),
        ip_access_list: Some(vec![IpAccessListEntryResponse {
            source: Some("0.0.0.0/0".to_string()),
            description: None,
        }]),
        tags: Some(vec![ResourceTagsV1Response {
            key: Some("env".to_string()),
            value: None,
        }]),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&svc).unwrap(),
        serde_json::json!({
            "name": "svc",
            "ipAccessList": [{ "source": "0.0.0.0/0" }],
            "tags": [{ "key": "env" }],
        })
    );
}

#[test]
fn shared_leaves_stay_strict_on_the_request_side() {
    // `ipAccessListEntry` and `resourceTagsV1` are sent as well as returned, so
    // each splits: the request variant keeps the schema's required fields as
    // `T`, and only the response variant is all-`Option`.
    let entry = IpAccessListEntry {
        source: "0.0.0.0/0".to_string(),
        description: None,
    };
    assert_eq!(
        serde_json::to_value(&entry).unwrap(),
        serde_json::json!({ "source": "0.0.0.0/0" })
    );
    // A request payload missing a required field is rejected, not defaulted.
    assert!(serde_json::from_str::<IpAccessListEntry>("{}").is_err());
    assert!(serde_json::from_str::<ResourceTagsV1>("{}").is_err());
    // The response variants accept the same payload.
    assert_eq!(
        serde_json::from_str::<IpAccessListEntryResponse>("{}").unwrap(),
        IpAccessListEntryResponse::default()
    );
    assert_eq!(
        serde_json::from_str::<ResourceTagsV1Response>("{}").unwrap(),
        ResourceTagsV1Response::default()
    );
}

#[test]
fn resource_tag_response_converts_back_into_a_request_tag() {
    let tag = ResourceTagsV1::try_from(ResourceTagsV1Response {
        key: Some("env".to_string()),
        value: Some("dev".to_string()),
    })
    .unwrap();
    assert_eq!(tag.key, "env");
    assert_eq!(tag.value.as_deref(), Some("dev"));

    // A tag is identified by its key, so a keyless response tag cannot be
    // written back.
    let missing = ResourceTagsV1::try_from(ResourceTagsV1Response {
        key: None,
        value: Some("dev".to_string()),
    })
    .unwrap_err();
    assert_eq!(missing.fields(), ["key"]);
}

#[test]
fn api_response_pagination_tolerates_missing_null_and_unknown_fields() {
    for value in [
        serde_json::json!({}),
        serde_json::json!({
            "limit": null, "totalCount": null, "nextCursor": null, "futureMetadata": true
        }),
    ] {
        let response: ApiResponse<Vec<ApiKey>> = serde_json::from_value(value).unwrap();
        assert_eq!(response.limit, None);
        assert_eq!(response.total_count, None);
        assert_eq!(response.next_cursor, None);
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({})
        );
    }
}

#[test]
fn api_response_pagination_round_trips_values_and_omits_last_page_cursor() {
    for (total_count, next_cursor) in [(0, None), (500, Some("opaque +/=&?雪"))] {
        let mut value = serde_json::json!({
            "status": 200, "requestId": "req-page", "result": [],
            "limit": 250, "totalCount": total_count
        });
        if let Some(cursor) = next_cursor {
            value["nextCursor"] = serde_json::json!(cursor);
        }
        let response: ApiResponse<Vec<ApiKey>> = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(response.limit, Some(250));
        assert_eq!(response.total_count, Some(total_count));
        assert_eq!(response.next_cursor.as_deref(), next_cursor);
        assert_eq!(serde_json::to_value(response).unwrap(), value);
    }
}
