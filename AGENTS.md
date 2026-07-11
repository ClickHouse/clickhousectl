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

- `.github/workflows/openapi-drift.yml` runs `scripts/check-openapi-drift.py` daily at 08:00 UTC (also triggerable via `workflow_dispatch`). The script fetches the live spec, invokes the private `clickhouse-openapi-analyzer` workspace crate, and opens a GitHub issue with the `openapi-drift` label when its typed report contains actionable findings.
- `crates/clickhouse-openapi-analyzer` is the single implementation of Rust-source parsing and code-vs-spec comparison semantics. It parses `client.rs`, `models.rs`, and `meta.rs` with `syn`, returns a versioned serializable `DriftReport`, and is a dev-only dependency of `clickhouse-cloud-api`; it must never become a normal dependency of either published crate.
- `spec_coverage_test.rs` and the daily Python workflow are thin consumers of the same report. The former analyzes the vendored snapshot in CI; the latter analyzes the live spec while also comparing it with the snapshot. Python owns fetching, issue rendering, and GitHub orchestration only.
- `--dry-run` prints the report without opening an issue.
- When resolving drift: work from the auto-opened issue. Fix `clickhouse-cloud-api` first in its own PR: update `client.rs`, `models.rs`, and refresh the vendored snapshot. Then decide separately whether to expose the new surface in the CLI.

##### Field optionality and the OpenAPI spec

The OpenAPI spec uses two different conventions for required vs optional fields:

- **Schemas with a `required` array** (newer/beta endpoints) — standard OpenAPI semantics.
- **Schemas without `required`** (GA/legacy endpoints) — optional fields start their description with `"Optional"`. All other fields are implicitly required. The `"Optional"` marker may be preceded by status prefixes like `"Private preview."` (e.g. `"Private preview. Optional ..."`), so the heuristic should strip known prefixes before checking, not anchor strictly to the first character.
- **Mixed schemas (legacy endpoints that have started adding a `required` array)** — the array only covers newly-added fields, so the presence of `required` does not mean it is exhaustive. Treat fields listed in `required` as required, then run the `"Optional"`-description heuristic over the remaining fields (pre-existing required fields will not be in the array, but still aren't marked `"Optional"`).
- **PATCH request schemas** — always all-optional (partial update semantics), identified by name containing "Patch" and ending with "Request".
- **Nullable fields** (`type: ["string", "null"]` or `oneOf` with null) — always `Option<T>` in Rust, even if "required".

In `models.rs`, required non-nullable fields use bare types (`T`), optional/nullable fields use `Option<T>`. All fields keep `#[serde(default)]` for robust deserialization.

**Tooling:**

- `scripts/resolve-field-requirements.py` — resolves required/optional for every schema field, outputs a JSON manifest. Handles both conventions + PATCH + nullable.
- `crates/clickhouse-openapi-analyzer` — canonical Rust/OpenAPI inventories, mapping, comparison, exemptions, unsupported-enum acknowledgements, and typed report.
- `scripts/check-openapi-drift.py` — fetches the live spec, invokes the analyzer executable, renders its JSON report, and creates the issue.
- `spec_coverage_test.rs::vendored_openapi_snapshot_matches_rust_api` — asserts the shared report has no actionable findings for the snapshot.

When adding a drift check, add its typed finding and pure comparison logic to the analyzer, cover it with a focused fixture, include it in the text/JSON report, and teach the Python renderer how to present it. Do not add comparison logic to `spec_coverage_test.rs` or Python.

Field coverage is **bidirectional**, mirroring the missing/extra split used for client methods:

- `struct_fields_cover_every_spec_property` (spec → code) — every spec property has a matching struct field; catches fields *added* to the spec.
- `struct_fields_have_no_extras_vs_spec` (code → spec) — every struct field maps to a spec property; catches fields *removed* from the spec but left behind in `models.rs` (a superset model would otherwise pass every other field check). Schemas with no/empty `properties` are skipped, so composition/marker schemas don't flag every field. The drift script's "Extra Struct Fields" section reports the same finding.

Field optionality is maintained by hand. When the drift check or test flags a mismatch, edit `models.rs` directly to flip the field (`T` ↔ `Option<T>`) and adjust the `#[serde(skip_serializing_if = "Option::is_none")]` attribute to match.

**Optionality exemptions:**

Sometimes the spec marks a field as required but the API rejects empty/default values, meaning the field is effectively optional. These fields are kept as `Option<T>` in `models.rs` and listed in the analyzer's ClickHouse configuration (`src/config.rs`). The analyzer emits a stale-exemption finding when the spec and model start agreeing. Add a `("RustStructName", "specFieldName")` entry with a comment explaining the API behavior.

**Extra-field exemptions:**

A struct field that intentionally has no spec property (a code-only/computed field, or a standard attribute the upstream spec omits) goes in `extra_field_exemptions` in the analyzer's ClickHouse configuration, analogous to its optionality and non-OpenAPI-method exemptions. The analyzer reports stale entries. The list is empty by default — only add an entry for a *deliberate* addition, not to silence a field that should be removed.

##### Enum value coverage

Enum **values** are checked bidirectionally too, mirroring the field checks:

- `enum_values_cover_every_spec_enum` (spec → code) — every value in a spec `enum` array has a matching Rust variant; catches values *added* to the spec (responses would silently fall into the untagged catch-all, and requests couldn't express the value).
- `enum_values_have_no_extras_vs_spec` (code → spec) — every Rust variant serializes a value the spec enum lists; catches values *removed* from the spec but left behind in `models.rs`, which the API rejects on requests. The drift script's "Extra Enum Values" section reports the same finding.

The mapping from spec enum to Rust enum is **structural**, not name- or comment-based: named schemas resolve to model types; property, nested `items`, composition, and operation-parameter enums resolve through the actual Rust field/argument type. Comparison uses Serde container/variant renames and excludes catch-alls through `untagged`/`other` attributes, never names. Numeric, mixed, or scalar-backed enum constraints are serialized as explicit unsupported diagnostics. The current snapshot's known unsupported pointers are acknowledged in analyzer configuration; new locations are actionable and vanished acknowledgements are stale. When a value is flagged, add/remove the variant *and its `Display` arm* in `models.rs`.

An enum variant that intentionally diverges from the spec goes in `extra_enum_value_exemptions` in analyzer configuration as a `("RustEnumName", "wireValue")` entry with a comment. It behaves like the field exemptions and produces a stale finding when it is no longer needed.

##### Deprecated field hiding

Fields the spec marks `deprecated: true` — on both response schemas (e.g. `Service.tier`, `ApiKey.roles`) and request schemas (e.g. `ServicePostRequest.tier`, `InvitationPostRequest.role`) — are removed from the struct entirely so consumers, including the CLI, can't even reference a field the API has deprecated. Each carries `#[cfg(feature = "deprecated-fields")]` in `models.rs`: absent from the struct by default, present only when the `deprecated-fields` Cargo feature is on. On a **response** struct that means reading it is a compile error and it never appears in output (deserializing a payload that still contains it just ignores the extra key — no schema uses `deny_unknown_fields`). On a **request** struct it means callers can't set it and `skip_serializing_if` keeps it off the wire entirely.

Because the field is gone by default, table/list output built by direct field access (e.g. `member list`, `invitation list`) can no longer leak a deprecated field — the compiler rejects it. Where a deprecated field had a non-deprecated replacement (e.g. `Member.role` → `assignedRoles`), the list column was switched to the replacement. CLI request builders (`commands.rs`, `service_query.rs`) likewise drop the deprecated fields; where they still construct a struct under the feature, the inert assignment (`field: None`) carries its own `#[cfg(feature = "deprecated-fields")]` so both feature configs compile.

A deprecated request field that the spec marks required (description heuristic, e.g. `InvitationPostRequest.role`, `OrganizationPrivateEndpointsPatch.add`) is modelled as `Option<T>` so it can be gated out and omitted — these carry an optionality exemption in analyzer configuration.

The list is the `DEPRECATED_FIELDS` constant in `src/meta.rs`. `scripts/regenerate-deprecated-fields.py` regenerates it from the snapshot; the analyzer compares the spec, constant, and `models.rs` markers in one report consumed by both snapshot tests and the daily workflow.

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
