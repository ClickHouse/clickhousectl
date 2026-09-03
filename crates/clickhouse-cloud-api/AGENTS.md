# AGENTS.md — `clickhouse-cloud-api` and `clickhouse-openapi-analyzer`

Read with the root `AGENTS.md` (commands, workspace rules, CI gates). This file covers the published API library
and the private drift analyzer, which are always edited together.

## Layout

- `src/client.rs` — `Client` and shared HTTP machinery; endpoint methods live in private per-domain `src/client/*.rs`.
- `src/models.rs` — the public model facade and the shared `discriminated_union!` macro (its rustdoc owns the
  grammar). Request/response structs, enums, aliases and impls live in private per-domain `src/models/*.rs` and
  are re-exported without changing the crate-root or `models::*` paths.
- `src/convert.rs` — `MissingRequiredFields` and conversion documentation; explicit response→request conversions
  live in private per-domain `src/convert/*.rs`.
- `src/error.rs` — `Error` is the structural contract for failure modes: a failure a caller must tell apart gets its
  own variant, never a recognizable message. `Error::Sql` (the Query API rejecting a statement) exists precisely
  so callers stop sniffing a `SQL error ` prefix. Keep each variant's `Display` stable — it is what the user sees.
- `crates/clickhouse-openapi-analyzer/` — OpenAPI and Rust inventory, direction-aware comparison, policy config, and
  stable drift reports. Private (`publish = false`), a dev-dependency of this crate. Parser/tooling deps such as
  `syn` must not enter either published crate's normal dependency graph. It recursively traverses the private module
  trees rooted at `client.rs`, `models.rs`, `meta.rs`; model declarations must remain literal source in that tree,
  and declarations in conversion files do not count as models.

## Request and response models

A response must never fail to deserialize because the API dropped a field, sent it as `null`, or added one —
several teams evolve the Cloud API independently, so in a published crate every strict response field is a latent
outage. Tolerance lives in the **type system**, not in serde attributes:

- **Request types are strict.** A spec-required field is `T`; optional or nullable fields are `Option<T>` plus
  `#[serde(skip_serializing_if = "Option::is_none")]`. The compiler enforces "strict in what we send".
- **Response types are all-`Option`.** Every field of every type reachable from a `Client` return type is `Option<T>`
  plus `skip_serializing_if`; a missing key *and* an explicit `null` both land as `None` natively. Nothing is
  fabricated, so "the server sent `0`" and "the server dropped the field" stay distinguishable.
- **`#[serde(default)]` is banned in the model module tree.** On a required request field it invents `""`/`0`/
  `false` that a get → edit → write-back caller silently persists; on an `Option` field it is dead weight.
  Sweeping it across every field was the superseded policy of #312/#313 — do not reintroduce it.
- **Unknown fields are ignored.** Never `deny_unknown_fields`.

### Naming and the split

- A schema used in one direction keeps its Rust name — most schemas are one-directional, so most models are
  simply all-`Option` in place (response) or strict in place (request body, orphan schema).
- A schema used in **both** directions becomes two types: the request variant keeps the schema's name; the
  response variant is exactly `{Name}Response`, matching spec-derived names like `ApiKeyPostResponse`. The
  analyzer resolves the pair from that convention, so the suffix is load-bearing, not cosmetic.
- Splitting propagates: a nested type reachable from both a strict and a tolerant parent splits too, and a field
  of a response type must point at the `*Response` variant of anything that has one. Re-point the element type of
  any shared alias (`pub type PgTagsResponse = Vec<ResourceTagsV1Response>`).
- Object unions follow the variant structs they hold. When those split, duplicate the whole
  `discriminated_union!` invocation for a `{Name}Response` enum over the `*Response` structs, keeping the
  discriminator and `none unless` guards identical. The macro emits only `Deserialize`, so the enum declaration,
  derives, `Display` and any `Default` stay literal source for the syn analyzer to inventory.
- Enums and unions are otherwise **unchanged** by the split: string enums keep `Unknown(String)`, unions keep the
  lossless `Unknown(serde_json::Value)` fallback, and dispatch reads raw JSON rather than struct fields, so
  all-`Option` variants cannot weaken it.

### Serialization and write-back

- Response types keep `derive(Serialize)` — `--json` and `print_human` serialize them directly.
  `skip_serializing_if` on every response field means an absent field is **omitted**, never `null`. A test pins it.
- A caller that fetches, edits and writes back must resolve absence explicitly. The owning `src/convert/<domain>.rs`
  holds those conversions: `TryFrom<{Name}Response> for {Name}` where a required request field can be absent,
  `From` where the conversion is total. A fallible one returns `MissingRequiredFields` (re-exported at the crate
  root; `.fields()` lists the missing **wire** names). Give each nested object its own conversion so a missing
  field is named at the level it is missing from; reuse `MissingRequiredFields` rather than adding another error type.
- Residual risk: a key *present* with a changed type still fails — enums absorb it via catch-alls and unions via
  `Unknown(Value)`, a plain struct field does not. Spec conformance is the drift job's responsibility.

### How the policy is enforced

`tests/spec_coverage_test.rs` pins all of it against the analyzer's `response_tree()`, which derives response
reachability from return types across the client module tree, so a newly wired operation is covered automatically:

- `every_response_tree_field_is_option` — no non-`Option` field in the tree. Its exception list is empty; an entry
  needs the same bar as an analyzer exemption. A vacuity guard fails the test if the tree collapses.
- `every_response_tree_option_field_omits_none_when_serialized` — `skip_serializing_if` on every response `Option` field.
- `models_carry_no_serde_default` — via the analyzer's `model_fields_with_serde_default()`.
- `scim_models_are_outside_the_response_tree` — the 40 `Scim*` schemas have no spec path and no `Client` method,
  so they are legitimately strict; the test fails if one becomes response-reachable.
- `integer_schema_fields_are_not_typed_as_float` — integer schemas do not use floating-point Rust fields.

Scope enforcement to the response tree, never to "every model type": operation-unreferenced and request-only
schemas resolve in request position, so making them all-`Option` reports genuine `FieldOptionalityMismatch` drift.

## OpenAPI drift

Spec: https://api.clickhouse.cloud/v1

- `.github/workflows/openapi-drift.yml` runs `scripts/check-openapi-drift.py` daily.
  `python3 scripts/check-openapi-drift.py --dry-run` reproduces the rendered issue without creating one.
- The analyzer is the single implementation of parsing and comparison. Do not duplicate source parsing, exemptions,
  or comparison logic in tests or in Python; Python owns fetching, issue rendering, and GitHub orchestration only.
  Module cfg evaluation uses the analyzer host target, excludes `test`, treats feature-gated API as enabled, and
  conservatively retains unknown custom cfgs.
- `tests/spec_coverage_test.rs` analyzes the vendored snapshot; its `#[ignore]`d test analyzes the live spec.
  Both and the scheduled workflow call the same analyzer and must agree.

### Remediating a drift issue

Work from the issue's typed findings: `spec_pointer` is an RFC 6901 location in the target spec, `rust_item` the
intended Rust location. The analyzer executable exits successfully after producing a valid report even when
`findings` is non-empty — use `has_drift`/`actionable_count`, not its process status.

1. Reproduce with `python3 scripts/check-openapi-drift.py --dry-run`. It does not update the snapshot.
2. Replace `clickhouse_cloud_openapi.json` with the same live document being remediated; never hand-edit the
   spec. Snapshot operation/schema findings mean this file is stale.
3. Fix the API library before considering CLI exposure, following the finding's pointer and Rust item:
   - Missing/extra operations: add or remove the `Client` method in the owning `src/client/<domain>.rs`. Only
     intentional non-OpenAPI helpers belong in `non_openapi_client_methods`.
   - Missing models/fields, extra fields: update structs/enums/aliases and serde names in the owning
     `src/models/<domain>.rs`, then re-export from the `models.rs` facade. An undefined `$ref`
     (`missing_schema_definition`) is an upstream-spec defect, not a model to invent locally. A new schema needs
     one Rust type per position it is used in (`{Name}`, `{Name}Response`, or both) with the same field on each.
   - The model tree uses explicit `#[serde(rename = "...")]` wire names exclusively; `rename_all` is rejected by the
     analyzer parser, since wire vocabulary (Postgres GUCs, SCIM URNs, region IDs, duration literals) cannot be
     derived from Rust identifiers by any casing rule.
   - Optionality: request-position fields are `T` when the resolved spec requires them, else `Option<T>` plus
     `skip_serializing_if`; every response-position field is `Option<T>` plus `skip_serializing_if` whatever the
     spec says. Never add `#[serde(default)]`. A request field deliberately optional against the resolved spec
     needs an `optionality_exemptions` entry keyed on the **request** variant's name.
   - Missing/extra enum values: update the typed enum, its serde wire value, and its `Display`. Preserve
     data-carrying catch-alls.
   - Beta/deprecation: regenerate `BETA_OPERATIONS` with `python3 scripts/regenerate-beta-lists.py` and
     `DEPRECATED_FIELDS` with `python3 scripts/regenerate-deprecated-fields.py`; deprecated fields also need the
     matching `#[cfg(feature = "deprecated-fields")]` marker in their model domain file. The generators work from
     the spec, which knows nothing about split variants, so a deprecated field on a split schema needs the
     `{Name}Response` entry and its marker added by hand.
   - Stale exemption: remove or narrow the configuration entry. Never change comparison logic to preserve one.
   - Unsupported enum constraint: prefer changing the Rust scalar to a concrete value enum. Acknowledgement is
     the fallback policy below.
4. Add focused library tests for changed models/methods: a new response type wants a missing-key → `None` and an
   explicit-`null` → `None` case; a new split pair wants the request variant's strictness and the `TryFrom`
   write-back asserted. If the unsupported inventory changes, update `acknowledged_unsupported_enum_pointers` — the
   snapshot test derives its expected inventory from that configuration.
5. Verify with the crate commands in the root `AGENTS.md`, then re-run the dry run against the live document.

### Field optionality and the spec

- Checking is direction-aware: the analyzer classifies every spec schema by the position(s) it is used in —
  request position (reachable from a request body or operation parameter) and/or response position.
- Requiredness applies in **request position only**. Response-side optionality drift is invisible by design:
  every response field is `Option<T>` by policy, so comparing requiredness would fire on all of them.
- Field *presence* (missing/extra) and enum values are checked in **both** directions.
- A both-directions schema resolves to `{Name}` in request position and `{Name}Response` in response position,
  falling back to `{Name}` when no split type exists. An orphan schema resolves in request position, so it stays strict.
- Requiredness semantics in `openapi.rs`: PATCH request schemas are all-optional; nullable fields are always
  `Option<T>`; schemas with `required[]` use it; schemas without it use the `"Optional"` description convention.
  A schema whose `required[]` is known to be partial uses the union of both and must be listed in
  `partial_required_schemas`. (`scripts/resolve-field-requirements.py` is a code-generation aid only.)
- `response_tree()` exposes response-tree membership for the policy enforcement tests.

### Analyzer configuration and exemptions

All policy lives in `crates/clickhouse-openapi-analyzer/src/config.rs`; edit `clickhouse_cloud_config()` or its
backing constants. Introduce a named, documented constant when an empty policy list first gains entries. Keys use
Rust type names but spec/wire field and enum values:

- `non_openapi_client_methods` — intentional `Client` helpers with no operation, keyed by snake-case method name.
- `optionality_exemptions` — fields deliberately optional despite the resolved spec, keyed by
  `(RustStructName, specFieldName)`. Request-position only, so a response-only entry can never hit and surfaces as stale.
- `extra_field_exemptions` — deliberate code-only fields, keyed by `(RustStructName, specFieldName)`.
- `deprecated_field_exemptions` — spec-deprecated fields deliberately excluded from hiding, same key shape.
- `extra_enum_value_exemptions` — intentional Rust-only wire values, keyed by `(RustEnumName, wireValue)`.
- `partial_required_schemas` — upstream schemas whose `required[]` is non-exhaustive, keyed by spec schema name.
  This changes requiredness resolution (request position only) and is not a shortcut for one optionality mismatch.
- `acknowledged_unsupported_enum_pointers` — exact RFC 6901 pointers the analyzer inventories but cannot map to a
  concrete Rust value enum.

Add an exemption only for intentional, verified runtime behavior, with a nearby comment stating why the spec cannot
be followed. Never exempt missing API surface or ordinary model drift. Pair a new unsupported-enum acknowledgement
with a tracking issue; do not acknowledge it merely to make CI green. Pair-keyed exemptions and acknowledgements
produce actionable stale findings when no longer needed — remove them during remediation.

### Enum value coverage, `VALUES` consts, deprecated hiding

- Enum mapping is structural: named schemas resolve to model types; properties, array items, compositions and
  operation parameters resolve through their Rust field/argument type. Serde container/variant renames determine
  wire values. Catch-alls are recognized through `untagged`/`other` attributes, never variant names — a genuine
  unit variant named `Unknown` remains a value. Numeric, mixed and scalar-backed enum constraints are reported
  explicitly as unsupported rather than silently skipped.
- Enums the CLI validates against declare `pub const VALUES: &'static [&'static str]` — a hand-written literal
  slice of the enum's non-catch-all wire values. The analyzer requires it to equal the variant wire values as a
  set (`FindingKind::EnumValuesMismatch`); this is opt-in, so enums without a `VALUES` const are unchecked.
  **When adding a value to a `VALUES`-bearing enum, update both the variant and the const or CI fails.**
- Every spec-deprecated request or response field belongs in `meta.rs::DEPRECATED_FIELDS` and carries
  `#[cfg(feature = "deprecated-fields")]` on the field in its model domain file, so it is absent from the public
  model by default. Request fields that must be gated out but resolve as required are `Option<T>` with a
  documented optionality exemption. Entries are keyed per Rust type, so a deprecated field on a split schema needs
  one entry and one marker for `{Name}` and one for `{Name}Response`. Update CLI code that accesses or constructs
  an affected model so **both** feature configurations compile.

### Extending the analyzer

Add a typed `FindingKind` and pure comparison in the analyzer, focused inventory/comparison fixtures,
deterministic JSON/text coverage, and Python issue rendering. Keep `spec_coverage_test.rs` a thin consumer.
New report fields or semantics require a report `schema_version` change; never make Python infer drift by
reparsing Rust or OpenAPI.

## Tests

Real cloud integration tests, 100% OpenAPI spec coverage. Cost is not a reason to skip a test. Call `Client` directly
from Rust.

- `tests/common/support.rs` — generic infra (polling, logging, env helpers, ClickHouse provisioning & cleanup,
  HTTP query helper). Used by every integration binary.
- `tests/integration_test.rs`, `tests/integration_postgres_test.rs`, `tests/integration_org_test.rs` —
  cloud-service, Postgres-service, and organization lifecycle.
- `tests/client_test.rs`, `tests/models_test.rs`, `tests/model_facade_test.rs`, `tests/run_query_test.rs`,
  `tests/service_query_key_cli_test.rs` — offline client, model, facade and query-path coverage.
- `tests/clickpipes/` — ClickPipes E2E, including external cloud services. Only Postgres CDC (ClickHouse and
  Postgres inside ClickHouse Cloud) runs in CI; its `clickpipe_postgres_cli_cdc_test` target exercises the built
  `clickhousectl` supplied through `CLICKHOUSE_CLOUD_TEST_CLICKHOUSECTL_BIN`, which
  `.github/workflows/cloud-integration.yml` builds first. Third-party-service tests must be run manually. CI also
  optionally runs `clickpipe_smoke_test` against a long-lived service when the repo variable
  `CLICKHOUSE_CLOUD_TEST_CLICKPIPE_SERVICE_ID` is set; the step is skipped when it is unset.
- `tests/spec_coverage_test.rs` — runs the shared analyzer against the vendored snapshot and requires an
  actionable-drift-free report.
