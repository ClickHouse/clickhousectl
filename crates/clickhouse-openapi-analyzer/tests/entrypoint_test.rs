use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

use clickhouse_openapi_analyzer::config::clickhouse_cloud_config;
use clickhouse_openapi_analyzer::report::DriftReport;
use clickhouse_openapi_analyzer::{AnalysisInput, analyze};

const SPEC: &str = include_str!("../../clickhouse-cloud-api/clickhouse_cloud_openapi.json");
const CLIENT: &str = include_str!("../../clickhouse-cloud-api/src/client.rs");
const MODELS: &str = include_str!("../../clickhouse-cloud-api/src/models.rs");
const META: &str = include_str!("../../clickhouse-cloud-api/src/meta.rs");

#[test]
fn executable_and_library_return_the_same_vendored_report() {
    let config = clickhouse_cloud_config();
    let expected = analyze(
        AnalysisInput {
            spec_json: SPEC,
            snapshot_json: SPEC,
            client_rs: CLIENT,
            models_rs: MODELS,
            meta_rs: META,
        },
        &config,
    )
    .unwrap();

    let directory = tempfile::tempdir().unwrap();
    let spec = directory.path().join("spec.json");
    let snapshot = directory.path().join("snapshot.json");
    let client = directory.path().join("client.rs");
    let models = directory.path().join("models.rs");
    let meta = directory.path().join("meta.rs");
    fs::write(&spec, SPEC).unwrap();
    fs::write(&snapshot, SPEC).unwrap();
    fs::write(&client, CLIENT).unwrap();
    fs::write(&models, MODELS).unwrap();
    fs::write(&meta, META).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_openapi-drift-analyzer"))
        .args(["--spec", spec.to_str().unwrap()])
        .args(["--snapshot", snapshot.to_str().unwrap()])
        .args(["--client", client.to_str().unwrap()])
        .args(["--models", models.to_str().unwrap()])
        .args(["--meta", meta.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: DriftReport = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(actual, expected);
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
