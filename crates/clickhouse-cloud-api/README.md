# clickhouse-cloud-api

Typed Rust client for the [ClickHouse Cloud API](https://clickhouse.com/docs/en/cloud/manage/openapi).

## Development

### Structure

| Path | Purpose |
|------|---------|
| `src/client.rs` | `Client` struct with an async method per API endpoint |
| `src/models.rs` | Request/response types matching the OpenAPI spec |
| `src/error.rs` | Error types (`Http`, `Json`, `Api`) |
| `clickhouse_cloud_openapi.json` | Checked-in copy of the spec (used by tests) |
| `tests/spec_coverage_test.rs` | Validates client methods, model types, and field optionality match the spec |

### Field optionality

The OpenAPI spec uses two conventions for marking fields required vs optional:

- **Schemas with a `required` array** (newer/beta endpoints) use standard OpenAPI semantics.
- **Schemas without `required`** (GA/legacy endpoints) treat fields whose description starts with `"Optional"` as optional. Everything else is implicitly required.

Additional rules:

- **PATCH request schemas** (name contains `Patch` and ends with `Request`) are always all-optional.
- **Nullable fields** (`type: ["string", "null"]` or `oneOf` with null) are always `Option<T>`, even if required.

In `models.rs`, required non-nullable fields use bare types (`T`) and optional/nullable fields use `Option<T>`. All fields keep `#[serde(default)]` so deserialization is tolerant of partial data.

### Scripts

```bash
# Show a JSON manifest of required/optional fields per schema
python3 scripts/resolve-field-requirements.py

# Update models.rs field types to match the spec
python3 scripts/update-models-optionality.py

# Check for drift between the live spec and the library (dry run)
python3 scripts/check-openapi-drift.py --dry-run
```

### When the spec adds proper `required` arrays everywhere

The spec team plans to add standard `required` metadata to all schemas in a future version. When that happens:

```bash
# 1. Download the updated spec
curl -s https://api.clickhouse.cloud/v1 -o clickhouse_cloud_openapi.json

# 2. Update models.rs
python3 scripts/update-models-optionality.py

# 3. Fix any test assertions for fields that changed
cargo test -p clickhouse-cloud-api
```

The resolution logic automatically prefers `required` arrays over the description heuristic, so no structural changes are needed.

### Testing

```bash
cargo test -p clickhouse-cloud-api          # all tests
cargo test --test spec_coverage_test        # spec coverage + field optionality only
cargo test --test client_test               # wiremock-based client tests
cargo test --test models_test               # serde round-trip tests
```

The `spec_coverage_test` suite checks three things against the checked-in spec:

1. Every OpenAPI operation has a matching `pub async fn` in `client.rs`
2. Every OpenAPI schema has a matching `pub struct`/`pub enum` in `models.rs`
3. Every field's `Option<T>` vs `T` matches the spec's required/optional semantics

There are also `#[ignore]`d variants that run the same checks against the live spec.
