use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use clickhouse_openapi_analyzer::config::clickhouse_cloud_config;
use clickhouse_openapi_analyzer::report::{DriftReport, REPORT_SCHEMA_VERSION};
use clickhouse_openapi_analyzer::{AnalysisInput, analyze};

const SPEC: &str = include_str!("../../clickhouse-cloud-api/clickhouse_cloud_openapi.json");
const API_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../clickhouse-cloud-api");

#[test]
fn executable_and_library_return_the_same_vendored_report() {
    let config = clickhouse_cloud_config();
    let api_root = Path::new(API_ROOT);
    let spec = api_root.join("clickhouse_cloud_openapi.json");
    let source_root = api_root.join("src");

    let expected = analyze(
        AnalysisInput {
            spec_json: SPEC,
            snapshot_json: SPEC,
            rust_source_root: &source_root,
        },
        &config,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_openapi-drift-analyzer"))
        .args(["--spec", spec.to_str().unwrap()])
        .args(["--snapshot", spec.to_str().unwrap()])
        .args(["--source-root", source_root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: DriftReport = serde_json::from_slice(&output.stdout).unwrap();

    // Exact executable/library equality checks the source-tree input boundary.
    // The vendored spec is the independent inventory oracle: a dropped method,
    // model, field, or enum produces actionable drift, while the response-tree
    // policy tests separately retain their model-count vacuity guard.
    assert_eq!(actual, expected);
    assert_eq!(actual.schema_version, REPORT_SCHEMA_VERSION);
    assert!(!actual.has_drift(), "{}", actual.render_text());
    let reported_pointers = actual
        .unsupported_enum_constraints
        .iter()
        .map(|constraint| constraint.spec_pointer.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        reported_pointers, config.acknowledged_unsupported_enum_pointers,
        "the executable must report the exact configured unsupported enum inventory"
    );
}
