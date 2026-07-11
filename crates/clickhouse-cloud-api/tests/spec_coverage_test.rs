use clickhouse_openapi_analyzer::config::clickhouse_cloud_config;
use clickhouse_openapi_analyzer::{AnalysisInput, analyze};

const SPEC_JSON: &str = include_str!("../clickhouse_cloud_openapi.json");
const CLIENT_RS: &str = include_str!("../src/client.rs");
const MODELS_RS: &str = include_str!("../src/models.rs");
const META_RS: &str = include_str!("../src/meta.rs");
const LIVE_SPEC_URL: &str = "https://api.clickhouse.cloud/v1";

#[test]
fn vendored_openapi_snapshot_matches_rust_api() {
    let report = analyze_spec(SPEC_JSON);
    assert!(!report.has_drift(), "{}", report.render_text());
    assert_eq!(report.unsupported_enum_constraints.len(), 11);
    assert!(
        report
            .unsupported_enum_constraints
            .iter()
            .all(|constraint| constraint.acknowledged),
        "the vendored snapshot contains an unacknowledged unsupported enum constraint"
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
    let report = analyze_spec(&live_spec);
    assert!(!report.has_drift(), "{}", report.render_text());
}

fn analyze_spec(spec_json: &str) -> clickhouse_openapi_analyzer::report::DriftReport {
    analyze(
        AnalysisInput {
            spec_json,
            snapshot_json: SPEC_JSON,
            client_rs: CLIENT_RS,
            models_rs: MODELS_RS,
            meta_rs: META_RS,
        },
        &clickhouse_cloud_config(),
    )
    .unwrap()
}
