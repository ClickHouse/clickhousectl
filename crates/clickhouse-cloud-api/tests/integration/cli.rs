//! Wrapper around the `clickhousectl` binary for CLI-driven E2E tests.
//!
//! Each stage that converts to CLI-driven testing calls
//! `ClickhousectlCli::run(&["cloud", "clickpipe", "create", "...", ...])`,
//! which invokes the binary as a subprocess with the right env + `--url`
//! pointing at staging, then parses the `--json` stdout into a typed
//! `ClickPipe` (or similar) for the rest of the assertion code.
//!
//! Why subprocess-driven: every other test layer in this repo builds the
//! `ClickPipePostRequest` directly in-process and skips the
//! `clickhousectl::cloud::commands::clickpipe_create_*` handlers — the exact
//! layer where Al's `4f6c2ba` bugs lived. Subprocess invocation exercises
//! clap parsing → handler → HTTP client → API → ClickHouse, structurally
//! catching any future `.unwrap_or_default()`-style regression.

use std::path::PathBuf;
use std::process::Command;

use crate::integration::support::*;

pub struct ClickhousectlCli {
    binary: PathBuf,
    api_key: String,
    api_secret: String,
    base_url: Option<String>,
}

impl ClickhousectlCli {
    /// Locate the `clickhousectl` binary, building it first if necessary so
    /// the test always exercises the current source.
    pub fn build_and_locate() -> TestResult<PathBuf> {
        let status = Command::new("cargo")
            .args(["build", "-p", "clickhousectl", "--quiet"])
            .status()?;
        if !status.success() {
            return Err("cargo build -p clickhousectl failed".into());
        }

        // Tests run from `crates/clickhouse-cloud-api/`. The binary lives at
        // workspace_root/target/debug/clickhousectl. CARGO_MANIFEST_DIR points
        // at this crate; walk two levels up to find the workspace root.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let workspace = std::path::Path::new(manifest)
            .parent()
            .ok_or("CARGO_MANIFEST_DIR has no parent")?
            .parent()
            .ok_or("CARGO_MANIFEST_DIR has no grandparent")?;
        let binary = workspace.join("target/debug/clickhousectl");
        if !binary.exists() {
            return Err(format!("binary not found at {}", binary.display()).into());
        }
        Ok(binary)
    }

    /// Construct a CLI invoker that picks up its credentials + base URL from
    /// the same env vars the test framework already requires.
    pub fn from_env() -> TestResult<Self> {
        let binary = Self::build_and_locate()?;
        let api_key = required_env("CLICKHOUSE_CLOUD_API_KEY")?;
        let api_secret = required_env("CLICKHOUSE_CLOUD_API_SECRET")?;
        let base_url = std::env::var("CLICKHOUSE_CLOUD_API_BASE_URL")
            .ok()
            .filter(|s| !s.is_empty());
        Ok(Self {
            binary,
            api_key,
            api_secret,
            base_url,
        })
    }

    /// Run the binary with the given args + injected `--url` and `--json`,
    /// returning the parsed JSON stdout on exit-0 or a propagated error
    /// containing the full stderr on non-zero exit. The args list should NOT
    /// include `--url`, `--json`, `--api-key`, or `--api-secret` — this
    /// helper injects those for you.
    pub fn run_cloud_json(&self, args: &[&str]) -> TestResult<serde_json::Value> {
        let mut full_args: Vec<&str> = vec!["cloud", "--json"];
        if let Some(url) = &self.base_url {
            full_args.push("--url");
            full_args.push(url);
        }
        full_args.extend(args);

        let mut cmd = Command::new(&self.binary);
        cmd.args(&full_args);
        cmd.env("CLICKHOUSE_CLOUD_API_KEY", &self.api_key)
            .env("CLICKHOUSE_CLOUD_API_SECRET", &self.api_secret);

        eprintln!("  $ clickhousectl {}", redact_args(&full_args).join(" "));
        let output = cmd.output()?;
        if !output.status.success() {
            return Err(format!(
                "clickhousectl exited {}: stderr=\n{}\nstdout=\n{}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout),
            )
            .into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).map_err(|e| {
            format!("clickhousectl returned non-JSON stdout: {e}\nstdout:\n{stdout}").into()
        })
    }
}

/// Write a string to a per-run-id temp file under `/tmp` and return the path
/// as a String. Used by stages that need to pass cert/key contents to the
/// CLI as `--ca-certificate <path>` / `--client-key <path>` etc.; the CLI
/// reads from disk so the test has to materialise the file. Files don't
/// need explicit cleanup — `/tmp` is fine for short-lived test material.
pub fn write_temp_file(run_id: &str, name: &str, contents: &str) -> TestResult<String> {
    let path = format!("/tmp/clickhousectl-e2e-{run_id}-{name}");
    std::fs::write(&path, contents)?;
    Ok(path)
}

/// Redact sensitive flag values for log lines. Argument names are kept so the
/// invocation is reconstructable; secrets get `***`.
fn redact_args(args: &[&str]) -> Vec<String> {
    const SECRET_FLAGS: &[&str] = &[
        "--password",
        "--secret-key",
        "--api-secret",
        "--schema-registry-password",
        "--api-key",
    ];
    let mut out = Vec::with_capacity(args.len());
    let mut i = 0;
    while i < args.len() {
        let arg = args[i];
        out.push(arg.to_string());
        if SECRET_FLAGS.contains(&arg) && i + 1 < args.len() {
            out.push("***".into());
            i += 2;
        } else {
            i += 1;
        }
    }
    out
}
