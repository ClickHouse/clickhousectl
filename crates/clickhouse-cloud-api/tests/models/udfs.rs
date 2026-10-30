use clickhouse_cloud_api::models::*;

#[test]
fn udf_argument_output_preserves_response_tolerance_and_old_name() {
    for value in [
        serde_json::json!({}),
        serde_json::json!({"name": null, "type": null}),
    ] {
        let argument: UdfArgumentOutput = serde_json::from_value(value).unwrap();
        let old_name: UdfArgumentResponse = argument.clone();
        assert_eq!(old_name, UdfArgumentOutput::default());
        assert_eq!(
            serde_json::to_value(argument).unwrap(),
            serde_json::json!({})
        );
    }
    let value = serde_json::json!({"name": "x", "type": "UInt64"});
    let argument: UdfArgumentOutput = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(argument).unwrap(), value);
    for invalid in [
        serde_json::json!({}),
        serde_json::json!({"name": "x"}),
        serde_json::json!({"type": "UInt64"}),
    ] {
        assert!(serde_json::from_value::<UdfArgument>(invalid).is_err());
    }
}

#[test]
fn udf_responses_tolerate_dropped_and_null_fields() {
    let dropped: Udf = serde_json::from_str("{}").unwrap();
    let nulled: Udf = serde_json::from_str(
        r#"{"functionName":null,"runtime":null,"arguments":null,"createdAt":null,"memoryLimitMib":null}"#,
    )
    .unwrap();

    assert_eq!(dropped, Udf::default());
    assert_eq!(nulled, Udf::default());
    assert_eq!(serde_json::to_value(nulled).unwrap(), serde_json::json!({}));
}

#[test]
fn udf_response_models_tolerate_absent_and_null_fields() {
    // These models are reached only from Client response types. A dropped key
    // and an explicit null must therefore both deserialize to None, and None
    // must be omitted again when rendered as JSON.
    macro_rules! assert_tolerant_response {
        ($response:ty, $null_payload:literal) => {{
            let dropped: $response = serde_json::from_str("{}").unwrap();
            let nulled: $response = serde_json::from_str($null_payload).unwrap();
            assert_eq!(dropped, <$response>::default());
            assert_eq!(nulled, <$response>::default());
            assert_eq!(serde_json::to_value(nulled).unwrap(), serde_json::json!({}));
        }};
    }

    assert_tolerant_response!(
        Pagination,
        r#"{"totalRecords":null,"currentCursor":null,"nextCursor":null,"limit":null}"#
    );
    assert_tolerant_response!(
        UdfAttachment,
        r#"{"functionName":null,"serviceId":null,"status":null,"version":null}"#
    );
    assert_tolerant_response!(
        UdfAttachmentListResponse,
        r#"{"items":null,"pagination":null}"#
    );
    assert_tolerant_response!(UdfListResponse, r#"{"items":null,"pagination":null}"#);
    assert_tolerant_response!(
        UdfUploadSession,
        r#"{"uploadId":null,"uploadUrl":null,"expiresAt":null}"#
    );
    assert_tolerant_response!(
        UdfVersionListResponse,
        r#"{"items":null,"pagination":null}"#
    );
}

#[test]
fn udf_create_request_variants_keep_required_fields_strict() {
    // The discriminator is intentionally lossless at the union boundary. Once
    // it selects a known request shape, serde rejects a body missing a required
    // field rather than inventing a default value.
    assert!(
        serde_json::from_str::<UdfCreateRequestV1>(
            r#"{"arguments":[],"functionName":"my_udf","returnType":"String","runtime":"native","type":"executable"}"#,
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<UdfCreateRequestV2>(
            r#"{"arguments":[],"functionName":"my_udf","returnType":"String","runtime":"native","type":"executable_pool"}"#,
        )
        .is_err()
    );

    let request = UdfCreateRequestV1 {
        arguments: vec![UdfArgument {
            name: "x".to_string(),
            r#type: "UInt64".to_string(),
        }],
        function_name: "my_udf".to_string(),
        return_type: "UInt64".to_string(),
        runtime: UdfRuntime::Native,
        r#type: UdfCreateRequestV1Type::Executable,
        upload_id: "upload-1".to_string(),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({
            "arguments": [{"name": "x", "type": "UInt64"}],
            "functionName": "my_udf",
            "returnType": "UInt64",
            "runtime": "native",
            "type": "executable",
            "uploadId": "upload-1",
        })
    );
}

#[test]
fn udf_request_inline_variants_preserve_determinism_and_memory() {
    fn check<T: serde::de::DeserializeOwned + serde::Serialize>(kind: &str, create: bool) {
        let mut input = serde_json::json!({"uploadId": "upload-1", "runtime": "native",
            "arguments": [], "returnType": "UInt64", "type": kind});
        if create {
            input["functionName"] = serde_json::json!("my_udf");
        }
        let minimal: T = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(minimal).unwrap(), input);
        input["deterministic"] = serde_json::json!(false);
        input["memoryLimitMib"] = serde_json::json!(128);
        let full: T = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(full).unwrap(), input);
        input["memoryLimitMib"] = serde_json::Value::Null;
        let nullable: T = serde_json::from_value(input.clone()).unwrap();
        input.as_object_mut().unwrap().remove("memoryLimitMib");
        assert_eq!(serde_json::to_value(nullable).unwrap(), input);
        input.as_object_mut().unwrap().remove("runtime");
        assert!(serde_json::from_value::<T>(input).is_err());
    }
    check::<UdfCreateRequestV1>("executable", true);
    check::<UdfCreateRequestV2>("executable_pool", true);
    check::<UdfVersionCreateRequestV1>("executable", false);
    check::<UdfVersionCreateRequestV2>("executable_pool", false);
}
