# Analyzer policy and configuration

Detail for drift remediation and analyzer changes. The model policy it enforces is in
`crates/clickhouse-cloud-api/AGENTS.md`.

## Analyzer configuration and exemptions

All policy lives in `crates/clickhouse-openapi-analyzer/src/config.rs`; edit `clickhouse_cloud_config()` or its
backing constants. Introduce a named, documented constant when an empty policy list first gains entries. Keys use
Rust type names but spec/wire field and enum values:

- `non_openapi_client_methods` — intentional `Client` helpers with no operation, keyed by snake-case method name.
  A removed helper or one that gains a matching operation is reported as stale.
- `optionality_exemptions` — fields deliberately optional despite the resolved spec, keyed by
  `(RustStructName, specFieldName)`. Request-position only, so a response-only entry can never hit and surfaces as stale.
- `fractional_response_exemptions` — verified fractional runtime measurements declared as integers by the spec,
  keyed by `(RustStructName, specFieldName)`. Only response-only `f64` fields qualify; request fields cannot be
  exempted. Entries become stale when the field, response reachability, Rust type, or upstream integer type changes.
- `extra_field_exemptions` — deliberate code-only fields, keyed by `(RustStructName, specFieldName)`.
- `deprecated_field_exemptions` — spec-deprecated fields deliberately excluded from hiding, same key shape.
- `extra_enum_value_exemptions` — intentional Rust-only wire values, keyed by `(RustEnumName, wireValue)`.
- `partial_required_schemas` — upstream schemas whose `required[]` is non-exhaustive, keyed by spec schema name.
  This changes requiredness resolution (request position only) and is not a shortcut for one optionality mismatch.
  Entries are stale when the schema is gone, response-only, or the override no longer changes requiredness.
- `acknowledged_unsupported_enum_pointers` — exact RFC 6901 pointers the analyzer inventories but cannot map to a
  concrete Rust value enum. The acknowledgement covers the snapshot's value set; additions or removals in the
  target spec are actionable, including numeric and mixed values. Reordering does not change the set.

Add an exemption only for intentional, verified runtime behavior, with a nearby comment stating why the spec cannot
be followed. Never exempt missing API surface or ordinary model drift. Pair a new unsupported-enum acknowledgement
with a tracking issue; do not acknowledge it merely to make CI green. Exemptions and acknowledgements
produce actionable stale findings when no longer needed — remove them during remediation.

## Enum value coverage, `VALUES` consts, deprecated hiding

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

## Extending the analyzer

Add a typed `FindingKind` and pure comparison in the analyzer, focused inventory/comparison fixtures,
deterministic JSON/text coverage, and Python issue rendering. Keep `spec_coverage_test.rs` a thin consumer.
New report fields or semantics require a report `schema_version` change; never make Python infer drift by
reparsing Rust or OpenAPI.
