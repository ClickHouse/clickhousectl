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

use config::AnalyzerConfig;
use openapi::OpenApiInventory;
use report::DriftReport;
use rust_inventory::RustInventory;
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct AnalysisInput<'a> {
    pub spec_json: &'a str,
    pub snapshot_json: &'a str,
    pub client_rs: &'a str,
    pub models_rs: &'a str,
    pub meta_rs: &'a str,
}

#[derive(Debug, Error)]
pub enum AnalyzeError {
    #[error("failed to parse target OpenAPI JSON: {0}")]
    SpecJson(#[source] serde_json::Error),
    #[error("failed to parse snapshot OpenAPI JSON: {0}")]
    SnapshotJson(#[source] serde_json::Error),
    // Covers both syn parse failures and analyzer policy rejections (e.g. a banned
    // `rename_all`), which both surface as a `syn::Error`; the source message is
    // self-explanatory, so the wrapper text stays neutral rather than claiming a parse failure.
    #[error("invalid Rust source: {0}")]
    RustSource(#[source] syn::Error),
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
    let rust = RustInventory::parse(input.client_rs, input.models_rs, input.meta_rs)
        .map_err(AnalyzeError::RustSource)?;
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

/// Computes response-tree membership from the library's `client.rs` and
/// `models.rs` sources.
pub fn response_tree(client_rs: &str, models_rs: &str) -> Result<ResponseTree, AnalyzeError> {
    let rust = RustInventory::parse(client_rs, models_rs, "").map_err(AnalyzeError::RustSource)?;
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

/// Lists every public model struct field in `models_rs` that carries a
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
pub fn model_fields_with_serde_default(models_rs: &str) -> Result<Vec<String>, AnalyzeError> {
    rust_inventory::model_fields_with_serde_default(models_rs).map_err(AnalyzeError::RustSource)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let tree = response_tree(client, models).unwrap();
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
}
