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
}

/// Computes response-tree membership from the library's `client.rs` and
/// `models.rs` sources.
pub fn response_tree(client_rs: &str, models_rs: &str) -> Result<ResponseTree, AnalyzeError> {
    let rust = RustInventory::parse(client_rs, models_rs, "").map_err(AnalyzeError::RustSource)?;
    let types = rust.response_reachable_types();
    let mut non_option_fields = BTreeSet::new();
    for type_name in &types {
        let Some(struct_info) = rust.structs.get(type_name) else {
            continue;
        };
        for (spec_name, field) in &struct_info.fields {
            if !field.rust_type.is_option() {
                non_option_fields.insert((type_name.clone(), spec_name.clone()));
            }
        }
    }
    Ok(ResponseTree {
        types,
        non_option_fields,
    })
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
                pub name: Option<String>,
                #[serde(rename = "strictLeaf")]
                pub strict_leaf: WidgetLeaf,
            }
            pub struct WidgetLeaf { pub value: Option<String> }
            pub struct WidgetPostRequest { pub name: String }
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
    }
}
