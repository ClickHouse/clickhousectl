//! Executable UDF definitions shared by `local udf` and `cloud udf`.
//!
//! A UDF is a directory holding `udf.json`, the definition shape the Cloud API
//! accepts, next to the function's files. This module owns the target-neutral
//! parts: the file-name conventions and the `Value`-level validation both
//! targets apply before their own typed deserialization. It knows nothing
//! about `CloudError` or the local `Error`; each target converts.

use serde_json::Value;

/// Definition file inside a UDF directory.
pub const DEFINITION_FILE: &str = "udf.json";
/// Entrypoint the `python3.11` runtime requires at the root of the sources.
pub const PYTHON_ENTRYPOINT: &str = "main.py";
/// Entrypoint of the `native` runtime: `main` inside each architecture
/// directory (`amd64/`, `arm64/`), both locally and on Cloud.
pub const NATIVE_ENTRYPOINT: &str = "main";

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
}
