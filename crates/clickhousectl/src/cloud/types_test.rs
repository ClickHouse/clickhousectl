use super::types::DeleteResponse;

// Contract policy for this suite:
// - mirror GA request/response schemas and query surfaces only
// - exclude beta endpoints, deprecated endpoints, and deprecated request fields
// - allow deprecated or BYOC fields in responses when they appear in GA schemas
// - keep BYOC request/command surface out of scope until explicitly added

// ── Response deserialization tests ──────────────────────────────────

#[test]
fn test_delete_response_round_trips_its_camel_case_wire_names() {
    let json = serde_json::json!({
        "status": 200,
        "requestId": "0182edf5-8c5b-4586-a6f8-78452320e4b1"
    });
    let response: DeleteResponse = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(response.status, Some(200));
    assert_eq!(
        response.request_id.as_deref(),
        Some("0182edf5-8c5b-4586-a6f8-78452320e4b1")
    );
    assert_eq!(serde_json::to_value(&response).unwrap(), json);
}

#[test]
fn test_delete_response_omits_absent_fields_instead_of_fabricating() {
    let response: DeleteResponse = serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(response.status, None);
    assert_eq!(response.request_id, None);
    // `--json` output must reflect the key set the API actually sent — no
    // fabricated `"status": 0` or `"requestId": ""`.
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json, serde_json::json!({}));
}

#[test]
fn test_delete_response_tolerates_null_fields() {
    let response: DeleteResponse =
        serde_json::from_value(serde_json::json!({"status": null, "requestId": null})).unwrap();
    assert_eq!(response.status, None);
    assert_eq!(response.request_id, None);
    assert_eq!(
        serde_json::to_value(response).unwrap(),
        serde_json::json!({})
    );
}

#[test]
fn test_delete_response_accepts_integral_float_status() {
    let response: DeleteResponse =
        serde_json::from_value(serde_json::json!({"status": 200.0})).unwrap();
    assert_eq!(response.status, Some(200));
    assert_eq!(
        serde_json::to_value(response).unwrap(),
        serde_json::json!({"status": 200})
    );
    assert!(
        serde_json::from_value::<DeleteResponse>(serde_json::json!({"status": 200.5})).is_err()
    );
}
