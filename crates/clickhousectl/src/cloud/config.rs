use crate::cloud::client::{CloudError, Result as CloudResult};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::io::Read as _;

/// Read a JSON request body from a file or stdin.
pub(crate) fn read_config_value(config_file: &str) -> CloudResult<Value> {
    let contents = if config_file == "-" {
        let mut contents = String::new();
        std::io::stdin()
            .read_to_string(&mut contents)
            .map_err(|error| {
                CloudError::new(format!("failed to read config from stdin: {error}"))
            })?;
        contents
    } else {
        std::fs::read_to_string(config_file).map_err(|error| {
            CloudError::new(format!("failed to read config file {config_file}: {error}"))
        })?
    };

    serde_json::from_str(&contents).map_err(|error| {
        CloudError::new(format!(
            "failed to parse config {config_file} as JSON: {error}"
        ))
    })
}

/// Strictly deserialize a raw request body into a library request type.
///
/// Published response models intentionally ignore unknown fields. CLI request
/// input needs the opposite policy, so this wrapper reports every field serde
/// ignored, including nested fields, without changing the library's models.
pub(crate) fn deserialize_strict_config<T>(value: Value, source: &str) -> CloudResult<T>
where
    T: DeserializeOwned,
{
    let encoded = serde_json::to_vec(&value)?;
    let mut deserializer = serde_json::Deserializer::from_slice(&encoded);
    let mut ignored = Vec::new();
    let request = serde_ignored::deserialize(&mut deserializer, |path| {
        ignored.push(path.to_string());
    })
    .map_err(|error| {
        CloudError::new(format!("invalid request body in config {source}: {error}"))
    })?;

    if ignored.is_empty() {
        Ok(request)
    } else {
        Err(CloudError::new(format!(
            "invalid request body in config {source}: unknown field{} {}",
            if ignored.len() == 1 { "" } else { "s" },
            ignored
                .iter()
                .map(|path| format!("`{path}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

pub(crate) fn read_typed_config<T>(config_file: &str) -> CloudResult<T>
where
    T: DeserializeOwned,
{
    deserialize_strict_config(read_config_value(config_file)?, config_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Nested {
        enabled: Option<bool>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Example {
        name: String,
        nested: Option<Nested>,
    }

    #[test]
    fn strict_config_rejects_ignored_nested_fields_including_null() {
        let parsed: Example = deserialize_strict_config(
            serde_json::json!({"name": "ok", "nested": {"enabled": null}}),
            "test",
        )
        .unwrap();
        assert_eq!(
            parsed,
            Example {
                name: "ok".into(),
                nested: Some(Nested { enabled: None })
            }
        );

        let error = deserialize_strict_config::<Example>(
            serde_json::json!({"name": "bad", "nested": {"enabeld": null}}),
            "test",
        )
        .unwrap_err();
        assert!(error.message.contains("enabeld"), "{error}");
    }
}
