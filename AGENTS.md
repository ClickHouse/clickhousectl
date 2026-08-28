# CLAUDE.md

clickhousectl (or chctl) is the official CLI for ClickHouse, by ClickHouse Inc. clickhousectl supports both ClickHouse and Postgres, on your local machine or in ClickHouse Cloud.

## Architecture

This is a Cargo workspace with three crates:

### CLI (`crates/clickhousectl/`)

The user-facing CLI surface. Contains all logic for local commands, wraps `clickhouse-cloud-api` for cloud.

- New Cloud handlers go through `CloudClient` wrapper methods co-located in each domain module, not `clickhouse_cloud_api::Client` directly. `src/cloud/client.rs` owns the core client, credential precedence, error conversion, and response unwrapping. Some pre-modularization Postgres and service-query paths still call the API client directly; do not copy that pattern into new commands.
- Cloud handlers always support `--json` output unless there is good reason not to. JSON is emitted automatically when `--json` is passed or a coding agent is detected (`is_ai_agent::detect()` via the `json_output()` helper in `main.rs`).
- `CloudError` carries a `kind: CloudErrorKind` (`Auth` for 401/403 and missing credentials, else `Generic`). It maps to `Error::AuthRequired` / `Error::Cloud` in `cloud::run`. Dispatched commands exit with `0` on success or use `Error::exit_code()` for failures: `1` error, `3` cancelled, `4` auth required. Clap uses `2` for usage errors.

Use `--help` to learn the current command surface.

Project-local data lives in `.clickhouse/`. Globally installed ClickHouse binaries live in `~/.clickhouse/`. OAuth tokens (`~/.clickhouse/tokens.json`) are the exception — they're global user identity, not project-scoped.

The CLI does not need to have 100% coverage of endpoints exposed by the API library: be intentional about what is exposed to users.

#### Adding a command

Local clap definitions live in `src/local/cli.rs`. Cloud clap definitions, handlers, builders, wrapper methods, dispatch, and tests are co-located in the owning domain module under `src/cloud/`; `src/cloud/cli.rs` owns the top-level cloud arguments, command enum, domain re-exports, delegation, and top-level tests.

**Local subcommand:**

1. Add a variant to the relevant enum in `src/local/cli.rs` using clap derive macros.
2. Add the match arm in `run()` in `src/local/mod.rs`; `main.rs` delegates to that boundary.
3. Implement the handler in a dedicated module under `src/local/` (e.g. `src/local/server.rs`, `src/local/postgres.rs`). Don't pile new logic into `main.rs`.

**Cloud subcommand:**

1. Make sure `clickhouse-cloud-api` has already been updated to support necessary endpoints and models.
2. Add the clap variant and argument structs to the owning `src/cloud/<domain>.rs` module. Create a new domain module and privately re-export its command enum from `src/cloud/cli.rs` if the surface warrants its own grouping.
3. Classify the variant in the domain command enum's exhaustive `is_write()` match. OAuth (Bearer) auth is read-only; write commands require API key auth and fail fast on OAuth + write. `CloudCommands::is_write_command()` in `src/cloud/cli.rs` exhaustively delegates to each domain. Add read/write classification tests next to the domain clap definitions.
4. Add the exhaustive command match to the domain's `run()` dispatcher. `cloud::dispatch()` in `src/cloud/mod.rs` delegates only at the top-level `CloudCommands` boundary; add one delegation arm there only when introducing a new domain.
5. Add a thin wrapper method in the domain module's `impl CloudClient` block. It should delegate to `self.api().<lib_method>()`, map errors via `self.convert_error(e)` or `self.convert_error_for_organization(e, org_id)`, and unwrap with `Self::unwrap_response`. Use the library's request/response types here.
6. If the command sends a request body, extract a `build_<name>_request(...)` helper in the same domain module that returns the library's request struct. Cover the helper with minimal + maximal unit tests in that module's `mod tests`, asserting directly on library struct fields.
7. Implement the handler in the same domain module. For body-sending commands the handler calls the build helper, passes the result through the `CloudClient` wrapper, and prints with the `--json` output pattern. For detail/get views (rendering a single resource), drive human output through `print_human` so it shares serde's behaviour — including deprecated-field hiding — instead of hand-writing `println!` lines:
   ```rust
   if json {
       println!("{}", serde_json::to_string_pretty(&data)?);
   } else {
       print_human(&data)?;
   }
   ```
   List views stay as `tabled` tables, and short action confirmations (e.g. "Service X starting") stay as plain `println!`.

   Every field of a library response type is `Option` (see Request and response models), so never `unwrap()`/`expect()` one. Render absence with `crate::cloud::output::or_absent` (`-`) or `ABSENT` in `tabled` cells and plain output, and have `--filter`-style predicates treat an absent field as non-matching. `print_human` and `--json` serialize the model and need no per-field work.
8. Add `Cli::try_parse_from` coverage next to the domain command definition for the new command's body-related flags, asserting parsed values.

### API library (`crates/clickhouse-cloud-api/`)

Typed Rust client library for the ClickHouse Cloud API. The library owns typed HTTP interaction and all cloud integration testing; the private analyzer owns OpenAPI parsing and comparison.

- `src/client.rs` — `Client` and shared HTTP machinery; endpoint methods live in private per-domain `src/client/*.rs` files.
- `src/models.rs` — the public model facade and shared discriminated-union macro. Request/response structs, enums, aliases, and their implementations live in private per-domain `src/models/*.rs` files and are re-exported without changing the crate-root or `models::*` paths.
- `src/convert.rs` — `MissingRequiredFields` and conversion documentation; explicit response→request conversions live in private per-domain `src/convert/*.rs` files.

The drift analyzer recursively traverses the private module trees rooted at `client.rs`, `models.rs`, and `meta.rs`. Model declarations remain literal source in that tree; declarations in conversion files do not count as models.

### OpenAPI analyzer (`crates/clickhouse-openapi-analyzer/`)

Private workspace tooling for OpenAPI and Rust inventory, direction-aware comparison, policy configuration, and stable drift reports. It is not published.

The API library can be updated independently of the CLI. When OpenAPI drifts, prefer updating API library on its own, add to CLI separately.

#### Request and response models

A response must never fail to deserialize because the API dropped a field, sent it as `null`, or added one. Several teams evolve the Cloud API independently, so in a crate published to crates.io every strict response field is a latent outage. Tolerance lives in the **type system**, not in serde attributes:

- **Request types are strict.** A field the spec requires is `T`; optional or nullable fields are `Option<T>` plus `#[serde(skip_serializing_if = "Option::is_none")]`. The compiler is what enforces "strict in what we send".
- **Response types are all-`Option`.** Every field of every type reachable from a `Client` return type is `Option<T>` plus `skip_serializing_if`. A missing key *and* an explicit JSON `null` both land as `None`, natively — no attribute needed. Nothing is fabricated, so "the server sent `0`" and "the server dropped the field" stay distinguishable, and each caller resolves absence where it is used.
- **`#[serde(default)]` is banned in the model module tree.** On a required request field it invents `""`/`0`/`false` that a get → edit → write-back caller silently persists; on an `Option` field it is dead weight. Sweeping it across every model field was the superseded policy of issues 312 and 313 — do not reintroduce it.
- **Unknown fields are ignored.** Never `deny_unknown_fields`.

##### Naming and the split

A schema used in one direction only keeps its Rust name: most schemas are one-directional, so most models are simply all-`Option` in place (response) or strict in place (request body, orphan schema). A schema used in **both** directions becomes two types — the request variant keeps the schema's name, the response variant is exactly `{Name}Response`, matching spec-derived names like `ApiKeyPostResponse`. The analyzer resolves the pair from that convention, so the suffix is load-bearing rather than cosmetic. Splitting propagates: a nested type reachable from both a strict and a tolerant parent splits too, and a field of a response type must point at the `*Response` variant of anything that has one. Also re-point the element type of any shared alias (`pub type PgTagsResponse = Vec<ResourceTagsV1Response>`).

Object unions follow the variant structs they hold. When those split, duplicate the whole `discriminated_union!` invocation for a `{Name}Response` enum over the `*Response` structs and keep the discriminator and `none unless` guards identical; the macro emits only `Deserialize`, so the enum declaration, derives, `Display` and any `Default` stay literal source for the syn analyzer to inventory. Enums and unions are otherwise **unchanged** by the split: string enums keep their `Unknown(String)` catch-all, unions keep the lossless `Unknown(serde_json::Value)` fallback, and dispatch reads the raw JSON rather than struct fields, so all-`Option` variants cannot weaken it. The `discriminated_union!` rustdoc in `models.rs` owns the grammar.

##### Serialization: absent means omitted

Response types keep `derive(Serialize)` — `--json` and `print_human` serialize them directly. `skip_serializing_if` on every response field means an absent field is **omitted**, never emitted as `null`. That is a deliberate decision with a test pinning it: `--json` consumers see the key set the API actually sent.

##### Write-back conversions

Because the variants are distinct types, a caller that fetches a resource, edits it and writes it back must resolve absence explicitly — that is the point of the split, not an inconvenience of it. The owning `src/convert/<domain>.rs` file contains those conversions: `TryFrom<{Name}Response> for {Name}` where a required request field can be absent, `From` where the conversion is total. A fallible one returns `MissingRequiredFields` (re-exported at the crate root; `.fields()` lists the missing **wire** names). Give each nested object its own conversion so a missing field is named at the level it is missing from, and reuse `MissingRequiredFields` rather than adding a second error type.

##### The residual, honestly

A key that is *present* with a changed type still fails. `Option<T>` absorbs absence and `null`, not a string where an array used to be — no more than `serde(default)` did. Enums absorb it through their catch-alls and `discriminated_union!` unions through the `Unknown(Value)` fallback, but a plain struct field does not. Spec conformance is the drift job's responsibility, not the runtime's.

##### How the policy is enforced

`crates/clickhouse-cloud-api/tests/spec_coverage_test.rs` pins all of it against the analyzer's `response_tree()`, which derives response reachability from return types across the client module tree so a newly wired operation is covered automatically:

- `every_response_tree_field_is_option` — no non-`Option` field in the tree. Its exception list is empty; an entry needs the same bar as an analyzer exemption. A vacuity guard fails the test if the tree collapses.
- `every_response_tree_option_field_omits_none_when_serialized` — `skip_serializing_if` on every response `Option` field.
- `models_carry_no_serde_default` — via the analyzer's `model_fields_with_serde_default()`.
- `scim_models_are_outside_the_response_tree` — the 40 `Scim*` schemas have no path in the spec and no `Client` method, so they are legitimately strict, and the test fails if one becomes response-reachable.
- `integer_schema_fields_are_not_typed_as_float` — integer schemas do not use floating-point Rust fields.

Scope enforcement to the response tree, never to "every model type": operation-unreferenced and request-only schemas resolve in request position, so making them all-`Option` reports genuine `FieldOptionalityMismatch` drift.

#### OpenAPI drift

ClickHouse Cloud OpenAPI spec: https://api.clickhouse.cloud/v1

- `.github/workflows/openapi-drift.yml` runs `scripts/check-openapi-drift.py` daily. Python owns fetching, issue rendering, and GitHub orchestration only; `python3 scripts/check-openapi-drift.py --dry-run` reproduces the rendered issue without creating one.
- `crates/clickhouse-openapi-analyzer` is the single implementation of parsing and comparison. `rust_inventory.rs` recursively walks and parses private and public modules rooted at `client.rs`, `models.rs`, and `meta.rs` (including both `<module>.rs` and `<module>/mod.rs`) with `syn`; module cfg evaluation uses the analyzer host target, excludes `test`, treats feature-gated API as enabled, and conservatively retains unknown custom cfgs. `openapi.rs` inventories the target spec and vendored snapshot; `compare.rs` maps them and emits typed findings; `config.rs` owns ClickHouse-specific policy; `report.rs` defines the stable JSON/text report; `main.rs` is the executable used by Python. Do not duplicate source parsing, exemptions, or comparison logic in tests or Python.
- The analyzer is private (`publish = false`) and a dev dependency of `clickhouse-cloud-api`. Parser/tooling dependencies such as `syn` must not enter either published crate's normal dependency graph.
- `crates/clickhouse-cloud-api/tests/spec_coverage_test.rs` analyzes the vendored snapshot; its ignored test analyzes the live spec. Both and the scheduled workflow call the same analyzer and must agree.

##### Remediating a drift issue

Work from the issue's typed findings. `spec_pointer` is an RFC 6901 location in the target spec and `rust_item` is the intended Rust location. The analyzer executable exits successfully after producing a valid report even when `findings` is non-empty; use `has_drift`/`actionable_count`, not its process status, to decide whether drift exists.

1. Reproduce with `python3 scripts/check-openapi-drift.py --dry-run`. The command does not update the snapshot.
2. Replace `crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json` with the same live document being remediated; do not hand-edit the spec. Snapshot operation/schema findings mean this file is stale.
3. Fix the API library before considering CLI exposure. Follow the finding's pointer and Rust item:
   - Missing/extra operations: add or remove the corresponding `Client` method in the owning `src/client/<domain>.rs` file; only intentional non-OpenAPI helpers belong in `non_openapi_client_methods`.
   - Missing models, fields, or extra fields: update public structs/enums/type aliases and Serde names in the owning `src/models/<domain>.rs` file, then re-export a new type from the `models.rs` facade. An undefined `$ref` (`missing_schema_definition`) is an upstream-spec defect, not a model to invent locally. The model tree uses explicit `#[serde(rename = "...")]` wire names exclusively; `rename_all` is rejected by the analyzer parser, because wire vocabulary (Postgres GUCs, SCIM URNs, region IDs, duration literals) cannot be derived from Rust identifiers by any casing rule, and explicit literals keep the code↔spec mapping verbatim and greppable. A new schema needs one Rust type per position it is used in: `{Name}` if the finding is in request position, `{Name}Response` if in response position, both if the spec uses it in both — and the same field added to every variant. See Request and response models above.
   - Optionality: express requiredness in the type, and pick the shape from the position. Request-position fields are `T` when the resolved spec requires them and `Option<T>` plus `skip_serializing_if` otherwise; every response-position field is `Option<T>` plus `skip_serializing_if`, whatever the spec says its requiredness is. Never add `#[serde(default)]`. A request field deliberately optional against the resolved spec needs an `optionality_exemptions` entry keyed on the **request** variant's name.
   - Missing/extra enum values: update the typed enum, its Serde wire value, and its `Display` implementation. Preserve data-carrying catch-all variants.
   - Beta/deprecation findings: regenerate `BETA_OPERATIONS` with `python3 scripts/regenerate-beta-lists.py` and `DEPRECATED_FIELDS` with `python3 scripts/regenerate-deprecated-fields.py`; deprecated fields also need the matching `#[cfg(feature = "deprecated-fields")]` marker in their model domain file. The generator works from the spec, which knows nothing about split variants, so a deprecated field on a split schema needs the `{Name}Response` entry and its marker added by hand.
   - Stale exemption: remove or narrow the configuration entry. Do not change comparison logic to preserve a stale exception.
   - Unsupported enum constraint: prefer changing the Rust scalar to a concrete value enum. Acknowledgement is the fallback policy below, not a model-drift fix.
4. Add focused library tests for changed models/methods. A new response type wants a missing-key → `None` and an explicit-`null` → `None` case; a new split pair wants the request variant's strictness and the `TryFrom` write-back asserted. If the unsupported inventory changes, update `acknowledged_unsupported_enum_pointers`; the snapshot test derives its exact expected inventory from that configuration.
5. Verify with `cargo test -p clickhouse-cloud-api -p clickhouse-openapi-analyzer`, `cargo clippy -p clickhouse-cloud-api -p clickhouse-openapi-analyzer --all-targets -- -D warnings`, and `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`. If deprecated fields changed, also run `cargo check --workspace --all-features`. Re-run the dry run to check the live document.

##### Field optionality and the OpenAPI spec

Checking is direction-aware, matching the request/response model split above: the analyzer classifies every spec schema by the position(s) it is used in — request position (reachable from a request body or operation parameter) and/or response position (reachable from an operation response). Requiredness applies in request position only. Response-side optionality drift is therefore invisible by design and that is deliberate, not a gap: every response field is `Option<T>` by policy, so a requiredness comparison would fire on all of them and signal nothing. Field *presence* (missing/extra fields) and enum values — the drift that actually matters — are checked in both directions. A schema used in both directions resolves to Rust `{Name}` in request position and `{Name}Response` in response position, falling back to `{Name}` when no split type exists, so the naming convention is what makes the dual mapping mechanical instead of hand-maintained. Response-tree membership is exposed via `clickhouse_openapi_analyzer::response_tree()` for the policy enforcement tests.

An operation-unreferenced (orphan) schema resolves in request position, so it stays strict.

Requiredness has repository-specific semantics implemented in `openapi.rs`: PATCH request schemas are all-optional; nullable fields are always `Option<T>`; ordinary schemas with `required[]` use it; schemas without it use the `"Optional"` description convention. A schema whose `required[]` is known to be partial uses the union of that array and the description heuristic and must be listed in `partial_required_schemas`. `scripts/resolve-field-requirements.py` is a code-generation aid, not a comparison implementation or policy source.

##### Analyzer configuration and exemptions

All analyzer policy lives in `crates/clickhouse-openapi-analyzer/src/config.rs`; edit `clickhouse_cloud_config()` or its backing constants. Introduce a named, documented constant when an empty policy list first gains entries. Keys use Rust type names but spec/wire field and enum values:

- `non_openapi_client_methods`: intentional `Client` helpers with no operation, keyed by snake-case method name.
- `optionality_exemptions`: fields deliberately kept optional despite the resolved spec, keyed by `(RustStructName, specFieldName)`. Request-position-only: optionality findings are suppressed in response position, so a response-only entry can never hit and surfaces as stale.
- `extra_field_exemptions`: deliberate code-only fields, keyed by `(RustStructName, specFieldName)`.
- `deprecated_field_exemptions`: spec-deprecated fields deliberately excluded from the hiding mechanism, keyed by `(RustStructName, specFieldName)`.
- `extra_enum_value_exemptions`: intentional Rust-only wire values, keyed by `(RustEnumName, wireValue)`.
- `partial_required_schemas`: upstream schemas whose `required[]` is non-exhaustive, keyed by spec schema name. This changes requiredness resolution — request-position-only semantics — and is not a shortcut for one optionality mismatch.
- `acknowledged_unsupported_enum_pointers`: exact RFC 6901 pointers the analyzer inventories but cannot map to a concrete Rust value enum.

Add an exemption only for intentional, verified runtime behavior, with a nearby comment stating why the spec cannot be followed. Never exempt missing API surface or ordinary model drift. Pair a new unsupported-enum acknowledgement with a tracking issue to make the Rust type checkable; do not acknowledge it merely to make CI green. Pair-keyed field/enum exemptions and unsupported acknowledgements produce actionable stale findings when no longer needed, so remove them during normal remediation.

##### Enum value coverage

Enum values and struct fields are checked bidirectionally. Enum mapping is structural: named schemas resolve to model types; properties, array items, compositions, and operation parameters resolve through their Rust field/argument type. Serde container/variant renames determine wire values. Catch-alls are recognized through `untagged`/`other` attributes, never variant names; a genuine unit variant named `Unknown` remains a value. Numeric, mixed, and scalar-backed enum constraints are reported explicitly as unsupported rather than silently skipped.

##### `VALUES` const checking

Enums that the CLI validates against declare `pub const VALUES: &'static [&'static str]` in an `impl` block — a hand-written literal slice of the enum's non-catch-all wire values. The analyzer verifies that any enum with a `VALUES` const has it exactly equal (as a set) to its variant wire values; a mismatch produces `FindingKind::EnumValuesMismatch`. This is opt-in: enums without a `VALUES` const are not checked. When adding a new enum value to a `VALUES`-bearing enum, update both the variant and the const or CI will fail.

##### Deprecated field hiding

Every spec-deprecated request or response field belongs in `meta.rs::DEPRECATED_FIELDS` and carries `#[cfg(feature = "deprecated-fields")]` on the field in its model domain file. It is therefore absent from the public model by default. Request fields that must be gated out but resolve as required are `Option<T>` with a documented optionality exemption. Entries are keyed per Rust type, so a deprecated field on a split schema needs one entry and one marker for `{Name}` and one for `{Name}Response`. Update CLI code that directly accesses or constructs an affected model so both feature configurations compile.

##### Extending the analyzer

Add a typed `FindingKind` and pure comparison in the analyzer, focused inventory/comparison fixtures, deterministic JSON/text coverage, and Python issue rendering. Keep `spec_coverage_test.rs` as a thin consumer. New report fields or semantics require a report `schema_version` change; never make Python infer drift by reparsing Rust or OpenAPI.

## Tests

Test coverage is non-negotiable.

CI enforces clippy, ensure you fix all warnings.

CI enforces rustfmt (`cargo fmt --all --check`); run `cargo fmt` before committing. Bulk formatting commits are listed in `.git-blame-ignore-revs`.

Use cargo build, cargo test, cargo clippy, locally.

### clickhouse-cloud-api library

Real cloud integration tests, 100% OpenAPI spec coverage. Cost is not a reason to skip a test.

- `tests/common/support.rs` — generic test infra (polling, logging, env helpers, ClickHouse provisioning & cleanup, HTTP query helper). Used by every integration binary. Call `Client` directly from Rust.
- `tests/integration_test.rs`, `tests/integration_postgres_test.rs`, `tests/integration_org_test.rs` — cloud-service, Postgres-service, and organization lifecycle tests.
- `tests/clickpipes/` — ClickPipes E2E suite, including external cloud services. Only Postgres CDC (uses ClickHouse & Postgres inside ClickHouse Cloud) is run in CI. Its `clickpipe_postgres_cli_cdc_test` target exercises the built `clickhousectl` binary supplied through `CLICKHOUSE_CLOUD_TEST_CLICKHOUSECTL_BIN`; `.github/workflows/cloud-integration.yml` builds the binary before running it. Tests for third party services must be executed manually. CI also optionally runs `clickpipe_smoke_test` against a long-lived service when the `CLICKHOUSE_CLOUD_TEST_CLICKPIPE_SERVICE_ID` repo variable is set; the step is skipped when the variable is unset.
- `spec_coverage_test.rs`: runs the shared analyzer against the vendored OpenAPI snapshot and requires an actionable-drift-free report.
- Internal PRs classify the exact base-to-head diff with `scripts/classify-cloud-integration.py` on every push via a secret-free planner job; the `Cloud integration decision` check goes green automatically when no suites are affected. Affected `service`, `postgres`, `organization`, and `clickpipes` suites only run after the `run-cloud-integration` label is applied (one-shot, bound to the labeled head SHA). New or renamed API source/test files must be added to the classifier's explicit mappings; unknown paths fail closed to all suites. Scheduled runs still select all suites, while manual runs use the requested scope.

### clickhousectl CLI

- **Clap parsing** — `Cli::try_parse_from` tests next to each command definition (`src/cli.rs`, the owning `src/cloud/<domain>.rs`, and `src/local/cli.rs`). Assert flag names, types, defaults, and repeatability.
- **Request builders** — unit tests for `build_*_request` helpers next to the owning cloud domain code, asserting on library request-struct fields with minimal + maximal inputs.
- **Subprocess + wiremock** — `tests/cli_request_shape_test.rs`. Spawn the real binary against a local mock server and assert on requests, auth, errors, and output across Cloud domains. Use it when handler runtime behavior is not covered by clap or request-builder tests.
- **Pure logic** — inline `mod tests` blocks across `src/` for version resolution, auth precedence, output formatting, platform detection, and other module-local helpers.

## Dependencies

Use `cargo add` to add new dependencies. Use the latest version of packages. Specify the crate with `-p`, e.g. `cargo add -p clickhouse-cloud-api url`.

## Releases

- Releases are triggered by pushing a version tag (e.g. `git tag v0.2.3 && git push origin v0.2.3`), which runs the GitHub Actions workflow
- Bump all of these to the same version in lockstep: `crates/clickhousectl/Cargo.toml` (`version` and the `clickhouse-cloud-api` dep version), `crates/clickhouse-cloud-api/Cargo.toml`, and `npm/package.json`. The workflow also re-aligns `npm/package.json` to the tag at publish time, but bump it in the repo too so the source-of-truth matches. `pypi/pyproject.toml` does *not* need a manual bump — maturin pulls the wheel version from `crates/clickhousectl/Cargo.toml` (via `dynamic = ["version"]`), and the `build-wheels` job also re-aligns the Cargo version to the tag at publish time.
- For `clickhouse-cloud-api`, the crate is published to crates.io.
- For `clickhousectl`, releases are published to GitHub releases, crates.io, npm, and PyPI. The npm and PyPI packages are thin wrappers to make it easier for LLMs to find and install. crates.io uses a token, while npm & PyPI use OIDC. All of these releases are triggered by the same release workflow, in separate jobs.

## Git workflow

- Branch per feature/issue & use PR workflow.
- PRs should have an associated issue.

## GitHub Actions

Must pin deps in GH Actions to SHA hashes, not tags.
Secrets used by GH Actions must be protected from exfiltration, e.g., do not populate secrets in Actions triggered by external PRs.

## Documentation

- PRs should include doc updates to `README.md` for functionality/behaviour that needs to be understood by users/developers.
- CLAUDE.md should be kept up to date if there is material change to development practices.
