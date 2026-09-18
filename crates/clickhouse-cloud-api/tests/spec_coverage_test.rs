use std::collections::BTreeSet;
use std::path::Path;

use clickhouse_openapi_analyzer::config::clickhouse_cloud_config;
use clickhouse_openapi_analyzer::{
    AnalysisInput, analyze, integer_model_fields_typed_as_float, model_fields_with_serde_default,
    model_types, response_tree,
};

const SPEC_JSON: &str = include_str!("../clickhouse_cloud_openapi.json");
const RUST_SOURCE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
const LIVE_SPEC_URL: &str = "https://api.clickhouse.cloud/v1";

#[test]
fn vendored_openapi_snapshot_matches_rust_api() {
    let config = clickhouse_cloud_config();
    let report = analyze_spec(SPEC_JSON, &config);
    assert!(!report.has_drift(), "{}", report.render_text());
    assert!(
        report
            .unsupported_enum_constraints
            .iter()
            .all(|constraint| constraint.acknowledged),
        "the vendored snapshot contains an unacknowledged unsupported enum constraint"
    );
    let reported_pointers = report
        .unsupported_enum_constraints
        .iter()
        .map(|constraint| constraint.spec_pointer.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        reported_pointers, config.acknowledged_unsupported_enum_pointers,
        "the snapshot's unsupported enum inventory must exactly match analyzer configuration"
    );
}

/// Fields deliberately kept non-`Option` in response-tree types, as
/// `(RustTypeName, specFieldName)` pairs. The tolerant-response policy admits
/// no exceptions today: a strict response field is a latent outage point the
/// moment the API stops sending it, so adding an entry here requires the same
/// bar as an analyzer exemption — verified runtime behavior and a comment
/// saying why absence genuinely cannot happen.
const ALL_OPTION_EXCEPTIONS: &[(&str, &str)] = &[];

/// Every type the library deserializes API responses into is tolerant by
/// construction: a field the server drops or sends as `null` lands as `None`
/// instead of failing the response. The response tree is computed by the
/// analyzer from `client.rs` return types, so newly wired operations are
/// covered automatically. Types outside the tree (request bodies, orphan
/// schemas like `Scim*`) stay strict — scoping this to "every model type"
/// would itself create `FieldOptionalityMismatch` drift.
#[test]
fn every_response_tree_field_is_option() {
    let tree = response_tree(Path::new(RUST_SOURCE_ROOT)).unwrap();
    assert!(
        tree.types.len() >= 300,
        "vacuous test: the response tree collapsed to {} types — did client.rs \
         return types or the analyzer's reachability change shape?",
        tree.types.len()
    );
    let strict_fields = tree
        .non_option_fields
        .iter()
        .filter(|(type_name, field)| {
            !ALL_OPTION_EXCEPTIONS.contains(&(type_name.as_str(), field.as_str()))
        })
        .collect::<Vec<_>>();
    assert!(
        strict_fields.is_empty(),
        "response-reachable fields must be Option<T> so a dropped or null field \
         cannot fail deserialization: {strict_fields:?}"
    );
}

/// Pins the serialization half of the policy: an absent response field is
/// OMITTED from serialized output (`--json`, `print_human`), never emitted as
/// `null` — so every `Option` field in the response tree must carry
/// `#[serde(skip_serializing_if = "Option::is_none")]`.
#[test]
fn every_response_tree_option_field_omits_none_when_serialized() {
    let tree = response_tree(Path::new(RUST_SOURCE_ROOT)).unwrap();
    assert!(
        !tree.types.is_empty(),
        "vacuous test: the response tree is empty"
    );
    assert!(
        tree.option_fields_missing_skip_serializing_if.is_empty(),
        "response fields must omit None instead of serializing null; add \
         skip_serializing_if to: {:?}",
        tree.option_fields_missing_skip_serializing_if
    );
}

/// Integer-valued fields must use the repository-standard signed integer type
/// rather than `f64`; otherwise serde changes API integers such as `3` into
/// `3.0` when the CLI renders a model as JSON. The analyzer derives this set
/// from the vendored schema and request/response model mapping.
#[test]
fn integer_schema_fields_are_not_typed_as_float() {
    let offenders = integer_model_fields_typed_as_float(
        SPEC_JSON,
        Path::new(RUST_SOURCE_ROOT),
        &clickhouse_cloud_config(),
    )
    .unwrap();
    assert!(
        offenders.is_empty(),
        "OpenAPI integer fields must use i64, not f64: {offenders:?}"
    );
}

/// `#[serde(default)]` is banned in the model module tree. On a required request
/// field it fabricates a value (`""`/`0`/`false`) indistinguishable from a genuine
/// server-sent one — a consumer doing get → tweak → post would silently persist
/// it (the write-back hazard that sank the superseded issue-312 policy). On
/// response fields it is dead weight: every response-tree field is `Option<T>`
/// (enforced above), where a missing key already deserializes to `None`.
#[test]
fn models_carry_no_serde_default() {
    let offenders = model_fields_with_serde_default(Path::new(RUST_SOURCE_ROOT)).unwrap();
    assert!(
        offenders.is_empty(),
        "remove #[serde(default)] from: {offenders:?}"
    );
}

/// The SCIM models stay strict because they are in neither direction's tree: the
/// spec defines 40 `Scim*` schemas but no SCIM path, so no `Client` method sends
/// or returns one. Operation-unreferenced schemas resolve in request position, so
/// making a SCIM list/response envelope all-`Option` reports
/// `FieldOptionalityMismatch` drift while protecting no actual response.
///
/// If this fails, SCIM operations were added to `client.rs`: split the envelopes
/// they return into all-`Option` `{Name}Response` variants before wiring them up.
#[test]
fn scim_models_are_outside_the_response_tree() {
    let model_types = model_types(Path::new(RUST_SOURCE_ROOT)).unwrap();
    let scim_model_types = model_types
        .iter()
        .filter(|name| name.starts_with("Scim"))
        .collect::<Vec<_>>();
    assert!(
        scim_model_types.len() >= 40,
        "vacuous test: the SCIM model family collapsed to {} types",
        scim_model_types.len()
    );
    let tree = response_tree(Path::new(RUST_SOURCE_ROOT)).unwrap();
    let scim_response_types = tree
        .types
        .iter()
        .filter(|name| name.starts_with("Scim"))
        .collect::<Vec<_>>();
    assert!(
        scim_response_types.is_empty(),
        "SCIM types became response-reachable and must be split: {scim_response_types:?}"
    );
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn live_openapi_spec_matches_rust_api() {
    let response = reqwest::Client::new()
        .get(
            std::env::var("CLICKHOUSE_OPENAPI_SPEC_URL")
                .unwrap_or_else(|_| LIVE_SPEC_URL.to_string()),
        )
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let live_spec = response.text().await.unwrap();
    let config = clickhouse_cloud_config();
    let report = analyze_spec(&live_spec, &config);
    assert!(!report.has_drift(), "{}", report.render_text());
}

fn analyze_spec(
    spec_json: &str,
    config: &clickhouse_openapi_analyzer::config::AnalyzerConfig,
) -> clickhouse_openapi_analyzer::report::DriftReport {
    analyze(
        AnalysisInput {
            spec_json,
            snapshot_json: SPEC_JSON,
            rust_source_root: Path::new(RUST_SOURCE_ROOT),
        },
        config,
    )
    .unwrap()
}
