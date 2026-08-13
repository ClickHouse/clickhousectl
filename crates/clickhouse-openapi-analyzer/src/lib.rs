//! Internal OpenAPI drift analyzer used by tests and repository automation.
//!
//! # Direction-aware checking
//!
//! Every spec schema is classified by the position(s) it is used in: request
//! position (reachable from a request body or operation parameter) and/or
//! response position (reachable from an operation response). Requiredness and
//! optionality rules (`required[]`, PATCH-all-optional, the description
//! heuristic, `partial_required_schemas`) apply only in request position. In
//! response position every field is `Option<T>` by policy — a server-dropped
//! or `null` field must never fail deserialization — so response-side
//! optionality drift is invisible by design; field *presence* (missing/extra
//! fields) and enum-value drift are the retained signals and are checked in
//! both directions.
//!
//! A schema used in both directions maps to two Rust types: the request
//! variant keeps the schema's name, and the response variant is
//! `{Name}Response` when that type exists (falling back to `{Name}` while the
//! split has not happened yet).

mod compare;
mod openapi;
mod rust_inventory;

pub mod config;
pub mod report;

use std::collections::BTreeSet;
use std::path::Path;

use config::AnalyzerConfig;
use openapi::OpenApiInventory;
use report::DriftReport;
use rust_inventory::RustInventory;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct AnalysisInput<'a> {
    pub spec_json: &'a str,
    pub snapshot_json: &'a str,
    /// Directory containing the `client`, `models`, and `meta` root modules.
    /// Both `<name>.rs` and `<name>/mod.rs` facade layouts are supported.
    pub rust_source_root: &'a Path,
}

#[derive(Debug, Error)]
pub enum AnalyzeError {
    #[error("failed to parse target OpenAPI JSON: {0}")]
    SpecJson(#[source] serde_json::Error),
    #[error("failed to parse snapshot OpenAPI JSON: {0}")]
    SnapshotJson(#[source] serde_json::Error),
    // Covers module loading, syn parse failures, and source-policy rejections
    // such as a banned `rename_all`.
    #[error("invalid Rust source tree: {0}")]
    RustSource(String),
    #[error("invalid target OpenAPI document: {0}")]
    SpecInventory(String),
    #[error("invalid snapshot OpenAPI document: {0}")]
    SnapshotInventory(String),
}

pub fn analyze(
    input: AnalysisInput<'_>,
    config: &AnalyzerConfig,
) -> Result<DriftReport, AnalyzeError> {
    let spec = serde_json::from_str(input.spec_json).map_err(AnalyzeError::SpecJson)?;
    let snapshot = serde_json::from_str(input.snapshot_json).map_err(AnalyzeError::SnapshotJson)?;
    let rust = load_rust_inventory(input.rust_source_root)?;
    let spec = OpenApiInventory::build(&spec, config).map_err(AnalyzeError::SpecInventory)?;
    let snapshot =
        OpenApiInventory::build(&snapshot, config).map_err(AnalyzeError::SnapshotInventory)?;
    Ok(compare::compare(&rust, &spec, &snapshot, config))
}

/// The Rust response tree: model types reachable from `Client` method return
/// types, for policy enforcement tests ("every field of every
/// response-reachable type is `Option<T>`").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseTree {
    /// Model type names transitively reachable from `Client` method return
    /// types via struct fields, enum variant payloads, and type aliases.
    pub types: BTreeSet<String>,
    /// `(type name, spec/wire field name)` pairs for struct fields in the
    /// response tree whose Rust type is not `Option<T>` — the pairs a policy
    /// enforcement test must require to be empty (modulo its exception list).
    pub non_option_fields: BTreeSet<(String, String)>,
    /// `(type name, spec/wire field name)` pairs for `Option<T>` struct fields
    /// in the response tree lacking `#[serde(skip_serializing_if = "...")]`.
    /// The policy pins the serialization decision: an absent response field is
    /// OMITTED from serialized output (`--json`, `print_human`), never emitted
    /// as `null`, so this set must also be empty.
    pub option_fields_missing_skip_serializing_if: BTreeSet<(String, String)>,
}

/// Computes response-tree membership from the library's client and model module
/// trees.
pub fn response_tree(rust_source_root: &Path) -> Result<ResponseTree, AnalyzeError> {
    let rust = load_rust_inventory(rust_source_root)?;
    let types = rust.response_reachable_types();
    let mut non_option_fields = BTreeSet::new();
    let mut option_fields_missing_skip_serializing_if = BTreeSet::new();
    for type_name in &types {
        let Some(struct_info) = rust.structs.get(type_name) else {
            continue;
        };
        for (spec_name, field) in &struct_info.fields {
            if !field.rust_type.is_option() {
                non_option_fields.insert((type_name.clone(), spec_name.clone()));
            } else if !field.skip_serializing_if {
                option_fields_missing_skip_serializing_if
                    .insert((type_name.clone(), spec_name.clone()));
            }
        }
    }
    Ok(ResponseTree {
        types,
        non_option_fields,
        option_fields_missing_skip_serializing_if,
    })
}

/// Lists direct OpenAPI `integer` properties represented as `f64` in a request
/// model or a model reachable from a wired `Client` response.
///
/// The analyzer resolves request/response split variants with the same mapping
/// used by drift analysis. Response targets are additionally constrained to the
/// Rust response tree, so unused schemas do not expand the policy surface.
pub fn integer_model_fields_typed_as_float(
    spec_json: &str,
    rust_source_root: &Path,
    config: &AnalyzerConfig,
) -> Result<BTreeSet<(String, String)>, AnalyzeError> {
    let spec = serde_json::from_str(spec_json).map_err(AnalyzeError::SpecJson)?;
    let spec = OpenApiInventory::build(&spec, config).map_err(AnalyzeError::SpecInventory)?;
    let rust = load_rust_inventory(rust_source_root)?;
    let response_types = rust.response_reachable_types();
    let mut offenders = BTreeSet::new();

    for ((schema_name, property_name), property) in &spec.properties {
        if property.schema_type.as_deref() != Some("integer") {
            continue;
        }
        for (rust_name, direction) in compare::field_check_targets(&rust, &spec, schema_name) {
            if direction == compare::Direction::Response && !response_types.contains(&rust_name) {
                continue;
            }
            let Some(field) = rust
                .structs
                .get(&rust_name)
                .and_then(|info| info.fields.get(property_name))
            else {
                continue;
            };
            if rust.terminal_type(&field.rust_type).as_deref() == Some("f64") {
                offenders.insert((rust_name, property_name.clone()));
            }
        }
    }

    Ok(offenders)
}

/// Lists every public model struct field in the model module tree that carries a
/// field-level `#[serde(default)]` (a container-level one reports every field
/// of its struct), as `StructName.rust_field_name`.
///
/// `#[serde(default)]` is banned repository-wide: on a required request field
/// it fabricates a value (`""`/`0`/`false`) indistinguishable from a genuine
/// server-sent one — the write-back hazard that sank the superseded
/// tolerant-deserialization policy — and on all-`Option` response fields it is
/// meaningless, because a missing key or explicit `null` already deserializes
/// to `None`. This backs the policy test in `clickhouse-cloud-api`'s
/// `spec_coverage_test.rs`; it is intentionally not a drift `FindingKind`,
/// because it compares Rust source against a repository policy rather than
/// against the OpenAPI spec. The parsing stays behind this narrow function so
/// `syn` never enters the `clickhouse-cloud-api` dependency graph.
pub fn model_fields_with_serde_default(
    rust_source_root: &Path,
) -> Result<Vec<String>, AnalyzeError> {
    Ok(load_rust_inventory(rust_source_root)?.model_fields_with_serde_default())
}

/// Lists every public struct, enum, and type alias in the model module tree.
pub fn model_types(rust_source_root: &Path) -> Result<BTreeSet<String>, AnalyzeError> {
    Ok(load_rust_inventory(rust_source_root)?.model_types)
}

/// Lists every model type in the model module tree with a hand-written `impl
/// Default for` block, sorted by name.
///
/// Backs the completeness half of
/// `discriminated_union_defaults_round_trip_to_the_same_variant` in
/// `clickhouse-cloud-api`'s `models_test.rs`: a `discriminated_union!` enum's
/// `Default` must round-trip to the same variant, and every union with a
/// `Default` uses a manual impl (derived `Default` cannot pick a payload
/// variant), so requiring the test's covered-type list to equal this set means
/// a new manual `Default` impl cannot silently escape the invariant. Like
/// [`model_fields_with_serde_default`], this is a repository-policy check
/// rather than a drift `FindingKind`, and it keeps `syn` out of the published
/// crate's dependency graph.
pub fn model_types_with_manual_default_impl(
    rust_source_root: &Path,
) -> Result<Vec<String>, AnalyzeError> {
    Ok(load_rust_inventory(rust_source_root)?
        .manual_default_impls
        .into_iter()
        .collect())
}

fn load_rust_inventory(rust_source_root: &Path) -> Result<RustInventory, AnalyzeError> {
    RustInventory::load(rust_source_root)
        .map_err(|error| AnalyzeError::RustSource(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn source_tree(client: &str, models: &str) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("client.rs"), client).unwrap();
        fs::write(directory.path().join("models.rs"), models).unwrap();
        fs::write(directory.path().join("meta.rs"), "").unwrap();
        directory
    }

    #[test]
    fn response_tree_reports_membership_and_non_option_fields() {
        let client = r#"
            pub struct Client;
            impl Client {
                pub async fn get_widget(&self) -> Result<Widget, Error> { unimplemented!() }
                pub async fn create_widget(&self, body: WidgetPostRequest) {}
            }
        "#;
        let models = r#"
            pub struct Widget {
                #[serde(skip_serializing_if = "Option::is_none")]
                pub name: Option<String>,
                #[serde(rename = "strictLeaf")]
                pub strict_leaf: WidgetLeaf,
            }
            pub struct WidgetLeaf { pub value: Option<String> }
            pub struct WidgetPostRequest { pub name: String, pub note: Option<String> }
        "#;
        let source = source_tree(client, models);
        let tree = response_tree(source.path()).unwrap();
        assert_eq!(
            tree.types,
            BTreeSet::from(["Widget".to_string(), "WidgetLeaf".to_string()])
        );
        assert_eq!(
            tree.non_option_fields,
            BTreeSet::from([("Widget".to_string(), "strictLeaf".to_string())]),
            "request-only strictness (WidgetPostRequest.name) must not be reported"
        );
        assert_eq!(
            tree.option_fields_missing_skip_serializing_if,
            BTreeSet::from([("WidgetLeaf".to_string(), "value".to_string())]),
            "request-only fields (WidgetPostRequest.note) must not be reported, \
             and Option fields with skip_serializing_if (Widget.name) are compliant"
        );
    }

    #[test]
    fn integer_float_inventory_resolves_split_models_and_ignores_number_fields() {
        let spec = r##"{
            "paths": {
                "/widgets": {
                    "get": {
                        "operationId": "getWidgets",
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {"$ref": "#/components/schemas/Widget"}
                                    }
                                }
                            }
                        }
                    },
                    "post": {
                        "operationId": "createWidget",
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/Widget"}
                                }
                            }
                        },
                        "responses": {}
                    }
                }
            },
            "components": {
                "schemas": {
                    "Widget": {
                        "type": "object",
                        "properties": {
                            "count": {"type": "integer"},
                            "nullableCount": {"type": ["integer", "null"]},
                            "ratio": {"type": "number"}
                        }
                    }
                }
            }
        }"##;
        let client = r#"
            pub struct Client;
            impl Client {
                pub async fn get_widgets(&self) -> Result<WidgetResponse, Error> {
                    unimplemented!()
                }
            }
        "#;
        let models = r#"
            pub struct Widget {
                pub count: f64,
                #[serde(rename = "nullableCount")]
                pub nullable_count: Option<f64>,
                pub ratio: f64,
            }
            pub struct WidgetResponse {
                pub count: Option<f64>,
                #[serde(rename = "nullableCount")]
                pub nullable_count: Option<f64>,
                pub ratio: Option<f64>,
            }
        "#;

        let source = source_tree(client, models);
        assert_eq!(
            integer_model_fields_typed_as_float(spec, source.path(), &AnalyzerConfig::default())
                .unwrap(),
            BTreeSet::from([
                ("Widget".to_string(), "count".to_string()),
                ("Widget".to_string(), "nullableCount".to_string()),
                ("WidgetResponse".to_string(), "count".to_string()),
                ("WidgetResponse".to_string(), "nullableCount".to_string()),
            ])
        );
    }
}
