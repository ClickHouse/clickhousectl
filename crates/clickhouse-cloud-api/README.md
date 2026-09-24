# clickhouse-cloud-api

Typed Rust client for the [ClickHouse Cloud API](https://clickhouse.com/docs/en/cloud/manage/openapi).

## Updated Cloud API surface

`openapi_key_get_list(organization_id, limit, cursor)` returns one page of API keys.
Migrate existing calls by passing `None, None` for the new arguments. The server
accepts limits from 1–250 and defaults to 250; continue with the response's
`next_cursor` until it is `None` to read all keys, including empty intermediate
pages. Cursors are opaque: pass even an empty string unchanged. `ApiResponse<T>`
now preserves optional `limit`, `total_count`, and `next_cursor` envelope metadata.
Missing or null metadata becomes `None` and is omitted when serialized. Struct
literal callers must supply the new fields or use `..Default::default()` when
available. Callers that iterate pages should detect repeated cursors to avoid loops.

`service_profiles_list` now takes `region_id: Option<&str>`: wrap existing region
arguments in `Some(...)`, or pass `None` with `byoc_id` to use the infrastructure's
region. When both are supplied, the region must match that infrastructure.

The beta `snapshot_get_list` and `snapshot_get` methods return service `Snapshot`
records, including the `throttled` status and full snapshot type. Snapshot response
fields tolerate missing and null values; provider-specific bucket properties and
unknown status/type values remain lossless.

The beta `snapshot_configuration_get` and `snapshot_configuration_update` methods
read and update scheduled snapshots using `SnapshotConfiguration` and
`SnapshotConfigurationPatchRequest`. `gap` and `time_frame` are measured in
minutes. Updates require an ADMIN API key and at least one field; `None` omits a
field to leave it unchanged, while `Some(false)` explicitly disables scheduling.
The API rejects null and validates supported enabled cadence pairs: gap/time-frame
values of `(30, 1440)` or `(60, 2880)`. Response fields tolerate absence and null.

ClickPipes requests now include destination-table `ttl`, MongoDB
`initial_load_parallelism`, and `start_paused`. Empty TTL and false `start_paused`
values are omitted to preserve existing create behavior; set a nonempty TTL SQL
expression or `start_paused: true` to send them. Starting paused is unsupported
for database ClickPipes. `UdfArgumentOutput` matches the newly named UDF response
schema; `UdfArgumentResponse` remains a compatible alias.

The live snapshot adds `credit_balances_get` (trial and prepaid credit balances), `service_profiles_list` (region and optional BYOC infrastructure), and `click_pipes_service_context_get` (GCP workload identity readiness and principal).

BigQuery and Pub/Sub source models now distinguish service-account and workload-identity authentication with typed unions. Build `ClickPipePostBigQueryServiceAccountSource` or `ClickPipePostPubSubServiceAccountSource` and call `.into()` for existing service-account flows; workload-identity variants omit customer credentials. BigQuery settings and table mappings now permit the optional fields the API accepts. Kafka requests gain optional `protobuf_schema`, which is only supported for Protobuf without a schema registry; MySQL table mappings gain `partition_by_expr`.

`ServiceProfile` now represents the profile discovery response (`profile`, `cpu_cores`, `memory_gi`); the former `Service.profile` value enum is named `ServiceProfileName`. Dynamic profile names remain lossless through its `Unknown(String)` variant.

ClickStack models now include alert channel lists, 30-second alert intervals, query-timeout errors, chart formulas and series limits, dashboard variables and broadcast filters, service-version expressions, and typed SQL/variable saved-filter unions. New formula and saved-filter response/request pairs support explicit fallible write-back through `TryFrom`; absent nested required fields return their wire names. UDF responses include `deterministic`. UDF request models preserve `deterministic` and nullable `memoryLimitMib` in both executable variants, including version creation.

The current alert request schemas have no `required` array or optional marker on either `channel` or `channels`, so both fields remain strict in the Rust request models. This mirrors the documented requiredness policy; it does not establish whether the server accepts a channels-only request. Supply the channel list explicitly rather than relying on the empty `Default` value (the API specifies 1–10 channels).

Postgres slow-query aggregate durations (`*DurationUs`) and execution `durationUs` use `Option<f64>` to preserve fractional microseconds returned by the API. Counts remain integral. The analyzer tracks the upstream integer-schema discrepancy with response-only, stale-checked exceptions ([#758](https://github.com/ClickHouse/clickhousectl/issues/758)).

Kinesis source format enums now include `Protobuf`. Set `ClickPipePostKinesisSource.protobuf_schema` to the base64-encoded `.proto` source or serialized `FileDescriptorSet` for that format; omit it for other formats. Organization Prometheus discovery has graduated from beta and is no longer listed in `BETA_OPERATIONS`.

`Organization.capabilities.snapshots` reports snapshot eligibility as an optional boolean. Kinesis create requests accept `ClickPipeKinesisSchemaRegistry` for AWS Glue; supply `type`, `glue_region`, and `glue_registry_name`, and optionally `glue_role_arn` to assume a different role from the Kinesis source. Kinesis responses use `ClickPipeKinesisSchemaRegistryResponse`, whose fields tolerate absence and null; convert it with `TryFrom` before writing it back. Kafka create requests accept `tombstone_mode: Some(Delete)` to delete matching destination rows for tombstone records. This requires exactly-once delivery and is set only at creation. Struct-literal callers of `Organization`, `ClickPipeKinesisSource`, `ClickPipeKafkaSource`, `ClickPipePostKinesisSource`, and `ClickPipePostKafkaSource` need to supply the new optional fields as `None` or use `..Default::default()`.

The beta Query API endpoint management methods are `query_api_endpoint_create`, `query_api_endpoint_get`, `query_api_endpoint_list`, `query_api_endpoint_update`, and `query_api_endpoint_delete`. Create and update take `PublicQueryApiEndpointRequest`; list accepts an optional cursor and limit (1–100) and returns `items` with `pagination.next_cursor`. User-owned endpoints can be listed and read, but cannot be updated or deleted through this API.

### ClickHouse settings models

`ServiceClickhouseSettingsPatchRequest` uses a map of setting names to JSON values, and `ServiceClickhouseSettingsPatchResponse.settings` returns the applied map. The published OpenAPI now describes both fields as nonempty objects with string or integer values. Use `ServiceClickhouseSettingsPatchRequest<ServiceClickhouseSettingsMap>` and `ServiceClickhouseSettingValue` for typed requests; the default JSON-value map remains source-compatible with existing callers. Explicit `ServiceClickhouseSettingsPatchRequest<String>` callers remain supported: encoded objects are validated and serialized as objects before sending.

`ServiceClickhouseSetting.value` is `Option<serde_json::Value>`: the published contract permits strings and integers. The response aliases `ServiceClickhouseSettingValueResponse` and `ServiceClickhouseSettingsMapResponse` retain arbitrary JSON to tolerate future response types. Values retain their JSON types; missing and null values remain absent.

## Development

### Structure

| Path | Purpose |
|------|---------|
| `src/client.rs`, `src/client/*.rs` | `Client`, shared HTTP machinery, and per-domain endpoint methods |
| `src/models.rs`, `src/models/*.rs` | Public facade/macro and private per-domain request/response models |
| `src/convert.rs`, `src/convert/*.rs` | Conversion error and per-domain response-to-request conversions |
| `src/error.rs` | Error types (`Http`, `Json`, `Api`) |
| `clickhouse_cloud_openapi.json` | Checked-in copy of the spec (used by tests) |
| `tests/spec_coverage_test.rs` | Thin snapshot/live-spec consumer of the shared drift analyzer |
| `../clickhouse-openapi-analyzer` | Canonical Rust/OpenAPI parsing, comparison, report, and exemptions |

### Field optionality

The OpenAPI spec uses two conventions for marking fields required vs optional:

- **Schemas with a `required` array** (newer/beta endpoints) use standard OpenAPI semantics.
- **Schemas without `required`** (GA/legacy endpoints) treat fields whose description starts with `"Optional"` as optional. Everything else is implicitly required.
- **Known partial `required` arrays** use the union of that array and the description heuristic, as configured by the analyzer.

Additional rules:

- **PATCH request schemas** (name contains `Patch` and ends with `Request`) are always all-optional.
- **Nullable fields** (`type: ["string", "null"]` or `oneOf` with null) are always `Option<T>`, even if required.

In `src/models/<domain>.rs`, request fields follow those rules: required non-nullable fields use `T`, while optional or nullable fields use `Option<T>`. Every response field uses `Option<T>` plus `skip_serializing_if`, so missing keys and explicit `null` deserialize natively to `None` and absent fields are omitted when serialized. `#[serde(default)]` is banned because it can fabricate required request values and is redundant for `Option` response fields.

API key PATCH expiry uses `Option<Option<DateTime<Utc>>>`: `None` leaves the current expiry unchanged, `Some(Some(time))` sets it, and `Some(None)` removes it. These three states survive serialization and deserialization. API key creation still uses `Option<DateTime<Utc>>`, where omission creates a key without an expiry.

### Deprecated fields

The OpenAPI spec marks some response fields as deprecated (e.g. `Service.tier`, `ApiKey.roles`, `Member.role`). In almost all cases, these are not needed. The Cloud API library disables them by default, gated by a Cargo feature flag `deprecated-fields`. Enable this feature if you need to consume deprecated fields.

### Scripts

```bash
# Show a JSON manifest of required/optional fields per schema
python3 scripts/resolve-field-requirements.py

# Regenerate the DEPRECATED_FIELDS constant from the snapshot
python3 scripts/regenerate-deprecated-fields.py

# Regenerate the BETA_OPERATIONS constant from the snapshot
python3 scripts/regenerate-beta-lists.py

# Check for drift between the live spec and the library (dry run)
python3 scripts/check-openapi-drift.py --dry-run
```

Field optionality is maintained by hand. Edit the owning model domain file when the drift check flags a mismatch, and add new domain types to the private module and facade re-exports in `models.rs`.

### Testing

```bash
cargo test -p clickhouse-cloud-api          # all tests
cargo test -p clickhouse-openapi-analyzer   # analyzer fixtures + executable parity
cargo test -p clickhouse-cloud-api --test spec_coverage_test # shared snapshot report
cargo test -p clickhouse-cloud-api --test client_test        # wiremock client tests
cargo test -p clickhouse-cloud-api --test models_test        # serde round trips
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
cargo test --test clickpipe_postgres_cli_cdc_test -- --ignored --nocapture # CHC-managed Postgres CDC via CLI
cargo test --test clickpipe_smoke_test -- --ignored --nocapture          # create-only smoke against a shared service
```

All require `CLICKHOUSE_CLOUD_API_KEY`, `CLICKHOUSE_CLOUD_API_SECRET`, `CLICKHOUSE_CLOUD_TEST_ORG_ID`, `CLICKHOUSE_CLOUD_TEST_PROVIDER`, and `CLICKHOUSE_CLOUD_TEST_REGION` in the environment, and are wired into the scheduled `Cloud Integration` GitHub Actions workflow. The ClickPipes E2E suites additionally need AWS credentials and an `eu-west-1` region quota; `clickpipe_smoke_test` reads a pre-provisioned service ID from `CLICKHOUSE_CLOUD_TEST_CLICKPIPE_SERVICE_ID`.

The managed-Postgres target uses the CLI's default table mapping and compares exact source and destination IDs and values after snapshot, INSERT, UPDATE, and DELETE. Destination reads use `FINAL WHERE _peerdb_is_deleted = 0`. It requires `CLICKHOUSE_CLOUD_TEST_CLICKHOUSECTL_BIN` to point to a built `clickhousectl` binary. From the workspace root, build and run it with:

```bash
cargo build -p clickhousectl
CLICKHOUSE_CLOUD_TEST_CLICKHOUSECTL_BIN=target/debug/clickhousectl \
  cargo test -p clickhouse-cloud-api --test clickpipe_postgres_cli_cdc_test -- --ignored --nocapture
```

The trusted planner runs automatically on every same-repository PR push and selects suites from that PR's merge-base-to-head diff; it is read-only and never receives Cloud secrets. When it selects no live suites, the `Cloud integration decision` check turns green with no action needed. When live suites are selected, the decision check names them and stays `action_required` until `run-cloud-integration` is applied — the labeled event is the one-shot, exact-SHA authorization that admits the environment-bearing job, so re-apply the label after any push. Unknown API source or test paths select all suites, and changes to the classifier, its tests, or the workflow also select all suites through an independent workflow guard. Manual `Cloud Integration` dispatches accept `scope=all`, `service`, `postgres`, `organization`, or `clickpipes`. The focused scopes run only their corresponding suite; `clickpipes` runs Postgres CDC plus the fixture-gated smoke test, while `all` runs all four mandatory suites plus that optional smoke test. Because stacked-PR diffs exclude inherited changes, manually run `scope=all` against the top stack branch for full-stack validation.

`spec_coverage_test` sends the checked-in sources and snapshot through the
private `clickhouse-openapi-analyzer` crate. The analyzer recursively traverses
the module trees rooted at `client.rs`, `models.rs`, and `meta.rs`, including
private per-domain files. That same analyzer powers the scheduled live-spec
issue, so operation, model, field, optionality, beta, deprecation, enum,
snapshot, and stale-exemption findings share one implementation. The single
ignored test runs the same report against the live spec. The analyzer also checks
inline union payload fields and request requiredness. Report schema version 9 also
compares effective operation parameters against the snapshot (additions, removals,
requiredness, and schema constraints), resolving local references and ignoring
prose/example changes. Missing Rust arguments and incompatible optionality or
scalar/array shapes remain actionable even after a snapshot refresh. Array
arguments may use a pluralized name and represent omission with an empty slice.

Successful JSON envelopes are checked against the method's actual returned
struct, including generic `ApiResponse<T>`, aliases, and flattened structs. Missing
envelope fields retain exact definition pointers through response/schema refs
and `allOf`; unmappable envelopes and unsupported inline unions are actionable.
These checks cover envelope field presence, not generic payload type substitution
or HTTP serialization behavior. Parameter defaults, bounds, and other constraints
are snapshot comparisons; scalar/array shape checks compare Rust argument types.
External or unresolved contract references fail analysis rather than report clean.

### Optionality exemptions

Occasionally the spec cannot be followed literally because verified API
behavior differs. All such policy lives in
`crates/clickhouse-openapi-analyzer/src/config.rs`, including optionality,
extra-field, deprecated-field, extra-enum-value, non-OpenAPI-method, partial
required-schema, and unsupported-enum configuration.

Add an exemption only for a deliberate runtime behavior and document why the
spec cannot be followed. New unsupported-enum acknowledgements also require a
tracking issue. The analyzer reports stale field/enum exemptions and vanished
unsupported locations so obsolete entries are removed during normal drift
remediation. Acknowledged locations also report changed enum values against
the snapshot, including numeric and mixed values; unchanged sets remain
acknowledged. See the repository `AGENTS.md` for exact key formats and the full
remediation and verification procedure.

### ClickStack list pagination

`click_stack_list_alerts`, `click_stack_list_webhooks`, and
`click_stack_list_saved_searches` take `limit: Option<i64>` and
`offset: Option<i64>` after the organization and service IDs. Pass `None, None`
for the server defaults (1,000 records, offset zero). For a complete inventory,
request pages with an explicit limit from 1 to 1,000 and advance the offset
until the returned page is shorter than that limit. Existing callers upgrading
to 0.5.0 should add `None, None` to preserve their current request behavior.

### UDF attachment errors

Rust callers receive `Error::UdfAttachmentUnavailable` for a structured attachment failure (HTTP 424); its `UdfAttachResponse424` payload preserves the error code, service state, wake eligibility, and request ID. Fields tolerate absence and null, and enums retain unknown values. Malformed responses remain `Error::Api` with the original error message.

### OpenAPI response coverage

The OpenAPI analyzer checks inline union payload fields and request requiredness, plus inline JSON response objects named `{PascalizedOperationId}Response{Status}` and reachable through client return types or error payloads. It reports obsolete helper exclusions and requiredness overrides as stale exemptions. Acknowledged unsupported enum locations are checked against the snapshot: changed value sets are actionable, while reordering is ignored. Deprecated API-key `roles` fields remain strings behind `deprecated-fields` for source compatibility. Its report format is version 8.
