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
    pub(crate) schema_type: Option<String>,
}

/// A named schema whose dynamic keys have a declared value schema.
#[derive(Debug, Clone)]
pub(crate) struct AdditionalPropertiesInfo {
    pub(crate) pointer: String,
    pub(crate) value_schema: Value,
}

/// One hop in a property chain from a named schema down to an enum position.
///
/// `property` is the spec property name entered at this step. `array_item` is
/// set when an `items` traversal (array element) occurs after this property but
/// before the next step or the terminal enum — mirroring how the Rust field's
/// type must be unwrapped one array level to reach the next type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PropertyStep {
    pub(crate) property: String,
    pub(crate) array_item: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum EnumContext {
    NamedSchema {
        schema: String,
    },
    Property {
        schema: String,
        steps: Vec<PropertyStep>,
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
    Integers(BTreeSet<i64>),
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
    /// Pascalized Rust type names of every named spec schema. Used to
    /// distinguish a split `{Name}Response` Rust variant from a Rust type that
    /// models a spec schema literally named `{Name}Response`.
    pub(crate) rust_schema_names: BTreeSet<String>,
    pub(crate) properties: BTreeMap<(String, String), PropertyInfo>,
    pub(crate) additional_properties: BTreeMap<String, AdditionalPropertiesInfo>,
    pub(crate) referenced_schemas: BTreeMap<String, String>,
    /// Schemas transitively reachable from a request body or an operation
    /// parameter. Requiredness/optionality drift is checked only here.
    pub(crate) request_position_schemas: BTreeSet<String>,
    /// Schemas transitively reachable from an operation response. Optionality
    /// findings are suppressed here by policy (every response field is
    /// `Option<T>`); presence and enum-value checks still apply.
    pub(crate) response_position_schemas: BTreeSet<String>,
    pub(crate) beta_operations: BTreeMap<String, String>,
    /// Deprecated spec properties keyed by (spec schema name, property name).
    pub(crate) deprecated_fields: BTreeMap<(String, String), String>,
    pub(crate) enum_constraints: Vec<EnumConstraint>,
}

impl OpenApiInventory {
    pub(crate) fn build(spec: &Value, config: &AnalyzerConfig) -> Result<Self, String> {
        let mut inventory = Self::default();
        inventory.collect_operations(spec)?;
        inventory.collect_schemas(spec, config)?;
        inventory.collect_schema_positions(spec);
        collect_refs(spec, &mut Vec::new(), &mut inventory.referenced_schemas);
        collect_enums(spec, &mut inventory.enum_constraints);
        inventory
            .enum_constraints
            .sort_by(|left, right| left.pointer.cmp(&right.pointer));
        Ok(inventory)
    }

    /// Request-position semantics apply when the schema is reachable from a
    /// request body or parameter, and also when it is reachable from neither
    /// direction (e.g. defined but unused schemas), so unclassified schemas
    /// keep the historical strict checks.
    pub(crate) fn is_request_position(&self, schema_name: &str) -> bool {
        self.request_position_schemas.contains(schema_name)
            || !self.response_position_schemas.contains(schema_name)
    }

    pub(crate) fn is_response_position(&self, schema_name: &str) -> bool {
        self.response_position_schemas.contains(schema_name)
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
            self.rust_schema_names.insert(pascalize(schema_name));
            let schema_pointer = json_pointer(&[
                "components".to_string(),
                "schemas".to_string(),
                schema_name.clone(),
            ]);
            self.schemas
                .insert(schema_name.clone(), schema_pointer.clone());
            if let Some(additional) = schema
                .get("additionalProperties")
                .filter(|value| value.as_object().is_some_and(|object| !object.is_empty()))
            {
                self.additional_properties.insert(
                    schema_name.clone(),
                    AdditionalPropertiesInfo {
                        pointer: format!("{schema_pointer}/additionalProperties"),
                        value_schema: additional.clone(),
                    },
                );
            }
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
                        schema_type: schema_type(property).map(str::to_string),
                    },
                );
                if property.get("deprecated").and_then(Value::as_bool) == Some(true) {
                    self.deprecated_fields
                        .insert((schema_name.clone(), property_name.clone()), pointer);
                }
            }
        }
        Ok(())
    }

    /// Classifies every named schema as request-position and/or
    /// response-position by walking `$ref`s from each operation's parameters
    /// and request body (request roots) and responses (response roots), then
    /// taking the transitive closure through the schema reference graph. A
    /// schema can be in both positions. Reusable component objects
    /// (`#/components/responses/...` and friends) used in those positions are
    /// resolved as part of the walk, so a schema reachable only through one is
    /// still classified by direction.
    fn collect_schema_positions(&mut self, spec: &Value) {
        let mut request_roots = BTreeSet::new();
        let mut response_roots = BTreeSet::new();
        if let Some(paths) = spec.get("paths").and_then(Value::as_object) {
            for (path_name, path_item) in paths {
                if path_name.starts_with("x-") {
                    continue;
                }
                let Some(path_object) = path_item.as_object() else {
                    continue;
                };
                if let Some(parameters) = path_object.get("parameters") {
                    referenced_schema_names(spec, parameters, &mut request_roots);
                }
                for (method, operation) in path_object {
                    if !HTTP_METHODS.contains(&method.as_str()) {
                        continue;
                    }
                    if let Some(parameters) = operation.get("parameters") {
                        referenced_schema_names(spec, parameters, &mut request_roots);
                    }
                    if let Some(request_body) = operation.get("requestBody") {
                        referenced_schema_names(spec, request_body, &mut request_roots);
                    }
                    if let Some(responses) = operation.get("responses") {
                        referenced_schema_names(spec, responses, &mut response_roots);
                    }
                }
            }
        }

        let mut schema_refs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        if let Some(schemas) = spec
            .pointer("/components/schemas")
            .and_then(Value::as_object)
        {
            for (schema_name, schema) in schemas {
                let mut refs = BTreeSet::new();
                referenced_schema_names(spec, schema, &mut refs);
                schema_refs.insert(schema_name.clone(), refs);
            }
        }
        self.request_position_schemas = transitive_schema_closure(request_roots, &schema_refs);
        self.response_position_schemas = transitive_schema_closure(response_roots, &schema_refs);
    }
}

/// Collects the names of every `#/components/schemas/...` schema referenced
/// from `value`, resolving `$ref`s to reusable non-schema components against
/// `spec` and continuing the walk through them.
///
/// An operation may name a component object instead of inlining it —
/// `responses: {"200": {"$ref": "#/components/responses/Widget"}}`, or the
/// `#/components/requestBodies/...` and `#/components/parameters/...`
/// equivalents — and those components may in turn reference further
/// components. Without following them the schemas behind such an operation
/// would be classified in neither direction.
fn referenced_schema_names(spec: &Value, value: &Value, output: &mut BTreeSet<String>) {
    walk_schema_references(spec, value, output, &mut BTreeSet::new());
}

/// `visited` holds the component references already entered, so a reference
/// cycle between components terminates instead of recursing forever.
fn walk_schema_references(
    spec: &Value,
    value: &Value,
    output: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                    output.insert(name.to_string());
                } else if is_component_reference(reference)
                    && visited.insert(reference.to_string())
                    && let Some(component) = spec.pointer(&reference[1..])
                {
                    walk_schema_references(spec, component, output, visited);
                }
            }
            for child in object.values() {
                walk_schema_references(spec, child, output, visited);
            }
        }
        Value::Array(items) => {
            for child in items {
                walk_schema_references(spec, child, output, visited);
            }
        }
        _ => {}
    }
}

/// True for a local reference to a reusable component other than a schema —
/// a response, request body, parameter, header, and so on. Schema references
/// are recorded by name instead, because the transitive closure runs over the
/// schema reference graph.
fn is_component_reference(reference: &str) -> bool {
    reference.starts_with("#/components/") && !reference.starts_with("#/components/schemas/")
}

fn transitive_schema_closure(
    roots: BTreeSet<String>,
    schema_refs: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut stack: Vec<String> = roots.into_iter().collect();
    while let Some(name) = stack.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        if let Some(references) = schema_refs.get(&name) {
            for reference in references {
                if !seen.contains(reference) {
                    stack.push(reference.clone());
                }
            }
        }
    }
    seen
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

fn schema_type(property: &Value) -> Option<&str> {
    match property.get("type")? {
        Value::String(value) => Some(value),
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .find(|value| *value != "null"),
        _ => None,
    }
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

/// Discovers enum constraints, walking only genuine JSON Schema positions.
///
/// Traversal starts from schema roots (`/components/schemas/*` and every
/// OpenAPI-defined schema position under `paths`) and recurses only through
/// schema-composing keywords. Non-schema content such as `example`, `examples`,
/// `default`, or vendor extensions is never inspected, so an `enum` key that
/// merely appears inside an example payload is not mistaken for an enum
/// constraint.
fn collect_enums(root: &Value, output: &mut Vec<EnumConstraint>) {
    if let Some(schemas) = root
        .pointer("/components/schemas")
        .and_then(Value::as_object)
    {
        for (schema_name, schema) in schemas {
            let mut path = vec![
                "components".to_string(),
                "schemas".to_string(),
                schema_name.clone(),
            ];
            walk_schema(root, schema, &mut path, output);
        }
    }
    if let Some(paths) = root.get("paths") {
        let mut path = vec!["paths".to_string()];
        collect_path_schemas(root, paths, &mut path, output);
    }
}

/// Walks the OpenAPI structure under `paths`, entering only fields that the
/// specification defines as schema-bearing positions.
///
/// This must not be a recursive search for members named `schema`: example,
/// default, and extension payloads are arbitrary JSON and may legitimately
/// contain such a member without defining an OpenAPI schema.
fn collect_path_schemas(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(paths) = value.as_object() else {
        return;
    };
    for (path_name, path_item) in paths {
        if path_name.starts_with("x-") {
            continue;
        }
        path.push(path_name.clone());
        collect_path_item_schemas(root, path_item, path, output);
        path.pop();
    }
}

fn collect_path_item_schemas(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(path_item) = value.as_object() else {
        return;
    };
    if let Some(parameters) = path_item.get("parameters") {
        path.push("parameters".to_string());
        collect_parameter_schemas(root, parameters, path, output);
        path.pop();
    }
    for method in HTTP_METHODS {
        if let Some(operation) = path_item.get(*method) {
            path.push((*method).to_string());
            collect_operation_schemas(root, operation, path, output);
            path.pop();
        }
    }
}

fn collect_operation_schemas(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(operation) = value.as_object() else {
        return;
    };
    if let Some(parameters) = operation.get("parameters") {
        path.push("parameters".to_string());
        collect_parameter_schemas(root, parameters, path, output);
        path.pop();
    }
    if let Some(request_body) = operation.get("requestBody") {
        path.push("requestBody".to_string());
        collect_content_owner_schemas(root, request_body, path, output);
        path.pop();
    }
    if let Some(responses) = operation.get("responses").and_then(Value::as_object) {
        path.push("responses".to_string());
        for (status, response) in responses {
            if status.starts_with("x-") {
                continue;
            }
            path.push(status.clone());
            collect_response_schemas(root, response, path, output);
            path.pop();
        }
        path.pop();
    }
    if let Some(callbacks) = operation.get("callbacks").and_then(Value::as_object) {
        path.push("callbacks".to_string());
        for (callback_name, callback) in callbacks {
            path.push(callback_name.clone());
            collect_callback_schemas(root, callback, path, output);
            path.pop();
        }
        path.pop();
    }
}

fn collect_parameter_schemas(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(parameters) = value.as_array() else {
        return;
    };
    for (index, parameter) in parameters.iter().enumerate() {
        path.push(index.to_string());
        collect_schema_or_content(root, parameter, path, output);
        path.pop();
    }
}

fn collect_schema_or_content(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(object) = value.as_object() else {
        return;
    };
    if let Some(schema) = object.get("schema") {
        path.push("schema".to_string());
        walk_schema(root, schema, path, output);
        path.pop();
    }
    if let Some(content) = object.get("content") {
        path.push("content".to_string());
        collect_media_type_schemas(root, content, path, output);
        path.pop();
    }
}

fn collect_content_owner_schemas(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(object) = value.as_object() else {
        return;
    };
    if let Some(content) = object.get("content") {
        path.push("content".to_string());
        collect_media_type_schemas(root, content, path, output);
        path.pop();
    }
}

fn collect_response_schemas(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(response) = value.as_object() else {
        return;
    };
    if let Some(headers) = response.get("headers") {
        path.push("headers".to_string());
        collect_header_schemas(root, headers, path, output);
        path.pop();
    }
    collect_content_owner_schemas(root, value, path, output);
}

fn collect_header_schemas(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(headers) = value.as_object() else {
        return;
    };
    for (header_name, header) in headers {
        path.push(header_name.clone());
        collect_schema_or_content(root, header, path, output);
        path.pop();
    }
}

fn collect_media_type_schemas(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(content) = value.as_object() else {
        return;
    };
    for (media_type_name, media_type) in content {
        if media_type_name.starts_with("x-") {
            continue;
        }
        path.push(media_type_name.clone());
        collect_media_type_schema(root, media_type, path, output);
        path.pop();
    }
}

fn collect_media_type_schema(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(media_type) = value.as_object() else {
        return;
    };
    if let Some(schema) = media_type.get("schema") {
        path.push("schema".to_string());
        walk_schema(root, schema, path, output);
        path.pop();
    }
    if let Some(encodings) = media_type.get("encoding").and_then(Value::as_object) {
        path.push("encoding".to_string());
        for (property_name, encoding) in encodings {
            let Some(headers) = encoding.get("headers") else {
                continue;
            };
            path.push(property_name.clone());
            path.push("headers".to_string());
            collect_header_schemas(root, headers, path, output);
            path.pop();
            path.pop();
        }
        path.pop();
    }
}

fn collect_callback_schemas(
    root: &Value,
    value: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(callback) = value.as_object() else {
        return;
    };
    for (expression, path_item) in callback {
        if expression == "$ref" || expression.starts_with("x-") {
            continue;
        }
        path.push(expression.clone());
        collect_path_item_schemas(root, path_item, path, output);
        path.pop();
    }
}

/// Records an `enum` constraint at this schema position and recurses only
/// through keywords whose value is itself a schema (or a map/array of schemas).
fn walk_schema(
    root: &Value,
    schema: &Value,
    path: &mut Vec<String>,
    output: &mut Vec<EnumConstraint>,
) {
    let Some(object) = schema.as_object() else {
        return;
    };
    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        let enum_values = if values.iter().all(Value::is_string) {
            EnumValues::Strings(
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
            )
        } else if values
            .iter()
            .all(|value| integer_enum_value(value).is_some())
        {
            EnumValues::Integers(values.iter().filter_map(integer_enum_value).collect())
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
    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        path.push("properties".to_string());
        for (property_name, property) in properties {
            path.push(property_name.clone());
            walk_schema(root, property, path, output);
            path.pop();
        }
        path.pop();
    }
    if let Some(items) = object.get("items") {
        path.push("items".to_string());
        match items {
            Value::Array(schemas) => {
                for (index, child) in schemas.iter().enumerate() {
                    path.push(index.to_string());
                    walk_schema(root, child, path, output);
                    path.pop();
                }
            }
            _ => walk_schema(root, items, path, output),
        }
        path.pop();
    }
    if let Some(additional) = object.get("additionalProperties")
        && additional.is_object()
    {
        path.push("additionalProperties".to_string());
        walk_schema(root, additional, path, output);
        path.pop();
    }
    for keyword in ["oneOf", "anyOf", "allOf"] {
        if let Some(schemas) = object.get(keyword).and_then(Value::as_array) {
            path.push(keyword.to_string());
            for (index, child) in schemas.iter().enumerate() {
                path.push(index.to_string());
                walk_schema(root, child, path, output);
                path.pop();
            }
            path.pop();
        }
    }
    if let Some(not) = object.get("not")
        && not.is_object()
    {
        path.push("not".to_string());
        walk_schema(root, not, path, output);
        path.pop();
    }
}

fn integer_enum_value(value: &Value) -> Option<i64> {
    if let Some(value) = value.as_i64() {
        return Some(value);
    }

    let value = value.as_f64()?;
    const I64_MIN: f64 = -9_223_372_036_854_775_808.0;
    const I64_MAX_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
    (value.fract() == 0.0 && (I64_MIN..I64_MAX_EXCLUSIVE).contains(&value)).then_some(value as i64)
}

fn enum_context(root: &Value, path: &[String]) -> EnumContext {
    if path.len() == 3 && path[0] == "components" && path[1] == "schemas" {
        return EnumContext::NamedSchema {
            schema: path[2].clone(),
        };
    }
    if path.len() >= 5
        && path[0] == "components"
        && path[1] == "schemas"
        && let Some(properties_index) = path[3..]
            .iter()
            .position(|part| part == "properties")
            .map(|index| index + 3)
    {
        return EnumContext::Property {
            schema: path[2].clone(),
            steps: property_steps(&path[properties_index..]),
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

/// Derives the property chain from schema-relative pointer segments.
///
/// `segments` starts at the first `properties` keyword under the named schema
/// (i.e. `path[3..]`). Each `properties/<name>` pair becomes a [`PropertyStep`];
/// an `items` segment marks an array-element hop on the most recent step; and
/// composition keywords (`oneOf`/`anyOf`/`allOf`/`not`), `additionalProperties`,
/// and numeric indices are transparent so nested inline objects, arrays, and
/// unions all collapse to the ordered list of properties actually traversed.
fn property_steps(segments: &[String]) -> Vec<PropertyStep> {
    let mut steps: Vec<PropertyStep> = Vec::new();
    let mut index = 0;
    while index < segments.len() {
        match segments[index].as_str() {
            "properties" => {
                if let Some(name) = segments.get(index + 1) {
                    steps.push(PropertyStep {
                        property: name.clone(),
                        array_item: false,
                    });
                    index += 2;
                    continue;
                }
                index += 1;
            }
            "items" => {
                if let Some(step) = steps.last_mut() {
                    step.array_item = true;
                }
                index += 1;
            }
            _ => index += 1,
        }
    }
    steps
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

    #[test]
    fn distinguishes_integer_and_non_integer_numeric_enums() {
        let spec = serde_json::json!({
            "paths": {},
            "components": {"schemas": {
                "Widget": {"properties": {
                    "integer": {"enum": [-6.0, 0, 12.0]},
                    "numeric": {"enum": [0.5, 1.5]}
                }}
            }}
        });
        let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default()).unwrap();

        assert!(inventory.enum_constraints.iter().any(|constraint| {
            matches!(
                &constraint.values,
                EnumValues::Integers(values)
                    if values == &BTreeSet::from([-6, 0, 12])
            )
        }));
        assert!(
            inventory
                .enum_constraints
                .iter()
                .any(|constraint| matches!(constraint.values, EnumValues::Numeric))
        );
    }

    #[test]
    fn inventories_schema_enums_across_composition_positions() {
        let spec = serde_json::json!({
            "paths": {},
            "components": {"schemas": {
                "Named": {"enum": ["a", "b"]},
                "Widget": {"properties": {
                    "status": {"enum": ["on", "off"]},
                    "states": {"type": "array", "items": {"enum": ["ready"]}},
                    "mode": {"oneOf": [{"enum": ["fast"]}, {"type": "null"}]}
                }}
            }}
        });
        let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default()).unwrap();
        let pointers: BTreeSet<String> = inventory
            .enum_constraints
            .iter()
            .map(|constraint| constraint.pointer.clone())
            .collect();
        assert_eq!(
            pointers,
            BTreeSet::from([
                "/components/schemas/Named".to_string(),
                "/components/schemas/Widget/properties/status".to_string(),
                "/components/schemas/Widget/properties/states/items".to_string(),
                "/components/schemas/Widget/properties/mode/oneOf/0".to_string(),
            ])
        );
    }

    #[test]
    fn attributes_nested_property_enums_to_the_full_chain() {
        let spec = serde_json::json!({
            "paths": {},
            "components": {"schemas": {
                "Widget": {"properties": {
                    "foo": {"properties": {
                        "bar": {"enum": ["on", "off"]}
                    }},
                    "rows": {"type": "array", "items": {"properties": {
                        "cell": {"type": "array", "items": {"enum": ["ready"]}}
                    }}}
                }}
            }}
        });
        let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default()).unwrap();
        let by_pointer: BTreeMap<String, EnumContext> = inventory
            .enum_constraints
            .iter()
            .map(|constraint| (constraint.pointer.clone(), constraint.context.clone()))
            .collect();

        let nested = &by_pointer["/components/schemas/Widget/properties/foo/properties/bar"];
        match nested {
            EnumContext::Property { schema, steps } => {
                assert_eq!(schema, "Widget");
                assert_eq!(
                    steps,
                    &vec![
                        PropertyStep {
                            property: "foo".to_string(),
                            array_item: false,
                        },
                        PropertyStep {
                            property: "bar".to_string(),
                            array_item: false,
                        },
                    ]
                );
            }
            other => panic!("expected property context, got {other:?}"),
        }

        let under_items =
            &by_pointer["/components/schemas/Widget/properties/rows/items/properties/cell/items"];
        match under_items {
            EnumContext::Property { schema, steps } => {
                assert_eq!(schema, "Widget");
                assert_eq!(
                    steps,
                    &vec![
                        PropertyStep {
                            property: "rows".to_string(),
                            array_item: true,
                        },
                        PropertyStep {
                            property: "cell".to_string(),
                            array_item: true,
                        },
                    ]
                );
            }
            other => panic!("expected property context, got {other:?}"),
        }
    }

    #[test]
    fn attributes_property_enums_beneath_top_level_compositions() {
        let spec = serde_json::json!({
            "paths": {},
            "components": {"schemas": {
                "AllOfWidget": {"allOf": [{"properties": {
                    "status": {"enum": ["on"]}
                }}]},
                "OneOfWidget": {"oneOf": [{"properties": {
                    "states": {"type": "array", "items": {"enum": ["ready"]}}
                }}]},
                "AnyOfWidget": {"anyOf": [{"properties": {
                    "settings": {"properties": {
                        "mode": {"enum": ["fast"]}
                    }}
                }}]},
                "NamedChoice": {"allOf": [{"enum": ["a"]}]}
            }}
        });
        let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default()).unwrap();
        let by_pointer: BTreeMap<String, EnumContext> = inventory
            .enum_constraints
            .iter()
            .map(|constraint| (constraint.pointer.clone(), constraint.context.clone()))
            .collect();

        for (pointer, schema, steps) in [
            (
                "/components/schemas/AllOfWidget/allOf/0/properties/status",
                "AllOfWidget",
                vec![PropertyStep {
                    property: "status".to_string(),
                    array_item: false,
                }],
            ),
            (
                "/components/schemas/OneOfWidget/oneOf/0/properties/states/items",
                "OneOfWidget",
                vec![PropertyStep {
                    property: "states".to_string(),
                    array_item: true,
                }],
            ),
            (
                "/components/schemas/AnyOfWidget/anyOf/0/properties/settings/properties/mode",
                "AnyOfWidget",
                vec![
                    PropertyStep {
                        property: "settings".to_string(),
                        array_item: false,
                    },
                    PropertyStep {
                        property: "mode".to_string(),
                        array_item: false,
                    },
                ],
            ),
        ] {
            match &by_pointer[pointer] {
                EnumContext::Property {
                    schema: actual_schema,
                    steps: actual_steps,
                } => {
                    assert_eq!(actual_schema, schema);
                    assert_eq!(actual_steps, &steps);
                }
                other => panic!("expected property context for {pointer}, got {other:?}"),
            }
        }

        assert!(matches!(
            &by_pointer["/components/schemas/NamedChoice/allOf/0"],
            EnumContext::NamedSchema { schema } if schema == "NamedChoice"
        ));
    }

    #[test]
    fn keeps_single_property_chains_for_the_common_case() {
        let spec = serde_json::json!({
            "paths": {},
            "components": {"schemas": {
                "Widget": {"properties": {
                    "status": {"enum": ["on"]},
                    "states": {"type": "array", "items": {"enum": ["ready"]}}
                }}
            }}
        });
        let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default()).unwrap();
        let by_pointer: BTreeMap<String, EnumContext> = inventory
            .enum_constraints
            .iter()
            .map(|constraint| (constraint.pointer.clone(), constraint.context.clone()))
            .collect();

        match &by_pointer["/components/schemas/Widget/properties/status"] {
            EnumContext::Property { steps, .. } => assert_eq!(
                steps,
                &vec![PropertyStep {
                    property: "status".to_string(),
                    array_item: false,
                }]
            ),
            other => panic!("expected property context, got {other:?}"),
        }
        match &by_pointer["/components/schemas/Widget/properties/states/items"] {
            EnumContext::Property { steps, .. } => assert_eq!(
                steps,
                &vec![PropertyStep {
                    property: "states".to_string(),
                    array_item: true,
                }]
            ),
            other => panic!("expected property context, got {other:?}"),
        }
    }

    #[test]
    fn classifies_schema_positions_transitively_per_direction() {
        let spec = serde_json::json!({
            "paths": {
                "/widgets": {
                    "parameters": [{"name": "f", "schema": {"$ref": "#/components/schemas/PathParamFilter"}}],
                    "post": {
                        "operationId": "createWidget",
                        "parameters": [{"name": "sort", "schema": {"$ref": "#/components/schemas/SortOrder"}}],
                        "requestBody": {"content": {"application/json": {
                            "schema": {"$ref": "#/components/schemas/WidgetPostRequest"}
                        }}},
                        "responses": {"200": {"content": {"application/json": {
                            "schema": {"$ref": "#/components/schemas/WidgetPostResponse"}
                        }}}}
                    }
                }
            },
            "components": {"schemas": {
                "PathParamFilter": {"type": "string"},
                "SortOrder": {"type": "string"},
                "WidgetPostRequest": {"properties": {
                    "shared": {"$ref": "#/components/schemas/SharedNested"}
                }},
                "WidgetPostResponse": {"properties": {
                    "shared": {"$ref": "#/components/schemas/SharedNested"},
                    "detail": {"$ref": "#/components/schemas/ResponseOnlyDetail"}
                }},
                "SharedNested": {"properties": {
                    "leaf": {"$ref": "#/components/schemas/SharedLeaf"}
                }},
                "SharedLeaf": {"type": "string"},
                "ResponseOnlyDetail": {"type": "string"},
                "Unreferenced": {"type": "string"}
            }}
        });
        let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default()).unwrap();
        assert_eq!(
            inventory.request_position_schemas,
            BTreeSet::from([
                "PathParamFilter".to_string(),
                "SortOrder".to_string(),
                "WidgetPostRequest".to_string(),
                "SharedNested".to_string(),
                "SharedLeaf".to_string(),
            ])
        );
        assert_eq!(
            inventory.response_position_schemas,
            BTreeSet::from([
                "WidgetPostResponse".to_string(),
                "SharedNested".to_string(),
                "SharedLeaf".to_string(),
                "ResponseOnlyDetail".to_string(),
            ])
        );
        // Shared schemas are in both positions; unclassified schemas keep
        // request-position semantics.
        assert!(inventory.is_request_position("SharedNested"));
        assert!(inventory.is_response_position("SharedNested"));
        assert!(inventory.is_request_position("Unreferenced"));
        assert!(!inventory.is_response_position("Unreferenced"));
        assert!(!inventory.is_request_position("ResponseOnlyDetail"));
    }

    #[test]
    fn classifies_schemas_reached_through_reusable_components() {
        // An operation may name a reusable component object instead of
        // inlining it, and that component may reference another component. The
        // schemas behind either hop must still be classified by direction.
        let spec = serde_json::json!({
            "paths": {
                "/widgets": {
                    "post": {
                        "operationId": "createWidget",
                        "parameters": [{"$ref": "#/components/parameters/SortOrder"}],
                        "requestBody": {"$ref": "#/components/requestBodies/WidgetPost"},
                        "responses": {"200": {"$ref": "#/components/responses/Widget"}}
                    }
                }
            },
            "components": {
                "parameters": {"SortOrder": {
                    "name": "sort",
                    "schema": {"$ref": "#/components/schemas/SortOrder"}
                }},
                "requestBodies": {"WidgetPost": {"content": {"application/json": {
                    "schema": {"$ref": "#/components/schemas/WidgetPostRequest"}
                }}}},
                "responses": {"Widget": {
                    "headers": {"X-Widget-Mode": {"$ref": "#/components/headers/WidgetMode"}},
                    "content": {"application/json": {
                        "schema": {"$ref": "#/components/schemas/WidgetPostResponse"}
                    }}
                }},
                "headers": {"WidgetMode": {
                    "schema": {"$ref": "#/components/schemas/WidgetMode"}
                }},
                "schemas": {
                    "SortOrder": {"type": "string"},
                    "WidgetPostRequest": {"properties": {
                        "nested": {"$ref": "#/components/schemas/RequestNested"}
                    }},
                    "RequestNested": {"type": "string"},
                    "WidgetPostResponse": {"properties": {
                        "nested": {"$ref": "#/components/schemas/ResponseNested"}
                    }},
                    "ResponseNested": {"type": "string"},
                    "WidgetMode": {"type": "string"}
                }
            }
        });
        let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default()).unwrap();
        assert_eq!(
            inventory.request_position_schemas,
            BTreeSet::from([
                "SortOrder".to_string(),
                "WidgetPostRequest".to_string(),
                "RequestNested".to_string(),
            ])
        );
        assert_eq!(
            inventory.response_position_schemas,
            BTreeSet::from([
                "WidgetPostResponse".to_string(),
                "ResponseNested".to_string(),
                "WidgetMode".to_string(),
            ])
        );
        assert!(!inventory.is_request_position("WidgetPostResponse"));
    }

    #[test]
    fn terminates_on_a_component_reference_cycle() {
        // Cyclic component references are malformed, but the walk must not
        // recurse forever on them, and schemas seen on the way still classify.
        let spec = serde_json::json!({
            "paths": {
                "/widgets": {
                    "get": {
                        "operationId": "getWidget",
                        "responses": {"200": {"$ref": "#/components/responses/First"}}
                    }
                }
            },
            "components": {
                "responses": {
                    "First": {
                        "content": {"application/json": {
                            "schema": {"$ref": "#/components/schemas/Widget"}
                        }},
                        "x-next": {"$ref": "#/components/responses/Second"},
                        "x-self": {"$ref": "#/components/responses/SelfReferential"}
                    },
                    "Second": {"x-next": {"$ref": "#/components/responses/First"}},
                    "SelfReferential": {"x-next": {"$ref": "#/components/responses/SelfReferential"}}
                },
                "schemas": {"Widget": {"type": "string"}}
            }
        });
        let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default()).unwrap();
        assert_eq!(
            inventory.response_position_schemas,
            BTreeSet::from(["Widget".to_string()])
        );
    }

    #[test]
    fn inventories_only_openapi_schema_positions_under_paths() {
        // Objects named `schema` inside example/default payloads or vendor
        // extensions are ordinary JSON data, not schema constraints. Genuine
        // path-level, parameter, request, response, and header schemas must
        // still be inventoried.
        let spec = serde_json::json!({
            "paths": {
                "x-vendor": {
                    "schema": {"enum": ["extension-path"]}
                },
                "/widgets": {
                    "parameters": [{
                        "name": "apiVersion",
                        "schema": {"enum": ["v1"]}
                    }],
                    "post": {
                        "operationId": "createWidget",
                        "parameters": [
                            {
                                "name": "sortOrder",
                                "example": {
                                    "schema": {"enum": ["example-schema"]}
                                },
                                "x-vendor": {
                                    "schema": {"enum": ["extension-schema"]}
                                },
                                "schema": {
                                    "enum": ["asc", "desc"],
                                    "default": {
                                        "schema": {"enum": ["default-schema"]}
                                    }
                                }
                            },
                            {
                                "name": "filter",
                                "content": {
                                    "application/json": {
                                        "schema": {"enum": ["active"]}
                                    }
                                }
                            }
                        ],
                        "requestBody": {
                            "x-vendor": {
                                "schema": {"enum": ["extension-request"]}
                            },
                            "content": {
                                "application/json": {
                                    "example": {
                                        "schema": {"enum": ["example-request"]}
                                    },
                                    "schema": {"enum": ["create"]}
                                }
                            }
                        },
                        "responses": {
                            "200": {
                                "headers": {
                                    "X-Widget-Mode": {
                                        "schema": {"enum": ["full", "compact"]}
                                    }
                                },
                                "content": {
                                    "application/json": {
                                        "examples": {
                                            "example": {"value": {
                                                "schema": {"enum": ["example-response"]}
                                            }}
                                        },
                                        "schema": {"enum": ["created"]}
                                    }
                                }
                            },
                            "x-vendor": {
                                "content": {"application/json": {
                                    "schema": {"enum": ["extension-response"]}
                                }}
                            }
                        }
                    }
                }
            },
            "components": {"schemas": {
                "Widget": {
                    "example": {"enum": ["example-value"]},
                    "default": {"enum": ["default-value"]},
                    "x-vendor": {"enum": ["extension-value"]},
                    "properties": {
                        "status": {
                            "enum": ["on", "off"],
                            "example": {"enum": ["example-only"]},
                            "default": {"enum": ["default-only"]}
                        }
                    }
                }
            }}
        });
        let inventory = OpenApiInventory::build(&spec, &AnalyzerConfig::default()).unwrap();
        let pointers: BTreeSet<String> = inventory
            .enum_constraints
            .iter()
            .map(|constraint| constraint.pointer.clone())
            .collect();
        assert_eq!(
            pointers,
            BTreeSet::from([
                "/components/schemas/Widget/properties/status".to_string(),
                "/paths/~1widgets/parameters/0/schema".to_string(),
                "/paths/~1widgets/post/parameters/0/schema".to_string(),
                "/paths/~1widgets/post/parameters/1/content/application~1json/schema".to_string(),
                "/paths/~1widgets/post/requestBody/content/application~1json/schema".to_string(),
                "/paths/~1widgets/post/responses/200/content/application~1json/schema".to_string(),
                "/paths/~1widgets/post/responses/200/headers/X-Widget-Mode/schema".to_string(),
            ])
        );
    }
}
