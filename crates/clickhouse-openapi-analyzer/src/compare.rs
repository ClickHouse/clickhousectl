use std::collections::{BTreeMap, BTreeSet};

use crate::config::AnalyzerConfig;
use crate::openapi::{
    EnumConstraint, EnumContext, EnumValues, OpenApiInventory, PropertyStep, pascalize,
};
use crate::report::{DriftReport, Finding, FindingKind, UnsupportedEnumConstraint};
use crate::rust_inventory::{RustInventory, TypeNode};

pub(crate) fn compare(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    snapshot: &OpenApiInventory,
    config: &AnalyzerConfig,
) -> DriftReport {
    let mut report = DriftReport::default();
    compare_operations(rust, spec, config, &mut report);
    compare_models_and_refs(rust, spec, &mut report);
    compare_fields(rust, spec, config, &mut report);
    compare_additional_properties(rust, spec, &mut report);
    compare_beta_and_deprecation(rust, spec, config, &mut report);
    compare_enums(rust, spec, config, &mut report);
    compare_snapshot(spec, snapshot, &mut report);
    report.finish();
    report
}

fn compare_operations(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    config: &AnalyzerConfig,
    report: &mut DriftReport,
) {
    for (name, operation) in &spec.operations {
        if !rust.client_methods.contains_key(name) {
            report.findings.push(
                Finding::new(
                    FindingKind::MissingClientMethod,
                    format!(
                        "{} {} is missing Client::{name}",
                        operation.method, operation.path
                    ),
                )
                .at_spec(&operation.pointer)
                .at_rust(format!("client.rs::Client::{name}"))
                .detail("method_name", name)
                .detail("operation_id", &operation.operation_id)
                .detail("method", &operation.method)
                .detail("path", &operation.path)
                .detail("summary", &operation.summary),
            );
        }
    }

    for name in rust.client_methods.keys() {
        if !spec.operations.contains_key(name) && !config.non_openapi_client_methods.contains(name)
        {
            report.findings.push(
                Finding::new(
                    FindingKind::ExtraClientMethod,
                    format!("Client::{name} has no matching OpenAPI operation"),
                )
                .at_rust(format!("client.rs::Client::{name}"))
                .detail("method_name", name),
            );
        }
    }
}

fn compare_models_and_refs(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    report: &mut DriftReport,
) {
    for (schema_name, pointer) in &spec.schemas {
        let rust_name = pascalize(schema_name);
        if !rust.model_types.contains(&rust_name) {
            report.findings.push(
                Finding::new(
                    FindingKind::MissingModelType,
                    format!("schema {schema_name} has no public model type {rust_name}"),
                )
                .at_spec(pointer)
                .at_rust(format!("models.rs::{rust_name}"))
                .detail("schema", schema_name)
                .detail("rust_type", &rust_name),
            );
        }
    }

    for (schema_name, reference_pointer) in &spec.referenced_schemas {
        if !spec.schemas.contains_key(schema_name) {
            report.findings.push(
                Finding::new(
                    FindingKind::MissingSchemaDefinition,
                    format!("reference to undefined component schema {schema_name}"),
                )
                .at_spec(reference_pointer)
                .detail("schema", schema_name),
            );
        }
        // Referenced-and-defined schemas without a Rust model are already
        // reported as MissingModelType by the loop over spec.schemas above.
    }
}

/// The direction a schema usage was reached from. Requiredness/optionality is
/// request-position-only semantics: response-position types are all-`Option`
/// by policy, so optionality findings are suppressed there and field presence
/// (plus enum values) is the retained drift signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Request,
    Response,
}

/// The Rust struct name a schema resolves to in response position: the split
/// `{Name}Response` variant when such a Rust type exists (and is not itself
/// the model of a spec schema literally named `{Name}Response`), otherwise
/// the unsplit `{Name}` type.
fn response_position_type(rust: &RustInventory, spec: &OpenApiInventory, base: &str) -> String {
    let candidate = format!("{base}Response");
    if rust.model_types.contains(&candidate) && !spec.rust_schema_names.contains(&candidate) {
        candidate
    } else {
        base.to_string()
    }
}

/// Rust struct targets for a schema's field checks, one per direction the
/// schema is used in. A bidirectional schema maps to two Rust types once the
/// split `{Name}Response` variant exists; until then both directions collapse
/// onto `{Name}` and only the request-direction target is kept.
pub(crate) fn field_check_targets(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    schema_name: &str,
) -> Vec<(String, Direction)> {
    let base = pascalize(schema_name);
    let mut targets = Vec::new();
    if spec.is_request_position(schema_name) {
        targets.push((base.clone(), Direction::Request));
    }
    if spec.is_response_position(schema_name) {
        let response_name = response_position_type(rust, spec, &base);
        if targets.iter().all(|(name, _)| *name != response_name) {
            targets.push((response_name, Direction::Response));
        }
    }
    targets
}

fn compare_fields(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    config: &AnalyzerConfig,
    report: &mut DriftReport,
) {
    let mut optionality_hits = BTreeSet::new();
    for ((schema_name, property_name), property) in &spec.properties {
        for (rust_name, direction) in field_check_targets(rust, spec, schema_name) {
            let Some(struct_info) = rust.structs.get(&rust_name) else {
                continue;
            };
            let Some(field) = struct_info.fields.get(property_name) else {
                report.findings.push(
                    Finding::new(
                        FindingKind::MissingStructField,
                        format!("{rust_name}.{property_name} is missing from models.rs"),
                    )
                    .at_spec(&property.pointer)
                    .at_rust(format!("models.rs::{rust_name}::{property_name}"))
                    .detail("schema", &rust_name)
                    .detail("field", property_name),
                );
                continue;
            };
            if direction == Direction::Response {
                // Every response field is Option<T> by policy, so requiredness
                // drift is invisible in response position by design.
                continue;
            }
            let mismatch = property.required_non_nullable == field.rust_type.is_option();
            if mismatch {
                let key = (rust_name.clone(), property_name.clone());
                if config.optionality_exemptions.contains(&key) {
                    optionality_hits.insert(key);
                } else {
                    let expected = if property.required_non_nullable {
                        "T"
                    } else {
                        "Option<T>"
                    };
                    let actual = if field.rust_type.is_option() {
                        "Option<T>"
                    } else {
                        "T"
                    };
                    report.findings.push(
                        Finding::new(
                            FindingKind::FieldOptionalityMismatch,
                            format!(
                                "{rust_name}.{property_name} should be {expected}, found {actual}"
                            ),
                        )
                        .at_spec(&property.pointer)
                        .at_rust(format!("models.rs::{rust_name}::{}", field.rust_name))
                        .detail("schema", &rust_name)
                        .detail("field", property_name)
                        .detail("expected", expected)
                        .detail("actual", actual),
                    );
                }
            }
        }
    }
    stale_pairs(
        "optionality",
        &config.optionality_exemptions,
        &optionality_hits,
        report,
    );

    let mut extra_hits = BTreeSet::new();
    let mut properties_by_schema: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (schema, property) in spec.properties.keys() {
        properties_by_schema
            .entry(schema)
            .or_default()
            .insert(property);
    }
    for (schema_name, property_names) in properties_by_schema {
        if property_names.is_empty() {
            continue;
        }
        for (rust_name, _direction) in field_check_targets(rust, spec, schema_name) {
            let Some(struct_info) = rust.structs.get(&rust_name) else {
                continue;
            };
            for (spec_name, field) in &struct_info.fields {
                if field.flatten || property_names.contains(spec_name.as_str()) {
                    continue;
                }
                let key = (rust_name.clone(), spec_name.clone());
                if config.extra_field_exemptions.contains(&key) {
                    extra_hits.insert(key);
                } else {
                    report.findings.push(
                        Finding::new(
                            FindingKind::ExtraStructField,
                            format!("{rust_name}.{spec_name} has no matching OpenAPI property"),
                        )
                        .at_spec(&spec.schemas[schema_name])
                        .at_rust(format!("models.rs::{rust_name}::{}", field.rust_name))
                        .detail("schema", &rust_name)
                        .detail("field", spec_name),
                    );
                }
            }
        }
    }
    stale_pairs(
        "extra_field",
        &config.extra_field_exemptions,
        &extra_hits,
        report,
    );
}

/// Dynamic keys are invisible to ordinary named-property comparison. Require a
/// typed string-keyed map (directly, through aliases, or in a flattened field)
/// so deserialization cannot silently discard those keys.
fn compare_additional_properties(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    report: &mut DriftReport,
) {
    for (schema_name, additional) in &spec.additional_properties {
        for (name, direction) in field_check_targets(rust, spec, schema_name) {
            if !rust.model_types.contains(&name) {
                continue;
            }
            let named = TypeNode::Path(name.clone());
            let resolved = rust.resolve_alias(&named);
            let candidate = match resolved {
                TypeNode::Map(_, _) => Some(resolved),
                TypeNode::Path(struct_name) => rust.structs.get(struct_name).and_then(|info| {
                    info.fields
                        .values()
                        .find(|field| field.flatten)
                        .map(|field| &field.rust_type)
                }),
                _ => None,
            };
            let candidate = candidate.map(|ty| {
                let ty = rust.resolve_alias(ty);
                // A flattened response map can itself be absent.
                if let TypeNode::Option(inner) = ty {
                    rust.resolve_alias(inner)
                } else {
                    ty
                }
            });
            let matches = candidate.is_some_and(|ty| {
                let TypeNode::Map(key, value) = ty else {
                    return false;
                };
                matches!(rust.resolve_alias(key), TypeNode::Path(name) if name == "String")
                    && map_value_matches(rust, spec, value, &additional.value_schema, direction)
            });
            if !matches {
                let actual = candidate.unwrap_or(resolved).display();
                report.findings.push(Finding::new(
                    FindingKind::AdditionalPropertiesMismatch,
                    format!("{name} must preserve additionalProperties in a typed string-keyed map, found {actual}"),
                )
                .at_spec(&additional.pointer)
                .at_rust(format!("models.rs::{name}"))
                .detail("schema", schema_name)
                .detail("expected_value_schema", additional.value_schema.to_string())
                .detail("actual", actual));
            }
        }
    }
}

fn map_value_matches(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    ty: &TypeNode,
    schema: &serde_json::Value,
    direction: Direction,
) -> bool {
    let ty = rust.resolve_alias(ty);
    if schema.get("nullable").and_then(serde_json::Value::as_bool) == Some(true) {
        let TypeNode::Option(inner) = ty else {
            return false;
        };
        let mut non_nullable = schema.clone();
        non_nullable.as_object_mut().unwrap().remove("nullable");
        return map_value_matches(rust, spec, inner, &non_nullable, direction);
    }
    if let Some(reference) = schema.get("$ref").and_then(serde_json::Value::as_str) {
        let Some(schema_name) = reference.strip_prefix("#/components/schemas/") else {
            return false;
        };
        let schema_name = schema_name.replace("~1", "/").replace("~0", "~");
        let expected = match direction {
            Direction::Request => pascalize(&schema_name),
            Direction::Response => response_position_type(rust, spec, &pascalize(&schema_name)),
        };
        return ty == rust.resolve_alias(&TypeNode::Path(expected));
    }
    match schema.get("type").and_then(serde_json::Value::as_str) {
        Some("string") => matches!(ty, TypeNode::Path(name) if name == "String"),
        Some("boolean") => matches!(ty, TypeNode::Path(name) if name == "bool"),
        Some("integer") => {
            matches!(ty, TypeNode::Path(name) if matches!(name.as_str(), "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize"))
        }
        Some("number") => {
            matches!(ty, TypeNode::Path(name) if matches!(name.as_str(), "f32" | "f64"))
        }
        Some("array") => match ty {
            TypeNode::Vec(inner) => schema
                .get("items")
                .is_some_and(|items| map_value_matches(rust, spec, inner, items, direction)),
            _ => false,
        },
        Some("object") => match ty {
            TypeNode::Map(key, value) => {
                matches!(rust.resolve_alias(key), TypeNode::Path(name) if name == "String")
                    && schema
                        .get("additionalProperties")
                        .is_some_and(|additional| {
                            map_value_matches(rust, spec, value, additional, direction)
                        })
            }
            _ => false,
        },
        _ => false,
    }
}

fn compare_beta_and_deprecation(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    config: &AnalyzerConfig,
    report: &mut DriftReport,
) {
    for (operation, pointer) in &spec.beta_operations {
        if !rust.metadata.beta_operations.contains(operation) {
            report.findings.push(
                Finding::new(
                    FindingKind::NewlyBetaOperation,
                    format!("{operation} is Beta in the spec but absent from BETA_OPERATIONS"),
                )
                .at_spec(pointer)
                .at_rust("meta.rs::BETA_OPERATIONS")
                .detail("operation", operation),
            );
        }
    }
    for operation in &rust.metadata.beta_operations {
        if !spec.beta_operations.contains_key(operation) {
            report.findings.push(
                Finding::new(
                    FindingKind::GraduatedBetaOperation,
                    format!("{operation} is no longer Beta but remains in BETA_OPERATIONS"),
                )
                .at_rust("meta.rs::BETA_OPERATIONS")
                .detail("operation", operation),
            );
        }
    }

    // A deprecated property is expected once per Rust type the schema maps to:
    // just `{Name}` today, and additionally `{Name}Response` once a schema used
    // in both directions is split, doubling the DEPRECATED_FIELDS bookkeeping.
    let mut deprecated_exemption_hits = BTreeSet::new();
    let mut expected_pointers: BTreeMap<(String, String), &String> = BTreeMap::new();
    for ((schema_name, field), pointer) in &spec.deprecated_fields {
        for (rust_name, _direction) in field_check_targets(rust, spec, schema_name) {
            let key = (rust_name, field.clone());
            if config.deprecated_field_exemptions.contains(&key) {
                deprecated_exemption_hits.insert(key);
            } else {
                expected_pointers.insert(key, pointer);
            }
        }
    }
    let expected: BTreeSet<(String, String)> = expected_pointers.keys().cloned().collect();
    stale_pairs(
        "deprecated_field",
        &config.deprecated_field_exemptions,
        &deprecated_exemption_hits,
        report,
    );

    for (schema, field) in expected.difference(&rust.metadata.deprecated_fields) {
        let pointer = expected_pointers[&(schema.clone(), field.clone())];
        report.findings.push(
            Finding::new(
                FindingKind::NewlyDeprecatedField,
                format!(
                    "{schema}.{field} is deprecated in the spec but absent from DEPRECATED_FIELDS"
                ),
            )
            .at_spec(pointer)
            .at_rust("meta.rs::DEPRECATED_FIELDS")
            .detail("schema", schema)
            .detail("field", field),
        );
    }
    for (schema, field) in rust.metadata.deprecated_fields.difference(&expected) {
        report.findings.push(
            Finding::new(
                FindingKind::UndeprecatedField,
                format!(
                    "{schema}.{field} is no longer deprecated but remains in DEPRECATED_FIELDS"
                ),
            )
            .at_rust("meta.rs::DEPRECATED_FIELDS")
            .detail("schema", schema)
            .detail("field", field),
        );
    }

    let marked: BTreeSet<(String, String)> = rust
        .structs
        .iter()
        .flat_map(|(struct_name, info)| {
            info.fields
                .iter()
                .filter(|(_, field)| field.deprecated_marker)
                .map(move |(spec_name, _)| (struct_name.clone(), spec_name.clone()))
        })
        .collect();
    for (schema, field) in rust.metadata.deprecated_fields.difference(&marked) {
        report.findings.push(
            Finding::new(
                FindingKind::MissingDeprecatedMarker,
                format!("{schema}.{field} is declared deprecated but lacks the cfg marker"),
            )
            .at_rust(format!("models.rs::{schema}::{field}"))
            .detail("schema", schema)
            .detail("field", field),
        );
    }
    for (schema, field) in marked.difference(&rust.metadata.deprecated_fields) {
        report.findings.push(
            Finding::new(
                FindingKind::StrayDeprecatedMarker,
                format!("{schema}.{field} has a deprecated cfg marker but is not declared"),
            )
            .at_rust(format!("models.rs::{schema}::{field}"))
            .detail("schema", schema)
            .detail("field", field),
        );
    }
}

enum EnumMapping {
    ValueEnum {
        name: String,
        rust_item: String,
    },
    Unsupported {
        rust_item: Option<String>,
        reason: String,
    },
    Unmapped,
}

fn compare_enums(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    config: &AnalyzerConfig,
    report: &mut DriftReport,
) {
    let mut unsupported_hits = BTreeSet::new();
    let mut extra_enum_hits = BTreeSet::new();

    for constraint in &spec.enum_constraints {
        let mapping = map_enum(rust, constraint);
        match (&constraint.values, mapping) {
            (EnumValues::Strings(spec_values), EnumMapping::ValueEnum { name, rust_item }) => {
                let enum_info = &rust.enums[&name];
                if !enum_info.integer_values.is_empty() {
                    record_unsupported(
                        constraint,
                        Some(rust_item),
                        &format!("Rust enum {name} uses integer discriminants for a string enum"),
                        config,
                        &mut unsupported_hits,
                        report,
                    );
                    continue;
                }
                let rust_values = &enum_info.values;
                for value in spec_values.difference(rust_values) {
                    report.findings.push(
                        Finding::new(
                            FindingKind::MissingEnumValue,
                            format!("{name} has no variant for wire value {value:?}"),
                        )
                        .at_spec(&constraint.pointer)
                        .at_rust(&rust_item)
                        .detail("enum", &name)
                        .detail("value", value),
                    );
                }
                for value in rust_values.difference(spec_values) {
                    let key = (name.clone(), value.clone());
                    if config.extra_enum_value_exemptions.contains(&key) {
                        extra_enum_hits.insert(key);
                    } else {
                        report.findings.push(
                            Finding::new(
                                FindingKind::ExtraEnumValue,
                                format!(
                                    "{name} serializes wire value {value:?}, absent from the spec"
                                ),
                            )
                            .at_spec(&constraint.pointer)
                            .at_rust(&rust_item)
                            .detail("enum", &name)
                            .detail("value", value),
                        );
                    }
                }
            }
            (EnumValues::Integers(spec_values), EnumMapping::ValueEnum { name, rust_item }) => {
                let enum_info = &rust.enums[&name];
                if !enum_info.uses_i64_serde_conversion {
                    record_unsupported(
                        constraint,
                        Some(rust_item),
                        &format!(
                            "Rust enum {name} does not declare Serde conversion to and from i64"
                        ),
                        config,
                        &mut unsupported_hits,
                        report,
                    );
                    continue;
                }
                if !enum_info.values.is_empty() {
                    record_unsupported(
                        constraint,
                        Some(rust_item),
                        &format!("Rust enum {name} uses string values for an integer enum"),
                        config,
                        &mut unsupported_hits,
                        report,
                    );
                    continue;
                }
                let rust_values = &enum_info.integer_values;
                for value in spec_values.difference(rust_values) {
                    report.findings.push(
                        Finding::new(
                            FindingKind::MissingEnumValue,
                            format!("{name} has no variant for wire value {value}"),
                        )
                        .at_spec(&constraint.pointer)
                        .at_rust(&rust_item)
                        .detail("enum", &name)
                        .detail("value", value.to_string()),
                    );
                }
                for value in rust_values.difference(spec_values) {
                    let wire_value = value.to_string();
                    let key = (name.clone(), wire_value.clone());
                    if config.extra_enum_value_exemptions.contains(&key) {
                        extra_enum_hits.insert(key);
                    } else {
                        report.findings.push(
                            Finding::new(
                                FindingKind::ExtraEnumValue,
                                format!(
                                    "{name} serializes wire value {value}, absent from the spec"
                                ),
                            )
                            .at_spec(&constraint.pointer)
                            .at_rust(&rust_item)
                            .detail("enum", &name)
                            .detail("value", wire_value),
                        );
                    }
                }
            }
            (EnumValues::Numeric, mapping) => record_unsupported(
                constraint,
                mapping_rust_item(&mapping),
                "numeric enum constraints that are non-integer or outside i64 cannot be represented by Rust unit variants",
                config,
                &mut unsupported_hits,
                report,
            ),
            (EnumValues::Mixed, mapping) => record_unsupported(
                constraint,
                mapping_rust_item(&mapping),
                "mixed enum constraints cannot be represented by one Rust value enum",
                config,
                &mut unsupported_hits,
                report,
            ),
            (_, EnumMapping::Unsupported { rust_item, reason }) => record_unsupported(
                constraint,
                rust_item,
                &reason,
                config,
                &mut unsupported_hits,
                report,
            ),
            (_, EnumMapping::Unmapped) => {}
        }
    }

    stale_pairs(
        "extra_enum_value",
        &config.extra_enum_value_exemptions,
        &extra_enum_hits,
        report,
    );
    for pointer in config
        .acknowledged_unsupported_enum_pointers
        .difference(&unsupported_hits)
    {
        report.findings.push(
            Finding::new(
                FindingKind::StaleExemption,
                format!("unsupported enum acknowledgement {pointer} is stale"),
            )
            .at_spec(pointer)
            .detail("exemption_kind", "unsupported_enum")
            .detail("key", pointer),
        );
    }

    compare_enum_values_consts(rust, report);
}

fn compare_enum_values_consts(rust: &RustInventory, report: &mut DriftReport) {
    for (name, info) in &rust.enums {
        let Some(values_const) = &info.values_const else {
            continue;
        };
        let rust_values = &info.values;
        let missing: Vec<&String> = rust_values.difference(values_const).collect();
        let extra: Vec<&String> = values_const.difference(rust_values).collect();
        if missing.is_empty() && extra.is_empty() {
            continue;
        }
        let mut parts = Vec::new();
        if !missing.is_empty() {
            let list = missing
                .iter()
                .map(|v| format!("{v:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("missing {list}"));
        }
        if !extra.is_empty() {
            let list = extra
                .iter()
                .map(|v| format!("{v:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("extra {list}"));
        }
        let rust_item = format!("models.rs::{name}::VALUES");
        let finding = Finding::new(
            FindingKind::EnumValuesMismatch,
            format!(
                "{name}::VALUES does not match enum wire values: {}",
                parts.join("; ")
            ),
        )
        .at_rust(&rust_item)
        .detail("enum", name);
        let finding = if !missing.is_empty() {
            finding.detail(
                "missing",
                missing
                    .iter()
                    .map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        } else {
            finding
        };
        let finding = if !extra.is_empty() {
            finding.detail(
                "extra",
                extra
                    .iter()
                    .map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        } else {
            finding
        };
        report.findings.push(finding);
    }
}

fn mapping_rust_item(mapping: &EnumMapping) -> Option<String> {
    match mapping {
        EnumMapping::ValueEnum { rust_item, .. } => Some(rust_item.clone()),
        EnumMapping::Unsupported { rust_item, .. } => rust_item.clone(),
        EnumMapping::Unmapped => None,
    }
}

fn map_enum(rust: &RustInventory, constraint: &EnumConstraint) -> EnumMapping {
    let (type_name, rust_item) = match &constraint.context {
        EnumContext::NamedSchema { schema } => {
            let name = pascalize(schema);
            if !rust.model_types.contains(&name) {
                return EnumMapping::Unmapped;
            }
            (Some(name.clone()), format!("models.rs::{name}"))
        }
        EnumContext::Property { schema, steps } => {
            match resolve_property_chain(rust, schema, steps) {
                ChainResolution::Unmapped => return EnumMapping::Unmapped,
                ChainResolution::Unsupported { rust_item, reason } => {
                    return EnumMapping::Unsupported { rust_item, reason };
                }
                ChainResolution::Mapped {
                    type_name,
                    rust_item,
                } => (type_name, rust_item),
            }
        }
        EnumContext::Parameter {
            operation_id,
            parameter,
        } => {
            let Some(method) = rust.client_methods.get(operation_id) else {
                return EnumMapping::Unmapped;
            };
            let Some(argument) = method.arguments.get(parameter) else {
                return EnumMapping::Unmapped;
            };
            (
                rust.terminal_type(argument),
                format!("client.rs::Client::{operation_id}::{parameter}"),
            )
        }
        EnumContext::Unknown => {
            return EnumMapping::Unsupported {
                rust_item: None,
                reason: "enum location cannot be mapped to a concrete Rust item".to_string(),
            };
        }
    };

    let Some(type_name) = type_name else {
        return EnumMapping::Unsupported {
            rust_item: Some(rust_item),
            reason: "Rust type shape is not a supported named type".to_string(),
        };
    };
    let Some(enum_info) = rust.enums.get(&type_name) else {
        return EnumMapping::Unsupported {
            rust_item: Some(rust_item),
            reason: format!("Rust type {type_name} is not an enum"),
        };
    };
    if !enum_info.is_value_enum {
        return EnumMapping::Unsupported {
            rust_item: Some(rust_item),
            reason: format!("Rust enum {type_name} is a data-carrying or untagged union"),
        };
    }
    EnumMapping::ValueEnum {
        name: type_name,
        rust_item,
    }
}

/// Outcome of walking a property chain from a named schema to an enum position.
enum ChainResolution {
    /// The chain resolved end to end; `type_name` is the final field's candidate
    /// enum type (or `None` when its shape is not a named type).
    Mapped {
        type_name: Option<String>,
        rust_item: String,
    },
    /// A struct or field along the chain simply does not exist in the model, so
    /// the enum has no counterpart to check — same semantics as a missing
    /// top-level property.
    Unmapped,
    /// The chain exists but an intermediate hop cannot be followed to a struct,
    /// so the enum cannot be mapped to a concrete value enum.
    Unsupported {
        rust_item: Option<String>,
        reason: String,
    },
}

/// Walks the property chain, resolving each step's Rust field type and stepping
/// into the resulting struct for the next property, until the final step yields
/// the candidate enum type. Intermediate hops must land on a named struct;
/// otherwise the shape cannot be followed.
fn resolve_property_chain(
    rust: &RustInventory,
    schema: &str,
    steps: &[PropertyStep],
) -> ChainResolution {
    let mut current_struct = pascalize(schema);
    for (index, step) in steps.iter().enumerate() {
        let Some(struct_info) = rust.structs.get(&current_struct) else {
            return ChainResolution::Unmapped;
        };
        let Some(field) = struct_info.fields.get(&step.property) else {
            return ChainResolution::Unmapped;
        };
        let rust_item = format!("models.rs::{current_struct}::{}", field.rust_name);
        let type_name = if step.array_item {
            rust.array_item_type(&field.rust_type)
        } else {
            rust.terminal_type(&field.rust_type)
        };
        if index + 1 == steps.len() {
            return ChainResolution::Mapped {
                type_name,
                rust_item,
            };
        }
        let Some(type_name) = type_name else {
            return ChainResolution::Unsupported {
                rust_item: Some(rust_item),
                reason: "intermediate property type shape is not a supported named type"
                    .to_string(),
            };
        };
        if !rust.structs.contains_key(&type_name) {
            return ChainResolution::Unsupported {
                rust_item: Some(rust_item),
                reason: format!("intermediate property type {type_name} is not a struct"),
            };
        }
        current_struct = type_name;
    }
    ChainResolution::Unmapped
}

fn record_unsupported(
    constraint: &EnumConstraint,
    rust_item: Option<String>,
    reason: &str,
    config: &AnalyzerConfig,
    hits: &mut BTreeSet<String>,
    report: &mut DriftReport,
) {
    let acknowledged = config
        .acknowledged_unsupported_enum_pointers
        .contains(&constraint.pointer);
    if acknowledged {
        hits.insert(constraint.pointer.clone());
    }
    report
        .unsupported_enum_constraints
        .push(UnsupportedEnumConstraint {
            spec_pointer: constraint.pointer.clone(),
            rust_item: rust_item.clone(),
            reason: reason.to_string(),
            acknowledged,
        });
    if !acknowledged {
        let mut finding = Finding::new(
            FindingKind::UnsupportedEnumConstraint,
            format!("unacknowledged unsupported enum constraint: {reason}"),
        )
        .at_spec(&constraint.pointer)
        .detail("reason", reason);
        if let Some(item) = rust_item {
            finding = finding.at_rust(item);
        }
        report.findings.push(finding);
    }
}

fn compare_snapshot(
    spec: &OpenApiInventory,
    snapshot: &OpenApiInventory,
    report: &mut DriftReport,
) {
    let live_operations: BTreeSet<_> = spec.operations.keys().cloned().collect();
    let snapshot_operations: BTreeSet<_> = snapshot.operations.keys().cloned().collect();
    for operation in live_operations.difference(&snapshot_operations) {
        report.findings.push(
            Finding::new(
                FindingKind::SnapshotAddedOperation,
                format!("operation {operation} is present in the target spec but absent from the snapshot"),
            )
            .at_spec(&spec.operations[operation].pointer)
            .detail("operation", operation),
        );
    }
    for operation in snapshot_operations.difference(&live_operations) {
        report.findings.push(
            Finding::new(
                FindingKind::SnapshotRemovedOperation,
                format!("operation {operation} remains in the snapshot but is absent from the target spec"),
            )
            .at_spec(&snapshot.operations[operation].pointer)
            .detail("operation", operation),
        );
    }

    let live_schemas: BTreeSet<_> = spec.schemas.keys().cloned().collect();
    let snapshot_schemas: BTreeSet<_> = snapshot.schemas.keys().cloned().collect();
    for schema in live_schemas.difference(&snapshot_schemas) {
        report.findings.push(
            Finding::new(
                FindingKind::SnapshotAddedSchema,
                format!(
                    "schema {schema} is present in the target spec but absent from the snapshot"
                ),
            )
            .at_spec(&spec.schemas[schema])
            .detail("schema", schema),
        );
    }
    for schema in snapshot_schemas.difference(&live_schemas) {
        report.findings.push(
            Finding::new(
                FindingKind::SnapshotRemovedSchema,
                format!(
                    "schema {schema} remains in the snapshot but is absent from the target spec"
                ),
            )
            .at_spec(&snapshot.schemas[schema])
            .detail("schema", schema),
        );
    }
}

fn stale_pairs(
    kind: &str,
    configured: &BTreeSet<(String, String)>,
    hit: &BTreeSet<(String, String)>,
    report: &mut DriftReport,
) {
    for (left, right) in configured.difference(hit) {
        report.findings.push(
            Finding::new(
                FindingKind::StaleExemption,
                format!("{kind} exemption ({left}, {right}) is stale"),
            )
            .at_rust(format!("analyzer_config::{left}::{right}"))
            .detail("exemption_kind", kind)
            .detail("left", left)
            .detail("right", right),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi::OpenApiInventory;
    use crate::rust_inventory::RustInventory;

    fn analyze_fixture(
        models: &str,
        schema: serde_json::Value,
        config: AnalyzerConfig,
    ) -> DriftReport {
        let spec = serde_json::json!({
            "paths": {},
            "components": {"schemas": {"Widget": schema}}
        });
        let rust = RustInventory::parse(
            "pub struct Client; impl Client {}",
            models,
            "pub const BETA_OPERATIONS: &[&str] = &[]; pub const DEPRECATED_FIELDS: &[(&str, &str)] = &[];",
        )
        .unwrap();
        let openapi = OpenApiInventory::build(&spec, &config).unwrap();
        compare(&rust, &openapi, &openapi, &config)
    }

    /// A minimal spec with one operation that sends `Widget` as the request
    /// body when `request` is set, and returns `Widget` when `response` is set,
    /// so fixtures can place the schema in either or both positions.
    fn directional_spec(
        schema: serde_json::Value,
        request: bool,
        response: bool,
    ) -> serde_json::Value {
        let mut operation = serde_json::json!({"operationId": "mutateWidget"});
        if request {
            operation["requestBody"] = serde_json::json!({"content": {"application/json": {
                "schema": {"$ref": "#/components/schemas/Widget"}
            }}});
        }
        if response {
            operation["responses"] = serde_json::json!({"200": {"content": {"application/json": {
                "schema": {"$ref": "#/components/schemas/Widget"}
            }}}});
        }
        serde_json::json!({
            "paths": {
                "/widgets": {"post": operation},
                // Matches the fixture Client's second method so no-drift
                // assertions are not polluted by ExtraClientMethod.
                "/widgets/{id}": {"get": {"operationId": "getWidget"}}
            },
            "components": {"schemas": {"Widget": schema}}
        })
    }

    fn analyze_directional(
        models: &str,
        spec: serde_json::Value,
        config: AnalyzerConfig,
    ) -> DriftReport {
        let rust = RustInventory::parse(
            r#"
                pub struct Client;
                impl Client {
                    pub async fn mutate_widget(&self) {}
                    pub async fn get_widget(&self) {}
                }
            "#,
            models,
            "pub const BETA_OPERATIONS: &[&str] = &[]; pub const DEPRECATED_FIELDS: &[(&str, &str)] = &[];",
        )
        .unwrap();
        let openapi = OpenApiInventory::build(&spec, &config).unwrap();
        compare(&rust, &openapi, &openapi, &config)
    }

    #[test]
    fn additional_properties_detects_empty_structs_in_both_directions() {
        let report = analyze_directional(
            "pub struct Widget {} pub struct WidgetResponse {}",
            directional_spec(
                serde_json::json!({
                    "type": "object", "additionalProperties": {"type": "string"}
                }),
                true,
                true,
            ),
            AnalyzerConfig::default(),
        );
        assert_eq!(report.findings.len(), 2, "{}", report.render_text());
        for finding in &report.findings {
            assert_eq!(finding.kind, FindingKind::AdditionalPropertiesMismatch);
            assert_eq!(
                finding.spec_pointer.as_deref(),
                Some("/components/schemas/Widget/additionalProperties")
            );
        }
        assert_eq!(
            report.findings[0].rust_item.as_deref(),
            Some("models.rs::Widget")
        );
        assert_eq!(
            report.findings[1].rust_item.as_deref(),
            Some("models.rs::WidgetResponse")
        );
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["schema_version"], 4);
        assert_eq!(
            json["findings"][0]["kind"],
            "additional_properties_mismatch"
        );
        assert!(
            report
                .render_text()
                .contains("AdditionalPropertiesMismatch")
        );
    }

    #[test]
    fn additional_properties_accepts_map_aliases_and_flattened_maps() {
        for models in [
            "pub type Widget = std::collections::BTreeMap<String, String>; pub type WidgetResponse = Widget;",
            "pub type Text = String; pub type Widget = HashMap<Text, Text>; pub type WidgetResponse = Widget;",
            "pub struct Widget { #[serde(flatten)] pub values: BTreeMap<String, String> } pub type WidgetResponse = Widget;",
        ] {
            let report = analyze_directional(
                models,
                directional_spec(
                    serde_json::json!({
                        "type": "object", "additionalProperties": {"type": "string"}
                    }),
                    true,
                    true,
                ),
                AnalyzerConfig::default(),
            );
            assert!(!report.has_drift(), "{}", report.render_text());
        }
    }

    #[test]
    fn additional_properties_rejects_losing_or_untyped_representations() {
        for models in [
            "pub type Widget = serde_json::Value;",
            "pub type Widget = BTreeMap<String, serde_json::Value>;",
            "pub type Widget = BTreeMap<i64, String>;",
            "pub type Widget = BTreeMap<String, i64>;",
            "pub type Widget = BTreeMap<String, Option<String>>;",
            "pub struct Widget { pub values: BTreeMap<String, String> }",
            "pub type Widget = Cycle; pub type Cycle = Widget;",
        ] {
            let report = analyze_fixture(
                models,
                serde_json::json!({
                    "type": "object", "additionalProperties": {"type": "string"}
                }),
                AnalyzerConfig::default(),
            );
            assert_eq!(
                report.findings.len(),
                1,
                "{models}: {}",
                report.render_text()
            );
            assert_eq!(
                report.findings[0].kind,
                FindingKind::AdditionalPropertiesMismatch
            );
        }
    }

    #[test]
    fn additional_properties_leaves_untyped_and_closed_objects_alone() {
        for additional in [
            serde_json::json!(true),
            serde_json::json!(false),
            serde_json::json!({}),
        ] {
            let report = analyze_fixture(
                "pub struct Widget {}",
                serde_json::json!({
                    "type": "object", "additionalProperties": additional
                }),
                AnalyzerConfig::default(),
            );
            assert!(!report.has_drift(), "{}", report.render_text());
        }
    }

    #[test]
    fn additional_properties_maps_coexist_with_named_properties() {
        let report = analyze_fixture(
            "pub struct Widget { pub name: String, #[serde(flatten)] pub values: BTreeMap<String, String> }",
            serde_json::json!({"type": "object", "properties": {"name": {"type": "string"}}, "additionalProperties": {"type": "string"}}),
            AnalyzerConfig::default(),
        );
        assert!(!report.has_drift(), "{}", report.render_text());
    }

    #[test]
    fn additional_properties_checks_nested_and_nullable_values() {
        for (value_type, value_schema) in [
            ("bool", serde_json::json!({"type": "boolean"})),
            ("i64", serde_json::json!({"type": "integer"})),
            ("f64", serde_json::json!({"type": "number"})),
            (
                "Vec<String>",
                serde_json::json!({"type": "array", "items": {"type": "string"}}),
            ),
            (
                "BTreeMap<String, String>",
                serde_json::json!({"type": "object", "additionalProperties": {"type": "string"}}),
            ),
            (
                "Option<String>",
                serde_json::json!({"type": "string", "nullable": true}),
            ),
        ] {
            let report = analyze_fixture(
                &format!("pub type Widget = BTreeMap<String, {value_type}>;"),
                serde_json::json!({"type": "object", "additionalProperties": value_schema}),
                AnalyzerConfig::default(),
            );
            assert!(!report.has_drift(), "{}", report.render_text());
        }
    }

    #[test]
    fn additional_properties_resolves_referenced_value_models_by_direction() {
        let mut spec = directional_spec(
            serde_json::json!({"type": "object", "additionalProperties": {"$ref": "#/components/schemas/Entry"}}),
            true,
            true,
        );
        spec["components"]["schemas"]["Entry"] =
            serde_json::json!({"type": "object", "properties": {"name": {"type": "string"}}});
        let report = analyze_directional(
            "pub type Widget = BTreeMap<String, Entry>; pub type WidgetResponse = BTreeMap<String, EntryResponse>; pub struct Entry {pub name: String} pub struct EntryResponse {pub name: Option<String>}",
            spec,
            AnalyzerConfig::default(),
        );
        assert!(!report.has_drift(), "{}", report.render_text());
    }

    #[test]
    fn suppresses_optionality_but_keeps_presence_checks_in_response_position() {
        // Response-only schema: `name` is required in the spec but Option<T>
        // in Rust (all-Option response policy) — no optionality finding.
        // Presence drift (missing `gone`, extra `extraCode`) still fires.
        let report = analyze_directional(
            r#"
                pub struct Widget {
                    pub name: Option<String>,
                    #[serde(rename = "extraCode")]
                    pub extra_code: Option<String>,
                }
            "#,
            directional_spec(
                serde_json::json!({
                    "required": ["name", "gone"],
                    "properties": {"name": {"type": "string"}, "gone": {"type": "string"}}
                }),
                false,
                true,
            ),
            AnalyzerConfig::default(),
        );
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| finding.kind == FindingKind::FieldOptionalityMismatch),
            "{}",
            report.render_text()
        );
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::MissingStructField
                && finding.details.get("field").map(String::as_str) == Some("gone")
        }));
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::ExtraStructField
                && finding.details.get("field").map(String::as_str) == Some("extraCode")
        }));
    }

    #[test]
    fn keeps_optionality_checks_in_request_position() {
        let report = analyze_directional(
            "pub struct Widget { pub name: Option<String> }",
            directional_spec(
                serde_json::json!({
                    "required": ["name"],
                    "properties": {"name": {"type": "string"}}
                }),
                true,
                false,
            ),
            AnalyzerConfig::default(),
        );
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::FieldOptionalityMismatch
                && finding.rust_item.as_deref() == Some("models.rs::Widget::name")
        }));
    }

    #[test]
    fn bidirectional_schema_checks_both_split_variants() {
        // `Widget` is used in both directions and the split `WidgetResponse`
        // exists: the request variant is checked strictly, the response
        // variant gets presence checks only.
        let models = r#"
            pub struct Widget {
                pub name: String,
                pub mode: Option<String>,
            }
            pub struct WidgetResponse {
                pub name: Option<String>,
                #[serde(rename = "extraCode")]
                pub extra_code: Option<String>,
            }
        "#;
        let schema = serde_json::json!({
            "required": ["name"],
            "properties": {
                "name": {"type": "string"},
                "mode": {"type": "string", "description": "Optional mode"}
            }
        });
        let report = analyze_directional(
            models,
            directional_spec(schema, true, true),
            AnalyzerConfig::default(),
        );
        // The strict request variant is compliant and the response variant is
        // all-Option: no optionality findings anywhere.
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| finding.kind == FindingKind::FieldOptionalityMismatch),
            "{}",
            report.render_text()
        );
        // Presence drift is reported per variant: `mode` is missing from the
        // response variant, `extraCode` exists only there.
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::MissingStructField
                && finding.rust_item.as_deref() == Some("models.rs::WidgetResponse::mode")
        }));
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::ExtraStructField
                && finding.details.get("schema").map(String::as_str) == Some("WidgetResponse")
                && finding.details.get("field").map(String::as_str) == Some("extraCode")
        }));
        assert!(!report.findings.iter().any(|finding| {
            finding.details.get("schema").map(String::as_str) == Some("Widget")
        }));
    }

    #[test]
    fn bidirectional_schema_without_split_type_falls_back_to_the_shared_struct() {
        // models.rs not yet split: both directions collapse onto `Widget`,
        // which is checked once under request-position rules.
        let report = analyze_directional(
            "pub struct Widget { pub name: String }",
            directional_spec(
                serde_json::json!({
                    "required": ["name"],
                    "properties": {"name": {"type": "string"}}
                }),
                true,
                true,
            ),
            AnalyzerConfig::default(),
        );
        assert!(!report.has_drift(), "{}", report.render_text());
    }

    #[test]
    fn response_mapping_does_not_hijack_a_type_modeling_a_response_named_schema() {
        // The spec defines both `Widget` (response-only) and `WidgetResponse`.
        // Rust `WidgetResponse` models the latter, so schema `Widget` must
        // keep resolving to Rust `Widget` — `mode` missing from Rust
        // `WidgetResponse` must be attributed to schema `WidgetResponse`,
        // and Rust `Widget` must satisfy schema `Widget`.
        let spec = serde_json::json!({
            "paths": {"/widgets": {"get": {
                "operationId": "getWidget",
                "responses": {"200": {"content": {"application/json": {"schema": {
                    "properties": {
                        "plain": {"$ref": "#/components/schemas/Widget"},
                        "named": {"$ref": "#/components/schemas/WidgetResponse"}
                    }
                }}}}}
            }}},
            "components": {"schemas": {
                "Widget": {"required": ["name"], "properties": {"name": {"type": "string"}}},
                "WidgetResponse": {
                    "required": ["name", "mode"],
                    "properties": {"name": {"type": "string"}, "mode": {"type": "string"}}
                }
            }}
        });
        let report = analyze_directional(
            r#"
                pub struct Widget { pub name: Option<String> }
                pub struct WidgetResponse { pub name: Option<String> }
            "#,
            spec,
            AnalyzerConfig::default(),
        );
        let missing: Vec<_> = report
            .findings
            .iter()
            .filter(|finding| finding.kind == FindingKind::MissingStructField)
            .collect();
        assert_eq!(missing.len(), 1, "{}", report.render_text());
        assert_eq!(
            missing[0].rust_item.as_deref(),
            Some("models.rs::WidgetResponse::mode")
        );
        assert_eq!(
            missing[0].spec_pointer.as_deref(),
            Some("/components/schemas/WidgetResponse/properties/mode")
        );
    }

    #[test]
    fn deprecated_fields_are_expected_on_both_split_variants() {
        let schema = serde_json::json!({
            "required": ["name"],
            "properties": {
                "name": {"type": "string"},
                "old": {"type": "string", "description": "Optional legacy", "deprecated": true}
            }
        });
        let models = r#"
            pub struct Widget {
                pub name: String,
                #[cfg(feature = "deprecated-fields")]
                pub old: Option<String>,
            }
            pub struct WidgetResponse {
                pub name: Option<String>,
                #[cfg(feature = "deprecated-fields")]
                pub old: Option<String>,
            }
        "#;
        let rust = RustInventory::parse(
            "pub struct Client; impl Client {}",
            models,
            r#"
                pub const BETA_OPERATIONS: &[&str] = &[];
                pub const DEPRECATED_FIELDS: &[(&str, &str)] = &[("Widget", "old")];
            "#,
        )
        .unwrap();
        let config = AnalyzerConfig::default();
        let openapi =
            OpenApiInventory::build(&directional_spec(schema, true, true), &config).unwrap();
        let report = compare(&rust, &openapi, &openapi, &config);
        // The split response variant carries the marker but is missing from
        // DEPRECATED_FIELDS: the doubled bookkeeping is enforced.
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::NewlyDeprecatedField
                && finding.details.get("schema").map(String::as_str) == Some("WidgetResponse")
        }));
        assert!(!report.findings.iter().any(|finding| {
            finding.kind == FindingKind::NewlyDeprecatedField
                && finding.details.get("schema").map(String::as_str) == Some("Widget")
        }));
    }

    #[test]
    fn catches_seek_type_snapshot_regression_bidirectionally() {
        let report = analyze_fixture(
            r#"
                pub struct Widget { #[serde(rename = "seekType")] pub seek_type: SeekType }
                pub enum SeekType {
                    #[serde(rename = "timestamp")] Timestamp,
                    #[serde(rename = "snapshot")] Snapshot,
                    #[serde(untagged)] Unknown(String),
                }
            "#,
            serde_json::json!({
                "required": ["seekType"],
                "properties": {"seekType": {"enum": ["timestamp"]}}
            }),
            AnalyzerConfig::default(),
        );
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::ExtraEnumValue
                && finding.details.get("value").map(String::as_str) == Some("snapshot")
        }));
    }

    #[test]
    fn new_and_stale_unsupported_enum_locations_are_actionable() {
        let mut config = AnalyzerConfig::default();
        config
            .acknowledged_unsupported_enum_pointers
            .insert("/components/schemas/Gone".to_string());
        let report = analyze_fixture(
            "pub struct Widget { pub values: Vec<String> }",
            serde_json::json!({
                "required": ["values"],
                "properties": {"values": {"type": "array", "items": {"enum": ["a"]}}}
            }),
            config,
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.kind == FindingKind::UnsupportedEnumConstraint })
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.kind == FindingKind::StaleExemption })
        );
    }

    #[test]
    fn checks_items_parameters_and_composition_when_they_map_to_enums() {
        let spec = serde_json::json!({
            "paths": {"/widgets": {"get": {
                "operationId": "listWidgets",
                "parameters": [{
                    "name": "sortOrder",
                    "schema": {"enum": ["asc", "desc"]}
                }]
            }}},
            "components": {"schemas": {
                "Widget": {
                    "required": ["states", "mode", "count"],
                    "properties": {
                        "states": {"type": "array", "items": {"enum": ["ready"]}},
                        "mode": {"$ref": "#/components/schemas/Mode"},
                        "count": {"type": "integer", "enum": [1, 2]}
                    }
                },
                "Mode": {"oneOf": [{"enum": ["fast"]}]}
            }}
        });
        let client = r#"
            pub struct Client;
            impl Client {
                pub async fn list_widgets(&self, sort_order: Option<&SortOrder>) {}
            }
        "#;
        let models = r#"
            pub struct Widget {
                pub states: Vec<State>,
                pub mode: Mode,
                pub count: Count,
            }
            pub enum State { #[serde(rename = "ready")] Ready }
            pub enum Mode { #[serde(rename = "fast")] Fast }
            pub enum SortOrder {
                #[serde(rename = "asc")] Asc,
                #[serde(rename = "desc")] Desc,
            }
            #[repr(i64)]
            #[serde(from = "i64", into = "i64")]
            pub enum Count {
                One = 1,
                Two = 2,
                #[serde(untagged)] Unknown(i64),
            }
        "#;
        let config = AnalyzerConfig::default();
        let rust = RustInventory::parse(
            client,
            models,
            "pub const BETA_OPERATIONS: &[&str] = &[]; pub const DEPRECATED_FIELDS: &[(&str, &str)] = &[];",
        )
        .unwrap();
        let openapi = OpenApiInventory::build(&spec, &config).unwrap();
        let report = compare(&rust, &openapi, &openapi, &config);
        assert!(!report.has_drift(), "{}", report.render_text());
        assert!(report.unsupported_enum_constraints.is_empty());
    }

    #[test]
    fn checks_integer_enum_values_bidirectionally() {
        let report = analyze_fixture(
            r#"
                pub struct Widget { pub count: Count }
                #[repr(i64)]
                #[serde(from = "i64", into = "i64")]
                pub enum Count {
                    Zero = 0,
                    Twelve = 12,
                    #[serde(untagged)] Unknown(i64),
                }
            "#,
            serde_json::json!({
                "required": ["count"],
                "properties": {"count": {"type": "integer", "enum": [0.0, 6.0]}}
            }),
            AnalyzerConfig::default(),
        );

        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::MissingEnumValue
                && finding.details.get("value").map(String::as_str) == Some("6")
        }));
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::ExtraEnumValue
                && finding.details.get("value").map(String::as_str) == Some("12")
        }));
        assert!(report.unsupported_enum_constraints.is_empty());
    }

    #[test]
    fn integer_enum_requires_explicit_i64_serde_conversion() {
        let report = analyze_fixture(
            r#"
                pub struct Widget { pub count: Count }
                #[repr(i64)]
                pub enum Count { Zero = 0, Six = 6 }
            "#,
            serde_json::json!({
                "required": ["count"],
                "properties": {"count": {"type": "integer", "enum": [0, 6]}}
            }),
            AnalyzerConfig::default(),
        );

        assert_eq!(report.unsupported_enum_constraints.len(), 1);
        assert!(
            report.unsupported_enum_constraints[0]
                .reason
                .contains("Serde conversion")
        );
    }

    #[test]
    fn acknowledgement_becomes_stale_when_an_integer_enum_is_checkable() {
        let pointer = "/components/schemas/Widget/properties/count";
        let mut config = AnalyzerConfig::default();
        config
            .acknowledged_unsupported_enum_pointers
            .insert(pointer.to_string());
        let report = analyze_fixture(
            r#"
                pub struct Widget { pub count: Count }
                #[repr(i64)]
                #[serde(from = "i64", into = "i64")]
                pub enum Count { Zero = 0, Six = 6 }
            "#,
            serde_json::json!({
                "required": ["count"],
                "properties": {"count": {"type": "integer", "enum": [0, 6]}}
            }),
            config,
        );

        assert!(report.unsupported_enum_constraints.is_empty());
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::StaleExemption
                && finding.spec_pointer.as_deref() == Some(pointer)
        }));
    }

    #[test]
    fn non_integer_numeric_enum_remains_unsupported() {
        let report = analyze_fixture(
            r#"
                pub struct Widget { pub ratio: Ratio }
                pub enum Ratio { Half }
            "#,
            serde_json::json!({
                "required": ["ratio"],
                "properties": {"ratio": {"type": "number", "enum": [0.5]}}
            }),
            AnalyzerConfig::default(),
        );

        assert_eq!(report.unsupported_enum_constraints.len(), 1);
        assert!(
            report.unsupported_enum_constraints[0]
                .reason
                .contains("non-integer or outside i64")
        );
    }

    #[test]
    fn resolves_nested_property_chain_through_intermediate_struct() {
        let report = analyze_fixture(
            r#"
                pub struct Widget { pub foo: FooType }
                pub struct FooType { pub bar: BarMode }
                pub enum BarMode {
                    #[serde(rename = "a")] A,
                    #[serde(rename = "c")] C,
                }
            "#,
            serde_json::json!({
                "required": ["foo"],
                "properties": {
                    "foo": {
                        "required": ["bar"],
                        "properties": {"bar": {"enum": ["a", "b"]}}
                    }
                }
            }),
            AnalyzerConfig::default(),
        );
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::MissingEnumValue
                && finding.details.get("enum").map(String::as_str) == Some("BarMode")
                && finding.details.get("value").map(String::as_str) == Some("b")
                && finding.rust_item.as_deref() == Some("models.rs::FooType::bar")
        }));
        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::ExtraEnumValue
                && finding.details.get("enum").map(String::as_str) == Some("BarMode")
                && finding.details.get("value").map(String::as_str) == Some("c")
        }));
    }

    #[test]
    fn maps_property_enum_beneath_top_level_composition() {
        let report = analyze_fixture(
            r#"
                pub struct Widget { pub status: Status }
                pub enum Status { #[serde(rename = "on")] On }
            "#,
            serde_json::json!({
                "allOf": [{
                    "properties": {"status": {"enum": ["on", "off"]}}
                }]
            }),
            AnalyzerConfig::default(),
        );

        assert!(report.findings.iter().any(|finding| {
            finding.kind == FindingKind::MissingEnumValue
                && finding.details.get("enum").map(String::as_str) == Some("Status")
                && finding.details.get("value").map(String::as_str) == Some("off")
                && finding.rust_item.as_deref() == Some("models.rs::Widget::status")
        }));
        assert!(report.unsupported_enum_constraints.is_empty());
    }

    #[test]
    fn nested_chain_through_non_struct_reports_unsupported() {
        let report = analyze_fixture(
            "pub struct Widget { pub foo: String }",
            serde_json::json!({
                "required": ["foo"],
                "properties": {
                    "foo": {"properties": {"bar": {"enum": ["a"]}}}
                }
            }),
            AnalyzerConfig::default(),
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == FindingKind::UnsupportedEnumConstraint)
        );
        assert_eq!(report.unsupported_enum_constraints.len(), 1);
        let unsupported = &report.unsupported_enum_constraints[0];
        assert!(unsupported.reason.contains("is not a struct"));
        assert_eq!(
            unsupported.rust_item.as_deref(),
            Some("models.rs::Widget::foo")
        );
        assert!(!unsupported.acknowledged);
    }

    #[test]
    fn reports_every_non_enum_drift_family() {
        let spec = serde_json::json!({
            "paths": {"/widgets": {"get": {
                "operationId": "listWidgets",
                "x-badges": [{"name": "Beta"}]
            }}},
            "components": {"schemas": {
                "Widget": {
                    "required": ["name", "extraSpec", "old", "newOld", "reference"],
                    "properties": {
                        "name": {"type": "string"},
                        "extraSpec": {"type": "string"},
                        "old": {"type": "string", "deprecated": true},
                        "newOld": {"type": "string", "deprecated": true},
                        "reference": {"$ref": "#/components/schemas/MissingType"},
                        "brokenReference": {"$ref": "#/components/schemas/Undefined"}
                    }
                },
                "MissingType": {"properties": {}}
            }}
        });
        let snapshot = serde_json::json!({"paths": {}, "components": {"schemas": {}}});
        let rust = RustInventory::parse(
            r#"
                pub struct Client;
                impl Client { pub async fn extra_method(&self) {} }
            "#,
            r#"
                pub struct Widget {
                    pub name: Option<String>,
                    pub old: String,
                    pub extra_code: String,
                    #[cfg(feature = "deprecated-fields")]
                    pub marked: String,
                }
            "#,
            r#"
                pub const BETA_OPERATIONS: &[&str] = &["old_beta"];
                pub const DEPRECATED_FIELDS: &[(&str, &str)] = &[
                    ("Widget", "gone"),
                    ("Widget", "old"),
                ];
            "#,
        )
        .unwrap();
        let mut config = AnalyzerConfig::default();
        config
            .optionality_exemptions
            .insert(("Gone".to_string(), "field".to_string()));
        let openapi = OpenApiInventory::build(&spec, &config).unwrap();
        let snapshot = OpenApiInventory::build(&snapshot, &config).unwrap();
        let report = compare(&rust, &openapi, &snapshot, &config);
        let kinds: BTreeSet<_> = report.findings.iter().map(|finding| finding.kind).collect();
        for expected in [
            FindingKind::MissingClientMethod,
            FindingKind::ExtraClientMethod,
            FindingKind::MissingModelType,
            FindingKind::MissingSchemaDefinition,
            FindingKind::MissingStructField,
            FindingKind::ExtraStructField,
            FindingKind::FieldOptionalityMismatch,
            FindingKind::NewlyBetaOperation,
            FindingKind::GraduatedBetaOperation,
            FindingKind::NewlyDeprecatedField,
            FindingKind::UndeprecatedField,
            FindingKind::MissingDeprecatedMarker,
            FindingKind::StrayDeprecatedMarker,
            FindingKind::SnapshotAddedOperation,
            FindingKind::SnapshotAddedSchema,
            FindingKind::StaleExemption,
        ] {
            assert!(
                kinds.contains(&expected),
                "missing fixture coverage for {expected:?}"
            );
        }
    }

    #[test]
    fn enum_values_const_matching_produces_no_finding() {
        let models = r#"
            pub enum Color {
                #[serde(rename = "red")]
                Red,
                #[serde(rename = "blue")]
                Blue,
                #[serde(untagged)]
                Unknown(String),
            }
            impl Color {
                pub const VALUES: &'static [&'static str] = &["red", "blue"];
            }
        "#;
        let report = analyze_fixture(
            models,
            serde_json::json!({"type": "string", "enum": ["red", "blue"]}),
            AnalyzerConfig::default(),
        );
        let mismatch = report
            .findings
            .iter()
            .find(|f| f.kind == FindingKind::EnumValuesMismatch);
        assert!(
            mismatch.is_none(),
            "unexpected EnumValuesMismatch: {:?}",
            mismatch
        );
    }

    #[test]
    fn enum_values_const_missing_value_reports_mismatch() {
        let models = r#"
            pub enum Color {
                #[serde(rename = "red")]
                Red,
                #[serde(rename = "blue")]
                Blue,
                #[serde(rename = "green")]
                Green,
                #[serde(untagged)]
                Unknown(String),
            }
            impl Color {
                pub const VALUES: &'static [&'static str] = &["red", "blue"];
            }
        "#;
        let report = analyze_fixture(
            models,
            serde_json::json!({"type": "string", "enum": ["red", "blue", "green"]}),
            AnalyzerConfig::default(),
        );
        let finding = report
            .findings
            .iter()
            .find(|f| f.kind == FindingKind::EnumValuesMismatch)
            .expect("expected EnumValuesMismatch finding");
        assert_eq!(finding.details.get("enum"), Some(&"Color".to_string()));
        assert_eq!(finding.details.get("missing"), Some(&"green".to_string()));
        assert!(
            finding.rust_item.as_deref() == Some("models.rs::Color::VALUES"),
            "unexpected rust_item: {:?}",
            finding.rust_item
        );
    }

    #[test]
    fn enum_values_const_extra_value_reports_mismatch() {
        let models = r#"
            pub enum Color {
                #[serde(rename = "red")]
                Red,
                #[serde(rename = "blue")]
                Blue,
                #[serde(untagged)]
                Unknown(String),
            }
            impl Color {
                pub const VALUES: &'static [&'static str] = &["red", "blue", "green"];
            }
        "#;
        let report = analyze_fixture(
            models,
            serde_json::json!({"type": "string", "enum": ["red", "blue"]}),
            AnalyzerConfig::default(),
        );
        let finding = report
            .findings
            .iter()
            .find(|f| f.kind == FindingKind::EnumValuesMismatch)
            .expect("expected EnumValuesMismatch finding");
        assert_eq!(finding.details.get("enum"), Some(&"Color".to_string()));
        assert_eq!(finding.details.get("extra"), Some(&"green".to_string()));
    }

    #[test]
    fn enum_without_values_const_produces_no_mismatch_finding() {
        let models = r#"
            pub enum Color {
                #[serde(rename = "red")]
                Red,
                #[serde(rename = "blue")]
                Blue,
                #[serde(untagged)]
                Unknown(String),
            }
        "#;
        let report = analyze_fixture(
            models,
            serde_json::json!({"type": "string", "enum": ["red", "blue"]}),
            AnalyzerConfig::default(),
        );
        let mismatch = report
            .findings
            .iter()
            .find(|f| f.kind == FindingKind::EnumValuesMismatch);
        assert!(mismatch.is_none());
    }
}
