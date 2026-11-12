use std::fs;
use std::path::PathBuf;

use clap::Parser;
use clickhouse_openapi_analyzer::config::clickhouse_cloud_config;
use clickhouse_openapi_analyzer::{
    AnalysisInput, analyze, generate_operation_metadata, generate_stability_lists,
};

#[derive(Debug, Parser)]
#[command(about = "Compare ClickHouse Cloud Rust API sources with an OpenAPI document")]
struct Args {
    #[arg(long)]
    spec: PathBuf,
    #[arg(long, required_unless_present_any = ["generate_operations", "generate_stability_lists"])]
    snapshot: Option<PathBuf>,
    #[arg(long, required_unless_present = "generate_operations")]
    source_root: Option<PathBuf>,
    /// Write the public operation catalog instead of a drift report
    #[arg(long, conflicts_with_all = ["snapshot", "source_root", "generate_stability_lists"])]
    generate_operations: Option<PathBuf>,
    /// Print the meta.rs BETA_OPERATIONS and DEPRECATED_FIELDS declarations instead of a drift report
    #[arg(long, conflicts_with = "snapshot")]
    generate_stability_lists: bool,
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
    if let Some(output) = args.generate_operations {
        let source = generate_operation_metadata(&spec)?;
        fs::write(output, source)?;
        return Ok(());
    }
    let source_root = args.source_root.expect("clap requires source-root");
    if args.generate_stability_lists {
        print!(
            "{}",
            generate_stability_lists(&spec, &source_root, &clickhouse_cloud_config())?
        );
        return Ok(());
    }
    let snapshot = fs::read_to_string(args.snapshot.expect("clap requires snapshot"))?;
    let report = analyze(
        AnalysisInput {
            spec_json: &spec,
            snapshot_json: &snapshot,
            rust_source_root: &source_root,
        },
        &clickhouse_cloud_config(),
    )?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
