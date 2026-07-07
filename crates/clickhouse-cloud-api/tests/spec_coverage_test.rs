use std::collections::{BTreeMap, BTreeSet, HashMap};

use clickhouse_cloud_api::{BETA_OPERATIONS, DEPRECATED_FIELDS};
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
/// operation. The Query API endpoint that backs `run_query` /
/// `run_query_bearer` is hosted at `queries.<environment-domain>` (e.g.
/// `queries.clickhouse.cloud`) and is not described by the control-plane spec.
const NON_OPENAPI_CLIENT_METHODS: &[&str] = &["run_query", "run_query_bearer"];

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

#[test]
fn struct_fields_have_no_extras_vs_spec() {
    assert_no_extra_struct_fields(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn struct_fields_have_no_extras_vs_live_spec() {
    let spec = load_live_spec().await;
    assert_no_extra_struct_fields(&spec);
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
    // Horizontal-autoscaling trio: mutually exclusive with numReplicas,
    // min/maxReplicaMemoryGb, and min/maxTotalMemoryGb per the field
    // descriptions, so they must be omittable despite the heuristic
    // inferring required.
    ("ServicePostRequest", "maxReplicas"),
    ("ServicePostRequest", "minReplicas"),
    ("ServicePostRequest", "replicaMemoryGb"),
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
    // `role` is deprecated in favour of `assignedRoleIds`. The schema has no
    // `required` array and the description starts with "DEPRECATED" (not
    // "Optional"), so the heuristic infers required and would emit a bare
    // `InvitationPostRequestRole`. We model it as `Option<_>` so it can be
    // gated out behind the `deprecated-fields` feature and omitted from the
    // wire — callers send only `assignedRoleIds`. See DEPRECATED_FIELDS.
    ("InvitationPostRequest", "role"),
    // `add` is the deprecated half of this patch (callers associate private
    // endpoints elsewhere now). Same heuristic as above infers required from
    // the "DEPRECATED" description; modelled as `Option<_>` so it can be gated
    // out behind the `deprecated-fields` feature. See DEPRECATED_FIELDS.
    ("OrganizationPrivateEndpointsPatch", "add"),
    // `pgConfig` has no `required` array and no field descriptions begin
    // with "Optional", so the resolver marks every property required. But
    // the endpoint is partial-update: sending the default value for any
    // field yields `Validation failed for following fields: pg_config.*`.
    // Every property is effectively optional. The wrapping
    // `postgresInstanceConfig` is *not* exempted — the live API requires
    // both `pgConfig` and `pgBouncerConfig` to be present in PATCH and
    // POST bodies (omitting either yields `BAD_REQUEST: ... 'undefined'`),
    // matching the spec's `required` listing. See #163 for the behaviour
    // matrix evidence.
    ("PgConfig", "autovacuum_analyze_scale_factor"),
    ("PgConfig", "autovacuum_max_workers"),
    ("PgConfig", "autovacuum_naptime"),
    ("PgConfig", "autovacuum_vacuum_cost_delay"),
    ("PgConfig", "autovacuum_vacuum_cost_limit"),
    ("PgConfig", "autovacuum_vacuum_insert_scale_factor"),
    ("PgConfig", "autovacuum_vacuum_scale_factor"),
    ("PgConfig", "autovacuum_work_mem"),
    ("PgConfig", "default_transaction_isolation"),
    ("PgConfig", "effective_cache_size"),
    ("PgConfig", "effective_io_concurrency"),
    ("PgConfig", "idle_in_transaction_session_timeout"),
    ("PgConfig", "idle_session_timeout"),
    ("PgConfig", "lock_timeout"),
    ("PgConfig", "maintenance_work_mem"),
    ("PgConfig", "max_connections"),
    ("PgConfig", "max_parallel_maintenance_workers"),
    ("PgConfig", "max_parallel_workers"),
    ("PgConfig", "max_parallel_workers_per_gather"),
    ("PgConfig", "max_slot_wal_keep_size"),
    ("PgConfig", "max_wal_size"),
    ("PgConfig", "max_worker_processes"),
    ("PgConfig", "min_wal_size"),
    ("PgConfig", "random_page_cost"),
    ("PgConfig", "ssl_min_protocol_version"),
    ("PgConfig", "statement_timeout"),
    ("PgConfig", "transaction_timeout"),
    ("PgConfig", "wal_compression"),
    ("PgConfig", "wal_keep_size"),
    ("PgConfig", "wal_sender_timeout"),
    ("PgConfig", "work_mem"),
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

/// Struct fields we deliberately keep in `models.rs` even though the mapped
/// spec schema has no such property. Analogous to `NON_OPENAPI_CLIENT_METHODS`
/// (intentional client methods with no spec operation): a code-only field we
/// add on purpose — a response-only/computed field, or a standard attribute the
/// upstream spec omits. Each entry is `("RustStructName", "specFieldName")`.
///
/// Empty by design. Every extra field the detector surfaces today is a real
/// drift finding (a field removed upstream but left in `models.rs`), not an
/// intentional addition — so nothing is exempted. The
/// `struct_fields_have_no_extras_vs_spec` test fails on a stale entry (one that
/// no longer corresponds to an actual extra field) so this list can't rot.
const EXTRA_FIELD_EXEMPTIONS: &[(&str, &str)] = &[];

/// Assert that no field in a Rust struct is absent from its mapped OpenAPI
/// schema. The mirror of `assert_field_coverage`: that catches spec properties
/// missing from structs (spec → code); this catches struct fields missing from
/// the spec (code → spec), e.g. a field removed from the schema upstream but
/// left behind in `models.rs`. Intentional code-only fields are listed in
/// `EXTRA_FIELD_EXEMPTIONS`.
///
/// Schemas with no `properties` (or an empty `properties` object) are skipped,
/// matching `assert_field_coverage` — composition/marker schemas carry their
/// fields elsewhere and would otherwise flag every struct field as extra.
fn assert_no_extra_struct_fields(spec: &Value) {
    let schemas = spec["components"]["schemas"].as_object().unwrap();
    let model_fields = parse_model_fields(MODELS_RS);

    let exemptions: BTreeSet<(&str, &str)> = EXTRA_FIELD_EXEMPTIONS.iter().copied().collect();
    let mut exemptions_hit: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut extras = Vec::new();

    for (spec_name, schema) in schemas {
        let rust_name = pascalize_identifier(spec_name);
        let fields = match model_fields.get(&rust_name) {
            Some(f) => f,
            None => continue, // Schema not in models — covered by other tests
        };

        let props = match schema.get("properties").and_then(Value::as_object) {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        for spec_field in fields.keys() {
            if props.contains_key(spec_field.as_str()) {
                continue;
            }

            if exemptions
                .iter()
                .any(|(s, f)| *s == rust_name && *f == spec_field.as_str())
            {
                exemptions_hit.insert(
                    *exemptions
                        .iter()
                        .find(|(s, f)| *s == rust_name && *f == spec_field.as_str())
                        .unwrap(),
                );
                eprintln!(
                    "NOTE: {}.{} extra-field exempted — see EXTRA_FIELD_EXEMPTIONS",
                    rust_name, spec_field
                );
                continue;
            }

            extras.push(format!("{}.{}", rust_name, spec_field));
        }
    }

    // Detect stale exemptions — entries that no longer correspond to an extra.
    let stale: Vec<_> = exemptions
        .difference(&exemptions_hit)
        .map(|(s, f)| format!("({}, {})", s, f))
        .collect();
    assert!(
        stale.is_empty(),
        "Stale EXTRA_FIELD_EXEMPTIONS (struct field now matches the spec or was removed):\n{}",
        stale.join("\n")
    );

    extras.sort();
    assert!(
        extras.is_empty(),
        "Struct fields with no matching spec property ({} total):\n{}\n\
         A field listed here was removed from (or never existed in) its OpenAPI \
         schema but still lives in models.rs. Remove it, or — if it's an \
         intentional code-only field — add it to EXTRA_FIELD_EXEMPTIONS.",
        extras.len(),
        extras.join("\n")
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
    /// The field's declared Rust type, e.g. `Option<ServiceRegion>`.
    rust_type: String,
    /// True if the field carries the `#[cfg(feature = "deprecated-fields")]`
    /// marker that removes it from the struct (and thus from output) unless the
    /// `deprecated-fields` feature is enabled.
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
                if line.contains("#[cfg(feature = \"deprecated-fields\")]") {
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
                            rust_type: type_str.to_string(),
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

/// Deprecated fields that we deliberately keep in the generated struct, even
/// though the spec marks them `deprecated: true`. Each entry is
/// `("RustStructName", "specFieldName")`. Empty today — every deprecated field
/// is gated out. The `deprecated_fields_match_spec` test fails on a stale entry
/// (one that no longer corresponds to a spec-deprecated field) so this list
/// can't rot.
const DEPRECATED_FIELD_EXEMPTIONS: &[(&str, &str)] = &[];

/// `DEPRECATED_FIELDS` must mirror the `deprecated: true` properties on every
/// schema in the spec (minus `DEPRECATED_FIELD_EXEMPTIONS`).
#[test]
fn deprecated_fields_match_spec() {
    assert_deprecated_fields_match(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn deprecated_fields_match_live_spec() {
    let spec = load_live_spec().await;
    assert_deprecated_fields_match(&spec);
}

/// Every field declared in `DEPRECATED_FIELDS` must carry the
/// `#[cfg(feature = "deprecated-fields")]` marker in `models.rs`, and no other
/// field may carry it. This keeps the consumer-facing constant in lockstep with
/// the fields that are actually removed from the struct by default.
#[test]
fn deprecated_fields_hidden() {
    let marked = model_deprecated_marked_fields(MODELS_RS);
    let declared: BTreeSet<(String, String)> = DEPRECATED_FIELDS
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
        "DEPRECATED_FIELDS is out of sync with the #[cfg(feature = \"deprecated-fields\")] markers in models.rs.\n\
         Declared but not marked (add the #[cfg(feature = \"deprecated-fields\")] marker): {:?}\n\
         Marked but not declared (add to DEPRECATED_FIELDS or remove the marker): {:?}",
        missing_markers,
        stray_markers,
    );
}

fn assert_deprecated_fields_match(spec: &Value) {
    let exemptions: BTreeSet<(&str, &str)> = DEPRECATED_FIELD_EXEMPTIONS.iter().copied().collect();
    let spec_fields = spec_deprecated_fields(spec);

    // Stale-exemption detection: an exemption must correspond to a field the
    // spec actually marks deprecated.
    let stale: Vec<_> = exemptions
        .iter()
        .filter(|(s, f)| !spec_fields.contains(&((*s).to_string(), (*f).to_string())))
        .map(|(s, f)| format!("({}, {})", s, f))
        .collect();
    assert!(
        stale.is_empty(),
        "Stale DEPRECATED_FIELD_EXEMPTIONS (no longer a deprecated field):\n{}",
        stale.join("\n")
    );

    let expected: BTreeSet<(String, String)> = spec_fields
        .into_iter()
        .filter(|(s, f)| !exemptions.contains(&(s.as_str(), f.as_str())))
        .collect();
    let declared: BTreeSet<(String, String)> = DEPRECATED_FIELDS
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
        "DEPRECATED_FIELDS drifted from the OpenAPI spec.\n\
         New deprecated fields in spec, missing from meta.rs: {:?}\n\
         No longer deprecated, still in meta.rs: {:?}\n\
         Regenerate with: python3 scripts/regenerate-deprecated-fields.py",
        missing,
        extra,
    );
}

/// `(RustStructName, specFieldName)` for every `deprecated: true` property on
/// any schema — both request-side and response-side.
fn spec_deprecated_fields(spec: &Value) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    let schemas = spec["components"]["schemas"].as_object().unwrap();

    for (spec_name, schema) in schemas {
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

// ---------------------------------------------------------------------------
// Enum value coverage
// ---------------------------------------------------------------------------

#[test]
fn enum_values_cover_every_spec_enum() {
    assert_enum_value_coverage(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn enum_values_cover_every_live_spec_enum() {
    let spec = load_live_spec().await;
    assert_enum_value_coverage(&spec);
}

#[test]
fn enum_values_have_no_extras_vs_spec() {
    assert_no_extra_enum_values(&serde_json::from_str(SPEC_JSON).unwrap());
}

#[tokio::test]
#[ignore = "hits the live published ClickHouse OpenAPI spec"]
async fn enum_values_have_no_extras_vs_live_spec() {
    let spec = load_live_spec().await;
    assert_no_extra_enum_values(&spec);
}

/// Enum variants we deliberately keep in `models.rs` even though the mapped
/// spec enum no longer (or never did) list the value. Analogous to
/// `EXTRA_FIELD_EXEMPTIONS`. Each entry is `("RustEnumName", "wireValue")`
/// with a comment explaining why the override exists.
///
/// Empty by design. The `enum_values_have_no_extras_vs_spec` test fails on a
/// stale entry (one that no longer corresponds to an actual extra value) so
/// this list can't rot.
const EXTRA_ENUM_VALUE_EXEMPTIONS: &[(&str, &str)] = &[];

/// Assert that every value in a spec `enum` is representable by the mapped
/// Rust enum. Catches values added to a spec enum that never made it into
/// models.rs (responses fall into the untagged catch-all, and requests can't
/// express the value at all).
fn assert_enum_value_coverage(spec: &Value) {
    let model_fields = parse_model_fields(MODELS_RS);
    let string_enums = parse_enum_variant_values(MODELS_RS);

    let mut missing = Vec::new();

    for ((schema, prop), spec_values) in spec_enum_values(spec) {
        let location = enum_location_display(&schema, prop.as_deref());
        match resolve_enum_for_location(&schema, prop.as_deref(), &model_fields, &string_enums) {
            // Missing struct/field — reported by the schema/field coverage tests.
            EnumResolution::Unmapped => continue,
            EnumResolution::NotAStringEnum(type_name) => {
                missing.push(format!(
                    "{}: spec declares an enum but models.rs type `{}` is not a value enum",
                    location, type_name
                ));
            }
            EnumResolution::StringEnum(enum_name) => {
                let variants = &string_enums[&enum_name];
                for value in spec_values.difference(variants) {
                    missing.push(format!("{} ({}): \"{}\"", enum_name, location, value));
                }
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Spec enum values missing from Rust enums ({} total):\n{}\n\
         Add the variant (with a #[serde(rename = ...)] where the value isn't a \
         valid identifier) and its Display arm in models.rs.",
        missing.len(),
        missing.join("\n")
    );
}

/// Assert that no Rust enum variant serializes a value absent from its mapped
/// spec enum. The mirror of `assert_enum_value_coverage`: that catches spec
/// values missing from enums (spec → code); this catches values removed from a
/// spec enum but left behind in `models.rs` (code → spec), which the API
/// rejects on requests — the #273 PubSub `seekType`/`"snapshot"` incident.
/// Intentional keepers are listed in `EXTRA_ENUM_VALUE_EXEMPTIONS`.
fn assert_no_extra_enum_values(spec: &Value) {
    let model_fields = parse_model_fields(MODELS_RS);
    let string_enums = parse_enum_variant_values(MODELS_RS);

    let exemptions: BTreeSet<(&str, &str)> = EXTRA_ENUM_VALUE_EXEMPTIONS.iter().copied().collect();
    let mut exemptions_hit: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut extras = BTreeSet::new();

    for ((schema, prop), spec_values) in spec_enum_values(spec) {
        let EnumResolution::StringEnum(enum_name) =
            resolve_enum_for_location(&schema, prop.as_deref(), &model_fields, &string_enums)
        else {
            // Unmapped/mistyped locations are assert_enum_value_coverage findings.
            continue;
        };

        let location = enum_location_display(&schema, prop.as_deref());
        for variant_value in string_enums[&enum_name].difference(&spec_values) {
            if let Some(entry) = exemptions
                .iter()
                .find(|(e, v)| *e == enum_name && *v == variant_value.as_str())
            {
                exemptions_hit.insert(*entry);
                eprintln!(
                    "NOTE: {}: \"{}\" extra-enum-value exempted — see EXTRA_ENUM_VALUE_EXEMPTIONS",
                    enum_name, variant_value
                );
                continue;
            }
            extras.insert(format!("{} ({}): \"{}\"", enum_name, location, variant_value));
        }
    }

    // Detect stale exemptions — entries that no longer correspond to an extra.
    let stale: Vec<_> = exemptions
        .difference(&exemptions_hit)
        .map(|(e, v)| format!("({}, {})", e, v))
        .collect();
    assert!(
        stale.is_empty(),
        "Stale EXTRA_ENUM_VALUE_EXEMPTIONS (enum value now matches the spec or was removed):\n{}",
        stale.join("\n")
    );

    let extras: Vec<_> = extras.into_iter().collect();
    assert!(
        extras.is_empty(),
        "Rust enum variants with no matching spec enum value ({} total):\n{}\n\
         A variant listed here serializes a value its spec enum no longer (or \
         never did) allow — the API will reject it. Remove the variant and its \
         Display arm, or — if it's an intentional keeper — add it to \
         EXTRA_ENUM_VALUE_EXEMPTIONS.",
        extras.len(),
        extras.join("\n")
    );
}

/// How a spec enum location maps onto `models.rs`.
enum EnumResolution {
    /// Resolved to a value enum (data-free variants + untagged catch-all).
    StringEnum(String),
    /// The location resolves to a Rust type that is not a value enum
    /// (e.g. a plain `String` field or a oneOf union enum).
    NotAStringEnum(String),
    /// The struct or field backing the location is missing from `models.rs`.
    Unmapped,
}

/// Resolve the Rust type that serializes a spec enum location. Named enum
/// schemas map by name (`pascalize_identifier`, as in the schema coverage
/// test); inline property enums map via the struct field's declared type — the
/// actual serialization path — so the mapping is structural and cannot rot the
/// way a naming convention or doc comment could.
fn resolve_enum_for_location(
    schema: &str,
    prop: Option<&str>,
    model_fields: &HashMap<String, HashMap<String, FieldInfo>>,
    string_enums: &HashMap<String, BTreeSet<String>>,
) -> EnumResolution {
    let type_name = match prop {
        None => pascalize_identifier(schema),
        Some(prop) => {
            let Some(fields) = model_fields.get(&pascalize_identifier(schema)) else {
                return EnumResolution::Unmapped;
            };
            let Some(field) = fields.get(prop) else {
                return EnumResolution::Unmapped;
            };
            inner_type(&field.rust_type).to_string()
        }
    };

    if string_enums.contains_key(&type_name) {
        EnumResolution::StringEnum(type_name)
    } else {
        EnumResolution::NotAStringEnum(type_name)
    }
}

fn enum_location_display(schema: &str, prop: Option<&str>) -> String {
    match prop {
        Some(prop) => format!("{}.{}", schema, prop),
        None => schema.to_string(),
    }
}

/// Strip `Option<`/`Vec<`/`Box<` wrappers down to the innermost type name.
fn inner_type(mut ty: &str) -> &str {
    ty = ty.trim();
    'outer: loop {
        for wrapper in ["Option<", "Vec<", "Box<"] {
            if let Some(rest) = ty.strip_prefix(wrapper) {
                ty = rest.strip_suffix('>').unwrap_or(rest);
                continue 'outer;
            }
        }
        return ty;
    }
}

/// Extract every string-valued `enum` in the spec, keyed by location: a named
/// enum schema is `(schemaName, None)`, an inline property enum is
/// `(schemaName, Some(propertyName))`. Non-string values (numeric enums) are
/// dropped and locations left with no string values are omitted. Enums nested
/// inside `items`/`oneOf` are out of scope — models.rs represents those as
/// plain collections/unions, not value enums.
fn spec_enum_values(spec: &Value) -> BTreeMap<(String, Option<String>), BTreeSet<String>> {
    let mut out = BTreeMap::new();
    let schemas = spec["components"]["schemas"].as_object().unwrap();

    for (spec_name, schema) in schemas {
        if let Some(values) = string_enum_values(schema) {
            out.insert((spec_name.clone(), None), values);
        }
        let Some(props) = schema.get("properties").and_then(Value::as_object) else {
            continue;
        };
        for (prop_name, prop) in props {
            if let Some(values) = string_enum_values(prop) {
                out.insert((spec_name.clone(), Some(prop_name.clone())), values);
            }
        }
    }

    out
}

/// The string values of a schema/property's `enum` array, if it has any.
fn string_enum_values(node: &Value) -> Option<BTreeSet<String>> {
    let values: BTreeSet<String> = node
        .get("enum")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    (!values.is_empty()).then_some(values)
}

/// Parse models.rs to extract the wire values of every value enum: enums whose
/// non-catch-all variants are all data-free. The wire value is the variant's
/// `#[serde(rename = "...")]` if present, else the variant identifier itself.
/// Variants marked `#[serde(untagged)]` (the `Unknown(String)`-style catch-all)
/// carry no spec value and are skipped — identified by attribute, not by name,
/// since one enum's catch-all is `Other(String)` and `Unknown` is a real spec
/// value there. Enums with data-carrying variants that are not untagged model
/// `oneOf` unions, not value enums, and are excluded entirely.
///
/// Returns: { RustEnumName: { wireValue } }
fn parse_enum_variant_values(source: &str) -> HashMap<String, BTreeSet<String>> {
    let mut result = HashMap::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim_start();

        if let Some(rest) = line.strip_prefix("pub enum ")
            && let Some(enum_name) = identifier_prefix(rest)
        {
            let enum_name = enum_name.to_string();
            i += 1;
            let mut values = BTreeSet::new();
            let mut is_value_enum = true;
            let mut pending_rename: Option<String> = None;
            let mut pending_untagged = false;

            while i < lines.len() {
                let line = lines[i].trim();

                if line == "}" {
                    break;
                }

                if line.starts_with("#[serde(") {
                    if let Some(rename) = extract_serde_rename(line) {
                        pending_rename = Some(rename.to_string());
                    }
                    if line.contains("untagged") {
                        pending_untagged = true;
                    }
                } else if !line.starts_with("#[")
                    && !line.starts_with("//")
                    && !line.is_empty()
                    && let Some(variant_name) = identifier_prefix(line)
                {
                    let is_unit = line[variant_name.len()..].trim_end_matches(',').trim().is_empty();

                    if pending_untagged {
                        // Catch-all variant — carries no spec value.
                    } else if is_unit {
                        values.insert(
                            pending_rename.take().unwrap_or_else(|| variant_name.to_string()),
                        );
                    } else {
                        is_value_enum = false;
                    }
                    pending_rename = None;
                    pending_untagged = false;
                }

                i += 1;
            }

            if is_value_enum && !values.is_empty() {
                result.insert(enum_name, values);
            }
        }

        i += 1;
    }

    result
}
