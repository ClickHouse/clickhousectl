# Development

[Contributing](../CONTRIBUTING.md) · [All documentation](README.md)

## Code layout

| Path | Responsibility |
| --- | --- |
| `crates/clickhousectl/src/main.rs`, `cli.rs` | Entrypoint, common CLI options, output mode |
| `crates/clickhousectl/src/local/` | Local command definitions, dispatch, servers, Docker/Postgres, output |
| `crates/clickhousectl/src/cloud/<domain>.rs` | Cloud domain definitions, handlers, builders, wrappers, tests |
| `crates/clickhousectl/src/cloud/<domain>/` | A large Cloud domain: `PERMISSIONS` and dispatch in `mod.rs`, command enums and shared arg groups in `cli.rs` (a source-specific arg struct may sit with its builder, as in `clickpipes/sources/<kind>.rs`), `CloudClient` wrappers in `client.rs`, one file per command family, tests beside the code they test |
| `crates/clickhousectl/src/cloud/client.rs` | Credential resolution and shared Cloud client/error handling |
| `crates/clickhousectl/src/stdout.rs` | CLI-owned stdout and typed broken-pipe handling |
| `crates/clickhouse-cloud-api/` | Published typed API client |
| `crates/clickhouse-openapi-analyzer/` | Private OpenAPI/Rust drift tooling |

The API/analyzer crates share [crate-level instructions](../crates/clickhouse-cloud-api/AGENTS.md). For command work, follow [add-cli-command](../.agents/skills/add-cli-command/SKILL.md) and its local/Cloud references. [AGENTS.md](../AGENTS.md) owns the invariants; avoid duplicating them in each procedure.

## Local tests

Run the standard CLI gates from [Contributing](../CONTRIBUTING.md#build-and-test). Pick focused checks during development:

- Clap parsing and request builders have tests beside command definitions.
- `crates/clickhousectl/tests/cli_request_shape_test.rs` spawns the binary against wiremock for Cloud handler behaviour that the unit tests can't reach: multi-call flows, error paths, auth wiring, and output contracts. [AGENTS.md](../AGENTS.md#tests) holds the rule for what belongs there.
- Local subprocess tests use separate `local_*` binaries for each concern.
- Pure version/auth/output logic uses inline tests.

Subprocess tests must not wait out production timeouts. `crates/clickhousectl/src/timing.rs` reads hidden overrides, each a whole number of milliseconds; any other value keeps the shipped default. They are for tests only, so no help text or user guide mentions them. Set them on the spawned command, after any `env_clear`:

| Variable | Shortens |
| --- | --- |
| `CHCTL_UDF_LOAD_TIMEOUT_MS` | `local udf deploy` reload window (10 s) |
| `CHCTL_TERM_TIMEOUT_MS` | SIGTERM grace before SIGKILL on local stop (2.5 s) |
| `CHCTL_SERVICE_STOP_POLL_INTERVAL_MS` | `cloud service delete --force` stop poll interval (5 s) |
| `CHCTL_SERVICE_STOP_TIMEOUT_MS` | `cloud service delete --force` stop wait (600 s) |
| `CHCTL_UDF_POLL_INTERVAL_MS` | `cloud udf` service poll interval (5 s) |
| `CHCTL_QUERY_READINESS_TIMEOUT_MS` | Query API readiness window after a key bind (120 s) |
| `CHCTL_KEY_PROPAGATION_TIMEOUT_MS` | Retry window for a first-use key-binding upsert while the key propagates (30 s) |
| `CHCTL_UPDATE_CHECK_TIMEOUT_MS` | Implicit update check (400 ms), run in the background by commands and awaited by `--version`; `0` skips the request. `update` still makes its explicit check. Users turn the check off with `CLICKHOUSECTL_NO_UPDATE_CHECK=1` (see the automation reference) |
| `CHCTL_BINARY_VERSION_TIMEOUT_MS` | `local install --binary` wait for the file's `--version` (60 s) |
| `CHCTL_SKILLS_DOWNLOAD_TIMEOUT_MS` | `skills` archive download, body included (300 s; connecting is capped at 30 s) |
| `CHCTL_FEEDBACK_SEND_TIMEOUT_MS` | `feedback` send (10 s) |
| `CHCTL_FEEDBACK_WHOAMI_TIMEOUT_MS` | `feedback` Cloud organization lookup (5 s) |

Add a new override to that module rather than parsing an environment variable at the call site. In `cli_request_shape_test.rs`, use `isolated_env()` in place of `env_clear()`: it clears the environment and turns the update check off.

Structural help tests check the full command tree, descriptions, shared flags, and agent context limits. CI runs them with default features only, so run the second command yourself when a change depends on telemetry or feedback being compiled out:

```bash
cargo test -p clickhousectl --bin clickhousectl cli::tests::
cargo test -p clickhousectl --no-default-features --bin clickhousectl cli::tests::
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
```

Never pin wording in help, README, error messages or human output; assert codes, fields and identifiers instead ([AGENTS.md](../AGENTS.md#tests)). Verify commands with `cargo run -q -p clickhousectl -- <command> --help` and inspect the parent screen too.

Each classifier fails closed for one set of paths, and a new or renamed file needs an entry only in the matching one:

- `scripts/classify-cloud-integration.py`: any file under `crates/clickhouse-cloud-api/` (`src/`, `tests/`, other `.rs`). An unmapped path selects all live Cloud suites, and a unit test requires every current file to be mapped. Keep entries for deleted or renamed paths so those diffs still select the right suites. Files elsewhere, including all of `crates/clickhousectl/` and the analyzer, select no suite by default; add a CLI path only when a change to it should select a live suite.
- `scripts/classify-install-integration.py`: a `crates/clickhousectl/src/**/*.rs` file outside `src/cloud/`, or any file under `crates/clickhousectl/tests/`, must be listed as install-relevant or not. A test requires every current file to be listed. Install-relevant entries must also be added to `.github/workflows/test-install.yml` `paths:`; a test checks the two match.

Documentation paths alone do not select live suites.

## Live Cloud integration

Use a dedicated test workspace: these tests create and change real Cloud resources. Maintainer labels, exact-SHA overrides, stacked-PR policy, and required checks are documented in [the CI guide](../.github/CLOUD_INTEGRATION.md). Affected suites run with `run-cloud-integration`; the decision check passes without the label when no suites are selected.

The library's suites cover services (`integration_test`), Postgres (`integration_postgres_test`), organization access (`integration_org_test`), and ClickPipes (`tests/clickpipes/`; only Postgres CDC runs in CI).

Provide API credentials through your secret manager and set the test scope:

```bash
export CLICKHOUSE_CLOUD_TEST_ORG_ID=<test-org-id>
export CLICKHOUSE_CLOUD_TEST_PROVIDER=aws
export CLICKHOUSE_CLOUD_TEST_REGION=eu-west-1
```

`CLICKHOUSE_CLOUD_API_KEY` and `CLICKHOUSE_CLOUD_API_SECRET` are required. The organization suite also requires `CLICKHOUSE_CLOUD_TEST_SECONDARY_USER_ID` for a second user in the test organization.

```bash
cargo test -p clickhouse-cloud-api --test integration_test -- --ignored --nocapture
cargo test -p clickhouse-cloud-api --test integration_postgres_test -- --ignored --nocapture
cargo test -p clickhouse-cloud-api --test integration_org_test -- --ignored --nocapture
```

Failures fail the run by default. `CONTINUE_ON_NON_BLOCKING_FAILURES=1` collects non-blocking capability failures for a final summary while continuing.

## Debugging endpoints

Cloud `--debug` prints the selected credential source and API URL. For a non-production control plane, `--url` overrides the Cloud API base URL; release builds accept it but hide it from help. `CLICKHOUSE_CLOUD_QUERY_HOST` separately overrides the Query API host. An API key saved by `cloud auth login --api-key … --url <API_URL>` remembers that API, so later commands need no `--url`, and a different `--url` is refused; see [saved API keys](reference/authentication.md#credential-precedence).

For OAuth login to an API host outside the built-in mappings, set `CLICKHOUSECTL_AUTH_URL` and `CLICKHOUSECTL_CLIENT_ID` when running `cloud auth login --url <API_URL>`; login saves the resolved configuration with the tokens, and refresh uses that saved configuration rather than the current environment. Built-in hosts ignore these overrides. Token files must include the complete configuration; older files without it require a new login, including for built-in hosts. Refresh never consults the built-in mappings or environment.

The authorization URL must use HTTPS; HTTP is allowed only for exact `localhost` or a loopback IP address (`127.0.0.0/8` or `::1`). Userinfo, query strings, and fragments are rejected. OAuth requests do not follow redirects, so configure the final authorization server URL. Saved authorization URLs are validated when loading tokens as well.

Keep tests isolated from real credentials and telemetry. Use the existing temporary-project and local-mock patterns in subprocess tests and set `DO_NOT_TRACK=1` for development checks.

## Telemetry changes

The [user telemetry reference](reference/telemetry.md) describes what is recorded and how to disable it. `src/telemetry.rs` builds events from argument definitions, not raw values; `src/failure.rs` defines the bounded failure vocabulary. Paths here are relative to `crates/clickhousectl/`.

Classify library errors through `CloudClient::convert_error*`. Local failures need no extra call: `main` records `local::output::error_code`, the same classification as the `--json` error object, so a new local error variant is classified for telemetry when it is mapped in `LocalErrorOutput::from_error`. Add a stage only at the boundary that owns it; the first stage wins. Preserve the failure object when rewriting diagnostics. Never derive classification from free text or introduce arbitrary values into telemetry fields. Validate both default and telemetry-compiled-out configurations when changing this code.

## API library development

This section covers `clickhouse-cloud-api` and the private `clickhouse-openapi-analyzer`. The rules for model, client and analyzer changes are in the [crate instructions](../crates/clickhouse-cloud-api/AGENTS.md); caller-facing changes go in the [library changelog](../crates/clickhouse-cloud-api/CHANGELOG.md). Paths are relative to `crates/clickhouse-cloud-api/` and commands run from the repository root.

### Structure

| Path | Purpose |
|------|---------|
| `src/client.rs`, `src/client/*.rs` | `Client`, shared HTTP machinery, and per-domain endpoint methods |
| `src/models.rs`, `src/models/*.rs` | Public facade/macro and private per-domain request/response models (`src/models/clickpipes/` is split by source kind, `src/models/clickstack/` by resource) |
| `src/convert.rs`, `src/convert/*.rs` | Conversion error and per-domain response-to-request conversions |
| `src/error.rs` | `Error` and its variants (listed in the crate README) |
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

In `src/models/<domain>.rs`, request fields follow those rules: required non-nullable fields use `T`, while optional or nullable fields use `Option<T>`. Every response field uses `Option<T>` plus `skip_serializing_if`, so missing keys and explicit `null` deserialize natively to `None` and absent fields are omitted when serialized. `#[serde(default)]` is banned because it can fabricate required request values and is redundant for `Option` response fields. For the same reason, a struct with a required field does not implement `Default`.

API key PATCH expiry uses `Option<Option<DateTime<Utc>>>`: `None` leaves the current expiry unchanged, `Some(Some(time))` sets it, and `Some(None)` removes it. These three states survive serialization and deserialization. API key creation still uses `Option<DateTime<Utc>>`, where omission creates a key without an expiry.

### Generators and drift check

```bash
# Print the BETA_OPERATIONS and DEPRECATED_FIELDS declarations for src/meta.rs
cargo run -p clickhouse-openapi-analyzer --bin openapi-drift-analyzer -- \
  --spec crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json \
  --source-root crates/clickhouse-cloud-api/src --generate-stability-lists

# Regenerate the operation catalog in src/meta/operations.rs (then cargo fmt --all)
cargo run -p clickhouse-openapi-analyzer --bin openapi-drift-analyzer -- \
  --spec crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json \
  --generate-operations crates/clickhouse-cloud-api/src/meta/operations.rs

# Check for drift between the live spec and the library (dry run)
python3 scripts/check-openapi-drift.py --dry-run
```

Replace both `meta.rs` declarations with the printed output. Add a split `{Name}Response` type before generating, so its deprecated fields are listed too.

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
cargo test --test clickpipe_smoke_test -- --ignored --nocapture          # create-only smoke against a shared service
```

The suites load AWS credentials through the default provider chain: environment variables, shared profiles, SSO, or `credential_process`. `aws login` sessions are not read directly. To use one, export its credentials first with `eval "$(aws configure export-credentials --format env)"`.

All require `CLICKHOUSE_CLOUD_API_KEY`, `CLICKHOUSE_CLOUD_API_SECRET`, `CLICKHOUSE_CLOUD_TEST_ORG_ID`, `CLICKHOUSE_CLOUD_TEST_PROVIDER`, and `CLICKHOUSE_CLOUD_TEST_REGION` in the environment, and are wired into the scheduled `Cloud Integration` GitHub Actions workflow. The ClickPipes E2E suites additionally need AWS credentials and an `eu-west-1` region quota; `clickpipe_smoke_test` reads a pre-provisioned service ID from `CLICKHOUSE_CLOUD_TEST_CLICKPIPE_SERVICE_ID`.

Two live tests drive the real `clickhousectl` binary, so they live in the CLI crate as `crates/clickhousectl/tests/live_*_test.rs`. Cargo builds the binary for them, and they share the library's harness in `crates/clickhouse-cloud-api/tests/common/` through `#[path]`. They need the same Cloud environment variables, are `#[ignore]`d, and assert JSON error codes, exit codes and Cloud API state rather than CLI wording:

```bash
cargo test -p clickhousectl --test live_service_query_bind_test -- --ignored --nocapture     # service query first-use bind
cargo test -p clickhousectl --test live_clickpipe_postgres_cdc_test -- --ignored --nocapture # CHC-managed Postgres CDC via CLI
```

The managed-Postgres target uses the CLI's default table mapping and compares exact source and destination IDs and values after snapshot, INSERT, UPDATE, and DELETE. Destination reads use `FINAL WHERE _peerdb_is_deleted = 0`.

The trusted planner runs automatically on every same-repository PR push and selects suites from that PR's merge-base-to-head diff; it is read-only and never receives Cloud secrets. When it selects no live suites, the `Cloud integration decision` check turns green with no action needed. When live suites are selected, the decision check names them and stays `action_required` until `run-cloud-integration` is applied — the labeled event is the one-shot, exact-SHA authorization that admits the environment-bearing job, so re-apply the label after any push. Unknown API source or test paths select all suites, and changes to the classifier, its tests, or the workflow also select all suites through an independent workflow guard. Manual `Cloud Integration` dispatches accept `scope=all`, `service`, `postgres`, `organization`, or `clickpipes`. The focused scopes run only their corresponding suite; `clickpipes` runs Postgres CDC plus the fixture-gated smoke test, while `all` runs all four mandatory suites plus that optional smoke test. Because stacked-PR diffs exclude inherited changes, manually run `scope=all` against the top stack branch for full-stack validation.

`spec_coverage_test` sends the checked-in sources and snapshot through the
private `clickhouse-openapi-analyzer` crate. The analyzer recursively traverses
the module trees rooted at `client.rs`, `models.rs`, and `meta.rs`, including
private per-domain files. That same analyzer powers the scheduled live-spec
issue, so operation, model, field, optionality, beta, deprecation, enum,
snapshot, and stale-exemption findings share one implementation. The single
ignored test runs the same report against the live spec. The analyzer also checks
inline union payload fields and request requiredness. Report schema version 10 also
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

`integration_udf_test` creates two disposable UDFs, multiple versions, and a disposable
service. It checks empty/nonempty lists, real continuation pages at `limit=1`, and
attachment/deletion visibility, then cleans up on both success and failure. UDF client,
model, and CLI paths select the service integration suite, which runs this test.

### Optionality exemptions

Occasionally the spec cannot be followed literally because verified API
behavior differs. All such policy lives in
`crates/clickhouse-openapi-analyzer/src/config.rs`, including optionality,
extra-field, deprecated-field, extra-enum-value, non-OpenAPI-method, partial
required-schema, all-optional request-schema, and unsupported-enum configuration.

Add an exemption only for a deliberate runtime behavior and document why the
spec cannot be followed. New unsupported-enum acknowledgements also require a
tracking issue. The analyzer reports stale field/enum exemptions and vanished
unsupported locations so obsolete entries are removed during normal drift
remediation. Acknowledged locations also report changed enum values against
the snapshot, including numeric and mixed values; unchanged sets remain
acknowledged. See the repository `AGENTS.md` for exact key formats and the full
remediation and verification procedure.

### Postgres Query API smoke test

The ignored smoke test checks a read and an empty result against an existing
service without creating resources. Supply
`CLICKHOUSE_CLOUD_TEST_BEARER_TOKEN` (a `clickhousectl` audience OAuth token),
`CLICKHOUSE_CLOUD_TEST_ORG_ID`, and `CLICKHOUSE_CLOUD_TEST_POSTGRES_SERVICE_ID`.
Optionally set `CLICKHOUSE_CLOUD_TEST_POSTGRES_DATABASE`,
`CLICKHOUSE_CLOUD_API_BASE_URL`, or `CLICKHOUSE_CLOUD_QUERY_HOST` for another database
or environment. Run:

```bash
cargo test -p clickhouse-cloud-api --test run_query_test live_postgres_query_bearer_smoke -- --ignored --nocapture
```

For additional live coverage, verify that the route rejects write statements,
invalid tokens, and tokens without access to the requested organization or service,
and repeat the smoke test with an explicit non-default database.

### OpenAPI response coverage

The OpenAPI analyzer checks inline union payload fields and request requiredness, plus inline JSON response objects named `{PascalizedOperationId}Response{Status}` and reachable through client return types or error payloads. It reports obsolete helper exclusions and requiredness overrides as stale exemptions. Acknowledged unsupported enum locations are checked against the snapshot: changed value sets are actionable, while reordering is ignored. Deprecated API-key `roles` fields remain strings behind `deprecated-fields` for source compatibility. Its report format is version 12.


Success response checking follows each operation's actual Rust return type. Generic
applications and aliases retain their type arguments, which are substituted through
response fields, flattened wrappers, arrays, and typed maps. An incompatible payload
kind or referenced element model produces `response_shape_mismatch` with an operation,
Rust field, and exact specification pointer. These checks use the target specification
independently of the snapshot; removing a component and refreshing the snapshot cannot
hide an obsolete return type. Response `Option` fields remain legitimate regardless of
specification requiredness. Nullable 3.0 and 3.1 forms are supported.

The same shape checker runs in request position. Every request-position model field is
checked against its property schema (`request_shape_mismatch`; generic models use their
default type arguments), and every operation with a JSON `requestBody` must take exactly
one argument that no parameter accounts for, typed as the body schema's request model
(`request_body_mismatch` when the argument is missing, ambiguous or has no request body;
an untyped `serde_json::Value` body is `unsupported_request_schema`). An inline object
body may instead be passed as one argument per property. Deliberate request-field
divergences are listed in `request_shape_exemptions`.

Resolved JSON success contracts are also compared between specifications, including
referenced schemas. Added, removed, and changed contracts have separate snapshot
findings with the previous/current contracts and locations. Prose, examples, and
ordering of property/required sets do not cause contract drift. These findings describe
specification changes, not necessarily a Rust incompatibility or a backend rollout.

A typed `additionalProperties` map whose value is an inline `oneOf`/`anyOf` must use an
untagged union whose tuple arms pair one-to-one with the branches; an inline object branch
pairs with a struct carrying exactly its properties, strict where required in request
position. A response map value may instead be `serde_json::Value`, the lossless form for an
undiscriminated composition.

This is structural coverage, not a proof of every possible live value: arbitrary JSON
payloads remain lossless catch-alls, existing union checks own named/inline variant
coverage, and numeric ranges, string patterns, custom deserializers, non-JSON responses,
and undocumented backend behavior are not validated by the shape checker. Unsupported
compositions, unresolved generic bindings, alias cycles, and analysis depth limits are
reported explicitly rather than counted as successful analysis.

### Operation metadata generation

The committed catalog is generated by the shared analyzer, without runtime parsing or network access:

```sh
cargo run -p clickhouse-openapi-analyzer --bin openapi-drift-analyzer -- \
  --spec crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json \
  --generate-operations crates/clickhouse-cloud-api/src/meta/operations.rs
cargo fmt --all
```

The analyzer's `clickhouse_cloud_config()` enables `check_operation_permissions`; generic `AnalyzerConfig::default()` leaves this Cloud-specific convention disabled. Enabled analysis resolves operation security overrides and root inheritance, normalizes permission sets, and reports unknown, malformed, anonymous, alternative, or multiple-scheme requirements as actionable findings. Generation refuses these unsupported forms. Permission changes are compared both against the snapshot and against the Rust descriptors, so refreshing the snapshot alone cannot hide stale permissions. Descriptor identity, matching client methods, coverage, and the complete sorted catalog are checked too.
