use std::collections::{BTreeSet, HashMap};

use clickhouse_cloud_api::{BETA_OPERATIONS, DEPRECATED_OUTPUT_FIELDS};
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

/// Public client methods that intentionally don't correspond to an OpenAPI
/// operation. The Query API endpoint that backs `run_query` is hosted at
/// `queries.clickhouse.cloud` and is not described by the control-plane spec.
const NON_OPENAPI_CLIENT_METHODS: &[&str] = &["run_query"];

fn assert_client_operation_coverage(spec: &Value) {
    let spec_operations = spec_operation_ids(spec);
    let client_methods = public_items(CLIENT_RS, "pub async fn ");
    let exempt: BTreeSet<String> = NON_OPENAPI_CLIENT_METHODS
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    let missing: Vec<_> = spec_operations.difference(&client_methods).cloned().collect();
    let extras: Vec<_> = client_methods
        .difference(&spec_operations)
        .filter(|m| !exempt.contains(m.as_str()))
        .cloned()
        .collect();

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
            if let Some(reference) = map.get("$ref").and_then(Value::as_str)
                && let Some(schema_name) = reference.strip_prefix("#/components/schemas/")
            {
                refs.insert(schema_name.to_string());
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

#[test]
fn struct_fields_cover_every_spec_property() {
    assert_field_coverage(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn struct_fields_cover_every_live_spec_property() {
    let spec = load_live_spec().await;
    assert_field_coverage(&spec);
}

/// Fields where our `Option<T>` vs `T` intentionally disagrees with the spec.
///
/// The spec sometimes marks fields as required when the API actually treats them
/// as optional (e.g. sending an empty string triggers a 400). Each entry is
/// `("RustStructName", "specFieldName")` with a comment explaining why the
/// override exists. When the spec is corrected upstream, remove the entry —
/// the test will confirm the fix by passing without it.
/// Fields where our `Option<T>` vs `T` intentionally disagrees with the spec.
///
/// The OpenAPI spec marks most ServicePostRequest fields as required (via the
/// description-heuristic — none start with "Optional"), but the API actually
/// treats them as optional. Sending zero-value defaults for these fields causes
/// 400 errors (invalid UUIDs, conflicting memory params, unsupported tiers, etc.).
/// Only `name`, `provider`, and `region` are truly required.
const OPTIONALITY_EXEMPTIONS: &[(&str, &str)] = &[
    ("ServicePostRequest", "byocId"),
    ("ServicePostRequest", "complianceType"),
    ("ServicePostRequest", "dataWarehouseId"),
    ("ServicePostRequest", "enableCoreDumps"),
    ("ServicePostRequest", "endpoints"),
    ("ServicePostRequest", "hasTransparentDataEncryption"),
    ("ServicePostRequest", "idleScaling"),
    ("ServicePostRequest", "idleTimeoutMinutes"),
    ("ServicePostRequest", "isReadonly"),
    ("ServicePostRequest", "maxReplicaMemoryGb"),
    ("ServicePostRequest", "maxTotalMemoryGb"),
    ("ServicePostRequest", "minReplicaMemoryGb"),
    ("ServicePostRequest", "minTotalMemoryGb"),
    ("ServicePostRequest", "numReplicas"),
    ("ServicePostRequest", "privateEndpointIds"),
    ("ServicePostRequest", "privatePreviewTermsChecked"),
    ("ServicePostRequest", "profile"),
    ("ServicePostRequest", "releaseChannel"),
    ("ServicePostRequest", "tags"),
    ("ServicePostRequest", "tier"),
    // ClickPipe{Post,Patch}Source.postgres is a $ref without the `oneOf: [ref,
    // null]` wrapper every other source uses, so the description-heuristic
    // infers "required" and the generator emits `T`. The API actually treats
    // postgres as optional — non-postgres creates reject on `postgres.host: ''`
    // when the empty default source is serialized. Modeled as `Option<T>` to
    // match real API behavior; spec bug to be fixed upstream.
    ("ClickPipePostSource", "postgres"),
    ("ClickPipePatchSource", "postgres"),
    // ClickPipeScaling sub-object: when default-serialized as {replicas: 0, …}
    // the API rejects ("replicas: Not between 1 and 40"). Spec heuristic marks
    // this as required, but in practice callers either set real values or
    // want it omitted entirely. Modeled as `Option<T>`.
    ("ClickPipePostRequest", "scaling"),
    // settings similarly rejects `{}` — either send real values or omit.
    ("ClickPipePostRequest", "settings"),
    // Both publicationName and replicationSlotName must be ABSENT (not just
    // empty) for `cdc` mode — the API rejects "" with "replicationSlotName: ''"
    // and rejects any value with "only valid for cdc_only mode". Spec heuristic
    // marks them as required because the schema has no `required` array, but
    // the field descriptions explicitly call them optional. Modeled as
    // `Option<String>` so callers can omit them.
    ("ClickPipePostgresPipeSettings", "publicationName"),
    ("ClickPipePostgresPipeSettings", "replicationSlotName"),
    // Numeric settings all carry `minimum: 1` (or `minimum: 1000` for
    // snapshotNumRowsPerPartition). The schema has no `required` array, so the
    // heuristic infers required for everyone and the generator emits bare `i64`.
    // `Default::default()` gives `0`, which the API rejects with
    // "Value must be >= 1". Confirmed via cloud_clickpipe_postgres_ec2 that the
    // API accepts the request when these keys are absent and picks server-side
    // defaults — so they're modelled as `Option<i64>` with skip_serializing_if.
    ("ClickPipePostgresPipeSettings", "syncIntervalSeconds"),
    ("ClickPipePostgresPipeSettings", "pullBatchSize"),
    ("ClickPipePostgresPipeSettings", "initialLoadParallelism"),
    ("ClickPipePostgresPipeSettings", "snapshotNumRowsPerPartition"),
    ("ClickPipePostgresPipeSettings", "snapshotNumberOfParallelTables"),
    // Four destination fields are "Required field for all pipe types except
    // database pipes (Postgres, MySQL, BigQuery)" per their descriptions, but
    // the schema lacks a `required` array so the heuristic infers required
    // for everyone. Live API rejects empty defaults for database pipes (e.g.
    // "destination.table: ''", "columns array length < minLength"). Modeled
    // as Optional so database pipes can omit the whole group.
    ("ClickPipeMutateDestination", "table"),
    ("ClickPipeMutateDestination", "managedTable"),
    ("ClickPipeMutateDestination", "tableDefinition"),
    // caCertificate is `undefinedOr(isValidPEMCertificate)` server-side, so
    // sending "" fails PEM validation. Modeled as Option<String> so callers
    // omit it when the user doesn't pass --ca-certificate.
    ("ClickPipeMutatePostgresSource", "caCertificate"),
    // iamRole only applies to RDS-style Postgres with IAM_ROLE auth — for
    // Basic-auth Postgres the API rejects an empty string. Spec heuristic
    // marks required because the schema lacks a `required` array; modeled
    // as Option<String> so non-RDS callers omit it.
    ("ClickPipeMutatePostgresSource", "iamRole"),
    // tlsHost is only used when the broker cert SAN doesn't match `host`.
    // Optional in practice; API rejects empty defaults.
    ("ClickPipeMutatePostgresSource", "tlsHost"),
    // `roles` is deprecated in favour of `assignedRoleIds`. The spec marks it
    // required (description heuristic) with `minLength=1`, so the generated
    // `Vec<String>` would serialize as `"roles":[]` and the API would reject
    // it. Model it as `Option<Vec<String>>` so callers using `assignedRoleIds`
    // can omit `roles` entirely.
    ("ApiKeyPostRequest", "roles"),
    // `hashData` lets callers pre-hash an API key instead of having the API
    // generate one. The spec marks it required (description heuristic), but
    // the API treats it as opt-in: sending the default object yields
    // `BAD_REQUEST: hashData.keyIdHash: Not a sha256sum`. Modelling it as
    // `Option<ApiKeyHashData>` lets callers omit it and have the API
    // generate the key as the spec's response description implies.
    ("ApiKeyPostRequest", "hashData"),
];

fn assert_field_optionality(spec: &Value) {
    let schemas = spec["components"]["schemas"].as_object().unwrap();
    let model_fields = parse_model_fields(MODELS_RS);

    let exemptions: BTreeSet<(&str, &str)> = OPTIONALITY_EXEMPTIONS.iter().copied().collect();
    let mut exemptions_hit: BTreeSet<(&str, &str)> = BTreeSet::new();
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

            let would_mismatch = (is_required && field_info.is_option)
                || (!is_required && !field_info.is_option);

            if !would_mismatch {
                continue;
            }

            // Check if this mismatch is exempted. We need to borrow rust_name
            // as &str for the lookup.
            if exemptions.iter().any(|(s, f)| *s == rust_name && *f == prop_name.as_str()) {
                exemptions_hit.insert(
                    *exemptions.iter().find(|(s, f)| *s == rust_name && *f == prop_name.as_str()).unwrap()
                );
                let direction = if is_required {
                    "spec=required, model=Option<T>"
                } else {
                    "spec=optional, model=T"
                };
                eprintln!(
                    "NOTE: {}.{} optionality exempted ({}) — see OPTIONALITY_EXEMPTIONS",
                    rust_name, prop_name, direction
                );
                continue;
            }

            if is_required && field_info.is_option {
                mismatches.push(format!(
                    "{}.{} should be required (T) but is Option<T>",
                    rust_name, prop_name
                ));
            } else {
                mismatches.push(format!(
                    "{}.{} should be optional (Option<T>) but is T",
                    rust_name, prop_name
                ));
            }
        }
    }

    // Detect stale exemptions — entries that no longer trigger a mismatch
    let stale: Vec<_> = exemptions
        .difference(&exemptions_hit)
        .map(|(s, f)| format!("({}, {})", s, f))
        .collect();
    if !stale.is_empty() {
        eprintln!(
            "NOTE: {} stale optionality exemption(s) can be removed: {}",
            stale.len(),
            stale.join(", ")
        );
    }
    assert!(
        stale.is_empty(),
        "Stale OPTIONALITY_EXEMPTIONS (spec now agrees with model):\n{}",
        stale.join("\n")
    );

    assert!(
        mismatches.is_empty(),
        "Field optionality mismatches ({} total):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Assert that every property in the OpenAPI spec has a corresponding field
/// in the Rust struct. Catches fields added or renamed in the spec that never
/// made it into models.rs.
fn assert_field_coverage(spec: &Value) {
    let schemas = spec["components"]["schemas"].as_object().unwrap();
    let model_fields = parse_model_fields(MODELS_RS);

    let mut missing = Vec::new();

    for (spec_name, schema) in schemas {
        let rust_name = pascalize_identifier(spec_name);
        let fields = match model_fields.get(&rust_name) {
            Some(f) => f,
            None => continue, // Missing struct — covered by models_cover_every_openapi_component_schema
        };

        let props = match schema.get("properties").and_then(Value::as_object) {
            Some(p) => p,
            None => continue,
        };

        for prop_name in props.keys() {
            if !fields.contains_key(prop_name.as_str()) {
                missing.push(format!("{}.{}", rust_name, prop_name));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Spec properties missing from Rust structs ({} total):\n{}",
        missing.len(),
        missing.join("\n")
    );
}

/// Schemas where the spec's `required` array lists only newly-added fields;
/// older fields on the same schema still rely on the description heuristic.
/// For these we union `required[]` with the description heuristic instead of
/// treating `required[]` as exclusive.
///
/// Remove an entry once the spec is corrected upstream. `PARTIAL_REQUIRED_SCHEMAS`
/// is mirrored in `scripts/resolve-field-requirements.py`.
const PARTIAL_REQUIRED_SCHEMAS: &[&str] = &[
    "Service",
    "ServiceScalingPatchResponse",
];

/// Determine which fields in a schema are required AND non-nullable.
///
/// Resolution strategy:
/// 1. PATCH request schemas (name contains "Patch" and ends with "Request") → all optional.
/// 2. Schemas in `PARTIAL_REQUIRED_SCHEMAS` → required = `required[]` ∪ description heuristic.
/// 3. Schemas with a `required` array → use it.
/// 4. Otherwise → fields whose description does NOT start with "Optional" are required.
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

    let is_partial = PARTIAL_REQUIRED_SCHEMAS.contains(&schema_name);

    let required_names: BTreeSet<&str> = if is_partial {
        let mut names: BTreeSet<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        for (name, prop) in props {
            let desc = prop.get("description").and_then(Value::as_str).unwrap_or("");
            if !desc.starts_with("Optional") {
                names.insert(name.as_str());
            }
        }
        names
    } else if let Some(required) = schema.get("required").and_then(Value::as_array) {
        required.iter().filter_map(Value::as_str).collect()
    } else {
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
    if let Some(types) = prop.get("type").and_then(Value::as_array)
        && types.iter().any(|t| t.as_str() == Some("null"))
    {
        return true;
    }
    // oneOf/anyOf with a null variant
    for key in &["oneOf", "anyOf"] {
        if let Some(variants) = prop.get(*key).and_then(Value::as_array)
            && variants.iter().any(|v| v.get("type").and_then(Value::as_str) == Some("null"))
        {
            return true;
        }
    }
    false
}

struct FieldInfo {
    is_option: bool,
    /// True if the field carries the
    /// `#[cfg_attr(not(feature = "deprecated-fields"), serde(skip_serializing))]`
    /// marker that hides it from serialized output by default.
    deprecated_marker: bool,
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
        if let Some(rest) = line.strip_prefix("pub struct ")
            && let Some(struct_name) = identifier_prefix(rest)
        {
            let struct_name = struct_name.to_string();
            i += 1;
            let mut fields: HashMap<String, FieldInfo> = HashMap::new();
            let mut pending_rename: Option<String> = None;
            let mut pending_deprecated_marker = false;

            while i < lines.len() {
                let line = lines[i].trim();

                if line == "}" {
                    break;
                }

                // Detect the deprecated-field hiding marker
                if line.contains("not(feature = \"deprecated-fields\")") {
                    pending_deprecated_marker = true;
                }

                // Extract rename from serde attribute
                if line.starts_with("#[serde(")
                    && let Some(rename) = extract_serde_rename(line)
                {
                    pending_rename = Some(rename.to_string());
                }

                // Extract field definition
                if let Some(rest) = line.strip_prefix("pub ")
                    && let Some(colon_pos) = rest.find(':')
                {
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

                    fields.insert(
                        spec_name,
                        FieldInfo {
                            is_option,
                            deprecated_marker: pending_deprecated_marker,
                        },
                    );
                    pending_deprecated_marker = false;
                }

                i += 1;
            }

            result.insert(struct_name, fields);
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

// ---------------------------------------------------------------------------
// Beta status (x-badges) coverage
// ---------------------------------------------------------------------------

#[test]
fn beta_operations_match_spec() {
    assert_beta_operations_match(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn beta_operations_match_live_spec() {
    let spec = load_live_spec().await;
    assert_beta_operations_match(&spec);
}

fn assert_beta_operations_match(spec: &Value) {
    let spec_beta: BTreeSet<String> = spec_beta_operation_ids(spec);
    let declared: BTreeSet<String> =
        BETA_OPERATIONS.iter().map(|s| (*s).to_string()).collect();

    let missing: Vec<_> = spec_beta.difference(&declared).cloned().collect();
    let extra: Vec<_> = declared.difference(&spec_beta).cloned().collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "BETA_OPERATIONS drifted from the OpenAPI spec.\n\
         New beta ops in spec, missing from meta.rs: {:?}\n\
         No longer beta in spec, still in meta.rs: {:?}\n\
         Regenerate with: python3 scripts/regenerate-beta-lists.py",
        missing,
        extra,
    );
}

fn spec_beta_operation_ids(spec: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for path_item in spec["paths"].as_object().unwrap().values() {
        for (method, operation) in path_item.as_object().unwrap() {
            if !matches!(
                method.as_str(),
                "get" | "put" | "post" | "delete" | "patch" | "options" | "head" | "trace"
            ) {
                continue;
            }
            let Some(badges) = operation.get("x-badges").and_then(Value::as_array) else {
                continue;
            };
            let is_beta = badges
                .iter()
                .any(|b| b.get("name").and_then(Value::as_str) == Some("Beta"));
            if is_beta {
                ids.insert(camel_to_snake(
                    operation["operationId"].as_str().unwrap(),
                ));
            }
        }
    }
    ids
}

// ---------------------------------------------------------------------------
// Deprecated output field hiding
// ---------------------------------------------------------------------------

/// Response-side deprecated fields that we deliberately keep visible in output,
/// even though the spec marks them `deprecated: true`. Each entry is
/// `("RustStructName", "specFieldName")`. Empty today — every deprecated
/// response field is hidden. The `deprecated_output_fields_match_spec` test
/// fails on a stale entry (one that no longer corresponds to a spec-deprecated
/// response field) so this list can't rot.
const DEPRECATED_OUTPUT_EXEMPTIONS: &[(&str, &str)] = &[];

/// `DEPRECATED_OUTPUT_FIELDS` must mirror the `deprecated: true` properties on
/// response-side schemas in the spec (minus `DEPRECATED_OUTPUT_EXEMPTIONS`).
#[test]
fn deprecated_output_fields_match_spec() {
    assert_deprecated_output_fields_match(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn deprecated_output_fields_match_live_spec() {
    let spec = load_live_spec().await;
    assert_deprecated_output_fields_match(&spec);
}

/// Every field declared in `DEPRECATED_OUTPUT_FIELDS` must carry the
/// `skip_serializing` marker in `models.rs`, and no other field may carry it.
/// This keeps the consumer-facing constant in lockstep with the actual serde
/// behaviour that hides the fields.
#[test]
fn deprecated_output_fields_hidden() {
    let marked = model_deprecated_marked_fields(MODELS_RS);
    let declared: BTreeSet<(String, String)> = DEPRECATED_OUTPUT_FIELDS
        .iter()
        .map(|(s, f)| (s.to_string(), f.to_string()))
        .collect();

    let missing_markers: Vec<_> = declared
        .difference(&marked)
        .map(|(s, f)| format!("{}.{}", s, f))
        .collect();
    let stray_markers: Vec<_> = marked
        .difference(&declared)
        .map(|(s, f)| format!("{}.{}", s, f))
        .collect();

    assert!(
        missing_markers.is_empty() && stray_markers.is_empty(),
        "DEPRECATED_OUTPUT_FIELDS is out of sync with the skip_serializing markers in models.rs.\n\
         Declared but not marked (add the cfg_attr marker): {:?}\n\
         Marked but not declared (add to DEPRECATED_OUTPUT_FIELDS or remove the marker): {:?}",
        missing_markers,
        stray_markers,
    );
}

fn assert_deprecated_output_fields_match(spec: &Value) {
    let exemptions: BTreeSet<(&str, &str)> =
        DEPRECATED_OUTPUT_EXEMPTIONS.iter().copied().collect();
    let spec_fields = spec_deprecated_output_fields(spec);

    // Stale-exemption detection: an exemption must correspond to a field the
    // spec actually marks deprecated on a response-side schema.
    let stale: Vec<_> = exemptions
        .iter()
        .filter(|(s, f)| !spec_fields.contains(&((*s).to_string(), (*f).to_string())))
        .map(|(s, f)| format!("({}, {})", s, f))
        .collect();
    assert!(
        stale.is_empty(),
        "Stale DEPRECATED_OUTPUT_EXEMPTIONS (no longer a deprecated response field):\n{}",
        stale.join("\n")
    );

    let expected: BTreeSet<(String, String)> = spec_fields
        .into_iter()
        .filter(|(s, f)| !exemptions.contains(&(s.as_str(), f.as_str())))
        .collect();
    let declared: BTreeSet<(String, String)> = DEPRECATED_OUTPUT_FIELDS
        .iter()
        .map(|(s, f)| (s.to_string(), f.to_string()))
        .collect();

    let missing: Vec<_> = expected
        .difference(&declared)
        .map(|(s, f)| format!("{}.{}", s, f))
        .collect();
    let extra: Vec<_> = declared
        .difference(&expected)
        .map(|(s, f)| format!("{}.{}", s, f))
        .collect();

    assert!(
        missing.is_empty() && extra.is_empty(),
        "DEPRECATED_OUTPUT_FIELDS drifted from the OpenAPI spec.\n\
         New deprecated response fields in spec, missing from meta.rs: {:?}\n\
         No longer deprecated (or not a response field), still in meta.rs: {:?}\n\
         Regenerate with: python3 scripts/regenerate-deprecated-fields.py",
        missing,
        extra,
    );
}

/// `(RustStructName, specFieldName)` for every `deprecated: true` property on a
/// response-side schema. Request-side schemas (`*Request`, `*Patch`, `*Input`)
/// are excluded — callers may still need to send a deprecated field.
fn spec_deprecated_output_fields(spec: &Value) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    let schemas = spec["components"]["schemas"].as_object().unwrap();

    for (spec_name, schema) in schemas {
        if is_request_side_schema(spec_name) {
            continue;
        }
        let Some(props) = schema.get("properties").and_then(Value::as_object) else {
            continue;
        };
        for (prop_name, prop) in props {
            if prop.get("deprecated").and_then(Value::as_bool) == Some(true) {
                out.insert((pascalize_identifier(spec_name), prop_name.clone()));
            }
        }
    }

    out
}

/// Request-side schemas keep their deprecated fields serializable so callers can
/// still send them. Note `*PatchResponse` ends in "Response", so it is treated
/// as a response schema even though it contains "Patch".
fn is_request_side_schema(schema_name: &str) -> bool {
    schema_name.ends_with("Request")
        || schema_name.ends_with("Patch")
        || schema_name.ends_with("Input")
}

/// `(RustStructName, specFieldName)` for every field in `models.rs` carrying the
/// deprecated-field hiding marker.
fn model_deprecated_marked_fields(source: &str) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    for (struct_name, fields) in parse_model_fields(source) {
        for (spec_field, info) in fields {
            if info.deprecated_marker {
                out.insert((struct_name.clone(), spec_field));
            }
        }
    }
    out
}
