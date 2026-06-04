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

### Deprecated fields

The OpenAPI spec marks some response fields as deprecated (e.g. `Service.tier`, `ApiKey.roles`, `Member.role`). In almost all cases, these are not needed. The Cloud API library disables them by default, gated by a Cargo feature flag `deprecated-fields`. Enable this feature if you need to consume deprecated fields.

### Scripts

```bash
# Show a JSON manifest of required/optional fields per schema
python3 scripts/resolve-field-requirements.py

# Regenerate the DEPRECATED_FIELDS constant from the snapshot
python3 scripts/regenerate-deprecated-fields.py

# Check for drift between the live spec and the library (dry run)
python3 scripts/check-openapi-drift.py --dry-run
```

Field optionality is maintained by hand — edit `models.rs` directly when the drift check flags a mismatch.

### Testing

```bash
cargo test -p clickhouse-cloud-api          # all tests
cargo test --test spec_coverage_test        # spec coverage + field optionality only
cargo test --test client_test               # wiremock-based client tests
cargo test --test models_test               # serde round-trip tests
```

Live-API lifecycle suites are `#[ignore]`d by default (they provision real resources):

```bash
cargo test --test integration_test -- --ignored --nocapture           # ClickHouse service CRUD
cargo test --test integration_postgres_test -- --ignored --nocapture  # Postgres service CRUD
```

ClickPipes E2E binaries live under `tests/clickpipes/` and are declared as named `[[test]]` entries in `Cargo.toml`. Each per-source binary provisions its own ClickHouse Cloud service and exercises one source; `clickpipe_e2e_test` runs every stage in parallel against a single shared service:

```bash
cargo test --test clickpipe_e2e_test -- --ignored --nocapture            # all sources, one CHC service
cargo test --test clickpipe_s3_test -- --ignored --nocapture             # per-source: S3
cargo test --test clickpipe_kafka_test -- --ignored --nocapture          # per-source: Kafka (Redpanda)
cargo test --test clickpipe_kinesis_test -- --ignored --nocapture        # per-source: Kinesis
cargo test --test clickpipe_mongo_test -- --ignored --nocapture          # per-source: MongoDB
cargo test --test clickpipe_mysql_test -- --ignored --nocapture          # per-source: MySQL
cargo test --test clickpipe_postgres_ec2_test -- --ignored --nocapture   # per-source: Postgres-on-EC2
cargo test --test clickpipe_postgres_cdc_test -- --ignored --nocapture   # CHC-managed Postgres CDC
cargo test --test clickpipe_smoke_test -- --ignored --nocapture          # create-only smoke against a shared service
```

All require `CLICKHOUSE_CLOUD_API_KEY`, `CLICKHOUSE_CLOUD_API_SECRET`, `CLICKHOUSE_CLOUD_TEST_ORG_ID`, `CLICKHOUSE_CLOUD_TEST_PROVIDER`, and `CLICKHOUSE_CLOUD_TEST_REGION` in the environment, and are wired into the scheduled `Cloud Integration` GitHub Actions workflow. The ClickPipes E2E suites additionally need AWS credentials and an `eu-west-1` region quota; `clickpipe_smoke_test` reads a pre-provisioned service ID from `CLICKHOUSE_CLOUD_TEST_CLICKPIPE_SERVICE_ID`.

The `spec_coverage_test` suite checks three things against the checked-in spec:

1. Every OpenAPI operation has a matching `pub async fn` in `client.rs`
2. Every OpenAPI schema has a matching `pub struct`/`pub enum` in `models.rs`
3. Every field's `Option<T>` vs `T` matches the spec's required/optional semantics
4. `DEPRECATED_FIELDS` matches the spec's `deprecated: true` fields (request- and response-side), and each one carries the `#[cfg(feature = "deprecated-fields")]` marker in `models.rs`

There are also `#[ignore]`d variants that run the same checks against the live spec.

### Optionality exemptions

Occasionally the spec marks a field as required but the API actually treats it as optional (e.g. sending an empty string triggers a `400 BAD_REQUEST`). In these cases we override the field to `Option<T>` in `models.rs` and add the field to the `OPTIONALITY_EXEMPTIONS` list in `spec_coverage_test.rs`.

The test prints a `NOTE:` line for every exemption that fires, and **fails** if an exemption becomes stale (i.e. the spec was corrected upstream and the override is no longer needed). To add a new exemption, add a `("RustStructName", "specFieldName")` entry to the constant with a comment explaining why.
