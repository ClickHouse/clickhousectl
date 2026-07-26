//! Internal OpenAPI drift analyzer used by tests and repository automation.

mod compare;
mod openapi;
mod rust_inventory;

pub mod config;
pub mod report;

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

/// Lists every public model struct field in `models_rs` that lacks a field-level
/// `#[serde(default)]`, as `StructName.rust_field_name`.
///
/// This backs the tolerant-deserialization policy test in `clickhouse-cloud-api` (see
/// AGENTS.md); it is intentionally not a drift `FindingKind`, because it compares Rust
/// source against a repository policy rather than against the OpenAPI spec. The parsing
/// stays behind this narrow function so `syn` never enters the `clickhouse-cloud-api`
/// dependency graph.
pub fn model_fields_missing_serde_default(models_rs: &str) -> Result<Vec<String>, AnalyzeError> {
    rust_inventory::model_fields_missing_serde_default(models_rs).map_err(AnalyzeError::RustSource)
}
