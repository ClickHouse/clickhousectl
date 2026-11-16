//! Shape checking of Rust types against spec schemas, in both directions.
//! Operation return checking is independent of snapshot and component membership.
//! Bind generic applications before walking fields; Option is transparent here
//! because requiredness is checked separately (request position only).
//! Request position reuses the same matcher for model fields and body arguments.
use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::compare::Direction;
use crate::config::AnalyzerConfig;
use crate::openapi::{OpenApiInventory, escape_pointer, pascalize};
use crate::report::{DriftReport, Finding, FindingKind};
use crate::rust_inventory::{RustInventory, TypeNode};

type Bindings = BTreeMap<String, TypeNode>;

fn substitute(ty: &TypeNode, bindings: &Bindings) -> TypeNode {
    match ty {
        TypeNode::Path(name) => bindings.get(name).cloned().unwrap_or_else(|| ty.clone()),
        TypeNode::Applied(name, args) => TypeNode::Applied(
            name.clone(),
            args.iter().map(|arg| substitute(arg, bindings)).collect(),
        ),
        TypeNode::Option(inner) => TypeNode::Option(Box::new(substitute(inner, bindings))),
        TypeNode::Vec(inner) => TypeNode::Vec(Box::new(substitute(inner, bindings))),
        TypeNode::Boxed(inner) => TypeNode::Boxed(Box::new(substitute(inner, bindings))),
        TypeNode::Reference(inner) => TypeNode::Reference(Box::new(substitute(inner, bindings))),
        TypeNode::Map(key, value) => TypeNode::Map(
            Box::new(substitute(key, bindings)),
            Box::new(substitute(value, bindings)),
        ),
        TypeNode::Other(_) => ty.clone(),
    }
}

fn type_name(ty: &TypeNode) -> Option<&str> {
    match ty {
        TypeNode::Path(name) | TypeNode::Applied(name, _) => Some(name),
        _ => None,
    }
}

impl RustInventory {
    fn bindings(&self, ty: &TypeNode) -> Result<Bindings, String> {
        let Some(name) = type_name(ty) else {
            return Ok(Bindings::new());
        };
        let args = match ty {
            TypeNode::Applied(_, args) => args.as_slice(),
            _ => &[],
        };
        let Some(parameters) = self.generic_parameters.get(name) else {
            return if args.is_empty() {
                Ok(Bindings::new())
            } else {
                Err(format!("unknown generic type {}", ty.display()))
            };
        };
        if args.len() > parameters.len() {
            return Err(format!("too many type arguments for {name}"));
        }
        let mut bindings = Bindings::new();
        for (index, (parameter, default)) in parameters.iter().enumerate() {
            let value = args
                .get(index)
                .cloned()
                .or_else(|| default.as_ref().map(|ty| substitute(ty, &bindings)))
                .ok_or_else(|| format!("unbound type parameter {name}::{parameter}"))?;
            bindings.insert(parameter.clone(), value);
        }
        Ok(bindings)
    }

    /// Resolve aliases (including generic aliases) and transparent wrappers.
    /// Alias cycles and expanding aliases are analysis failures, never matches.
    fn response_type(&self, ty: &TypeNode) -> Result<TypeNode, String> {
        let mut ty = ty.clone();
        let mut seen = BTreeSet::new();
        for _ in 0..128 {
            match ty {
                TypeNode::Option(inner) | TypeNode::Boxed(inner) | TypeNode::Reference(inner) => {
                    ty = *inner
                }
                _ => {
                    let Some(name) = type_name(&ty) else {
                        return Ok(ty);
                    };
                    let Some(alias) = self.aliases.get(name) else {
                        return Ok(ty);
                    };
                    if !seen.insert(ty.display()) {
                        return Err(format!("cyclic alias {}", ty.display()));
                    }
                    ty = substitute(alias, &self.bindings(&ty)?);
                }
            }
        }
        Err("type resolution exceeded 128 steps".into())
    }

    fn response_field(
        &self,
        ty: &TypeNode,
        field: &str,
        seen: &mut BTreeSet<String>,
    ) -> Result<Option<TypeNode>, String> {
        let ty = self.response_type(ty)?;
        if !seen.insert(ty.display()) {
            return Err(format!("cyclic flattened type {}", ty.display()));
        }
        let Some(name) = type_name(&ty) else {
            return Ok(None);
        };
        let Some(info) = self.structs.get(name) else {
            return Ok(None);
        };
        let bindings = self.bindings(&ty)?;
        if let Some(field) = info.fields.get(field) {
            return Ok(Some(substitute(&field.rust_type, &bindings)));
        }
        for flattened in info.fields.values().filter(|field| field.flatten) {
            let flattened = substitute(&flattened.rust_type, &bindings);
            if let Some(found) = self.response_field(&flattened, field, &mut seen.clone())? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
}

pub(crate) fn compare(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    config: &AnalyzerConfig,
    report: &mut DriftReport,
) {
    for response in &spec.success_responses {
        let Some(method) = rust.client_methods.get(&response.operation) else {
            continue;
        };
        // Envelope-only reference/composition cycles have no productive model
        // edge. Recursive object fields are handled separately by the pair guard.
        for pointer in response
            .unsupported
            .iter()
            .filter(|pointer| !pointer.ends_with("/oneOf") && !pointer.ends_with("/anyOf"))
        {
            report.findings.push(
                Finding::new(
                    FindingKind::UnsupportedResponseSchema,
                    "cyclic success response envelope reference/composition",
                )
                .at_spec(pointer)
                .at_rust(format!("client.rs::Client::{}", response.operation)),
            );
        }
        let Some(ty) = &method.success_type else {
            report.findings.push(
                Finding::new(
                    FindingKind::UnsupportedResponseSchema,
                    "success return type cannot be resolved",
                )
                .at_spec(&response.pointer)
                .at_rust(format!("client.rs::Client::{}", response.operation)),
            );
            continue;
        };
        let mut checker = Checker {
            rust,
            spec,
            config,
            report,
            direction: Direction::Response,
            context: ("operation", &response.operation),
            active: BTreeSet::new(),
        };
        checker.check(
            ty,
            &spec
                .document
                .pointer(&response.pointer)
                .cloned()
                .unwrap_or(Value::Null),
            &response.pointer,
            &format!("client.rs::Client::{}", response.operation),
            true,
        );
    }
}

/// Check one request-position model field against its property schema.
/// Presence and requiredness are owned by `compare_fields`; this checks shape.
/// A generic model's fields are checked with its default type arguments.
/// Returns the findings rather than recording them, so callers can apply
/// `request_shape_exemptions`.
pub(crate) fn check_request_field(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    config: &AnalyzerConfig,
    schema: &str,
    item: &str,
    ty: &TypeNode,
    pointer: &str,
) -> Vec<Finding> {
    let Some(property) = spec.document.pointer(pointer).cloned() else {
        return Vec::new();
    };
    let ty = rust
        .bindings(&TypeNode::Path(schema.to_owned()))
        .map_or_else(|_| ty.clone(), |bindings| substitute(ty, &bindings));
    let mut report = DriftReport::default();
    Checker {
        rust,
        spec,
        config,
        report: &mut report,
        direction: Direction::Request,
        context: ("schema", schema),
        active: BTreeSet::new(),
    }
    .check(&ty, &property, pointer, item, false);
    report.findings
}

/// Every operation with a JSON `requestBody` takes it as the one argument no
/// parameter accounts for, typed as the body schema's request-position model.
/// An inline object body may instead be passed as one argument per property.
/// A method whose operation has no `requestBody` takes no such argument.
pub(crate) fn compare_request_bodies(
    rust: &RustInventory,
    spec: &OpenApiInventory,
    config: &AnalyzerConfig,
    report: &mut DriftReport,
) {
    for (name, operation) in &spec.operations {
        let Some(method) = rust.client_methods.get(name) else {
            continue;
        };
        let consumed: BTreeSet<&str> = operation
            .parameters
            .keys()
            .filter_map(|(_, wire)| crate::compare::parameter_argument(method, wire, operation))
            .map(|(argument, _)| argument)
            .collect();
        let extra: Vec<(&String, &TypeNode)> = method
            .arguments
            .iter()
            .filter(|(argument, _)| !consumed.contains(argument.as_str()))
            .collect();
        let item = |argument: &str| format!("client.rs::Client::{name}::{argument}");
        let body_pointer = format!("{}/requestBody", operation.pointer);
        let mismatch = |message: String, pointer: &str, item: String| {
            Finding::new(FindingKind::RequestBodyMismatch, message)
                .at_spec(pointer)
                .at_rust(item)
                .detail("operation", name)
        };
        let Some((pointer, schema)) = request_body_schema(spec, &body_pointer) else {
            for (argument, ty) in extra {
                report.findings.push(
                    mismatch(
                        format!(
                            "{name} argument {argument} is neither a parameter nor a request body"
                        ),
                        &operation.pointer,
                        item(argument),
                    )
                    .detail("actual", ty.display()),
                );
            }
            continue;
        };
        let mut checker = Checker {
            rust,
            spec,
            config,
            report,
            direction: Direction::Request,
            context: ("operation", name),
            active: BTreeSet::new(),
        };
        if let [(argument, ty)] = extra.as_slice() {
            let resolved = rust.response_type(ty);
            if resolved
                .as_ref()
                .is_ok_and(|resolved| type_name(resolved) == Some("Value"))
            {
                checker.finding(
                    FindingKind::UnsupportedResponseSchema,
                    format!("{name} request body is untyped"),
                    &pointer,
                    &item(argument),
                    ty,
                );
                continue;
            }
            if exploded_property(&schema, argument).is_none() {
                checker.check(ty, &schema, &pointer, &item(argument), false);
                continue;
            }
        }
        // An inline object body passed property by property.
        let exploded: Option<Vec<_>> = extra
            .iter()
            .map(|(argument, ty)| {
                exploded_property(&schema, argument).map(|wire| (argument, ty, wire))
            })
            .collect();
        if let Some(exploded) = exploded.filter(|exploded| !exploded.is_empty()) {
            for (argument, ty, wire) in exploded {
                checker.check(
                    ty,
                    &schema["properties"][wire],
                    &format!("{pointer}/properties/{}", escape_pointer(wire)),
                    &item(argument),
                    false,
                );
            }
            continue;
        }
        if extra.len() != 1 {
            report.findings.push(
                mismatch(
                    format!(
                        "{name} needs exactly one request body argument, found {}",
                        extra.len()
                    ),
                    &pointer,
                    format!("client.rs::Client::{name}"),
                )
                .detail(
                    "arguments",
                    extra
                        .iter()
                        .map(|(argument, _)| argument.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            );
        }
    }
}

/// Whether a schema admits only `null`.
fn null_only(schema: &Value) -> bool {
    schema.get("type").and_then(Value::as_str) == Some("null")
        || ["oneOf", "anyOf"].iter().any(|composition| {
            schema
                .get(*composition)
                .and_then(Value::as_array)
                .is_some_and(|branches| !branches.is_empty() && branches.iter().all(null_only))
        })
}

/// The inline object body property that `argument` names, if any.
fn exploded_property<'a>(schema: &'a Value, argument: &str) -> Option<&'a str> {
    schema
        .get("properties")
        .and_then(Value::as_object)?
        .keys()
        .find(|wire| crate::openapi::camel_to_snake(wire) == argument)
        .map(String::as_str)
}

/// The JSON request body schema and its pointer, following one
/// `#/components/requestBodies` reference.
fn request_body_schema(spec: &OpenApiInventory, pointer: &str) -> Option<(String, Value)> {
    let body = spec.document.pointer(pointer)?;
    let pointer = body
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|reference| reference.strip_prefix('#'))
        .unwrap_or(pointer);
    let pointer = format!("{pointer}/content/application~1json/schema");
    spec.document
        .pointer(&pointer)
        .map(|schema| (pointer, schema.clone()))
}

struct Checker<'a> {
    rust: &'a RustInventory,
    spec: &'a OpenApiInventory,
    config: &'a AnalyzerConfig,
    report: &'a mut DriftReport,
    direction: Direction,
    /// The detail key and value naming what is checked: the operation for
    /// responses and request bodies, the owning schema for request fields.
    context: (&'static str, &'a str),
    active: BTreeSet<(String, String)>,
}

impl Checker<'_> {
    /// "response" or "request", for messages.
    fn side(&self) -> &'static str {
        match self.direction {
            Direction::Response => "response",
            Direction::Request => "request",
        }
    }

    /// Findings are raised with their response kinds and mapped here, so the
    /// matcher states each rule once for both directions.
    fn finding(
        &mut self,
        kind: FindingKind,
        message: impl Into<String>,
        pointer: &str,
        item: &str,
        ty: &TypeNode,
    ) {
        let kind = match (self.direction, kind) {
            (Direction::Request, FindingKind::ResponseShapeMismatch) => {
                FindingKind::RequestShapeMismatch
            }
            (Direction::Request, FindingKind::UnsupportedResponseSchema) => {
                FindingKind::UnsupportedRequestSchema
            }
            (_, kind) => kind,
        };
        if kind == FindingKind::MissingStructField
            && self.report.findings.iter().any(|finding| {
                finding.kind == kind
                    && finding.spec_pointer.as_deref() == Some(pointer)
                    && finding.rust_item.as_deref() == Some(item)
            })
        {
            return;
        }
        self.report.findings.push(
            Finding::new(kind, message)
                .at_spec(pointer)
                .at_rust(item)
                .detail(self.context.0, self.context.1)
                .detail("actual", ty.display()),
        );
    }

    fn check(&mut self, ty: &TypeNode, schema: &Value, pointer: &str, item: &str, root: bool) {
        let ty = match self.rust.response_type(ty) {
            Ok(ty) => ty,
            Err(reason) => {
                self.finding(
                    FindingKind::UnsupportedResponseSchema,
                    reason,
                    pointer,
                    item,
                    ty,
                );
                return;
            }
        };
        let key = (pointer.to_owned(), ty.display());
        // A repeated pair is a valid recursive model back-edge. Expanding
        // generic recursion is bounded and explicitly unsupported.
        if self.active.contains(&key) {
            return;
        }
        if self.active.len() >= 128 {
            self.finding(
                FindingKind::UnsupportedResponseSchema,
                "response analysis exceeded 128 nested shapes",
                pointer,
                item,
                &ty,
            );
            return;
        }
        self.active.insert(key.clone());
        self.check_inner(&ty, schema, pointer, item, root);
        self.active.remove(&key);
    }

    fn check_inner(
        &mut self,
        ty: &TypeNode,
        schema: &Value,
        pointer: &str,
        item: &str,
        root: bool,
    ) {
        // Value is a known lossless JSON catch-all. It accepts every payload
        // shape; it cannot claim typed envelope field coverage at the root.
        if !root && type_name(ty) == Some("Value") {
            return;
        }
        // A field the spec pins to null (`anyOf: [{type: null}]`) is unit.
        if matches!(ty, TypeNode::Other(unit) if unit == "()") && null_only(schema) {
            return;
        }
        if matches!(ty, TypeNode::Other(_)) {
            self.finding(
                FindingKind::UnsupportedResponseSchema,
                format!("unsupported Rust {} type", self.side()),
                pointer,
                item,
                ty,
            );
            return;
        }
        if let TypeNode::Applied(name, _) = ty
            && !self.rust.model_types.contains(name)
            && name != "DateTime"
        {
            self.finding(
                FindingKind::UnsupportedResponseSchema,
                "unknown generic response encoding",
                pointer,
                item,
                ty,
            );
            return;
        }
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            let Some(target) = reference.strip_prefix('#') else {
                self.finding(
                    FindingKind::UnsupportedResponseSchema,
                    format!("external {} reference", self.side()),
                    pointer,
                    item,
                    ty,
                );
                return;
            };
            let Some(resolved) = self.spec.document.pointer(target).cloned() else {
                self.finding(
                    FindingKind::UnsupportedResponseSchema,
                    format!("unresolved {} reference", self.side()),
                    pointer,
                    item,
                    ty,
                );
                return;
            };
            if let Some(name) = target.strip_prefix("/components/schemas/") {
                let base = pascalize(&name.replace("~1", "/").replace("~0", "~"));
                let expected = match self.direction {
                    Direction::Response => {
                        crate::compare::response_position_type(self.rust, self.spec, &base)
                    }
                    Direction::Request => base,
                };
                if !root && self.rust.model_types.contains(&expected) {
                    let expected = self.rust.response_type(&TypeNode::Path(expected.clone()));
                    // A request type may be generic over a caller-chosen
                    // parameter (`Request<T>`); its name identifies the model.
                    let same = |expected: &TypeNode| match (self.direction, type_name(expected)) {
                        (Direction::Request, Some(name)) => type_name(ty) == Some(name),
                        _ => expected == ty,
                    };
                    if expected.as_ref().is_ok_and(|expected| !same(expected)) {
                        self.finding(
                            FindingKind::ResponseShapeMismatch,
                            format!(
                                "{} {} expects {}, found {}",
                                self.context.1,
                                self.side(),
                                expected.unwrap().display(),
                                ty.display()
                            ),
                            pointer,
                            item,
                            ty,
                        );
                        return;
                    }
                    // A request-position named struct has its own field checks;
                    // re-walking it here would only duplicate them.
                    if self.direction == Direction::Request
                        && type_name(ty).is_some_and(|name| self.rust.structs.contains_key(name))
                    {
                        return;
                    }
                }
                // Named unions are checked by the existing direction-aware
                // enum/inline-variant inventory. Never grant this to Value.
                if (resolved.get("oneOf").is_some() || resolved.get("anyOf").is_some())
                    && self
                        .rust
                        .response_type(&TypeNode::Path(expected))
                        .as_ref()
                        .is_ok_and(|expected| expected == ty)
                    && type_name(ty).is_some_and(|name| {
                        self.rust
                            .enums
                            .get(name)
                            .is_some_and(|info| !info.is_value_enum)
                    })
                {
                    return;
                }
            }
            self.check(ty, &resolved, target, item, root);
            // OpenAPI 3.1 reference siblings retain their own pointers.
        }
        for composition in ["oneOf", "anyOf"] {
            if let Some(branches) = schema.get(composition).and_then(Value::as_array) {
                let non_null: Vec<_> = branches
                    .iter()
                    .enumerate()
                    .filter(|(_, branch)| {
                        branch.get("type").and_then(Value::as_str) != Some("null")
                    })
                    .collect();
                let union = type_name(ty).is_some_and(|name| {
                    self.rust
                        .enums
                        .get(name)
                        .is_some_and(|info| !info.is_value_enum)
                });
                // A nullable or single-branch composition is its one branch;
                // a one-variant union keeps its own variant checks.
                if non_null.len() == 1 && (non_null.len() < branches.len() || !union) {
                    let (index, branch) = non_null[0];
                    self.check(
                        ty,
                        branch,
                        &format!("{pointer}/{composition}/{index}"),
                        item,
                        root,
                    );
                } else if union {
                    // Existing inline union and enum checks own variant coverage.
                } else {
                    self.finding(
                        FindingKind::UnsupportedResponseSchema,
                        format!("unsupported {} {composition} composition", self.side()),
                        &format!("{pointer}/{composition}"),
                        item,
                        ty,
                    );
                }
                return;
            }
        }
        if let Some(branches) = schema.get("allOf").and_then(Value::as_array) {
            for (index, branch) in branches.iter().enumerate() {
                self.check(ty, branch, &format!("{pointer}/allOf/{index}"), item, root);
            }
        }
        if let Some(types) = schema.get("type").and_then(Value::as_array) {
            for kind in types.iter().filter(|kind| *kind != "null") {
                let mut branch = schema.clone();
                branch["type"] = kind.clone();
                self.check_inner(ty, &branch, pointer, item, root);
            }
            return;
        }
        let kind = schema
            .get("type")
            .and_then(Value::as_str)
            .or_else(|| schema.get("properties").map(|_| "object"));
        let scalar = type_name(ty).unwrap_or("");
        let integer = matches!(
            scalar,
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize"
        ) || self
            .rust
            .enums
            .get(scalar)
            .is_some_and(|info| info.uses_i64_serde_conversion);
        let matches = match kind {
            Some("string") => {
                matches!(
                    scalar,
                    "String" | "str" | "Uuid" | "DateTime" | "NaiveDateTime" | "NaiveDate"
                ) || self
                    .rust
                    .enums
                    .get(scalar)
                    .is_some_and(|info| info.is_value_enum && !info.uses_i64_serde_conversion)
            }
            Some("integer") => {
                integer
                    || self.direction == Direction::Response
                        && self
                            .config
                            .fractional_response_exemptions
                            .iter()
                            .any(|(name, field)| {
                                item == format!("models.rs::{name}::{field}") && scalar == "f64"
                            })
            }
            Some("number") => integer || matches!(scalar, "f32" | "f64"),
            Some("boolean") => scalar == "bool",
            Some("array") => {
                if let TypeNode::Vec(inner) = ty {
                    if let Some(items) = schema.get("items") {
                        self.check(inner, items, &format!("{pointer}/items"), item, false);
                    } else {
                        self.finding(
                            FindingKind::UnsupportedResponseSchema,
                            format!("array {} has no item schema", self.side()),
                            pointer,
                            item,
                            ty,
                        );
                    }
                    true
                } else {
                    false
                }
            }
            Some("object") => {
                // `properties: {}` beside `additionalProperties` declares no
                // named fields, so the dynamic-key rules below apply.
                if let Some(properties) = schema
                    .get("properties")
                    .and_then(Value::as_object)
                    .filter(|properties| {
                        !properties.is_empty() || schema.get("additionalProperties").is_none()
                    })
                {
                    if !self.rust.structs.contains_key(scalar) {
                        if root {
                            self.finding(
                                FindingKind::UnsupportedResponseSchema,
                                "success envelope has no identifiable returned struct",
                                pointer,
                                item,
                                ty,
                            );
                            return;
                        }
                        false
                    } else {
                        for (field, schema) in properties {
                            let pointer = format!("{pointer}/properties/{}", escape_pointer(field));
                            let item = format!("models.rs::{scalar}::{field}");
                            match self.rust.response_field(ty, field, &mut BTreeSet::new()) {
                                Ok(Some(field_type)) => {
                                    self.check(&field_type, schema, &pointer, &item, false)
                                }
                                Ok(None) => self.finding(
                                    FindingKind::MissingStructField,
                                    match self.direction {
                                        Direction::Response => format!(
                                            "{scalar}.{field} is missing from the {} success response",
                                            self.context.1
                                        ),
                                        Direction::Request => format!(
                                            "{scalar}.{field} is missing from the {} request",
                                            self.context.1
                                        ),
                                    },
                                    &pointer,
                                    &item,
                                    ty,
                                ),
                                Err(reason) => self.finding(
                                    FindingKind::UnsupportedResponseSchema,
                                    reason,
                                    &pointer,
                                    &item,
                                    ty,
                                ),
                            }
                        }
                        true
                    }
                } else if let Some(additional) =
                    schema.get("additionalProperties").filter(|value| {
                        value.is_object()
                            && value.as_object().is_some_and(|object| !object.is_empty())
                    })
                {
                    let bindings = match self.rust.bindings(ty) {
                        Ok(bindings) => bindings,
                        Err(reason) => {
                            self.finding(
                                FindingKind::UnsupportedResponseSchema,
                                reason,
                                pointer,
                                item,
                                ty,
                            );
                            return;
                        }
                    };
                    let flattened = self
                        .rust
                        .structs
                        .get(scalar)
                        .and_then(|info| info.fields.values().find(|field| field.flatten))
                        .map(|field| {
                            self.rust
                                .response_type(&substitute(&field.rust_type, &bindings))
                        });
                    let flattened = match flattened.transpose() {
                        Ok(flattened) => flattened,
                        Err(reason) => {
                            self.finding(
                                FindingKind::UnsupportedResponseSchema,
                                reason,
                                pointer,
                                item,
                                ty,
                            );
                            return;
                        }
                    };
                    if let TypeNode::Map(key, value) = flattened.as_ref().unwrap_or(ty) {
                        if self
                            .rust
                            .response_type(key)
                            .is_ok_and(|key| key == TypeNode::Path("String".into()))
                        {
                            self.check(
                                value,
                                additional,
                                &format!("{pointer}/additionalProperties"),
                                item,
                                false,
                            );
                            true
                        } else {
                            false
                        }
                    } else {
                        // Named dynamic-key schemas have a dedicated diagnostic.
                        if pointer.starts_with("/components/schemas/")
                            && self.report.findings.iter().any(|finding| {
                                finding.kind == FindingKind::AdditionalPropertiesMismatch
                                    && finding.spec_pointer.as_deref()
                                        == Some(&format!("{pointer}/additionalProperties"))
                            })
                        {
                            return;
                        }
                        false
                    }
                } else {
                    // An unconstrained object has no typed fields to verify.
                    scalar == "Value"
                        || self.rust.structs.contains_key(scalar)
                        || matches!(ty, TypeNode::Map(_, _))
                }
            }
            Some("null") | None => true,
            Some(_) => {
                self.finding(
                    FindingKind::UnsupportedResponseSchema,
                    format!("unsupported {} schema type", self.side()),
                    pointer,
                    item,
                    ty,
                );
                return;
            }
        };
        if !matches {
            self.finding(
                if scalar == "Value" || matches!(ty, TypeNode::Other(_)) {
                    FindingKind::UnsupportedResponseSchema
                } else {
                    FindingKind::ResponseShapeMismatch
                },
                format!(
                    "{} {} expects {}, found {}",
                    self.context.1,
                    self.side(),
                    kind.unwrap_or("unknown"),
                    ty.display()
                ),
                pointer,
                item,
                ty,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(return_type: &str, models: &str, payload: Value) -> DriftReport {
        let config = AnalyzerConfig::default();
        let spec = serde_json::json!({
            "paths": {"/widgets/~": {"get": {"operationId":"listWidgets", "responses":{"200":{"content":{"application/json":{"schema":{
                "type":"object", "required":["result"], "properties":{"result":payload}
            }}}}}}}}, "components":{"schemas":{}}
        });
        let spec = OpenApiInventory::build(&spec, &config).unwrap();
        let rust = RustInventory::parse(
            &format!("impl Client {{ pub async fn list_widgets(&self) -> Result<{return_type}, Error> {{}} }}"),
            models, "",
        ).unwrap();
        crate::compare::compare(&rust, &spec, &spec, &config)
    }

    #[test]
    fn generic_aliases_defaults_and_flattened_bindings_reach_the_payload() {
        let models = r#"
            pub type Page<T> = Envelope<Option<T>>;
            pub struct Envelope<T, Meta = String> { #[serde(flatten)] pub body: Body<T>, pub unused: Option<Meta> }
            pub struct Body<T> { pub result: Option<Box<T>> }
            pub type Rows<T> = Vec<T>;
            pub struct Dynamic<T> { #[serde(flatten)] pub values: BTreeMap<String, T> }
        "#;
        for (ty, payload) in [
            (
                "Page<Rows<String>>",
                serde_json::json!({"type":"array","items":{"type":"string"}}),
            ),
            (
                "Page<BTreeMap<String, Vec<bool>>>",
                serde_json::json!({"type":"object","additionalProperties":{"type":"array","items":{"type":"boolean"}}}),
            ),
            (
                "Page<Dynamic<Vec<bool>>>",
                serde_json::json!({"type":"object","additionalProperties":{"type":"array","items":{"type":"boolean"}}}),
            ),
        ] {
            let report = report(ty, models, payload);
            assert!(!report.has_drift(), "{}", report.render_text());
        }
        let report = report(
            "Page<Rows<i64>>",
            models,
            serde_json::json!({"type":"array","items":{"type":"string"}}),
        );
        assert_eq!(report.findings.len(), 1, "{}", report.render_text());
        assert_eq!(report.findings[0].kind, FindingKind::ResponseShapeMismatch);
        assert!(
            report.findings[0]
                .spec_pointer
                .as_ref()
                .unwrap()
                .ends_with("/result/items")
        );
        assert_eq!(
            report.findings[0].rust_item.as_deref(),
            Some("models.rs::Envelope::result")
        );
    }

    #[test]
    fn payload_kind_changes_are_not_hidden_by_option_or_snapshot_refresh() {
        for (rust_type, schema) in [
            (
                "OldPage",
                serde_json::json!({"type":"array","items":{"type":"string"}}),
            ),
            (
                "Vec<String>",
                serde_json::json!({"type":"object","properties":{"items":{"type":"array","items":{"type":"string"}}}}),
            ),
            ("String", serde_json::json!({"type":"integer"})),
            (
                "Vec<i64>",
                serde_json::json!({"type":"array","items":{"type":"string"}}),
            ),
            (
                "BTreeMap<String, i64>",
                serde_json::json!({"type":"object","additionalProperties":{"type":"boolean"}}),
            ),
        ] {
            let report = report(
                &format!("Envelope<{rust_type}>"),
                "pub struct Envelope<T> { pub result: Option<T> } pub struct OldPage { pub items: Option<Vec<String>> }",
                schema,
            );
            assert!(
                report
                    .findings
                    .iter()
                    .any(|finding| finding.kind == FindingKind::ResponseShapeMismatch),
                "{}",
                report.render_text()
            );
            assert!(
                !report
                    .findings
                    .iter()
                    .any(|finding| format!("{:?}", finding.kind).starts_with("Snapshot"))
            );
        }
    }

    #[test]
    fn nullable_forms_preserve_response_optionality() {
        for schema in [
            serde_json::json!({"type":"integer","nullable":true}),
            serde_json::json!({"type":["integer","null"]}),
            serde_json::json!({"anyOf":[{"type":"integer"},{"type":"null"}]}),
            serde_json::json!({"oneOf":[{"type":"null"},{"type":"integer"}]}),
        ] {
            let report = report(
                "Envelope<Option<i64>>",
                "pub struct Envelope<T> { pub result: Option<T> }",
                schema,
            );
            assert!(!report.has_drift(), "{}", report.render_text());
        }
    }

    #[test]
    fn unsupported_compositions_unbound_generics_and_alias_cycles_are_actionable() {
        for (ty, models, schema) in [
            (
                "Envelope<String>",
                "pub struct Envelope<T> { pub result: Option<T> }",
                serde_json::json!({"oneOf":[{"type":"integer"},{"type":"boolean"}]}),
            ),
            (
                "Envelope",
                "pub struct Envelope<T> { pub result: Option<T> }",
                serde_json::json!({"type":"string"}),
            ),
            (
                "Envelope<Cycle>",
                "pub struct Envelope<T> { pub result: Option<T> } pub type Cycle = Other; pub type Other = Cycle;",
                serde_json::json!({"type":"string"}),
            ),
            (
                "Envelope<External<String>>",
                "pub struct Envelope<T> { pub result: Option<T> }",
                serde_json::json!({"type":"string"}),
            ),
            (
                "Envelope<External<String, 3>>",
                "pub struct Envelope<T> { pub result: Option<T> }",
                serde_json::json!({"type":"string"}),
            ),
        ] {
            let report = report(ty, models, schema);
            assert!(
                report
                    .findings
                    .iter()
                    .any(|finding| finding.kind == FindingKind::UnsupportedResponseSchema),
                "{}",
                report.render_text()
            );
        }
    }

    #[test]
    fn unproductive_schema_reference_cycles_are_explicitly_unsupported() {
        let config = AnalyzerConfig::default();
        let spec = serde_json::json!({
            "paths":{"/widgets":{"get":{"operationId":"listWidgets","responses":{"200":{"content":{"application/json":{"schema":{"$ref":"#/components/schemas/Page"}}}}}}}},
            "components":{"schemas":{"Page":{"$ref":"#/components/schemas/Page"}}}
        });
        let inventory = OpenApiInventory::build(&spec, &config).unwrap();
        let rust = RustInventory::parse(
            "impl Client {pub async fn list_widgets(&self) -> Result<Page, Error> {}}",
            "pub struct Page;",
            "",
        )
        .unwrap();
        let report = crate::compare::compare(&rust, &inventory, &inventory, &config);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == FindingKind::UnsupportedResponseSchema),
            "{}",
            report.render_text()
        );
        let mut recursive = spec;
        recursive["components"]["schemas"]["Page"] = serde_json::json!({
            "type":"object", "properties":{"next":{"$ref":"#/components/schemas/Page"}}
        });
        let inventory = OpenApiInventory::build(&recursive, &config).unwrap();
        let rust = RustInventory::parse(
            "impl Client {pub async fn list_widgets(&self) -> Result<Page, Error> {}}",
            "pub struct Page {pub next: Option<Box<Page>>}",
            "",
        )
        .unwrap();
        let report = crate::compare::compare(&rust, &inventory, &inventory, &config);
        assert!(!report.has_drift(), "{}", report.render_text());
    }

    #[test]
    fn array_element_references_use_the_response_model_and_reject_other_models() {
        let config = AnalyzerConfig::default();
        let spec = serde_json::json!({
            "paths":{"/widgets":{"get":{"operationId":"listWidgets","responses":{"200":{"content":{"application/json":{"schema":{
                "properties":{"result":{"type":"array","items":{"$ref":"#/components/schemas/Widget"}}}
            }}}}}}}},
            "components":{"schemas":{"Widget":{"properties":{"name":{"type":"string"}}}}}
        });
        let spec = OpenApiInventory::build(&spec, &config).unwrap();
        for (element, expected) in [("Widget", false), ("Wrong", true), ("String", true)] {
            let rust = RustInventory::parse(
                &format!("impl Client {{pub async fn list_widgets(&self) -> Result<Envelope<Vec<{element}>>, Error> {{}}}}"),
                "pub struct Envelope<T> {pub result: Option<T>} pub struct Widget {pub name: Option<String>} pub struct Wrong {pub name: Option<String>}", "",
            ).unwrap();
            let report = crate::compare::compare(&rust, &spec, &spec, &config);
            assert_eq!(report.has_drift(), expected, "{}", report.render_text());
            if expected {
                assert_eq!(report.findings.len(), 1);
                assert_eq!(report.findings[0].kind, FindingKind::ResponseShapeMismatch);
                assert!(
                    report.findings[0]
                        .spec_pointer
                        .as_ref()
                        .unwrap()
                        .ends_with("/result/items")
                );
            }
        }
    }

    #[test]
    fn resolved_response_contract_deltas_ignore_prose_but_include_nested_and_status_changes() {
        let config = AnalyzerConfig::default();
        let before = serde_json::json!({
            "paths":{"/widgets":{"get":{"operationId":"listWidgets","responses":{"200":{"content":{"application/json":{"schema":{"$ref":"#/components/schemas/Page"}}}}}}}},
            "components":{"schemas":{"Page":{"properties":{"result":{"type":"string","description":"old prose"}}}}}
        });
        let previous = OpenApiInventory::build(&before, &config).unwrap();
        let rust = RustInventory::parse(
            "impl Client {pub async fn list_widgets(&self)-> Result<Page, Error> {}}",
            "pub struct Page {pub result: Option<String>}",
            "",
        )
        .unwrap();
        let mut after = before.clone();
        after["components"]["schemas"]["Page"]["properties"]["result"]["description"] =
            "new prose".into();
        after["components"]["schemas"]["Page"]["properties"]["result"]["example"] =
            "example".into();
        let target = OpenApiInventory::build(&after, &config).unwrap();
        assert!(!crate::compare::compare(&rust, &target, &previous, &config).has_drift());
        after["components"]["schemas"]["Page"]["properties"]["result"]["type"] = "integer".into();
        let target = OpenApiInventory::build(&after, &config).unwrap();
        let report = crate::compare::compare(&rust, &target, &previous, &config);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == FindingKind::ResponseShapeMismatch)
        );
        let changed = report
            .findings
            .iter()
            .find(|finding| finding.kind == FindingKind::SnapshotChangedResponse)
            .unwrap();
        assert!(changed.details["previous_contract"].contains("string"));
        assert!(changed.details["current_contract"].contains("integer"));
        let response = after["paths"]["/widgets"]["get"]["responses"]
            .as_object_mut()
            .unwrap()
            .remove("200")
            .unwrap();
        after["paths"]["/widgets"]["get"]["responses"]["201"] = response;
        let target = OpenApiInventory::build(&after, &config).unwrap();
        let report = crate::compare::compare(&rust, &target, &previous, &config);
        for kind in [
            FindingKind::SnapshotAddedResponse,
            FindingKind::SnapshotRemovedResponse,
        ] {
            let finding = report
                .findings
                .iter()
                .find(|finding| finding.kind == kind)
                .unwrap();
            assert!(
                after
                    .pointer(finding.spec_pointer.as_ref().unwrap())
                    .is_some()
            );
        }
    }
}
