use std::collections::BTreeSet;

use clickhouse_openapi_analyzer::config::clickhouse_cloud_config;
use clickhouse_openapi_analyzer::{AnalysisInput, analyze};

const SPEC_JSON: &str = include_str!("../clickhouse_cloud_openapi.json");
const CLIENT_RS: &str = include_str!("../src/client.rs");
const MODELS_RS: &str = include_str!("../src/models.rs");
const META_RS: &str = include_str!("../src/meta.rs");
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

/// Enforces the tolerant-response deserialization policy documented in AGENTS.md: every
/// public model field carries a field-level `#[serde(default)]`, so a response field the
/// server stops sending degrades to that field's default instead of failing the whole
/// response. There is deliberately no exception list — requests stay strict through the
/// type system (required fields are `T`, not `Option<T>`), not through missing defaults.
#[test]
fn every_model_field_carries_serde_default() {
    let offenders = clickhouse_openapi_analyzer::model_fields_missing_serde_default(MODELS_RS)
        .expect("models.rs must parse");
    assert!(
        offenders.is_empty(),
        "these model fields lack #[serde(default)], violating the tolerant-deserialization \
         policy in AGENTS.md:\n{}",
        offenders.join("\n")
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
            client_rs: CLIENT_RS,
            models_rs: MODELS_RS,
            meta_rs: META_RS,
        },
        config,
    )
    .unwrap()
}
