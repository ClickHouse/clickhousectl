use std::collections::{BTreeSet, HashMap};

use serde_json::Value;

const SPEC_JSON: &str = include_str!("../clickhouse_cloud_openapi.json");
const CLIENT_RS: &str = include_str!("../src/client.rs");
const MODELS_RS: &str = include_str!("../src/models.rs");
const LIVE_SPEC_URL: &str = "https://api.clickhouse.cloud/v1";

#[test]
fn client_methods_cover_every_openapi_operation() {
    assert_client_operation_coverage(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[test]
fn models_cover_every_openapi_component_schema() {
    assert_model_schema_coverage(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[test]
fn referenced_component_schemas_have_generated_model_types() {
    assert_ref_schema_coverage(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn client_methods_cover_live_openapi_operations() {
    let spec = load_live_spec().await;
    assert_client_operation_coverage(&spec);
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn models_cover_live_openapi_component_schemas() {
    let spec = load_live_spec().await;
    assert_model_schema_coverage(&spec);
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn referenced_live_component_schemas_have_generated_model_types() {
    let spec = load_live_spec().await;
    assert_ref_schema_coverage(&spec);
}

fn spec_operation_ids(spec: &Value) -> BTreeSet<String> {
    let mut operation_ids = BTreeSet::new();

    for path_item in spec["paths"].as_object().unwrap().values() {
        for (method, operation) in path_item.as_object().unwrap() {
            if matches!(
                method.as_str(),
                "get" | "put" | "post" | "delete" | "patch" | "options" | "head" | "trace"
            ) {
                operation_ids.insert(camel_to_snake(
                    operation["operationId"].as_str().unwrap(),
                ));
            }
        }
    }

    operation_ids
}

fn spec_schema_type_names(spec: &Value) -> BTreeSet<String> {
    spec["components"]["schemas"]
        .as_object()
        .unwrap()
        .keys()
        .map(|schema_name| pascalize_identifier(schema_name))
        .collect()
}

fn assert_client_operation_coverage(spec: &Value) {
    let spec_operations = spec_operation_ids(spec);
    let client_methods = public_items(CLIENT_RS, "pub async fn ");

    let missing: Vec<_> = spec_operations.difference(&client_methods).cloned().collect();
    let extras: Vec<_> = client_methods.difference(&spec_operations).cloned().collect();

    assert!(
        missing.is_empty() && extras.is_empty(),
        "OpenAPI operation coverage mismatch.\nMissing client methods: {:?}\nExtra client methods: {:?}",
        missing,
        extras
    );
}

fn assert_model_schema_coverage(spec: &Value) {
    let spec_schemas = spec_schema_type_names(spec);
    let model_types = model_type_names();

    let missing: Vec<_> = spec_schemas.difference(&model_types).cloned().collect();

    assert!(
        missing.is_empty(),
        "OpenAPI schema coverage mismatch.\nMissing model types: {:?}",
        missing
    );
}

fn assert_ref_schema_coverage(spec: &Value) {
    let schema_definitions = spec["components"]["schemas"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let referenced_schemas = collect_schema_refs(spec);
    let model_types = model_type_names();

    let missing_definitions: Vec<_> = referenced_schemas
        .iter()
        .filter(|schema_name| !schema_definitions.contains(*schema_name))
        .cloned()
        .collect();
    let missing_models: Vec<_> = referenced_schemas
        .iter()
        .map(|schema_name| pascalize_identifier(schema_name))
        .filter(|type_name| !model_types.contains(type_name))
        .collect();

    assert!(
        missing_definitions.is_empty() && missing_models.is_empty(),
        "Referenced component schema coverage mismatch.\nMissing schema definitions: {:?}\nMissing model types: {:?}",
        missing_definitions,
        missing_models
    );
}

fn model_type_names() -> BTreeSet<String> {
    public_items(MODELS_RS, "pub struct ")
        .into_iter()
        .chain(public_items(MODELS_RS, "pub enum "))
        .chain(public_items(MODELS_RS, "pub type "))
        .collect::<BTreeSet<_>>()
}

fn public_items(source: &str, needle: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix(needle))
        .filter_map(identifier_prefix)
        .map(str::to_string)
        .collect()
}

fn identifier_prefix(value: &str) -> Option<&str> {
    let end = value
        .char_indices()
        .find(|(_, ch)| !(ch.is_ascii_alphanumeric() || *ch == '_'))
        .map(|(idx, _)| idx)
        .unwrap_or(value.len());

    if end == 0 {
        None
    } else {
        Some(&value[..end])
    }
}

fn camel_to_snake(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut previous: Option<char> = None;

    for ch in value.chars() {
        if ch.is_ascii_uppercase() {
            if matches!(previous, Some(prev) if prev.is_ascii_lowercase() || prev.is_ascii_digit())
            {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
        previous = Some(ch);
    }

    output
}

fn collect_schema_refs(value: &Value) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    collect_schema_refs_inner(value, &mut refs);
    refs
}

fn collect_schema_refs_inner(value: &Value, refs: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                if let Some(schema_name) = reference.strip_prefix("#/components/schemas/") {
                    refs.insert(schema_name.to_string());
                }
            }

            for child in map.values() {
                collect_schema_refs_inner(child, refs);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_schema_refs_inner(item, refs);
            }
        }
        _ => {}
    }
}

async fn load_live_spec() -> Value {
    let response = reqwest::Client::new()
        .get(std::env::var("CLICKHOUSE_OPENAPI_SPEC_URL").unwrap_or_else(|_| LIVE_SPEC_URL.to_string()))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    response.json().await.unwrap()
}

fn pascalize_identifier(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut uppercase_next = true;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            if uppercase_next {
                output.push(ch.to_ascii_uppercase());
                uppercase_next = false;
            } else {
                output.push(ch);
            }
        } else {
            uppercase_next = true;
        }
    }

    output
}

// ---------------------------------------------------------------------------
// Field-level optionality validation
// ---------------------------------------------------------------------------

#[test]
fn field_optionality_matches_spec() {
    assert_field_optionality(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn field_optionality_matches_live_spec() {
    let spec = load_live_spec().await;
    assert_field_optionality(&spec);
}

fn assert_field_optionality(spec: &Value) {
    let schemas = spec["components"]["schemas"].as_object().unwrap();
    let model_fields = parse_model_fields(MODELS_RS);

    let mut mismatches = Vec::new();

    for (spec_name, schema) in schemas {
        let rust_name = pascalize_identifier(spec_name);
        let fields = match model_fields.get(&rust_name) {
            Some(f) => f,
            None => continue, // Schema not in models — covered by other tests
        };

        let props = match schema.get("properties").and_then(Value::as_object) {
            Some(p) => p,
            None => continue,
        };

        let required_fields = resolve_required_fields(spec_name, schema);

        for (prop_name, _prop_schema) in props {
            let is_required = required_fields.contains(prop_name.as_str());
            let field_info = match fields.get(prop_name.as_str()) {
                Some(f) => f,
                None => continue, // Field not in struct — could add a check later
            };

            if is_required && field_info.is_option {
                mismatches.push(format!(
                    "{}.{} should be required (T) but is Option<T>",
                    rust_name, prop_name
                ));
            } else if !is_required && !field_info.is_option {
                mismatches.push(format!(
                    "{}.{} should be optional (Option<T>) but is T",
                    rust_name, prop_name
                ));
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "Field optionality mismatches ({} total):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Determine which fields in a schema are required AND non-nullable.
///
/// Resolution strategy:
/// 1. PATCH request schemas (name contains "Patch" and ends with "Request") → all optional
/// 2. If schema has a `required` array → use it
/// 3. Otherwise → fields whose description does NOT start with "Optional" are required
///
/// Nullable fields (type: ["string", "null"] or oneOf/anyOf with null) are excluded.
fn resolve_required_fields<'a>(schema_name: &str, schema: &'a Value) -> BTreeSet<&'a str> {
    let props = match schema.get("properties").and_then(Value::as_object) {
        Some(p) => p,
        None => return BTreeSet::new(),
    };

    // PATCH schemas are always all-optional
    if schema_name.contains("Patch") && schema_name.ends_with("Request") {
        return BTreeSet::new();
    }

    let required_names: BTreeSet<&str> = if let Some(required) = schema.get("required").and_then(Value::as_array) {
        required.iter().filter_map(Value::as_str).collect()
    } else {
        // Description heuristic for legacy schemas
        props
            .iter()
            .filter(|(_, prop)| {
                let desc = prop
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                !desc.starts_with("Optional")
            })
            .map(|(name, _)| name.as_str())
            .collect()
    };

    // Exclude nullable fields
    required_names
        .into_iter()
        .filter(|name| {
            if let Some(prop) = props.get(*name) {
                !is_field_nullable(prop)
            } else {
                false
            }
        })
        .collect()
}

fn is_field_nullable(prop: &Value) -> bool {
    // type: ["string", "null"]
    if let Some(types) = prop.get("type").and_then(Value::as_array) {
        if types.iter().any(|t| t.as_str() == Some("null")) {
            return true;
        }
    }
    // oneOf/anyOf with a null variant
    for key in &["oneOf", "anyOf"] {
        if let Some(variants) = prop.get(*key).and_then(Value::as_array) {
            if variants.iter().any(|v| v.get("type").and_then(Value::as_str) == Some("null")) {
                return true;
            }
        }
    }
    false
}

struct FieldInfo {
    is_option: bool,
}

/// Parse models.rs to extract struct fields with their spec names and optionality.
///
/// Returns: { RustStructName: { specFieldName: FieldInfo } }
fn parse_model_fields(source: &str) -> HashMap<String, HashMap<String, FieldInfo>> {
    let mut result: HashMap<String, HashMap<String, FieldInfo>> = HashMap::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim_start();

        // Detect struct start
        if let Some(rest) = line.strip_prefix("pub struct ") {
            if let Some(struct_name) = identifier_prefix(rest) {
                let struct_name = struct_name.to_string();
                i += 1;
                let mut fields: HashMap<String, FieldInfo> = HashMap::new();
                let mut pending_rename: Option<String> = None;

                while i < lines.len() {
                    let line = lines[i].trim();

                    if line == "}" {
                        break;
                    }

                    // Extract rename from serde attribute
                    if line.starts_with("#[serde(") {
                        if let Some(rename) = extract_serde_rename(line) {
                            pending_rename = Some(rename.to_string());
                        }
                    }

                    // Extract field definition
                    if let Some(rest) = line.strip_prefix("pub ") {
                        if let Some(colon_pos) = rest.find(':') {
                            let rust_field_name = rest[..colon_pos].trim();
                            let type_str = rest[colon_pos + 1..].trim().trim_end_matches(',');
                            let is_option = type_str.starts_with("Option<");

                            // Use rename as spec name, or fall back to rust field name
                            // Strip r# prefix from raw identifiers (e.g., r#type -> type)
                            let spec_name = pending_rename.take().unwrap_or_else(|| {
                                rust_field_name
                                    .strip_prefix("r#")
                                    .unwrap_or(rust_field_name)
                                    .to_string()
                            });

                            fields.insert(spec_name, FieldInfo { is_option });
                        }
                    }

                    i += 1;
                }

                result.insert(struct_name, fields);
            }
        }

        i += 1;
    }

    result
}

fn extract_serde_rename(serde_line: &str) -> Option<&str> {
    let start = serde_line.find("rename = \"")?;
    let value_start = start + "rename = \"".len();
    let end = serde_line[value_start..].find('"')? + value_start;
    Some(&serde_line[value_start..end])
}
