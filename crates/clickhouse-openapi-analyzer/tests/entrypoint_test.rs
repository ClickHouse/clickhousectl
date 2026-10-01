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

/// Operation fragments captured from the vendored snapshot at a65a9ca6 and
/// https://api.clickhouse.cloud/v1 on 2026-09-18 for issue #978. Other operations
/// and their referenced models are isolated from the evolving library source.
fn contract_spec(after: bool) -> String {
    let mut spec: serde_json::Value = serde_json::from_str(SPEC).unwrap();
    let fragment: serde_json::Value = serde_json::from_str(if after {
        include_str!("fixtures/operation_contracts/after.json")
    } else {
        include_str!("fixtures/operation_contracts/before.json")
    })
    .unwrap();
    spec["paths"] = fragment;
    spec["components"]["schemas"]
        .as_object_mut()
        .unwrap()
        .retain(|name, _| {
            matches!(
                name.as_str(),
                "ApiKey" | "ServiceProfile" | "AssignedRole" | "IpAccessListEntry"
            )
        });
    spec.to_string()
}

// The historical client shape is part of the regression fixture: otherwise
// fixing the real library invalidates the before/after drift assertions.
fn contract_report(target: &str, snapshot: &str) -> DriftReport {
    let source = tempfile::tempdir().unwrap();
    std::fs::write(
        source.path().join("client.rs"),
        r#"
        impl Client {
            pub async fn openapi_key_get_list(&self, organization_id: &str)
                -> Result<ApiResponse<Vec<ApiKey>>, Error> { todo!() }
            pub async fn service_profiles_list(&self, organization_id: &str,
                region_id: &str, byoc_id: Option<&str>)
                -> Result<ApiResponse<Vec<ServiceProfile>>, Error> { todo!() }
        }
    "#,
    )
    .unwrap();
    std::fs::write(
        source.path().join("models.rs"),
        r#"
        pub struct ApiResponse<T> {
            pub status: Option<i64>,
            #[serde(rename = "requestId")] pub request_id: Option<String>,
            pub result: Option<T>,
            pub error: Option<String>,
        }
        pub struct ApiKey {
            pub id: Option<uuid::Uuid>,
            pub name: Option<String>,
            pub state: Option<ApiKeyState>,
            #[cfg(feature = "deprecated-fields")] pub roles: Option<Vec<String>>,
            #[serde(rename = "assignedRoles")] pub assigned_roles: Option<Vec<AssignedRole>>,
            #[serde(rename = "keySuffix")] pub key_suffix: Option<String>,
            #[serde(rename = "createdAt")] pub created_at: Option<chrono::DateTime<chrono::Utc>>,
            #[serde(rename = "expireAt")] pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
            #[serde(rename = "usedAt")] pub used_at: Option<chrono::DateTime<chrono::Utc>>,
            #[serde(rename = "ipAccessList")] pub ip_access_list: Option<Vec<IpAccessListEntry>>,
        }
        pub enum ApiKeyState {
            #[serde(rename = "enabled")] Enabled,
            #[serde(rename = "disabled")] Disabled,
            #[serde(untagged)] Unknown(String),
        }
        pub struct AssignedRole {
            #[serde(rename = "roleId")] pub role_id: Option<uuid::Uuid>,
            #[serde(rename = "roleName")] pub role_name: Option<String>,
            #[serde(rename = "roleType")] pub role_type: Option<AssignedRoleRoletype>,
        }
        pub enum AssignedRoleRoletype {
            #[serde(rename = "system")] System,
            #[serde(rename = "custom")] Custom,
            #[serde(untagged)] Unknown(String),
        }
        pub struct IpAccessListEntry { pub source: Option<String>, pub description: Option<String> }
        pub struct ServiceProfile {
            pub profile: Option<String>,
            #[serde(rename = "cpuCores")] pub cpu_cores: Option<f64>,
            #[serde(rename = "memoryGi")] pub memory_gi: Option<f64>,
        }
    "#,
    )
    .unwrap();
    std::fs::write(
        source.path().join("meta.rs"),
        r#"
        pub const BETA_OPERATIONS: &[&str] = &[];
        pub const DEPRECATED_FIELDS: &[(&str, &str)] = &[("ApiKey", "roles")];
    "#,
    )
    .unwrap();
    let mut config = clickhouse_openapi_analyzer::config::AnalyzerConfig::default();
    config
        .acknowledged_unsupported_enum_pointers
        .insert("/components/schemas/ApiKey/properties/roles/items".into());
    analyze(
        AnalysisInput {
            spec_json: target,
            snapshot_json: snapshot,
            rust_source_root: source.path(),
        },
        &config,
    )
    .unwrap()
}

#[test]
fn captured_pagination_and_byoc_changes_survive_snapshot_refresh() {
    use clickhouse_openapi_analyzer::report::FindingKind::*;
    let before = contract_spec(false);
    let after = contract_spec(true);
    assert!(!contract_report(&before, &before).has_drift());
    let report = contract_report(&after, &before);
    assert_eq!(report.findings.len(), 9, "{}", report.render_text());
    for kind in [
        MissingOperationParameter,
        OperationParameterMismatch,
        SnapshotAddedParameter,
        SnapshotChangedParameter,
        MissingStructField,
    ] {
        assert!(report.findings.iter().any(|finding| finding.kind == kind));
    }
    let target: serde_json::Value = serde_json::from_str(&after).unwrap();
    for finding in &report.findings {
        let pointer = finding.spec_pointer.as_ref().unwrap();
        assert!(target.pointer(pointer).is_some(), "{pointer}");
    }
    let refreshed = contract_report(&after, &after);
    assert_eq!(refreshed.findings.len(), 6, "{}", refreshed.render_text());
    assert_eq!(
        refreshed
            .findings
            .iter()
            .filter(|f| f.kind == MissingOperationParameter)
            .count(),
        2
    );
    assert_eq!(
        refreshed
            .findings
            .iter()
            .filter(|f| f.kind == OperationParameterMismatch)
            .count(),
        1
    );
    assert_eq!(
        refreshed
            .findings
            .iter()
            .filter(|f| f.kind == MissingStructField)
            .count(),
        3
    );
    let json = serde_json::to_string(&report).unwrap();
    assert_eq!(serde_json::from_str::<DriftReport>(&json).unwrap(), report);
    assert_eq!(
        contract_report(&after, &before).render_text(),
        report.render_text()
    );
}

#[test]
fn captured_contract_prose_and_parameter_order_do_not_produce_drift() {
    let before = contract_spec(false);
    let mut edited: serde_json::Value = serde_json::from_str(&before).unwrap();
    for path in [
        "/v1/organizations/{organizationId}/keys",
        "/v1/organizations/{organizationId}/serviceProfiles",
    ] {
        let operation = &mut edited["paths"][path]["get"];
        operation["description"] = "Revised documentation".into();
        operation["summary"] = "Revised summary".into();
        let parameters = operation["parameters"].as_array_mut().unwrap();
        for parameter in parameters.iter_mut() {
            parameter["description"] = "Revised parameter documentation".into();
            parameter["schema"]["example"] = "Example only".into();
        }
        parameters.reverse();
    }
    let report = contract_report(&edited.to_string(), &before);
    assert!(!report.has_drift(), "{}", report.render_text());
}

#[test]
fn executable_generates_catalog_and_preserves_output_on_unsupported_security() {
    let directory = tempfile::tempdir().unwrap();
    let spec = directory.path().join("spec.json");
    let catalog = directory.path().join("operations.rs");
    std::fs::write(&spec, SPEC).unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_openapi-drift-analyzer"))
            .arg("--spec")
            .arg(&spec)
            .arg("--generate-operations")
            .arg(&catalog)
            .output()
            .unwrap()
    };
    let output = run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected = clickhouse_openapi_analyzer::generate_operation_metadata(SPEC).unwrap();
    assert_eq!(std::fs::read_to_string(&catalog).unwrap(), expected);
    let mut malformed: serde_json::Value = serde_json::from_str(SPEC).unwrap();
    malformed["security"] = serde_json::json!([]);
    std::fs::write(&spec, malformed.to_string()).unwrap();
    assert!(!run().status.success());
    assert_eq!(std::fs::read_to_string(&catalog).unwrap(), expected);
}
