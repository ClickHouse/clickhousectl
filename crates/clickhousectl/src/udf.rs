//! Executable UDF definitions shared by `local udf` and `cloud udf`.
//!
//! A UDF is a directory `clickhouse/udfs/<NAME>/` holding `udf.json`, the
//! definition shape the Cloud API accepts, next to the function's files. This
//! module owns the target-neutral parts: the file-name conventions, the
//! `NAME` + parent directory resolution and the shared `--dir` flag, the
//! `Value`-level validation both targets apply before their own typed
//! deserialization, and the deterministic source walk shared by the Cloud
//! archive builder and the local copier. It knows nothing about `CloudError`
//! or the local `Error`; each target converts.

use serde_json::Value;
use std::fmt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Parent directory holding one `<NAME>/` directory per UDF, relative to the
/// project root. Matches the `udfs/` entry of the `local init` scaffold.
pub const DEFAULT_UDF_PARENT: &str = "clickhouse/udfs";
/// Definition file inside a UDF directory.
pub const DEFINITION_FILE: &str = "udf.json";
/// Entrypoint the `python3.11` runtime requires at the root of the sources.
pub const PYTHON_ENTRYPOINT: &str = "main.py";
/// Entrypoint of the `native` runtime: `main` inside each architecture
/// directory (`amd64/`, `arm64/`), both locally and on Cloud.
pub const NATIVE_ENTRYPOINT: &str = "main";
/// Architecture directories a `native` UDF ships, each holding a `main`
/// binary the user builds. Matches the Cloud upload layout; nothing outside
/// them is deployed.
pub const NATIVE_ARCH_DIRS: [&str; 2] = ["amd64", "arm64"];
/// Directory names skipped at any depth when collecting sources.
pub const EXCLUDED_DIRS: &[&str] = &["__pycache__"];

/// `--dir PATH`: the parent directory that contains `<NAME>/`. Flattened into
/// every command that takes a UDF name, so the flag reads identically
/// everywhere.
#[derive(clap::Args, Debug, Clone)]
pub struct UdfDirArg {
    /// Parent directory containing NAME/
    #[arg(long = "dir", value_name = "PATH", default_value = DEFAULT_UDF_PARENT)]
    pub dir: PathBuf,
}

/// Runtime a definition targets; drives entrypoint checks on both targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdfRuntimeKind {
    Python311,
    Native,
}

impl UdfRuntimeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Python311 => "python3.11",
            Self::Native => "native",
        }
    }
}

/// Failure reading, parsing or checking a UDF directory or definition file.
#[derive(Debug)]
pub enum UdfInputError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    Invalid {
        path: PathBuf,
        reason: String,
    },
}

impl fmt::Display for UdfInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Parse { path, source } => {
                write!(f, "{} is not valid JSON: {source}", path.display())
            }
            Self::Invalid { path, reason } => write!(f, "{} {reason}", path.display()),
        }
    }
}

impl std::error::Error for UdfInputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::Invalid { .. } => None,
        }
    }
}

fn invalid(path: &Path, reason: impl Into<String>) -> UdfInputError {
    UdfInputError::Invalid {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

/// Function, return and argument names: a letter, then letters, digits or
/// underscores. Matches the Cloud API pattern `^[A-Za-z][A-Za-z0-9_]*$`.
pub fn validate_function_name(value: &str) -> Result<(), String> {
    let mut bytes = value.bytes();
    if !bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
        || !bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("Use a letter followed by letters, digits or underscores".into());
    }
    Ok(())
}

/// `<parent>/<name>/`, which must be a real directory: a missing path, a
/// file, or a symbolic link (checked without following it) is rejected.
pub fn resolve_source_dir(parent: &Path, name: &str) -> Result<PathBuf, UdfInputError> {
    let dir = parent.join(name);
    match std::fs::symlink_metadata(&dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(invalid(&dir, "is a symbolic link"))
        }
        Ok(metadata) if metadata.is_dir() => Ok(dir),
        Ok(_) => Err(invalid(&dir, "is not a directory")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(invalid(&dir, "is not a directory"))
        }
        Err(source) => Err(UdfInputError::Read { path: dir, source }),
    }
}

/// Read and parse a definition file. Shape checks are left to
/// [`validate_definition`] so callers can report them separately.
pub fn load_definition(path: &Path) -> Result<Value, UdfInputError> {
    let contents = std::fs::read_to_string(path).map_err(|source| UdfInputError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&contents).map_err(|source| UdfInputError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

/// Read `dir/udf.json`.
pub fn load_definition_from_dir(dir: &Path) -> Result<Value, UdfInputError> {
    if !dir.is_dir() {
        return Err(invalid(dir, "is not a directory"));
    }
    load_definition(&dir.join(DEFINITION_FILE))
}

/// The directory name is the function name: a `udf.json` naming a different
/// function is rejected with both values. A missing `functionName` is left
/// to [`validate_definition`].
pub fn check_function_name(value: &Value, name: &str) -> Result<(), String> {
    match value.get("functionName").and_then(Value::as_str) {
        Some(found) if found != name => Err(format!(
            "functionName is {found}, but the command targets {name}"
        )),
        _ => Ok(()),
    }
}

/// Check the definition's shape and return its `type` value. Field names,
/// enum values and unknown keys are checked later by each target's strict
/// typed deserialization; this covers what that step cannot express: the
/// `uploadId` the CLI owns, integer ranges, nulls on non-nullable fields,
/// identifier syntax, and (`require_name`) the presence of `functionName`.
pub fn validate_definition(value: &Value, require_name: bool) -> Result<String, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "UDF definition must be a JSON object".to_string())?;
    if object.contains_key("uploadId") {
        return Err("Omit uploadId; the CLI creates a fresh upload session".into());
    }
    for name in [
        "commandReadTimeout",
        "commandWriteTimeout",
        "maxCommandExecutionTime",
        "poolSize",
        "memoryLimitMib",
    ] {
        if let Some(value) = object.get(name).filter(|v| !v.is_null()) {
            let max = if name == "memoryLimitMib" {
                1_048_576
            } else {
                i64::MAX
            };
            if !value.as_i64().is_some_and(|v| (1..=max).contains(&v)) {
                return Err(format!("UDF {name} must be an integer from 1 to {max}"));
            }
        }
    }
    let kind = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "UDF definition requires type".to_string())?
        .to_owned();
    // Option<T> is also used for non-nullable optional request fields; reject
    // explicit null here instead of silently converting it into omission.
    for name in [
        "runtime",
        "type",
        "deterministic",
        "sendChunkHeader",
        "format",
        "sandboxType",
        "sandboxVersion",
        "commandReadTimeout",
        "commandWriteTimeout",
    ] {
        if object.get(name).is_some_and(Value::is_null) {
            return Err(format!("UDF {name} cannot be null"));
        }
    }
    if kind == "executable_pool" {
        for name in ["poolSize", "maxCommandExecutionTime"] {
            if object.get(name).is_some_and(Value::is_null) {
                return Err(format!("UDF {name} cannot be null for executable_pool"));
            }
        }
    }
    for name in ["returnName", "functionName"] {
        if let Some(value) = object.get(name).filter(|v| !v.is_null()) {
            let valid = value
                .as_str()
                .is_some_and(|v| validate_function_name(v).is_ok());
            if !valid {
                return Err(format!("Invalid UDF {name}"));
            }
        }
    }
    if require_name && !object.get("functionName").is_some_and(Value::is_string) {
        return Err("UDF definition requires functionName".into());
    }
    if let Some(arguments) = object.get("arguments").and_then(Value::as_array) {
        for argument in arguments {
            if argument
                .get("name")
                .and_then(Value::as_str)
                .is_none_or(|v| validate_function_name(v).is_err())
            {
                return Err("Every UDF argument requires a valid name".into());
            }
        }
    }
    Ok(kind)
}

/// One file or directory inside a UDF source directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEntry {
    /// Path relative to the source directory.
    pub relative: PathBuf,
    pub absolute: PathBuf,
    pub is_dir: bool,
    /// Unix permission bits (`mode & 0o7777`).
    pub mode: u32,
}

/// Walk a UDF source directory in a deterministic order (sorted by name,
/// depth first). Symbolic links are rejected because Cloud rejects archives
/// that contain them; hidden entries and [`EXCLUDED_DIRS`] are skipped.
///
/// `python3.11` deploys the whole directory except `udf.json` and needs
/// `main.py` at the root. `native` deploys only `amd64/` and `arm64/`, each
/// of which must hold a regular `main` file; source code, build files and
/// anything else at the root are not inspected and not deployed.
pub fn collect_source_entries(
    dir: &Path,
    runtime: UdfRuntimeKind,
) -> Result<Vec<SourceEntry>, UdfInputError> {
    collect_source_entries_for(dir, runtime, &NATIVE_ARCH_DIRS)
}

/// [`collect_source_entries`], but a `native` UDF needs (and yields) only the
/// architecture directories in `native_arches`, a subset of
/// [`NATIVE_ARCH_DIRS`]. A local server runs only its host's binary, while
/// Cloud needs both.
pub fn collect_source_entries_for(
    dir: &Path,
    runtime: UdfRuntimeKind,
    native_arches: &[&str],
) -> Result<Vec<SourceEntry>, UdfInputError> {
    if !dir.is_dir() {
        return Err(invalid(dir, "is not a directory"));
    }
    let mut entries = Vec::new();
    match runtime {
        UdfRuntimeKind::Python311 => {
            walk(dir, dir, Path::new(""), true, &mut entries)?;
            if !entries.iter().any(|entry| !entry.is_dir) {
                return Err(invalid(dir, "contains no files to deploy"));
            }
            if !has_file(&entries, Path::new(PYTHON_ENTRYPOINT)) {
                return Err(invalid(
                    dir,
                    format!(
                        "is missing {PYTHON_ENTRYPOINT}, the entrypoint required by runtime python3.11"
                    ),
                ));
            }
        }
        UdfRuntimeKind::Native => {
            for arch in NATIVE_ARCH_DIRS
                .into_iter()
                .filter(|arch| native_arches.contains(arch))
            {
                let arch_dir = dir.join(arch);
                let metadata = match std::fs::symlink_metadata(&arch_dir) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(source) => {
                        return Err(UdfInputError::Read {
                            path: arch_dir,
                            source,
                        });
                    }
                };
                if metadata.file_type().is_symlink() {
                    return Err(invalid(dir, format!("contains a symbolic link at {arch}")));
                }
                if !metadata.is_dir() {
                    continue;
                }
                entries.push(SourceEntry {
                    relative: PathBuf::from(arch),
                    absolute: arch_dir.clone(),
                    is_dir: true,
                    mode: metadata.permissions().mode() & 0o7777,
                });
                walk(dir, &arch_dir, Path::new(arch), false, &mut entries)?;
            }
            let missing: Vec<String> = NATIVE_ARCH_DIRS
                .into_iter()
                .filter(|arch| native_arches.contains(arch))
                .map(|arch| format!("{arch}/{NATIVE_ENTRYPOINT}"))
                .filter(|binary| !has_file(&entries, Path::new(binary)))
                .collect();
            if !missing.is_empty() {
                let reason = if native_arches.len() == NATIVE_ARCH_DIRS.len() {
                    format!(
                        "is missing {}/{NATIVE_ENTRYPOINT} or {}/{NATIVE_ENTRYPOINT}, the binaries runtime native requires",
                        NATIVE_ARCH_DIRS[0], NATIVE_ARCH_DIRS[1]
                    )
                } else {
                    format!(
                        "is missing {}, the binary runtime native runs on this host",
                        missing.join(" and ")
                    )
                };
                return Err(invalid(dir, reason));
            }
        }
    }
    Ok(entries)
}

fn has_file(entries: &[SourceEntry], relative: &Path) -> bool {
    entries
        .iter()
        .any(|entry| !entry.is_dir && entry.relative == relative)
}

fn walk(
    root: &Path,
    current: &Path,
    prefix: &Path,
    at_root: bool,
    out: &mut Vec<SourceEntry>,
) -> Result<(), UdfInputError> {
    let read = |source| UdfInputError::Read {
        path: current.to_path_buf(),
        source,
    };
    let mut children = std::fs::read_dir(current)
        .map_err(read)?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(read)?;
    children.sort_by_key(|child| child.file_name());
    for child in children {
        let name = child.file_name();
        let name_text = name.to_string_lossy();
        if name_text.starts_with('.') || (at_root && name_text == DEFINITION_FILE) {
            continue;
        }
        let absolute = child.path();
        let relative = prefix.join(&name);
        let metadata =
            std::fs::symlink_metadata(&absolute).map_err(|source| UdfInputError::Read {
                path: absolute.clone(),
                source,
            })?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Err(invalid(
                root,
                format!("contains a symbolic link at {}", relative.display()),
            ));
        }
        let mode = metadata.permissions().mode() & 0o7777;
        if file_type.is_dir() {
            if EXCLUDED_DIRS.contains(&name_text.as_ref()) {
                continue;
            }
            out.push(SourceEntry {
                relative: relative.clone(),
                absolute: absolute.clone(),
                is_dir: true,
                mode,
            });
            walk(root, &absolute, &relative, false, out)?;
        } else if file_type.is_file() {
            out.push(SourceEntry {
                relative,
                absolute,
                is_dir: false,
                mode,
            });
        } else {
            return Err(invalid(
                root,
                format!(
                    "contains an unsupported file type at {}",
                    relative.display()
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn definition(kind: &str, with_name: bool) -> Value {
        let mut value = json!({
            "type": kind,
            "runtime": "native",
            "arguments": [{"name": "x", "type": "UInt64"}],
            "returnType": "UInt64"
        });
        if with_name {
            value["functionName"] = json!("my_udf");
        }
        value
    }

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn listed(entries: &[SourceEntry]) -> Vec<(String, bool)> {
        entries
            .iter()
            .map(|entry| (entry.relative.display().to_string(), entry.is_dir))
            .collect()
    }

    #[test]
    fn function_names_follow_the_api_pattern() {
        for ok in ["a", "A", "abc_1", "my_udf", "Z9"] {
            assert!(validate_function_name(ok).is_ok(), "{ok}");
        }
        for bad in ["", "1abc", "_x", "a-b", "a b", "../oops", "ünï"] {
            assert_eq!(
                validate_function_name(bad).unwrap_err(),
                "Use a letter followed by letters, digits or underscores",
                "{bad}"
            );
        }
    }

    #[test]
    fn resolve_source_dir_accepts_only_a_real_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path().join("udfs");
        std::fs::create_dir_all(parent.join("real")).unwrap();
        write(&parent.join("file"), "");
        std::os::unix::fs::symlink(parent.join("real"), parent.join("linked")).unwrap();

        assert_eq!(
            resolve_source_dir(&parent, "real").unwrap(),
            parent.join("real")
        );
        for (name, reason) in [
            ("missing", "is not a directory"),
            ("file", "is not a directory"),
            ("linked", "is a symbolic link"),
        ] {
            let error = resolve_source_dir(&parent, name).unwrap_err();
            assert!(
                matches!(&error, UdfInputError::Invalid { path, reason: found }
                    if path == &parent.join(name) && found == reason),
                "{name}: {error}"
            );
        }
    }

    #[test]
    fn check_function_name_compares_against_the_command_name() {
        assert!(check_function_name(&definition("executable", true), "my_udf").is_ok());
        assert!(check_function_name(&definition("executable", false), "other").is_ok());
        assert_eq!(
            check_function_name(&definition("executable", true), "other").unwrap_err(),
            "functionName is my_udf, but the command targets other"
        );
    }

    #[test]
    fn load_definition_reports_missing_dir_file_and_bad_json_distinctly() {
        let tmp = tempfile::tempdir().unwrap();
        let missing_dir = tmp.path().join("missing");
        assert!(matches!(
            load_definition_from_dir(&missing_dir).unwrap_err(),
            UdfInputError::Invalid { path, reason } if path == missing_dir && reason == "is not a directory"
        ));

        let dir = tmp.path().join("udf");
        std::fs::create_dir(&dir).unwrap();
        let error = load_definition_from_dir(&dir).unwrap_err();
        assert!(
            matches!(&error, UdfInputError::Read { path, .. } if path == &dir.join("udf.json"))
        );
        assert!(error.to_string().starts_with("cannot read "), "{error}");

        write(&dir.join("udf.json"), "{ not json");
        let error = load_definition_from_dir(&dir).unwrap_err();
        assert!(matches!(&error, UdfInputError::Parse { .. }));
        assert!(error.to_string().contains("is not valid JSON"), "{error}");

        write(&dir.join("udf.json"), r#"{"functionName": "f"}"#);
        assert_eq!(
            load_definition_from_dir(&dir).unwrap(),
            json!({"functionName": "f"})
        );
    }

    #[test]
    fn validate_definition_returns_the_kind_for_valid_input() {
        assert_eq!(
            validate_definition(&definition("executable", true), true).unwrap(),
            "executable"
        );
        assert_eq!(
            validate_definition(&definition("executable_pool", false), false).unwrap(),
            "executable_pool"
        );
        let mut nullable = definition("executable", true);
        nullable["memoryLimitMib"] = Value::Null;
        nullable["returnName"] = Value::Null;
        assert!(validate_definition(&nullable, true).is_ok());
        // Unknown enum values and keys are left to typed deserialization.
        let mut future = definition("future", true);
        future["typo"] = json!(1);
        assert_eq!(validate_definition(&future, true).unwrap(), "future");
    }

    #[test]
    fn validate_definition_rejects_shape_errors_with_stable_messages() {
        let cases: Vec<(&str, Value, &str)> = vec![
            (
                "deterministic",
                Value::Null,
                "UDF deterministic cannot be null",
            ),
            (
                "memoryLimitMib",
                json!(0),
                "UDF memoryLimitMib must be an integer from 1 to 1048576",
            ),
            (
                "memoryLimitMib",
                json!(1_048_577),
                "UDF memoryLimitMib must be an integer from 1 to 1048576",
            ),
            (
                "commandReadTimeout",
                json!(-1),
                "UDF commandReadTimeout must be an integer from 1 to 9223372036854775807",
            ),
            (
                "poolSize",
                json!("2"),
                "UDF poolSize must be an integer from 1 to 9223372036854775807",
            ),
            (
                "uploadId",
                json!("old"),
                "Omit uploadId; the CLI creates a fresh upload session",
            ),
            ("functionName", json!("../oops"), "Invalid UDF functionName"),
            ("returnName", json!("1x"), "Invalid UDF returnName"),
            (
                "arguments",
                json!([{"type": "String"}]),
                "Every UDF argument requires a valid name",
            ),
            ("type", json!(1), "UDF definition requires type"),
        ];
        for (key, value, message) in cases {
            let mut input = definition("executable", true);
            input[key] = value;
            assert_eq!(
                validate_definition(&input, true).unwrap_err(),
                message,
                "{key}"
            );
        }

        let mut pool = definition("executable_pool", true);
        pool["poolSize"] = Value::Null;
        assert_eq!(
            validate_definition(&pool, true).unwrap_err(),
            "UDF poolSize cannot be null for executable_pool"
        );

        assert_eq!(
            validate_definition(&definition("executable", false), true).unwrap_err(),
            "UDF definition requires functionName"
        );
        assert_eq!(
            validate_definition(&json!([]), false).unwrap_err(),
            "UDF definition must be a JSON object"
        );
    }

    #[test]
    fn python_entries_are_sorted_and_skip_definition_hidden_and_cache_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        write(&dir.join("udf.json"), "{}");
        write(&dir.join("main.py"), "print(1)\n");
        write(&dir.join(".env"), "SECRET=1\n");
        write(&dir.join("lib/helper.py"), "x = 1\n");
        write(&dir.join("lib/__pycache__/helper.pyc"), "");
        write(&dir.join("data/.hidden"), "");
        write(&dir.join("bin/tool"), "#!/bin/sh\n");
        std::fs::set_permissions(dir.join("bin/tool"), std::fs::Permissions::from_mode(0o755))
            .unwrap();

        let entries = collect_source_entries(dir, UdfRuntimeKind::Python311).unwrap();
        assert_eq!(
            listed(&entries),
            vec![
                ("bin".to_string(), true),
                ("bin/tool".to_string(), false),
                ("data".to_string(), true),
                ("lib".to_string(), true),
                ("lib/helper.py".to_string(), false),
                ("main.py".to_string(), false),
            ]
        );
        let tool = entries
            .iter()
            .find(|entry| entry.relative == Path::new("bin/tool"))
            .unwrap();
        assert_eq!(tool.mode & 0o111, 0o111);
        assert_eq!(tool.absolute, dir.join("bin/tool"));
    }

    #[test]
    fn python_entries_reject_symlinks_empty_dirs_and_a_missing_entrypoint() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("udf");
        std::fs::create_dir(&dir).unwrap();

        let error = collect_source_entries(&dir, UdfRuntimeKind::Python311).unwrap_err();
        assert!(
            error.to_string().ends_with("contains no files to deploy"),
            "{error}"
        );

        write(&dir.join("lib/helper.py"), "");
        let error = collect_source_entries(&dir, UdfRuntimeKind::Python311).unwrap_err();
        assert!(error.to_string().contains("is missing main.py"), "{error}");

        write(&dir.join("main.py"), "");
        std::os::unix::fs::symlink(dir.join("lib/helper.py"), dir.join("link.py")).unwrap();
        let error = collect_source_entries(&dir, UdfRuntimeKind::Python311).unwrap_err();
        assert!(
            error
                .to_string()
                .ends_with("contains a symbolic link at link.py"),
            "{error}"
        );

        let missing = tmp.path().join("missing");
        assert!(
            collect_source_entries(&missing, UdfRuntimeKind::Native)
                .unwrap_err()
                .to_string()
                .ends_with("is not a directory")
        );
    }

    #[test]
    fn native_entries_are_only_the_architecture_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("udf");
        write(&dir.join("udf.json"), "{}");
        write(&dir.join("Cargo.toml"), "[package]\n");
        write(&dir.join("src/main.rs"), "fn main() {}\n");
        write(&dir.join("amd64/main"), "binary\n");
        write(&dir.join("amd64/model.bin"), "weights\n");
        write(&dir.join("amd64/.cache"), "");
        write(&dir.join("arm64/main"), "binary\n");
        write(&dir.join("arm64/__pycache__/x.pyc"), "");
        // Entries outside the architecture directories are not inspected,
        // so even a symlink there is irrelevant.
        std::os::unix::fs::symlink(dir.join("Cargo.toml"), dir.join("link")).unwrap();

        let entries = collect_source_entries(&dir, UdfRuntimeKind::Native).unwrap();
        assert_eq!(
            listed(&entries),
            vec![
                ("amd64".to_string(), true),
                ("amd64/main".to_string(), false),
                ("amd64/model.bin".to_string(), false),
                ("arm64".to_string(), true),
                ("arm64/main".to_string(), false),
            ]
        );
    }

    #[test]
    fn native_entries_require_both_binaries_and_reject_links_inside() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("udf");
        write(&dir.join("udf.json"), "{}");
        write(&dir.join("amd64/main"), "binary\n");
        let error = collect_source_entries(&dir, UdfRuntimeKind::Native).unwrap_err();
        assert!(
            error.to_string().ends_with(
                "is missing amd64/main or arm64/main, the binaries runtime native requires"
            ),
            "{error}"
        );

        std::fs::create_dir_all(dir.join("arm64")).unwrap();
        assert!(collect_source_entries(&dir, UdfRuntimeKind::Native).is_err());
        write(&dir.join("arm64/main"), "binary\n");
        assert!(collect_source_entries(&dir, UdfRuntimeKind::Native).is_ok());

        std::os::unix::fs::symlink(dir.join("amd64/main"), dir.join("arm64/extra")).unwrap();
        let error = collect_source_entries(&dir, UdfRuntimeKind::Native).unwrap_err();
        assert!(
            error
                .to_string()
                .ends_with("contains a symbolic link at arm64/extra"),
            "{error}"
        );
        std::fs::remove_file(dir.join("arm64/extra")).unwrap();

        std::fs::remove_dir_all(dir.join("arm64")).unwrap();
        std::os::unix::fs::symlink(dir.join("amd64"), dir.join("arm64")).unwrap();
        let error = collect_source_entries(&dir, UdfRuntimeKind::Native).unwrap_err();
        assert!(
            error
                .to_string()
                .ends_with("contains a symbolic link at arm64"),
            "{error}"
        );
    }

    #[test]
    fn native_entries_for_one_architecture_need_and_yield_only_that_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("native_fn");
        write(&dir.join("arm64/main"), "binary\n");

        let error =
            collect_source_entries_for(&dir, UdfRuntimeKind::Native, &["amd64"]).unwrap_err();
        assert!(
            error
                .to_string()
                .ends_with("is missing amd64/main, the binary runtime native runs on this host"),
            "{error}"
        );

        write(&dir.join("amd64/main"), "binary\n");
        let entries = collect_source_entries_for(&dir, UdfRuntimeKind::Native, &["amd64"]).unwrap();
        assert_eq!(
            listed(&entries),
            vec![
                ("amd64".to_string(), true),
                ("amd64/main".to_string(), false)
            ]
        );
        std::fs::remove_file(dir.join("amd64/main")).unwrap();
        assert!(collect_source_entries_for(&dir, UdfRuntimeKind::Native, &["arm64"]).is_ok());
    }
}
