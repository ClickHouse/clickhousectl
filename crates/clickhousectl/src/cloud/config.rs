use crate::cloud::client::{CloudError, Result as CloudResult};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::io::Read as _;

/// User-facing label for a file-or-stdin configuration source.
pub(crate) fn config_source_label(config_file: &str) -> &str {
    if config_file == "-" {
        "stdin"
    } else {
        config_file
    }
}

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
            "failed to parse config {} as JSON: {error}",
            config_source_label(config_file)
        ))
    })
}

fn display_ignored_path(path: &serde_ignored::Path<'_>) -> String {
    fn collect(path: &serde_ignored::Path<'_>, parts: &mut Vec<String>) {
        match path {
            serde_ignored::Path::Root => {}
            serde_ignored::Path::Seq { parent, index } => {
                collect(parent, parts);
                parts.push(index.to_string());
            }
            serde_ignored::Path::Map { parent, key } => {
                collect(parent, parts);
                parts.push(key.clone());
            }
            // These variants are Rust deserializer traversal details, not
            // components of the JSON field path.
            serde_ignored::Path::Some { parent }
            | serde_ignored::Path::NewtypeStruct { parent }
            | serde_ignored::Path::NewtypeVariant { parent } => collect(parent, parts),
        }
    }

    let mut parts = Vec::new();
    collect(path, &mut parts);
    parts.join(".")
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
        ignored.push(display_ignored_path(&path));
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

    #[derive(Debug, Deserialize)]
    struct Empty {}

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
        assert!(
            error.message.contains("unknown field `nested.enabeld`"),
            "{error}"
        );
    }

    #[test]
    fn ignored_paths_preserve_map_keys_while_hiding_traversal_markers() {
        for key in ["?", "", "dotted.key"] {
            let error = deserialize_strict_config::<Empty>(serde_json::json!({key: true}), "test")
                .unwrap_err();
            assert!(
                error.message.contains(&format!("unknown field `{key}`")),
                "{error}"
            );
        }
    }

    #[test]
    fn stdin_has_a_clear_config_source_label() {
        assert_eq!(config_source_label("-"), "stdin");
        assert_eq!(config_source_label("patch.json"), "patch.json");
    }
}
