//! `local udf`: executable UDFs for project-local ClickHouse servers.
//!
//! A UDF lives in a directory (by default `clickhouse/udfs/<name>/`) holding
//! `udf.json`, the same definition `cloud udf` accepts, next to its files.

use crate::error::Result;
use crate::local::cli::{UdfCommands, UdfRuntimeArg, UdfTypeArg};
use crate::local::output::{self, UdfInitOutput};
use crate::udf::{DEFINITION_FILE, NATIVE_ENTRYPOINT, PYTHON_ENTRYPOINT};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Parent directory for scaffolded UDFs, relative to the project root. Matches
/// the `udfs/` entry of the `local init` scaffold.
pub(crate) const DEFAULT_UDF_PARENT: &str = "clickhouse/udfs";

pub async fn run(cmd: UdfCommands, json: bool) -> Result<()> {
    match cmd {
        UdfCommands::Init {
            name,
            runtime,
            kind,
            dir,
        } => init_udf(&name, runtime, kind, dir, json),
    }
}

fn init_udf(
    name: &str,
    runtime: UdfRuntimeArg,
    kind: UdfTypeArg,
    dir: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let parent = dir.unwrap_or_else(|| PathBuf::from(DEFAULT_UDF_PARENT));
    let target = parent.join(name);
    std::fs::create_dir_all(&target)?;

    let mut created = Vec::new();
    write_if_absent(
        &target.join(DEFINITION_FILE),
        &definition_template(name, runtime, kind),
        0o644,
        &mut created,
    )?;
    let (entrypoint, body) = match runtime {
        UdfRuntimeArg::Python311 => (PYTHON_ENTRYPOINT, python_template(name)),
        UdfRuntimeArg::Native => (NATIVE_ENTRYPOINT, native_template(name)),
    };
    write_if_absent(&target.join(entrypoint), &body, 0o755, &mut created)?;

    let out = UdfInitOutput {
        name: name.to_owned(),
        dir: target.display().to_string(),
        created,
    };
    output::print_output(&out, json);
    Ok(())
}

/// Create `path` with `contents` unless it already exists. Existing files are
/// kept untouched so re-running `init` never discards edits.
fn write_if_absent(
    path: &Path,
    contents: &str,
    mode: u32,
    created: &mut Vec<String>,
) -> Result<()> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            file.write_all(contents.as_bytes())?;
            file.set_permissions(std::fs::Permissions::from_mode(mode))?;
            created.push(
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// The scaffolded `udf.json`: a one-argument String function in the Cloud
/// API's field names, valid for both `local udf deploy` and `cloud udf create`.
pub(crate) fn definition_template(name: &str, runtime: UdfRuntimeArg, kind: UdfTypeArg) -> String {
    let value = serde_json::json!({
        "functionName": name,
        "type": kind.as_str(),
        "runtime": runtime.as_str(),
        "arguments": [{"name": "value", "type": "String"}],
        "returnType": "String",
        "format": "TabSeparated",
        "deterministic": false,
    });
    let mut text = serde_json::to_string_pretty(&value).expect("static template serializes");
    text.push('\n');
    text
}

fn python_template(name: &str) -> String {
    format!(
        "#!/usr/bin/env python3\n\
         \"\"\"{name}: executable UDF entrypoint.\n\
         \n\
         ClickHouse writes one TabSeparated row per line to stdin and reads one result\n\
         line per row from stdout. Flush after every row so pooled processes never stall.\n\
         \"\"\"\n\
         import sys\n\
         \n\
         \n\
         def transform(value: str) -> str:\n\
         \x20   return value\n\
         \n\
         \n\
         def main() -> None:\n\
         \x20   for line in sys.stdin:\n\
         \x20       print(transform(line.rstrip(\"\\n\")))\n\
         \x20       sys.stdout.flush()\n\
         \n\
         \n\
         if __name__ == \"__main__\":\n\
         \x20   main()\n"
    )
}

fn native_template(name: &str) -> String {
    format!(
        "#!/bin/sh\n\
         # {name}: executable UDF entrypoint. ClickHouse writes one TabSeparated row per\n\
         # line to stdin and reads one result line per row from stdout.\n\
         while IFS= read -r value; do\n\
         \x20   printf '%s\\n' \"$value\"\n\
         done\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_template_passes_shared_validation_for_every_variant() {
        for (runtime, kind) in [
            (UdfRuntimeArg::Python311, UdfTypeArg::Executable),
            (UdfRuntimeArg::Python311, UdfTypeArg::ExecutablePool),
            (UdfRuntimeArg::Native, UdfTypeArg::Executable),
            (UdfRuntimeArg::Native, UdfTypeArg::ExecutablePool),
        ] {
            let text = definition_template("my_fn", runtime, kind);
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(
                crate::udf::validate_definition(&value, true).unwrap(),
                kind.as_str()
            );
            assert_eq!(value["functionName"], "my_fn");
            assert_eq!(value["runtime"], runtime.as_str());
            assert_eq!(value["arguments"][0]["name"], "value");
            assert!(text.ends_with("}\n"));
        }
    }

    #[test]
    fn templates_start_with_a_shebang_and_mention_the_function() {
        let python = python_template("my_fn");
        assert!(python.starts_with("#!/usr/bin/env python3\n"));
        assert!(python.contains("\"\"\"my_fn: executable UDF entrypoint."));
        assert!(python.contains("sys.stdout.flush()"));
        assert!(python.contains("    return value\n"));

        let native = native_template("my_fn");
        assert!(native.starts_with("#!/bin/sh\n"));
        assert!(native.contains("# my_fn: executable UDF entrypoint."));
        assert!(native.contains("    printf '%s\\n' \"$value\"\n"));
    }

    #[test]
    fn write_if_absent_creates_once_and_keeps_existing_content() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("main.py");
        let mut created = Vec::new();

        write_if_absent(&path, "first\n", 0o755, &mut created).unwrap();
        assert_eq!(created, vec!["main.py"]);
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );

        write_if_absent(&path, "second\n", 0o755, &mut created).unwrap();
        assert_eq!(created, vec!["main.py"]);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first\n");
    }
}
