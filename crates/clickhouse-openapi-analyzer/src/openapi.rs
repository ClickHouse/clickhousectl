use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::config::AnalyzerConfig;

const HTTP_METHODS: &[&str] = &[
    "get", "put", "post", "delete", "patch", "options", "head", "trace",
];

#[derive(Debug, Clone)]
pub(crate) struct OperationInfo {
    pub(crate) pointer: String,
    pub(crate) operation_id: String,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) summary: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PropertyInfo {
    pub(crate) pointer: String,
    pub(crate) required_non_nullable: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum EnumContext {
    NamedSchema {
        schema: String,
    },
    Property {
        schema: String,
        property: String,
        array_item: bool,
    },
    Parameter {
        operation_id: String,
        parameter: String,
    },
    Unknown,
}

#[derive(Debug, Clone)]
pub(crate) enum EnumValues {
    Strings(BTreeSet<String>),
    Numeric,
    Mixed,
}

#[derive(Debug, Clone)]
pub(crate) struct EnumConstraint {
    pub(crate) pointer: String,
    pub(crate) context: EnumContext,
    pub(crate) values: EnumValues,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct OpenApiInventory {
    pub(crate) operations: BTreeMap<String, OperationInfo>,
    pub(crate) schemas: BTreeMap<String, String>,
    pub(crate) properties: BTreeMap<(String, String), PropertyInfo>,
    pub(crate) referenced_schemas: BTreeMap<String, String>,
    pub(crate) beta_operations: BTreeMap<String, String>,
    pub(crate) deprecated_fields: BTreeMap<(String, String), String>,
    pub(crate) enum_constraints: Vec<EnumConstraint>,
}

impl OpenApiInventory {
    pub(crate) fn build(spec: &Value, config: &AnalyzerConfig) -> Result<Self, String> {
        let mut inventory = Self::default();
        inventory.collect_operations(spec)?;
        inventory.collect_schemas(spec, config)?;
        collect_refs(spec, &mut Vec::new(), &mut inventory.referenced_schemas);
        collect_enums(spec, spec, &mut Vec::new(), &mut inventory.enum_constraints);
        inventory
            .enum_constraints
            .sort_by(|left, right| left.pointer.cmp(&right.pointer));
        Ok(inventory)
    }

    fn collect_operations(&mut self, spec: &Value) -> Result<(), String> {
        let paths = spec
            .get("paths")
            .and_then(Value::as_object)
            .ok_or_else(|| "OpenAPI document has no paths object".to_string())?;
        for (path, path_item) in paths {
            let Some(path_object) = path_item.as_object() else {
                continue;
            };
            for (method, operation) in path_object {
                if !HTTP_METHODS.contains(&method.as_str()) {
                    continue;
                }
                let operation_id = operation
                    .get("operationId")
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("{method} {path} has no operationId"))?;
                let rust_name = camel_to_snake(operation_id);
                let pointer = json_pointer(&["paths".to_string(), path.clone(), method.clone()]);
                let badges = operation
                    .get("x-badges")
                    .and_then(Value::as_array)
                    .is_some_and(|badges| {
                        badges
                            .iter()
                            .any(|badge| badge.get("name").and_then(Value::as_str) == Some("Beta"))
                    });
                if badges {
                    self.beta_operations
                        .insert(rust_name.clone(), pointer.clone());
                }
                self.operations.insert(
                    rust_name,
                    OperationInfo {
                        pointer,
                        operation_id: operation_id.to_string(),
                        method: method.to_ascii_uppercase(),
                        path: path.clone(),
                        summary: operation
                            .get("summary")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    },
                );
            }
        }
        Ok(())
    }

    fn collect_schemas(&mut self, spec: &Value, config: &AnalyzerConfig) -> Result<(), String> {
        let schemas = spec
            .pointer("/components/schemas")
            .and_then(Value::as_object)
            .ok_or_else(|| "OpenAPI document has no components.schemas object".to_string())?;
        for (schema_name, schema) in schemas {
            let rust_name = pascalize(schema_name);
            let schema_pointer = json_pointer(&[
                "components".to_string(),
                "schemas".to_string(),
                schema_name.clone(),
            ]);
            self.schemas
                .insert(schema_name.clone(), schema_pointer.clone());
            let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
                continue;
            };
            let required = required_fields(schema_name, schema, config);
            for (property_name, property) in properties {
                let pointer = format!(
                    "{schema_pointer}/properties/{}",
                    escape_pointer(property_name)
                );
                self.properties.insert(
                    (schema_name.clone(), property_name.clone()),
                    PropertyInfo {
                        pointer: pointer.clone(),
                        required_non_nullable: required.contains(property_name),
                    },
                );
                if property.get("deprecated").and_then(Value::as_bool) == Some(true) {
                    self.deprecated_fields
                        .insert((rust_name.clone(), property_name.clone()), pointer);
                }
            }
        }
        Ok(())
    }
}

fn required_fields(schema_name: &str, schema: &Value, config: &AnalyzerConfig) -> BTreeSet<String> {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return BTreeSet::new();
    };
    if schema_name.contains("Patch") && schema_name.ends_with("Request") {
        return BTreeSet::new();
    }

    let mut required = if config.partial_required_schemas.contains(schema_name) {
        let mut values = required_array(schema);
        for (name, property) in properties {
            let description = property
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !description.starts_with("Optional") {
                values.insert(name.clone());
            }
        }
        values
    } else if schema.get("required").and_then(Value::as_array).is_some() {
        required_array(schema)
    } else {
        properties
            .iter()
            .filter_map(|(name, property)| {
                let description = property
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                (!description.starts_with("Optional")).then(|| name.clone())
            })
            .collect()
    };
    required.retain(|name| {
        properties
            .get(name)
            .is_some_and(|property| !is_nullable(property))
    });
    required
}

fn required_array(schema: &Value) -> BTreeSet<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn is_nullable(property: &Value) -> bool {
    if property
        .get("type")
        .and_then(Value::as_array)
        .is_some_and(|types| types.iter().any(|value| value.as_str() == Some("null")))
    {
        return true;
    }
    ["oneOf", "anyOf"].iter().any(|key| {
        property
            .get(*key)
            .and_then(Value::as_array)
            .is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value.get("type").and_then(Value::as_str) == Some("null"))
            })
    })
}

fn collect_refs(value: &Value, path: &mut Vec<String>, refs: &mut BTreeMap<String, String>) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str)
                && let Some(name) = reference.strip_prefix("#/components/schemas/")
            {
                refs.entry(name.to_string())
                    .or_insert_with(|| json_pointer(path));
            }
            for (key, child) in object {
                path.push(key.clone());
                collect_refs(child, path, refs);
                path.pop();
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                path.push(index.to_string());
                collect_refs(child, path, refs);
                path.pop();
            }
        }
        _ => {}
    }
}

fn collect_enums(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    match value {
        Value::Object(object) => {
            if let Some(values) = object.get("enum").and_then(Value::as_array) {
                let enum_values = if values.iter().all(Value::is_string) {
                    EnumValues::Strings(
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect(),
                    )
                } else if values.iter().all(Value::is_number) {
                    EnumValues::Numeric
                } else {
                    EnumValues::Mixed
                };
                output.push(EnumConstraint {
                    pointer: json_pointer(path),
                    context: enum_context(root, path),
                    values: enum_values,
                });
            }
            for (key, child) in object {
                path.push(key.clone());
                collect_enums(root, child, path, output);
                path.pop();
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                path.push(index.to_string());
                collect_enums(root, child, path, output);
                path.pop();
            }
        }
        _ => {}
    }
}

fn enum_context(root: &Value, path: &[String]) -> EnumContext {
    if path.len() == 3 && path[0] == "components" && path[1] == "schemas" {
        return EnumContext::NamedSchema {
            schema: path[2].clone(),
        };
    }
    if path.len() >= 5 && path[0] == "components" && path[1] == "schemas" && path[3] == "properties"
    {
        return EnumContext::Property {
            schema: path[2].clone(),
            property: path[4].clone(),
            array_item: path[5..].iter().any(|part| part == "items"),
        };
    }
    if path.len() > 3
        && path[0] == "components"
        && path[1] == "schemas"
        && path[3..]
            .iter()
            .any(|part| matches!(part.as_str(), "oneOf" | "anyOf" | "allOf"))
    {
        return EnumContext::NamedSchema {
            schema: path[2].clone(),
        };
    }
    if path.len() >= 6 && path[0] == "paths" && path[3] == "parameters" {
        let operation = root
            .get("paths")
            .and_then(|paths| paths.get(&path[1]))
            .and_then(|path_item| path_item.get(&path[2]));
        let index = path[4].parse::<usize>().ok();
        if let (Some(operation_id), Some(parameter)) = (
            operation
                .and_then(|value| value.get("operationId"))
                .and_then(Value::as_str),
            index
                .and_then(|value| operation?.get("parameters")?.get(value))
                .and_then(|value| value.get("name"))
                .and_then(Value::as_str),
        ) {
            return EnumContext::Parameter {
                operation_id: camel_to_snake(operation_id),
                parameter: camel_to_snake(parameter),
            };
        }
    }
    EnumContext::Unknown
}

pub(crate) fn json_pointer(path: &[String]) -> String {
    if path.is_empty() {
        return String::new();
    }
    format!(
        "/{}",
        path.iter()
            .map(|part| escape_pointer(part))
            .collect::<Vec<_>>()
            .join("/")
    )
}

fn escape_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

pub(crate) fn camel_to_snake(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut previous = None;
    for character in value.chars() {
        if character.is_ascii_uppercase() {
            if previous.is_some_and(|previous: char| {
                previous.is_ascii_lowercase() || previous.is_ascii_digit()
            }) {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character);
        }
        previous = Some(character);
    }
    output
}

pub(crate) fn pascalize(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut uppercase_next = true;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if uppercase_next {
                output.push(character.to_ascii_uppercase());
                uppercase_next = false;
            } else {
                output.push(character);
            }
        } else {
            uppercase_next = true;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventories_nested_and_parameter_enums_with_stable_pointers() {
        let spec = serde_json::json!({
            "paths": {
                "/widgets": {"get": {
                    "operationId": "listWidgets",
                    "parameters": [{"name": "sortOrder", "schema": {"enum": ["asc", "desc"]}}]
                }}
            },
            "components": {"schemas": {
                "Widget": {"properties": {
                    "states": {"type": "array", "items": {"enum": ["ready"]}}
                }}
            }}
        });
        let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default()).unwrap();
        assert_eq!(inventory.enum_constraints.len(), 2);
        assert!(inventory.enum_constraints.iter().any(|constraint| {
            constraint.pointer == "/paths/~1widgets/get/parameters/0/schema"
        }));
        assert!(matches!(
            inventory.enum_constraints[1].context,
            EnumContext::Parameter { .. }
        ));
    }
}
