# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repo structure

This is a Cargo workspace with two crates:

- **`crates/clickhousectl/`** — the CLI binary (version manager + cloud CLI)
- **`crates/clickhouse-cloud-api/`** — typed Rust client library for the ClickHouse Cloud API (used by the CLI for all cloud commands)

## Build & Test

```bash
cargo build                          # dev build (whole workspace)
cargo build --release                # release build
cargo test                           # run all tests (both crates)
cargo test -p clickhousectl          # test CLI only
cargo test -p clickhouse-cloud-api   # test library only
cargo test test_detect_platform      # run a single test
cargo clippy                         # lint
```

No separate lint CI — just `cargo build` and `cargo test` must pass.

Cross-compilation for aarch64-linux uses `cross` (see `.github/workflows/release.yml`). The CLI crate uses `rustls-tls` instead of OpenSSL to support this.

## Architecture

### CLI (`crates/clickhousectl/`)

`clickhousectl` is the official ClickHouse CLI — a version manager + cloud CLI. Two top-level subcommands: `local` and `cloud`.

1. **Local** (`local install|list|use|remove|which|init|client|server`) — version management in `src/version_manager/`, server management in `src/server.rs`, client/init in `main.rs`. Binaries live in `~/.clickhouse/versions/{version}/clickhouse`, default tracked in `~/.clickhouse/default`. Project data lives in `.clickhouse/`.

2. **Cloud** (`cloud org|service|backup`) — handled by `src/cloud/`. `CloudClient` wraps reqwest with Basic or Bearer auth. Commands go through `cloud/commands.rs`, types in `cloud/types.rs`. All cloud commands support `--json` output.

3. **Auth** (`cloud auth login|logout|status`) — authentication subcommand under cloud. `login` defaults to OAuth device flow (`src/cloud/auth.rs`), supports `--interactive` for API key prompt, or `--api-key`/`--api-secret` for non-interactive. `logout` clears all credentials. Tokens stored in `.clickhouse/tokens.json`, API keys in `.clickhouse/credentials.json` (both project-local).

### API library (`crates/clickhouse-cloud-api/`)

Typed Rust client generated from the ClickHouse Cloud OpenAPI spec. Contains:

- `src/client.rs` — `Client` struct with async methods for every API endpoint
- `src/models.rs` — request/response types generated from the OpenAPI spec
- `src/error.rs` — error types (Http, Json, Api)
- `tests/spec_coverage_test.rs` — validates the client and models cover the OpenAPI spec

The CLI depends on this library for all cloud API calls.

#### Field optionality and the OpenAPI spec

The OpenAPI spec uses two different conventions for required vs optional fields:

- **Schemas with a `required` array** (newer/beta endpoints) — standard OpenAPI semantics.
- **Schemas without `required`** (GA/legacy endpoints) — optional fields start their description with `"Optional"`. All other fields are implicitly required.
- **PATCH request schemas** — always all-optional (partial update semantics), identified by name containing "Patch" and ending with "Request".
- **Nullable fields** (`type: ["string", "null"]` or `oneOf` with null) — always `Option<T>` in Rust, even if "required".

In `models.rs`, required non-nullable fields use bare types (`T`), optional/nullable fields use `Option<T>`. All fields keep `#[serde(default)]` for robust deserialization.

**Scripts:**

- `scripts/resolve-field-requirements.py` — resolves required/optional for every schema field, outputs a JSON manifest. Handles both conventions + PATCH + nullable.
- `scripts/update-models-optionality.py` — reads the spec and rewrites `models.rs` field types to match. Only converts `Option<T>` → `T`; does not convert in the reverse direction.

**Validation:**

- `spec_coverage_test.rs::field_optionality_matches_spec` — asserts every field's `Option<T>` vs `T` matches the spec.
- `scripts/check-openapi-drift.py` — daily CI drift check now also reports field-level optionality mismatches.

**Optionality exemptions:**

Sometimes the spec marks a field as required but the API rejects empty/default values, meaning the field is effectively optional. These fields are overridden to `Option<T>` in `models.rs` and listed in the `OPTIONALITY_EXEMPTIONS` constant in `spec_coverage_test.rs`. The test logs each exemption and fails if any become stale (spec was fixed upstream). When adding a new exemption, add a `("RustStructName", "specFieldName")` entry with a comment explaining the API behavior.

**When the spec adds proper `required` arrays to all schemas:**

1. Download the updated spec: `curl -s https://api.clickhouse.cloud/v1 -o crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json`
2. Re-run: `python3 scripts/update-models-optionality.py`
3. Fix any test assertions for fields that changed optionality.
4. Verify: `cargo test -p clickhouse-cloud-api`

The resolution logic automatically prefers `required` arrays over description parsing, so the description heuristic becomes dead code — no structural changes needed.

## Adding commands

### New local subcommand

1. Add variant to `LocalCommands` in `crates/clickhousectl/src/cli.rs` using clap derive macros
2. Add match arm in `run_local()` in `crates/clickhousectl/src/main.rs`
3. Implement handler (in `main.rs` for simple commands, or a dedicated module)

### New cloud subcommand

1. Add variant to the relevant enum in `crates/clickhousectl/src/cli.rs` (e.g. `ServiceCommands`)
2. Add match arm in `run_cloud()` in `crates/clickhousectl/src/main.rs`
3. Add method to `CloudClient` in `crates/clickhousectl/src/cloud/client.rs`
4. Add request/response types to `crates/clickhousectl/src/cloud/types.rs` — use `#[serde(rename_all = "camelCase")]` (API uses camelCase) and `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
5. Implement handler in `crates/clickhousectl/src/cloud/commands.rs` with the `--json` output pattern:
   ```rust
   if json {
       println!("{}", serde_json::to_string_pretty(&data)?);
   } else {
       println!("Human readable: {}", data.field);
   }
   ```

ClickHouse Cloud OpenAPI spec: https://api.clickhouse.cloud/v1

## Dependencies

Use `cargo add` to add new dependencies (not manual `Cargo.toml` edits). Always use the latest version of packages. Specify the crate with `-p`:

```bash
cargo add -p clickhousectl serde --features derive
cargo add -p clickhouse-cloud-api url
```

## Key details

- CLI is defined with clap derive macros in `crates/clickhousectl/src/cli.rs`, dispatched in `crates/clickhousectl/src/main.rs`
- `src/paths.rs` handles `~/.clickhouse/` paths (global install dir); `src/init.rs` handles `.clickhouse/` paths (project-local data dir)
- `local client` uses `exec()` (process replacement), so code after `cmd.exec()` only runs on failure
- Error types use `thiserror` in `src/error.rs`; cloud module has its own error type wrapped as `Error::Cloud(String)`
- Version resolution (`version_manager/resolve.rs`) handles specs like `stable`, `lts`, `25.12`, or exact `25.12.5.44` — all resolve to an exact version + channel via GitHub API
- Releases are triggered by pushing a version tag (`v0.1.3`), which runs the GitHub Actions workflow

## Git workflow

- **Branch per feature/issue.** When working on a new feature or issue, create a branch and use a PR workflow. Do not commit directly to `main`.
- If the user references a GitHub issue (e.g. "work on issue 3"), use `gh issue view 3` to get the details, then create a branch like `issue-3-short-description`.
- Update `README.md` and any relevant documentation as part of the change — PRs should include doc updates for new or changed functionality.
- Commit to the branch, push, and create a PR with `gh pr create`.
- Releases are done by tagging `main` (e.g. `git tag v0.1.4 && git push origin v0.1.4`), which triggers the GitHub Actions release workflow. Ensure version is updated in `crates/clickhousectl/Cargo.toml`.

## Testing locally

```bash
cargo run -p clickhousectl -- local install stable
cargo run -p clickhousectl -- local server start      # starts server in .clickhouse/servers/default/
cargo run -p clickhousectl -- local client --query "SELECT 1"
```

## Testing model

The repo follows a strict split between **API-library** testing and **CLI** testing. Both crates carry their own weight — neither leans on the other to cover what it should cover itself.

### Cloud API library (`crates/clickhouse-cloud-api/`) — real integration tests

- **Target: 100% coverage of the OpenAPI spec via real cloud integration tests.** Every path/method in the spec should have at least one integration test that hits the real ClickHouse Cloud API. Coverage gaps are bugs in the test suite.
- **Cost is not a valid reason to skip an integration test.** Spinning up real services, instances, or backups for a test is acceptable. If a path is expensive to exercise, the answer is to make the test efficient (reuse fixtures, parallelize, tear down promptly) — not to mock it out or skip it.
- **All cloud integration testing happens against the HTTP client library, invoked directly from Rust.** Tests call `Client` methods (`crates/clickhouse-cloud-api/src/client.rs`) directly — they do not shell out to the `clickhousectl` binary. The library is the unit under test; the CLI is irrelevant to library coverage.
- Integration test files live in `crates/clickhouse-cloud-api/tests/integration_*.rs`, with shared fixtures/stages in `crates/clickhouse-cloud-api/tests/integration/` (driver, support, per-source stages like `postgres.rs`, `kafka.rs`, etc.).
- `spec_coverage_test.rs` enforces that every spec operation has a corresponding client method and every schema field has a matching model field — it is the structural floor; integration tests are the behavioural ceiling.

### CLI (`crates/clickhousectl/`) — request-shape contract tests

- **The CLI's job is to translate user input into correct request bodies for the library.** That translation is a contract between the CLI and the library's request models, and it must be tested extensively.
- **CLI tests assert on the shape of the request the CLI produces**, not on cloud behaviour. Use `wiremock` to stand up a local mock API, point the CLI at it, and assert on the captured request body's JSON shape. See `crates/clickhousectl/tests/cli_request_shape_test.rs` for the pattern.
- This guards against handler-level regressions (e.g. `args.foo.clone().unwrap_or_default()` serializing `""` for an optional field that the API rejects when empty). These bugs are invisible to library integration tests because the library is never called with the broken shape — only the CLI produces it.
- **Every new CLI subcommand that builds a request body needs a request-shape test.** Cover at least: the minimum-required-args shape, the all-optional-flags-set shape, and any flag whose optionality affects serialization (`Option<T>` vs `T`, `skip_serializing_if`, enums, nested structs).
- CLI tests do not need cloud credentials and run in milliseconds — there is no cost excuse for missing coverage here either.

### Summary

| Layer | What it tests | Against what | Why |
|---|---|---|---|
| `clickhouse-cloud-api` integration tests | Library behaviour end-to-end | Real ClickHouse Cloud API | Catch API drift, real-world failures, schema mismatches |
| `clickhousectl` request-shape tests | CLI → library request bodies | `wiremock` mock server | Catch handler regressions that produce wrong JSON shapes |
| `spec_coverage_test.rs` | Client/model surface vs spec | OpenAPI spec JSON | Structural floor — every endpoint and field exists |
