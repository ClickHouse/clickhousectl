# CLAUDE.md

clickhousectl (or chctl) is the official CLI for ClickHouse, by ClickHouse Inc. clickhousectl supports both ClickHouse and Postgres, on your local machine or in ClickHouse Cloud.

## Architecture

This is a Cargo workspace with two crates:

### CLI (`crates/clickhousectl/`)

The user-facing CLI surface. Contains all logic for local commands, wraps `clickhouse-cloud-api` for cloud.

- Cloud handlers go through the `CloudClient` wrapper (`src/cloud/client.rs`), not `clickhouse_cloud_api::Client` directly. The wrapper handles credential precedence, error conversion, and response unwrapping.
- Cloud handlers always support `--json` output unless there is good reason not to. JSON is emitted automatically when `--json` is passed or a coding agent is detected (`is_ai_agent::detect()` via the `json_output()` helper in `main.rs`).
- `CloudError` carries a `kind: CloudErrorKind` (`Auth` for 401/403 and missing credentials, else `Generic`). It maps to `Error::AuthRequired` / `Error::Cloud` in `main.rs`, driving `gh`-style exit codes via `Error::exit_code()`: `0` success, `1` error, `2` cancelled, `4` auth required.

Use `--help` to learn the current command surface.

Project-local data lives in `.clickhouse/`. Globally installed ClickHouse binaries live in `~/.clickhouse/`. OAuth tokens (`~/.clickhouse/tokens.json`) are the exception — they're global user identity, not project-scoped.

The CLI does not need to have 100% coverage of endpoints exposed by the API library: be intentional about what is exposed to users.

#### Adding a command

For both local and cloud commands, define the clap variant in the appropriate `cli.rs`, then wire dispatch in `src/main.rs`.

**Local subcommand:**

1. Add a variant to the relevant enum in `src/local/cli.rs` using clap derive macros.
2. Add the match arm in `run_local()` in `src/main.rs`.
3. Implement the handler in a dedicated module under `src/local/` (e.g. `src/local/server.rs`, `src/local/postgres.rs`). Don't pile new logic into `main.rs`.

**Cloud subcommand:**

1. Make sure `clickhouse-cloud-api` has already been updated to support necessary endpoints & models.
2. Add the variant to the relevant sub-enum in `src/cloud/cli.rs` (or `src/cloud/postgres.rs` for Postgres). Create a new sub-enum if the surface warrants its own grouping.
3. Classify the new variant in `CloudCommands::is_write_command()` in `src/cloud/cli.rs` (Postgres variants go in the equivalent `is_write()` on the Postgres enum). OAuth (Bearer) auth is read-only; write commands require API key auth and we fail fast on OAuth + write. The match has no wildcards, so the compiler will reject a missing arm — but you still need to make the read/write call deliberately, and add a case to both the `is_write_command_read_only_commands` and `is_write_command_destructive_commands` tests.
4. Add the match arm in `run_cloud()` in `src/main.rs`.
5. Add a thin wrapper method on `CloudClient` in `src/cloud/client.rs`. It should delegate to `self.api().<lib_method>()`, map errors via `self.convert_error(e)`, and unwrap with `Self::unwrap_response`. Use the library's request/response types here.
6. If the command sends a request body, extract a `build_<name>_request(...)` helper in `src/cloud/commands.rs` that returns the library's request struct. Cover the helper with minimal + maximal unit tests in the `mod tests` block at the bottom of `commands.rs`, asserting directly on library struct fields.
7. Implement the handler in `src/cloud/commands.rs`. For body-sending commands the handler calls the build helper, passes the result through the `CloudClient` wrapper, and prints with the `--json` output pattern. For detail/get views (rendering a single resource), drive human output through `print_human` so it shares serde's behaviour — including deprecated-field hiding — instead of hand-writing `println!` lines:
   ```rust
   if json {
       println!("{}", serde_json::to_string_pretty(&data)?);
   } else {
       print_human(&data)?;
   }
   ```
   List views stay as `tabled` tables, and short action confirmations (e.g. "Service X starting") stay as plain `println!`.
8. Add `Cli::try_parse_from` coverage in `src/cloud/cli.rs` for the new command's body-related flags, asserting parsed values.

### API library (`crates/clickhouse-cloud-api/`)

Typed Rust client library for the ClickHouse Cloud API. The library owns all OpenAPI interaction and all cloud integration testing.

- `src/client.rs` — `Client` struct with one async method per OpenAPI operation.
- `src/models.rs` — request/response types matching the spec (see Field optionality below).

The API library can be updated independently of the CLI. When OpenAPI drifts, prefer updating API library on its own, add to CLI separately.

#### OpenAPI drift

ClickHouse Cloud OpenAPI spec: https://api.clickhouse.cloud/v1

- `.github/workflows/openapi-drift.yml` runs `scripts/check-openapi-drift.py` daily. Python owns fetching, issue rendering, and GitHub orchestration only; `python3 scripts/check-openapi-drift.py --dry-run` reproduces the rendered issue without creating one.
- `crates/clickhouse-openapi-analyzer` is the single implementation of parsing and comparison. `rust_inventory.rs` parses `client.rs`, `models.rs`, and `meta.rs` with `syn`; `openapi.rs` inventories the target spec and vendored snapshot; `compare.rs` maps them and emits typed findings; `config.rs` owns ClickHouse-specific policy; `report.rs` defines the stable JSON/text report; `main.rs` is the executable used by Python. Do not duplicate source parsing, exemptions, or comparison logic in tests or Python.
- The analyzer is private (`publish = false`) and a dev dependency of `clickhouse-cloud-api`. Parser/tooling dependencies such as `syn` must not enter either published crate's normal dependency graph.
- `crates/clickhouse-cloud-api/tests/spec_coverage_test.rs` analyzes the vendored snapshot; its ignored test analyzes the live spec. Both and the scheduled workflow call the same analyzer and must agree.

##### Remediating a drift issue

Work from the issue's typed findings. `spec_pointer` is an RFC 6901 location in the target spec and `rust_item` is the intended Rust location. The analyzer executable exits successfully after producing a valid report even when `findings` is non-empty; use `has_drift`/`actionable_count`, not its process status, to decide whether drift exists.

1. Reproduce with `python3 scripts/check-openapi-drift.py --dry-run`. The command does not update the snapshot.
2. Replace `crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json` with the same live document being remediated; do not hand-edit the spec. Snapshot operation/schema findings mean this file is stale.
3. Fix the API library before considering CLI exposure. Follow the finding's pointer and Rust item:
   - Missing/extra operations: add or remove the corresponding `Client` method in `client.rs`; only intentional non-OpenAPI helpers belong in `non_openapi_client_methods`.
   - Missing models, referenced models, fields, or extra fields: update public structs/enums/type aliases and Serde names in `models.rs`. An undefined `$ref` (`missing_schema_definition`) is an upstream-spec defect, not a model to invent locally.
   - Optionality: use `T` for required non-nullable fields and `Option<T>` plus `skip_serializing_if` for optional/nullable fields. Retain `#[serde(default)]` on model fields.
   - Missing/extra enum values: update the typed enum, its Serde wire value, and its `Display` implementation. Preserve data-carrying catch-all variants.
   - Beta/deprecation findings: regenerate `BETA_OPERATIONS` with `python3 scripts/regenerate-beta-lists.py` and `DEPRECATED_FIELDS` with `python3 scripts/regenerate-deprecated-fields.py`; deprecated fields also need the matching `#[cfg(feature = "deprecated-fields")]` marker in `models.rs`.
   - Stale exemption: remove or narrow the configuration entry. Do not change comparison logic to preserve a stale exception.
   - Unsupported enum constraint: prefer changing the Rust scalar to a concrete value enum. Acknowledgement is the fallback policy below, not a model-drift fix.
4. Add focused library tests for changed models/methods. If the unsupported inventory changes, update the snapshot test's expected inventory as well.
5. Verify with `cargo test -p clickhouse-cloud-api -p clickhouse-openapi-analyzer`, `cargo clippy -p clickhouse-cloud-api -p clickhouse-openapi-analyzer --all-targets -- -D warnings`, and `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`. If deprecated fields changed, also run `cargo check --workspace --all-features`. Re-run the dry run to check the live document.

##### Field optionality and the OpenAPI spec

Requiredness has repository-specific semantics implemented in `openapi.rs`: PATCH request schemas are all-optional; nullable fields are always `Option<T>`; ordinary schemas with `required[]` use it; schemas without it use the `"Optional"` description convention. A schema whose `required[]` is known to be partial uses the union of that array and the description heuristic and must be listed in `partial_required_schemas`. `scripts/resolve-field-requirements.py` is a code-generation aid, not a comparison implementation or policy source.

##### Analyzer configuration and exemptions

All analyzer policy lives in `crates/clickhouse-openapi-analyzer/src/config.rs`; edit `clickhouse_cloud_config()` or its backing constants. Introduce a named, documented constant when an empty policy list first gains entries. Keys use Rust type names but spec/wire field and enum values:

- `non_openapi_client_methods`: intentional `Client` helpers with no operation, keyed by snake-case method name.
- `optionality_exemptions`: fields deliberately kept optional despite the resolved spec, keyed by `(RustStructName, specFieldName)`.
- `extra_field_exemptions`: deliberate code-only fields, keyed by `(RustStructName, specFieldName)`.
- `deprecated_field_exemptions`: spec-deprecated fields deliberately excluded from the hiding mechanism, keyed by `(RustStructName, specFieldName)`.
- `extra_enum_value_exemptions`: intentional Rust-only wire values, keyed by `(RustEnumName, wireValue)`.
- `partial_required_schemas`: upstream schemas whose `required[]` is non-exhaustive, keyed by spec schema name. This changes requiredness resolution; it is not a shortcut for one optionality mismatch.
- `acknowledged_unsupported_enum_pointers`: exact RFC 6901 pointers the analyzer inventories but cannot map to a concrete Rust value enum.

Add an exemption only for intentional, verified runtime behavior, with a nearby comment stating why the spec cannot be followed. Never exempt missing API surface or ordinary model drift. Pair a new unsupported-enum acknowledgement with a tracking issue to make the Rust type checkable; do not acknowledge it merely to make CI green. Pair-keyed field/enum exemptions and unsupported acknowledgements produce actionable stale findings when no longer needed, so remove them during normal remediation.

##### Enum value coverage

Enum values and struct fields are checked bidirectionally. Enum mapping is structural: named schemas resolve to model types; properties, array items, compositions, and operation parameters resolve through their Rust field/argument type. Serde container/variant renames determine wire values. Catch-alls are recognized through `untagged`/`other` attributes, never variant names; a genuine unit variant named `Unknown` remains a value. Numeric, mixed, and scalar-backed enum constraints are reported explicitly as unsupported rather than silently skipped.

##### Deprecated field hiding

Every spec-deprecated request or response field belongs in `meta.rs::DEPRECATED_FIELDS` and carries `#[cfg(feature = "deprecated-fields")]` on the `models.rs` field. It is therefore absent from the public model by default. Request fields that must be gated out but resolve as required are `Option<T>` with a documented optionality exemption. Update CLI code that directly accesses or constructs an affected model so both feature configurations compile.

##### Extending the analyzer

Add a typed `FindingKind` and pure comparison in the analyzer, focused inventory/comparison fixtures, deterministic JSON/text coverage, and Python issue rendering. Keep `spec_coverage_test.rs` as a thin consumer. New report fields or semantics require a report `schema_version` change; never make Python infer drift by reparsing Rust or OpenAPI.

## Tests

Test coverage is non-negotiable.

CI enforces clippy, ensure you fix all warnings.

Use cargo build, cargo test, cargo clippy, locally.

### clickhouse-cloud-api library

Real cloud integration tests, 100% OpenAPI spec coverage. Cost is not a reason to skip a test.

- `tests/common/support.rs` — generic test infra (polling, logging, env helpers, ClickHouse provisioning & cleanup, HTTP query helper). Used by every integration binary. Call `Client` directly from Rust.
- `tests/integration_test.rs`, `tests/integration_postgres_test.rs` — cloud-service / Postgres-service CRUD lifecycle tests.
- `tests/clickpipes/` — ClickPipes E2E suite, including external cloud services. Only Postgres CDC (uses ClickHouse & Postgres inside ClickHouse Cloud) is run in CI. Tests for third party services must be executed manually. CI also optionally runs `clickpipe_smoke_test` against a long-lived service when the `CLICKHOUSE_CLOUD_TEST_CLICKPIPE_SERVICE_ID` repo variable is set (see `.github/workflows/cloud-integration.yml`); the step is skipped when the variable is unset.
- `spec_coverage_test.rs`: runs the shared analyzer against the vendored OpenAPI snapshot and requires an actionable-drift-free report.

### clickhousectl CLI

- **Clap parsing** — `Cli::try_parse_from` tests next to each command definition (`src/cli.rs`, `src/cloud/cli.rs`, `src/cloud/postgres.rs`, `src/local/cli.rs`). Assert flag names, types, defaults, and repeatability.
- **Request builders** — unit tests for `build_*_request` helpers in `src/cloud/commands.rs`, asserting on library request-struct fields with minimal + maximal inputs.
- **Subprocess + wiremock** — `tests/cli_request_shape_test.rs`. Spawn the real binary against a local mock server and assert on the recorded request JSON. Used when the handler has runtime behavior beyond struct construction (file reads, base64 encoding, etc.) — currently ClickPipes.
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
