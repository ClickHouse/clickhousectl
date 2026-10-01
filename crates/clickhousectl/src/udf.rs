//! Executable UDF definitions shared by `local udf` and `cloud udf`.
//!
//! A UDF is a directory holding `udf.json`, the definition shape the Cloud API
//! accepts, next to the function's files. This module owns the target-neutral
//! parts: the file-name conventions, the `Value`-level validation both targets
//! apply before their own typed deserialization, and the deterministic source
//! walk shared by the Cloud archive builder and the local copier. It knows
//! nothing about `CloudError` or the local `Error`; each target converts.

use serde_json::Value;
use std::fmt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Definition file inside a UDF directory.
pub const DEFINITION_FILE: &str = "udf.json";
/// Entrypoint the `python3.11` runtime requires at the root of the sources.
pub const PYTHON_ENTRYPOINT: &str = "main.py";
/// Default entrypoint used for the `native` runtime on local servers.
pub const NATIVE_ENTRYPOINT: &str = "main";
/// Directory names skipped at any depth when collecting sources.
pub const EXCLUDED_DIRS: &[&str] = &["__pycache__"];

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
        return Err("Omit uploadId; --artifact creates a fresh upload session".into());
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

/// Remove `functionName` from a definition, as version-create requests
/// carry the name in the URL instead. Returns the removed name when it was a
/// string.
pub fn strip_function_name(value: &mut Value) -> Option<String> {
    value
        .as_object_mut()?
        .remove("functionName")
        .and_then(|name| name.as_str().map(str::to_owned))
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
/// that contain them. `udf.json` at the root, hidden entries and
/// [`EXCLUDED_DIRS`] are skipped. The result must contain at least one file,
/// and `main.py` at the root for the `python3.11` runtime.
pub fn collect_source_entries(
    dir: &Path,
    runtime: UdfRuntimeKind,
) -> Result<Vec<SourceEntry>, UdfInputError> {
    if !dir.is_dir() {
        return Err(invalid(dir, "is not a directory"));
    }
    let mut entries = Vec::new();
    walk(dir, dir, Path::new(""), 0, &mut entries)?;
    if !entries.iter().any(|entry| !entry.is_dir) {
        return Err(invalid(dir, "contains no files to deploy"));
    }
    if runtime == UdfRuntimeKind::Python311
        && !entries
            .iter()
            .any(|entry| !entry.is_dir && entry.relative == Path::new(PYTHON_ENTRYPOINT))
    {
        return Err(invalid(
            dir,
            format!(
                "is missing {PYTHON_ENTRYPOINT}, the entrypoint required by runtime python3.11"
            ),
        ));
    }
    Ok(entries)
}

fn walk(
    root: &Path,
    current: &Path,
    prefix: &Path,
    depth: usize,
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
        if name_text.starts_with('.') || (depth == 0 && name_text == DEFINITION_FILE) {
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
            walk(root, &absolute, &relative, depth + 1, out)?;
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
                "Omit uploadId; --artifact creates a fresh upload session",
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
    fn strip_function_name_removes_and_returns_the_name() {
        let mut value = definition("executable", true);
        assert_eq!(strip_function_name(&mut value).as_deref(), Some("my_udf"));
        assert!(value.get("functionName").is_none());
        assert_eq!(strip_function_name(&mut value), None);
        assert_eq!(strip_function_name(&mut json!(1)), None);
    }

    #[test]
    fn collect_source_entries_is_sorted_and_skips_definition_hidden_and_cache_entries() {
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
        let listed: Vec<(String, bool)> = entries
            .iter()
            .map(|entry| (entry.relative.display().to_string(), entry.is_dir))
            .collect();
        assert_eq!(
            listed,
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
    fn collect_source_entries_rejects_symlinks_empty_dirs_and_missing_python_entrypoint() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("udf");
        std::fs::create_dir(&dir).unwrap();

        let error = collect_source_entries(&dir, UdfRuntimeKind::Native).unwrap_err();
        assert!(
            error.to_string().ends_with("contains no files to deploy"),
            "{error}"
        );

        write(&dir.join("lib/helper.py"), "");
        let error = collect_source_entries(&dir, UdfRuntimeKind::Python311).unwrap_err();
        assert!(error.to_string().contains("is missing main.py"), "{error}");
        assert!(collect_source_entries(&dir, UdfRuntimeKind::Native).is_ok());

        std::os::unix::fs::symlink(dir.join("lib/helper.py"), dir.join("link.py")).unwrap();
        let error = collect_source_entries(&dir, UdfRuntimeKind::Native).unwrap_err();
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
}
