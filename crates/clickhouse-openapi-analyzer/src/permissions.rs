//! ClickHouse Cloud's published basicAuth permission convention.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use serde_json::{Value, json};
use syn::{Expr, ItemConst, Lit, Member, Type};

use crate::AnalyzeError;
use crate::config::AnalyzerConfig;
use crate::openapi::{OpenApiInventory, OperationInfo};
use crate::report::{DriftReport, Finding, FindingKind};
use crate::rust_inventory::RustInventory;

#[derive(Debug, Clone)]
pub(crate) struct SecurityInfo {
    pub pointer: String,
    pub permissions: Result<BTreeSet<String>, String>,
}

impl SecurityInfo {
    fn contract(&self) -> String {
        match &self.permissions {
            Ok(permissions) => json!({"basicAuth": permissions}).to_string(),
            Err(reason) => json!({"unsupported": reason}).to_string(),
        }
    }
}

pub(crate) fn resolve_security(spec: &Value, operation: &Value, pointer: &str) -> SecurityInfo {
    let (security, pointer) = match operation.get("security") {
        Some(value) => (Some(value), format!("{pointer}/security")),
        None => (spec.get("security"), "/security".into()),
    };
    SecurityInfo {
        pointer,
        permissions: parse_security(spec, security),
    }
}

fn parse_security(spec: &Value, security: Option<&Value>) -> Result<BTreeSet<String>, String> {
    let requirements = security
        .and_then(Value::as_array)
        .ok_or("security must be an explicit array or inherit a root security array")?;
    if requirements.len() != 1 {
        return Err("expected one basicAuth requirement; anonymous or alternative requirements are unsupported".into());
    }
    let requirement = requirements[0]
        .as_object()
        .ok_or("security requirement must be an object")?;
    if requirement.len() != 1 || !requirement.contains_key("basicAuth") {
        return Err(
            "expected only basicAuth; anonymous, other or multiple schemes are unsupported".into(),
        );
    }
    let scheme = spec.pointer("/components/securitySchemes/basicAuth");
    if scheme.and_then(|v| v.get("type")).and_then(Value::as_str) != Some("http")
        || scheme.and_then(|v| v.get("scheme")).and_then(Value::as_str) != Some("basic")
    {
        return Err(
            "components.securitySchemes.basicAuth must declare HTTP basic authentication".into(),
        );
    }
    let permissions = requirement["basicAuth"]
        .as_array()
        .ok_or("basicAuth permissions must be an array")?;
    permissions
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|permission| !permission.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| "basicAuth permission IDs must be nonempty strings".into())
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationDescriptor {
    operation_id: String,
    rust_method: String,
    method: String,
    path: String,
    permissions: BTreeSet<String>,
}

pub(crate) fn is_operation_descriptor(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.path.segments.last().is_some_and(|s| s.ident == "OperationMetadata"))
}

pub(crate) fn parse_descriptor(item: &ItemConst) -> Result<OperationDescriptor, String> {
    if !matches!(item.vis, syn::Visibility::Public(_)) {
        return Err("operation descriptors must be public constants".into());
    }
    let Expr::Struct(expression) = item.expr.as_ref() else {
        return Err("operation descriptor must be a literal struct".into());
    };
    if expression.rest.is_some() {
        return Err("operation descriptor cannot use struct update syntax".into());
    }
    let fields: BTreeMap<_, _> = expression
        .fields
        .iter()
        .filter_map(|field| {
            let Member::Named(name) = &field.member else {
                return None;
            };
            Some((name.to_string(), &field.expr))
        })
        .collect();
    let string = |key: &str| -> Result<String, String> {
        fields
            .get(key)
            .and_then(|expression| literal_string(expression))
            .ok_or_else(|| format!("{key} must be a literal string"))
    };
    let permissions = fields
        .get("required_permissions")
        .ok_or("required_permissions is missing")?;
    let Expr::Array(array) = dereference(permissions) else {
        return Err("required_permissions must be a literal slice".into());
    };
    let permissions = array
        .elems
        .iter()
        .map(|expression| {
            literal_string(expression)
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| {
                    "required_permissions must contain nonempty literal strings".to_owned()
                })
        })
        .collect::<Result<_, _>>()?;
    Ok(OperationDescriptor {
        operation_id: string("operation_id")?,
        rust_method: string("rust_method")?,
        method: string("method")?,
        path: string("path")?,
        permissions,
    })
}

fn dereference(expression: &Expr) -> &Expr {
    match expression {
        Expr::Reference(reference) => dereference(&reference.expr),
        _ => expression,
    }
}

fn literal_string(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Lit(literal) => match &literal.lit {
            Lit::Str(value) => Some(value.value()),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn parse_catalog(expression: &Expr) -> Result<Vec<String>, String> {
    let Expr::Array(array) = dereference(expression) else {
        return Err("ALL must be a literal slice of operation constants".into());
    };
    array
        .elems
        .iter()
        .map(|expression| {
            let Expr::Path(path) = expression else {
                return Err("ALL must contain operation constant names".into());
            };
            path.path
                .get_ident()
                .map(ToString::to_string)
                .ok_or_else(|| "ALL must contain unqualified operation constant names".into())
        })
        .collect()
}

fn at_operation(
    kind: FindingKind,
    operation: &OperationInfo,
    name: &str,
    message: String,
) -> Finding {
    Finding::new(kind, message)
        .at_spec(&operation.security.pointer)
        .at_rust(format!("meta::operations::{}", name.to_ascii_uppercase()))
        .detail("operation_id", &operation.operation_id)
        .detail("rust_method", name)
        .detail("method", &operation.method)
        .detail("path", &operation.path)
}

fn permissions_details(
    finding: Finding,
    previous: &BTreeSet<String>,
    current: &BTreeSet<String>,
) -> Finding {
    finding
        .detail("previous_permissions", json!(previous).to_string())
        .detail("current_permissions", json!(current).to_string())
        .detail(
            "added_permissions",
            json!(current.difference(previous).collect::<Vec<_>>()).to_string(),
        )
        .detail(
            "removed_permissions",
            json!(previous.difference(current).collect::<Vec<_>>()).to_string(),
        )
}

pub(crate) fn compare_permissions(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    snapshot: &OpenApiInventory,
    report: &mut DriftReport,
) {
    for (name, operation) in &spec.operations {
        let permissions = match &operation.security.permissions {
            Ok(permissions) => Some(permissions),
            Err(reason) => {
                report.findings.push(at_operation(
                    FindingKind::UnsupportedOperationSecurity,
                    operation,
                    name,
                    format!("{}: {reason}", operation.operation_id),
                ));
                None
            }
        };
        if let Some(previous) = snapshot.operations.get(name)
            && previous.security.permissions != operation.security.permissions
        {
            let mut finding = at_operation(
                FindingKind::SnapshotChangedPermissions,
                operation,
                name,
                format!(
                    "{} permission requirements changed from {} to {}",
                    operation.operation_id,
                    previous.security.contract(),
                    operation.security.contract()
                ),
            )
            .detail("previous_requirement", previous.security.contract())
            .detail("current_requirement", operation.security.contract())
            .detail("previous_spec_pointer", &previous.security.pointer);
            if let (Ok(previous), Some(current)) = (&previous.security.permissions, permissions) {
                finding = permissions_details(finding, previous, current);
            }
            report.findings.push(finding);
        }
        let constant = name.to_ascii_uppercase();
        let Some(descriptor) = rust.metadata.operations.get(&constant) else {
            report.findings.push(at_operation(
                FindingKind::MissingOperationMetadata,
                operation,
                name,
                format!(
                    "{} has no public operation descriptor",
                    operation.operation_id
                ),
            ));
            continue;
        };
        let Ok(descriptor) = descriptor else {
            continue;
        };
        for (field, expected, actual) in [
            (
                "operation_id",
                &operation.operation_id,
                &descriptor.operation_id,
            ),
            ("rust_method", name, &descriptor.rust_method),
            ("method", &operation.method, &descriptor.method),
            ("path", &operation.path, &descriptor.path),
        ] {
            if expected != actual {
                report.findings.push(
                    at_operation(
                        FindingKind::OperationMetadataMismatch,
                        operation,
                        name,
                        format!("{constant}.{field} is {actual:?}; expected {expected:?}"),
                    )
                    .detail("field", field)
                    .detail("expected", expected)
                    .detail("actual", actual),
                );
            }
        }
        if let Some(permissions) = permissions
            && permissions != &descriptor.permissions
        {
            report.findings.push(permissions_details(
                at_operation(
                    FindingKind::OperationPermissionsMismatch,
                    operation,
                    name,
                    format!(
                        "{constant} permissions differ from the spec: {:?} -> {:?}",
                        descriptor.permissions, permissions
                    ),
                ),
                &descriptor.permissions,
                permissions,
            ));
        }
    }
    let expected_constants: BTreeSet<_> = spec
        .operations
        .keys()
        .map(|name| name.to_ascii_uppercase())
        .collect();
    for (constant, descriptor) in &rust.metadata.operations {
        if let Err(reason) = descriptor {
            report.findings.push(
                Finding::new(FindingKind::InvalidOperationMetadata, reason)
                    .at_rust(format!("meta::operations::{constant}")),
            );
        }
        if !expected_constants.contains(constant) {
            report.findings.push(
                Finding::new(
                    FindingKind::ExtraOperationMetadata,
                    format!("{constant} has no matching spec operation"),
                )
                .at_rust(format!("meta::operations::{constant}")),
            );
        }
        if let Ok(descriptor) = descriptor
            && !rust.client_methods.contains_key(&descriptor.rust_method)
        {
            report.findings.push(
                Finding::new(
                    FindingKind::OperationMetadataMismatch,
                    format!(
                        "{constant} references absent Client::{}",
                        descriptor.rust_method
                    ),
                )
                .at_rust(format!("meta::operations::{constant}")),
            );
        }
    }
    let catalog_error = match &rust.metadata.operation_catalog {
        None => Some("ALL operation catalog is missing".to_owned()),
        Some(Err(reason)) => Some(reason.clone()),
        Some(Ok(names)) => {
            let unique: BTreeSet<_> = names.iter().cloned().collect();
            let constants = rust.metadata.operations.keys().cloned().collect();
            let operation_ids = names
                .iter()
                .filter_map(|name| rust.metadata.operations.get(name))
                .filter_map(|entry| entry.as_ref().ok())
                .map(|entry| &entry.operation_id)
                .collect::<Vec<_>>();
            (unique.len() != names.len() || unique != constants || unique != expected_constants
                || operation_ids.len() != names.len() || operation_ids.windows(2).any(|pair| pair[0] >= pair[1]))
                .then(|| "ALL must contain each spec operation descriptor exactly once, sorted by operation ID".into())
        }
    };
    if let Some(reason) = catalog_error {
        report.findings.push(
            Finding::new(FindingKind::OperationCatalogMismatch, reason)
                .at_rust("meta::operations::ALL"),
        );
    }
}

/// Generate committed operation descriptors using the same resolved security
/// inventory as drift analysis. Unsupported or unknown security is an error;
/// an empty permission list is emitted only for a supported basicAuth requirement.
pub fn generate_operation_metadata(spec_json: &str) -> Result<String, AnalyzeError> {
    let spec = serde_json::from_str(spec_json).map_err(AnalyzeError::SpecJson)?;
    let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default())
        .map_err(AnalyzeError::SpecInventory)?;
    let mut source = String::from(
        "//! Generated from the vendored OpenAPI snapshot; do not edit by hand.\n//! Regenerate with `openapi-drift-analyzer --spec <snapshot> --generate-operations <file>`.\n\nuse super::OperationMetadata;\n\n",
    );
    let mut operations: Vec<_> = inventory.operations.iter().collect();
    operations.sort_by_key(|(_, operation)| &operation.operation_id);
    for (name, operation) in &operations {
        let permissions = operation.security.permissions.as_ref().map_err(|reason| {
            AnalyzeError::SpecInventory(format!(
                "{} [{}]: {reason}",
                operation.operation_id, operation.security.pointer
            ))
        })?;
        writeln!(source, "/// `{} {}` (`{}`).\npub const {}: OperationMetadata = OperationMetadata {{\n    operation_id: {:?},\n    rust_method: {:?},\n    method: {:?},\n    path: {:?},\n    required_permissions: &{:?},\n}};\n",
            operation.method, operation.path, operation.operation_id, name.to_ascii_uppercase(),
            operation.operation_id, name, operation.method, operation.path, permissions.iter().collect::<Vec<_>>()).unwrap();
    }
    source.push_str("/// Every supported OpenAPI operation, sorted by exact operation ID.\npub const ALL: &[OperationMetadata] = &[\n");
    for (name, _) in &operations {
        writeln!(source, "    {},", name.to_ascii_uppercase()).unwrap();
    }
    source.push_str("];\n\n/// Look up an exact OpenAPI operation ID. Unknown IDs return `None`.\npub fn by_operation_id(operation_id: &str) -> Option<&'static OperationMetadata> {\n    ALL.binary_search_by_key(&operation_id, |operation| operation.operation_id)\n        .ok()\n        .map(|index| &ALL[index])\n}\n");
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::compare;

    fn spec() -> Value {
        json!({
            "security": [{"basicAuth": ["root:view"]}],
            "paths": {
                "/widgets": {"get": {"operationId": "listWidgets"}},
                "/widget": {"get": {"operationId": "getWidget", "security": [{"basicAuth": ["widget:view", "widget:read"]}]}}
            },
            "components": {"schemas": {}, "securitySchemes": {"basicAuth": {"type": "http", "scheme": "basic"}}}
        })
    }

    fn inventory(spec: &Value) -> OpenApiInventory {
        OpenApiInventory::build(spec, &AnalyzerConfig::default()).unwrap()
    }

    fn analyze_with_metadata(target: &Value, snapshot: &Value, metadata: &str) -> DriftReport {
        let rust = RustInventory::parse(
            "impl Client { pub async fn get_widget(&self) {} pub async fn list_widgets(&self) {} }",
            "",
            metadata,
        )
        .unwrap();
        compare(
            &rust,
            &inventory(target),
            &inventory(snapshot),
            &AnalyzerConfig {
                check_operation_permissions: true,
                ..AnalyzerConfig::default()
            },
        )
    }

    fn report(target: &Value, snapshot: &Value) -> DriftReport {
        analyze_with_metadata(
            target,
            snapshot,
            &generate_operation_metadata(&snapshot.to_string()).unwrap(),
        )
    }

    #[test]
    fn security_inherits_root_and_explicit_requirements_override_it() {
        let mut spec = spec();
        let inv = inventory(&spec);
        assert_eq!(inv.operations["list_widgets"].security.pointer, "/security");
        assert_eq!(
            inv.operations["list_widgets"].security.permissions,
            Ok(BTreeSet::from(["root:view".into()]))
        );
        assert_eq!(
            inv.operations["get_widget"].security.permissions,
            Ok(BTreeSet::from(["widget:read".into(), "widget:view".into()]))
        );
        spec["paths"]["/widget"]["get"]["security"] = json!([{"basicAuth": []}]);
        assert_eq!(
            inventory(&spec).operations["get_widget"]
                .security
                .permissions,
            Ok(BTreeSet::new())
        );
        spec["paths"]["/widget"]["get"]
            .as_object_mut()
            .unwrap()
            .remove("security");
        assert_eq!(
            inventory(&spec).operations["get_widget"]
                .security
                .permissions,
            Ok(BTreeSet::from(["root:view".into()]))
        );
    }

    #[test]
    fn unsupported_security_is_actionable_and_never_generates_an_empty_list() {
        for security in [
            Value::Null,
            json!({}),
            json!([]),
            json!([{}]),
            json!([null]),
            json!([{"basicAuth": []}, {"basicAuth": ["alternate"]}]),
            json!([{"bearerAuth": []}]),
            json!([{"basicAuth": [], "other": []}]),
            json!([{"basicAuth": null}]),
            json!([{"basicAuth": "read"}]),
            json!([{"basicAuth": [1]}]),
            json!([{"basicAuth": ["read", null]}]),
            json!([{"basicAuth": [""]}]),
            json!([{"basicAuth": [" "]}]),
        ] {
            let baseline = spec();
            let mut target = baseline.clone();
            target["paths"]["/widget"]["get"]["security"] = security.clone();
            let report = report(&target, &baseline);
            let finding = report
                .findings
                .iter()
                .find(|f| f.kind == FindingKind::UnsupportedOperationSecurity)
                .unwrap_or_else(|| panic!("not reported: {security}"));
            assert_eq!(
                finding.spec_pointer.as_deref(),
                Some("/paths/~1widget/get/security")
            );
            assert!(
                report
                    .findings
                    .iter()
                    .any(|f| f.kind == FindingKind::SnapshotChangedPermissions)
            );
            assert!(
                generate_operation_metadata(&target.to_string()).is_err(),
                "generated unsupported {security}"
            );
        }
    }

    #[test]
    fn absent_root_and_unknown_scheme_are_explicit_findings() {
        for scenario in ["absent_root", "null_root", "absent_scheme", "wrong_scheme"] {
            let baseline = spec();
            let mut target = baseline.clone();
            match scenario {
                "absent_root" => {
                    target.as_object_mut().unwrap().remove("security");
                }
                "null_root" => target["security"] = Value::Null,
                "absent_scheme" => {
                    target["components"]
                        .as_object_mut()
                        .unwrap()
                        .remove("securitySchemes");
                }
                _ => {
                    target["components"]["securitySchemes"]["basicAuth"]["scheme"] = json!("bearer")
                }
            }
            assert!(
                report(&target, &baseline)
                    .findings
                    .iter()
                    .any(|f| f.kind == FindingKind::UnsupportedOperationSecurity),
                "{scenario}"
            );
            assert!(generate_operation_metadata(&target.to_string()).is_err());
        }
    }

    #[test]
    fn permission_add_remove_replace_and_root_changes_have_set_deltas() {
        let baseline = spec();
        for (current, added, removed) in [
            (
                json!(["widget:read", "widget:view", "widget:edit"]),
                json!(["widget:edit"]),
                json!([]),
            ),
            (json!(["widget:read"]), json!([]), json!(["widget:view"])),
            (
                json!(["widget:edit"]),
                json!(["widget:edit"]),
                json!(["widget:read", "widget:view"]),
            ),
            (json!([]), json!([]), json!(["widget:read", "widget:view"])),
        ] {
            let mut target = baseline.clone();
            target["paths"]["/widget"]["get"]["security"] = json!([{"basicAuth": current}]);
            let report = report(&target, &baseline);
            for kind in [
                FindingKind::SnapshotChangedPermissions,
                FindingKind::OperationPermissionsMismatch,
            ] {
                let finding = report.findings.iter().find(|f| f.kind == kind).unwrap();
                assert_eq!(finding.details["added_permissions"], added.to_string());
                assert_eq!(finding.details["removed_permissions"], removed.to_string());
            }
            assert_eq!(report.findings.len(), 2, "{}", report.render_text());
            let refreshed = analyze_with_metadata(
                &target,
                &target,
                &generate_operation_metadata(&baseline.to_string()).unwrap(),
            );
            assert_eq!(refreshed.findings.len(), 1);
            assert_eq!(
                refreshed.findings[0].kind,
                FindingKind::OperationPermissionsMismatch
            );
        }
        let mut target = baseline.clone();
        target["security"] = json!([{"basicAuth": ["root:edit"]}]);
        let report = report(&target, &baseline);
        assert_eq!(report.findings.len(), 2);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.spec_pointer.as_deref() == Some("/security")
                    && f.details["operation_id"] == "listWidgets")
        );
    }

    #[test]
    fn reordering_duplicates_and_equivalent_inheritance_do_not_drift() {
        let baseline = spec();
        let mut target = baseline.clone();
        target["paths"]["/widget"]["get"]["security"] =
            json!([{"basicAuth": ["widget:read", "widget:view", "widget:read"]}]);
        target["paths"]["/widgets"]["get"]["security"] = json!([{"basicAuth": ["root:view"]}]);
        assert!(!report(&target, &baseline).has_drift());
        assert_eq!(
            generate_operation_metadata(&target.to_string()).unwrap(),
            generate_operation_metadata(&baseline.to_string()).unwrap()
        );
    }

    #[test]
    fn catalog_validates_coverage_identity_and_removed_operations() {
        let baseline = spec();
        let metadata = generate_operation_metadata(&baseline.to_string()).unwrap();
        assert!(!analyze_with_metadata(&baseline, &baseline, &metadata).has_drift());
        let report = analyze_with_metadata(&baseline, &baseline, "");
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|f| f.kind == FindingKind::MissingOperationMetadata)
                .count(),
            2
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == FindingKind::OperationCatalogMismatch)
        );
        for (before, after) in [
            ("operation_id: \"getWidget\"", "operation_id: \"wrongId\""),
            (
                "rust_method: \"get_widget\"",
                "rust_method: \"absent_method\"",
            ),
            ("method: \"GET\"", "method: \"POST\""),
            ("path: \"/widget\"", "path: \"/wrong\""),
        ] {
            let report =
                analyze_with_metadata(&baseline, &baseline, &metadata.replace(before, after));
            assert!(
                report
                    .findings
                    .iter()
                    .any(|f| f.kind == FindingKind::OperationMetadataMismatch),
                "{before}"
            );
        }
        for replacement in ["", "    GET_WIDGET,\n    GET_WIDGET,\n"] {
            let changed = metadata.replace("    GET_WIDGET,\n", replacement);
            assert!(
                analyze_with_metadata(&baseline, &baseline, &changed)
                    .findings
                    .iter()
                    .any(|f| f.kind == FindingKind::OperationCatalogMismatch)
            );
        }
        let mut target = baseline.clone();
        target["paths"].as_object_mut().unwrap().remove("/widget");
        assert!(
            analyze_with_metadata(&target, &baseline, &metadata)
                .findings
                .iter()
                .any(|f| f.kind == FindingKind::ExtraOperationMetadata)
        );
    }

    #[test]
    fn malformed_literals_produce_actionable_metadata_findings() {
        let baseline = spec();
        let metadata = generate_operation_metadata(&baseline.to_string()).unwrap();
        for changed in [
            metadata.replace("pub const GET_WIDGET", "const GET_WIDGET"),
            metadata.replace(
                "required_permissions: &[\"widget:read\", \"widget:view\"]",
                "required_permissions: compute()",
            ),
            metadata.replace(
                "required_permissions: &[\"widget:read\", \"widget:view\"]",
                "required_permissions: &[\"widget:read\", 1]",
            ),
        ] {
            assert!(
                analyze_with_metadata(&baseline, &baseline, &changed)
                    .findings
                    .iter()
                    .any(|f| f.kind == FindingKind::InvalidOperationMetadata)
            );
        }
    }

    #[test]
    fn duplicate_descriptors_and_unsorted_catalogs_are_rejected() {
        let baseline = spec();
        let metadata = generate_operation_metadata(&baseline.to_string()).unwrap();
        let unsorted = metadata.replace(
            "    GET_WIDGET,\n    LIST_WIDGETS,",
            "    LIST_WIDGETS,\n    GET_WIDGET,",
        );
        assert!(
            analyze_with_metadata(&baseline, &baseline, &unsorted)
                .findings
                .iter()
                .any(|f| f.kind == FindingKind::OperationCatalogMismatch)
        );
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("client.rs"),
            "impl Client { pub async fn get_widget(&self) {} pub async fn list_widgets(&self) {} }",
        )
        .unwrap();
        std::fs::write(directory.path().join("models.rs"), "").unwrap();
        std::fs::write(
            directory.path().join("meta.rs"),
            format!("mod first {{ {metadata} }} mod second {{ {metadata} }}"),
        )
        .unwrap();
        let rust = RustInventory::load(directory.path()).unwrap();
        let report = compare(
            &rust,
            &inventory(&baseline),
            &inventory(&baseline),
            &AnalyzerConfig {
                check_operation_permissions: true,
                ..AnalyzerConfig::default()
            },
        );
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|f| f.kind == FindingKind::InvalidOperationMetadata)
                .count(),
            2
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == FindingKind::OperationCatalogMismatch)
        );
    }

    #[test]
    fn generation_rejects_colliding_operation_ids() {
        let mut spec = spec();
        spec["paths"]["/widget"]["get"]["operationId"] = json!("listWidgets");
        assert!(generate_operation_metadata(&spec.to_string()).is_err());
    }

    #[test]
    fn generation_matches_committed_catalog_after_formatting() {
        let spec = include_str!("../../clickhouse-cloud-api/clickhouse_cloud_openapi.json");
        let generated = generate_operation_metadata(spec).unwrap();
        let expected = RustInventory::parse("", "", &generated).unwrap();
        let actual = RustInventory::load(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../clickhouse-cloud-api/src"
        )))
        .unwrap();
        assert!(expected.metadata.operations.len() >= 150);
        assert_eq!(actual.metadata.operations, expected.metadata.operations);
        assert_eq!(
            actual.metadata.operation_catalog,
            expected.metadata.operation_catalog
        );
    }

    #[test]
    fn permission_reports_are_deterministic_and_preserve_details_in_json() {
        let baseline = spec();
        let mut target = baseline.clone();
        target["security"] = json!([{"basicAuth": ["root:edit"]}]);
        let report = report(&target, &baseline);
        let encoded = serde_json::to_string(&report).unwrap();
        let decoded: DriftReport = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, report);
        assert!(encoded.contains("snapshot_changed_permissions"));
        assert!(report.render_text().contains("root:view"));
        assert!(report.render_text().contains("root:edit"));
    }
}
