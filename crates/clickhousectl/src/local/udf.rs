//! `local udf`: executable UDFs for project-local ClickHouse servers.
//!
//! A UDF lives in a directory (`clickhouse/udfs/<name>/`) holding
//! `udf.json`, the same definition `cloud udf` accepts, next to its files.

use crate::error::{Error, Result};
use crate::local::cli::{UdfCommands, UdfRuntimeArg, UdfTypeArg};
use crate::local::output::{self, UdfInitOutput};
use crate::udf::{DEFINITION_FILE, NATIVE_ENTRYPOINT, PYTHON_ENTRYPOINT};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Parent directory for scaffolded UDFs, relative to the project root. Matches
/// the `udfs/` entry of the `local init` scaffold.
pub(crate) const DEFAULT_UDF_PARENT: &str = "clickhouse/udfs";

pub async fn run(cmd: UdfCommands, json: bool) -> Result<()> {
    match cmd {
        UdfCommands::Init {
            name,
            runtime,
            kind,
        } => init_udf(&name, runtime, kind, json),
    }
}

fn init_udf(name: &str, runtime: UdfRuntimeArg, kind: UdfTypeArg, json: bool) -> Result<()> {
    let parent = Path::new(DEFAULT_UDF_PARENT);
    if let Some(existing) = case_only_clash(parent, name)? {
        return Err(Error::Usage(Box::new(clap::Error::raw(
            clap::error::ErrorKind::ValueValidation,
            format!(
                "UDF {existing} already exists in {DEFAULT_UDF_PARENT} and differs from {name} \
                 only by case; use {existing} or choose another name\n"
            ),
        ))));
    }
    let target = parent.join(name);
    std::fs::create_dir_all(&target)?;

    let mut created = Vec::new();
    write_if_absent(
        &target,
        DEFINITION_FILE,
        &definition_template(name, runtime, kind),
        0o644,
        &mut created,
    )?;
    let next_step = match runtime {
        UdfRuntimeArg::Python311 => {
            write_if_absent(
                &target,
                PYTHON_ENTRYPOINT,
                &python_template(name),
                0o755,
                &mut created,
            )?;
            None
        }
        UdfRuntimeArg::Native => {
            for arch in NATIVE_ARCH_DIRS {
                let arch_dir = target.join(arch);
                if !arch_dir.is_dir() {
                    std::fs::create_dir_all(&arch_dir)?;
                    created.push(format!("{arch}/"));
                }
                // `.gitkeep` only keeps the empty directory in version control;
                // the directory is what gets reported.
                write_if_absent(
                    &target,
                    &format!("{arch}/.gitkeep"),
                    "",
                    0o644,
                    &mut Vec::new(),
                )?;
            }
            Some(native_next_step())
        }
    };

    let out = UdfInitOutput {
        name: name.to_owned(),
        dir: target.display().to_string(),
        created,
        next_step,
    };
    output::print_output(&out, json);
    Ok(())
}

/// An existing UDF directory under `parent` whose name equals `name` except
/// for case. Case-insensitive filesystems (the macOS default) would resolve
/// `name` to that directory, whose `functionName` then no longer matches.
fn case_only_clash(parent: &Path, name: &str) -> Result<Option<String>> {
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let existing = entry?.file_name().to_string_lossy().into_owned();
        if existing != name && existing.eq_ignore_ascii_case(name) {
            return Ok(Some(existing));
        }
    }
    Ok(None)
}

/// Architecture directories a `native` UDF ships, each holding a `main`
/// binary the user builds. Matches the Cloud upload layout.
const NATIVE_ARCH_DIRS: [&str; 2] = ["amd64", "arm64"];

/// What a `native` scaffold still needs before it can run.
fn native_next_step() -> String {
    format!(
        "Build linux/amd64 and linux/arm64 binaries into amd64/{NATIVE_ENTRYPOINT} and \
         arm64/{NATIVE_ENTRYPOINT} (see {NATIVE_DOCS_URL})."
    )
}

const NATIVE_DOCS_URL: &str = "https://clickhouse.com/docs/products/cloud/features/sql-console-features/user-defined-functions";

/// Create `dir/relative` with `contents` unless it already exists, recording
/// `relative` in `created`. Existing files are kept untouched so re-running
/// `init` never discards edits.
fn write_if_absent(
    dir: &Path,
    relative: &str,
    contents: &str,
    mode: u32,
    created: &mut Vec<String>,
) -> Result<()> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(relative))
    {
        Ok(mut file) => {
            file.write_all(contents.as_bytes())?;
            file.set_permissions(std::fs::Permissions::from_mode(mode))?;
            created.push(relative.to_owned());
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
         \x20   # Values arrive escaped (\\t, \\n, \\\\): unescape before transforming, then escape the result.\n\
         \x20   for line in sys.stdin:\n\
         \x20       print(transform(line.rstrip(\"\\n\")))\n\
         \x20       sys.stdout.flush()\n\
         \n\
         \n\
         if __name__ == \"__main__\":\n\
         \x20   main()\n"
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
        assert!(python.contains(r"(\t, \n, \\)"));
        assert!(python.contains("    return value\n"));
    }

    #[test]
    fn case_only_clash_finds_a_differently_cased_sibling() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            case_only_clash(&tmp.path().join("missing"), "rev").unwrap(),
            None
        );
        std::fs::create_dir(tmp.path().join("rev")).unwrap();
        assert_eq!(case_only_clash(tmp.path(), "rev").unwrap(), None);
        assert_eq!(case_only_clash(tmp.path(), "other").unwrap(), None);
        assert_eq!(
            case_only_clash(tmp.path(), "Rev").unwrap().as_deref(),
            Some("rev")
        );
    }

    #[test]
    fn write_if_absent_creates_once_and_keeps_existing_content() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("main.py");
        let mut created = Vec::new();

        write_if_absent(tmp.path(), "main.py", "first\n", 0o755, &mut created).unwrap();
        assert_eq!(created, vec!["main.py"]);
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );

        write_if_absent(tmp.path(), "main.py", "second\n", 0o755, &mut created).unwrap();
        assert_eq!(created, vec!["main.py"]);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first\n");
    }
}
