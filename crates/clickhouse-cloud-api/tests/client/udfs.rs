use clickhouse_cloud_api::models::*;
use wiremock::matchers::{basic_auth, body_json, body_partial_json, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

use crate::{created_json, ok_empty, ok_json, setup};

// ===========================================================================
// UDFs
// ===========================================================================

#[tokio::test]
async fn delete_udf() {
    let (s, c) = setup().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_delete("org-1", "my_udf").await.unwrap().status,
        Some(200)
    );
}

#[tokio::test]
async fn detach_udf() {
    let (s, c) = setup().await;
    Mock::given(method("DELETE"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_detach("org-1", "my_udf", "svc-1")
            .await
            .unwrap()
            .status,
        Some(200)
    );
}

#[tokio::test]
async fn delete_udf_version() {
    let (s, c) = setup().await;
    Mock::given(method("DELETE"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/versions/7"))
        .respond_with(ok_empty())
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_version_delete("org-1", "my_udf", 7)
            .await
            .unwrap()
            .status,
        Some(200)
    );
}

#[tokio::test]
async fn list_udfs_encodes_pagination() {
    let (s, c) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs"))
        .and(query_param("cursor", "next-page"))
        .and(query_param("limit", "25"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    assert!(
        c.udf_list("org-1", Some("next-page"), Some(25))
            .await
            .unwrap()
            .result
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn get_udf() {
    let (s, c) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf"))
        .respond_with(ok_json(serde_json::json!({"functionName": "my_udf"})))
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_get("org-1", "my_udf")
            .await
            .unwrap()
            .result
            .unwrap()
            .function_name
            .as_deref(),
        Some("my_udf")
    );
}

#[tokio::test]
async fn list_udf_attachments_encodes_pagination() {
    let (s, c) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/attachments"))
        .and(query_param("cursor", "next-page"))
        .and(query_param("limit", "25"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    assert!(
        c.udf_attachment_list("org-1", "my_udf", Some("next-page"), Some(25))
            .await
            .unwrap()
            .result
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn get_udf_attachment() {
    let (s, c) = setup().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
        ))
        .respond_with(ok_json(serde_json::json!({"serviceId": "svc-1"})))
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_attachment_get("org-1", "my_udf", "svc-1")
            .await
            .unwrap()
            .result
            .unwrap()
            .service_id
            .as_deref(),
        Some("svc-1")
    );
}

#[tokio::test]
async fn list_udf_versions_encodes_pagination() {
    let (s, c) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
        .and(query_param("cursor", "next-page"))
        .and(query_param("limit", "25"))
        .respond_with(ok_json(serde_json::json!([])))
        .mount(&s)
        .await;

    assert!(
        c.udf_version_list("org-1", "my_udf", Some("next-page"), Some(25))
            .await
            .unwrap()
            .result
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn udf_list_pages_decode_arrays_and_nullable_outer_metadata() {
    for endpoint in ["", "/my_udf/versions", "/my_udf/attachments"] {
        for results in [
            serde_json::json!([]),
            serde_json::json!([{
                "functionName":"my_udf", "serviceId":"svc-1", "version":7,
                "status":"future-status", "runtime":null, "futureField":true
            }]),
        ] {
            for metadata in [
                serde_json::json!({"limit":1,"totalCount":3,"nextCursor":"page /+?"}),
                serde_json::json!({"limit":null,"totalCount":null,"nextCursor":null}),
                serde_json::json!({}),
            ] {
                let (server, client) = setup().await;
                let mut wire = metadata.clone();
                wire["result"] = results.clone();
                wire["status"] = 200.into();
                wire["requestId"] = "udf-page".into();
                Mock::given(method("GET"))
                    .and(path(format!("/v1/organizations/org-1/udfs{endpoint}")))
                    .respond_with(ResponseTemplate::new(200).set_body_json(wire))
                    .expect(1)
                    .mount(&server)
                    .await;
                let decoded = match endpoint {
                    "" => serde_json::to_value(client.udf_list("org-1", None, None).await.unwrap())
                        .unwrap(),
                    "/my_udf/versions" => serde_json::to_value(
                        client
                            .udf_version_list("org-1", "my_udf", None, None)
                            .await
                            .unwrap(),
                    )
                    .unwrap(),
                    _ => serde_json::to_value(
                        client
                            .udf_attachment_list("org-1", "my_udf", None, None)
                            .await
                            .unwrap(),
                    )
                    .unwrap(),
                };
                assert_eq!(
                    decoded["result"].as_array().unwrap().len(),
                    results.as_array().unwrap().len()
                );
                assert_eq!(decoded["status"], 200);
                assert_eq!(decoded["requestId"], "udf-page");
                for field in ["limit", "totalCount", "nextCursor"] {
                    assert_eq!(decoded[field], metadata[field]);
                    if metadata[field].is_null() {
                        assert!(decoded.get(field).is_none());
                    }
                }
                if !results.as_array().unwrap().is_empty() {
                    assert_eq!(decoded["result"][0]["version"], 7);
                    assert_eq!(decoded["result"][0]["status"], "future-status");
                    assert!(decoded["result"][0].get("futureField").is_none());
                }
            }
        }
    }
}

#[tokio::test]
async fn create_udf_upload_session() {
    let (s, c) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfUploads/url"))
        .respond_with(created_json(serde_json::json!({"uploadId": "upload-1"})))
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_upload_session_create("org-1")
            .await
            .unwrap()
            .result
            .unwrap()
            .upload_id
            .as_deref(),
        Some("upload-1")
    );
}

#[tokio::test]
async fn create_udf_encodes_request_body() {
    let (s, c) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs"))
        .and(body_partial_json(serde_json::json!({
            "arguments": [],
            "functionName": "my_udf",
            "returnType": "String",
            "runtime": "python3.11",
            "type": "executable",
            "uploadId": "upload-1", "deterministic": false, "memoryLimitMib": 256
        })))
        .respond_with(created_json(serde_json::json!({"functionName": "my_udf"})))
        .mount(&s)
        .await;

    let body = UdfCreateRequest::UdfCreateRequestV1(UdfCreateRequestV1 {
        deterministic: Some(false),
        memory_limit_mib: Some(256),
        arguments: vec![],
        function_name: "my_udf".to_string(),
        return_type: "String".to_string(),
        runtime: UdfRuntime::Python3_11,
        r#type: UdfCreateRequestV1Type::Executable,
        upload_id: "upload-1".to_string(),
        ..Default::default()
    });
    assert_eq!(
        c.udf_create("org-1", &body)
            .await
            .unwrap()
            .result
            .unwrap()
            .function_name
            .as_deref(),
        Some("my_udf")
    );
}

#[tokio::test]
async fn create_udf_version_encodes_request_body() {
    let (s, c) = setup().await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/org-1/udfs/my_udf/versions"))
        .and(body_partial_json(serde_json::json!({
            "arguments": [],
            "returnType": "String",
            "runtime": "python3.11",
            "type": "executable",
            "uploadId": "upload-1", "deterministic": false, "memoryLimitMib": 256
        })))
        .respond_with(created_json(serde_json::json!({"functionName": "my_udf"})))
        .mount(&s)
        .await;

    let body = UdfVersionCreateRequest::UdfVersionCreateRequestV1(UdfVersionCreateRequestV1 {
        deterministic: Some(false),
        memory_limit_mib: Some(256),
        arguments: vec![],
        return_type: "String".to_string(),
        runtime: UdfRuntime::Python3_11,
        r#type: UdfVersionCreateRequestV1Type::Executable,
        upload_id: "upload-1".to_string(),
        ..Default::default()
    });
    assert_eq!(
        c.udf_version_create("org-1", "my_udf", &body)
            .await
            .unwrap()
            .result
            .unwrap()
            .function_name
            .as_deref(),
        Some("my_udf")
    );
}

#[tokio::test]
async fn attach_udf_encodes_optional_version() {
    let (s, c) = setup().await;
    let attachment_path = "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1";
    let attachment = serde_json::json!({"functionName": "my_udf", "serviceId": "svc-1"});

    Mock::given(method("PUT"))
        .and(path(attachment_path))
        .and(body_json(serde_json::json!({})))
        .respond_with(ok_json(attachment.clone()))
        .mount(&s)
        .await;
    Mock::given(method("PUT"))
        .and(path(attachment_path))
        .and(body_json(serde_json::json!({"version": 7})))
        .respond_with(ok_json(attachment))
        .mount(&s)
        .await;

    assert_eq!(
        c.udf_attach("org-1", "my_udf", "svc-1", None)
            .await
            .unwrap()
            .result
            .unwrap()
            .service_id
            .as_deref(),
        Some("svc-1")
    );
    assert_eq!(
        c.udf_attach("org-1", "my_udf", "svc-1", Some(7))
            .await
            .unwrap()
            .result
            .unwrap()
            .function_name
            .as_deref(),
        Some("my_udf")
    );
}

#[tokio::test]
async fn attach_udf_preserves_structured_dependency_errors() {
    for (wire_code, expected_code, state, can_wake) in [
        (
            "SERVICE_IDLE",
            UdfAttachErrorCode::ServiceIdle,
            "idle",
            true,
        ),
        (
            "SERVICE_STOPPED",
            UdfAttachErrorCode::ServiceStopped,
            "stopped",
            false,
        ),
        (
            "SERVICE_NOT_RUNNING",
            UdfAttachErrorCode::ServiceNotRunning,
            "starting",
            false,
        ),
        (
            "FUTURE_CODE",
            UdfAttachErrorCode::Unknown("FUTURE_CODE".into()),
            "future_state",
            false,
        ),
    ] {
        let (server, client) = setup().await;
        let body = serde_json::json!({
            "error": "service unavailable", "code": wire_code, "serviceState": state,
            "canWake": can_wake, "status": 424,
            "requestId": "00000000-0000-0000-0000-000000000001", "futureField": 1
        });
        Mock::given(method("PUT"))
            .and(path(
                "/v1/organizations/org-1/udfs/my_udf/attachments/svc-1",
            ))
            .and(basic_auth("key", "secret"))
            .respond_with(ResponseTemplate::new(424).set_body_json(&body))
            .expect(1)
            .mount(&server)
            .await;
        let error = client
            .udf_attach("org-1", "my_udf", "svc-1", None)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "API error (status 424): service unavailable"
        );
        let clickhouse_cloud_api::Error::UdfAttachmentUnavailable {
            status,
            message,
            response,
        } = error
        else {
            panic!("expected typed UDF dependency error");
        };
        assert_eq!(status, 424);
        assert_eq!(message, "service unavailable");
        assert_eq!(response.code, Some(expected_code));
        assert_eq!(response.service_state.as_ref().unwrap().to_string(), state);
        assert_eq!(response.can_wake, Some(can_wake));
        let mut expected = body;
        expected.as_object_mut().unwrap().remove("futureField");
        assert_eq!(serde_json::to_value(response).unwrap(), expected);
        server.verify().await;
    }
}

#[tokio::test]
async fn attach_udf_dependency_errors_tolerate_absent_and_null_fields() {
    for body in [
        serde_json::json!({}),
        serde_json::json!({
            "error": null, "code": null, "serviceState": null, "canWake": null,
            "status": null, "requestId": null
        }),
    ] {
        let (server, client) = setup().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(424).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let error = client
            .udf_attach("org-1", "my_udf", "svc-1", None)
            .await
            .unwrap_err();
        let clickhouse_cloud_api::Error::UdfAttachmentUnavailable { response, .. } = error else {
            panic!("expected tolerant UDF dependency error");
        };
        assert_eq!(*response, UdfAttachResponse424::default());
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({})
        );
    }
}

#[tokio::test]
async fn attach_udf_keeps_generic_errors_for_other_statuses_and_invalid_bodies() {
    for (status, body, expected_message) in [
        (424, "upstream unavailable", "upstream unavailable"),
        (424, r#"{"error":"bad response","code":42}"#, "bad response"),
        (
            403,
            r#"{"error":"forbidden","code":"SERVICE_IDLE"}"#,
            "forbidden",
        ),
        (
            500,
            r#"{"error":"failed","code":"SERVICE_STOPPED"}"#,
            "failed",
        ),
    ] {
        let (server, client) = setup().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        let error = client
            .udf_attach("org-1", "my_udf", "svc-1", None)
            .await
            .unwrap_err();
        assert!(
            matches!(error, clickhouse_cloud_api::Error::Api { status: actual, message }
            if actual == status && message == expected_message)
        );
    }
}
