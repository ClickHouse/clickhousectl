use std::collections::BTreeSet;

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
