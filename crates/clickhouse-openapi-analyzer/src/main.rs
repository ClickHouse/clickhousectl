use std::fs;
use std::path::PathBuf;

use clap::Parser;
use clickhouse_openapi_analyzer::config::clickhouse_cloud_config;
use clickhouse_openapi_analyzer::{AnalysisInput, analyze};

#[derive(Debug, Parser)]
#[command(about = "Compare ClickHouse Cloud Rust API sources with an OpenAPI document")]
struct Args {
    #[arg(long)]
    spec: PathBuf,
    #[arg(long)]
    snapshot: PathBuf,
    #[arg(long)]
    source_root: PathBuf,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("openapi-drift-analyzer: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let spec = fs::read_to_string(&args.spec)?;
    let snapshot = fs::read_to_string(&args.snapshot)?;
    let report = analyze(
        AnalysisInput {
            spec_json: &spec,
            snapshot_json: &snapshot,
            rust_source_root: &args.source_root,
        },
        &clickhouse_cloud_config(),
    )?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
