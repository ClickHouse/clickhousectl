# AGENTS.md

`CLAUDE.md` is a symlink to this file. Edit `AGENTS.md`; never replace the symlink.

clickhousectl (`chctl`) is the CLI for ClickHouse and Postgres, local and in ClickHouse Cloud. Use `--help` to
learn the current command surface. `README.md` is the user-facing doc; do not duplicate it here.

## Commands

- `cargo fmt --all` before every commit (`fmt.yml` runs `cargo fmt --all --check`; bulk formatting commits are in `.git-blame-ignore-revs`).
- `cargo clippy -p clickhousectl -- -D warnings` && `cargo test -p clickhousectl`.
- Keep telemetry-compiled-out building and linting, as CI does: `cargo check -p clickhousectl --no-default-features`
  && `cargo clippy -p clickhousectl --all-targets --no-default-features -- -D warnings`.
- Library crates: `cargo clippy -p clickhouse-cloud-api -p clickhouse-openapi-analyzer --all-targets -- -D warnings`
  && `cargo test -p clickhouse-cloud-api -p clickhouse-openapi-analyzer`; `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`.
  If `deprecated-fields` changed, also `cargo check --workspace --all-features`.

**Done** means: `cargo fmt --all`; both clippy configurations clean; tests pass for every crate touched;
classifier mappings updated if a file was added or renamed; `README.md` updated for user-visible behaviour;
work on a branch, with an associated issue and a PR.

## Workspace

- `crates/clickhousectl/` — the CLI. All local logic; wraps `clickhouse-cloud-api` for cloud.
- `crates/clickhouse-cloud-api/` — typed Cloud API client, published to crates.io.
- `crates/clickhouse-openapi-analyzer/` — private OpenAPI/Rust drift tooling.
  Both library crates are governed by `crates/clickhouse-cloud-api/AGENTS.md`; read it before touching either.
- Update the API library on its own; add CLI exposure separately. The CLI need not cover 100% of the library's
  endpoints — be intentional.
- Project-local data lives in `.clickhouse/`; globally installed ClickHouse binaries in `~/.clickhouse/`. OAuth
  tokens (`~/.clickhouse/tokens.json`) are the exception — global user identity, not project-scoped.

## CLI invariants

- New Cloud handlers go through `CloudClient` wrapper methods co-located in each domain module, not
  `clickhouse_cloud_api::Client` directly. `src/cloud/client.rs` owns the core client, credential precedence,
  error conversion, and response unwrapping.
- Exception, do not copy: `src/cloud/postgres.rs` handlers and the query paths in `src/cloud/services.rs` still
  call `client.api()` directly (`postgres.rs` also uses a local `unwrap_api` instead of `unwrap_response`).
- Cloud handlers support `--json` unless there is good reason not to. JSON is emitted automatically when `--json`
  is passed or a coding agent is detected — `json_output()` in `main.rs` wraps `is_ai_agent::detect()`.
- `CloudError` carries `kind: CloudErrorKind` (`Auth` for 401/403 and missing credentials, else `Generic`) and an
  optional `details: CloudErrorDetail`. `cloud_error_to_top_level` (entered from `cloud::run`) maps `Auth` →
  `Error::AuthRequired`, `Generic` + details → `Error::CloudDetailed`, else `Error::Cloud`. A `CloudDetailed`
  replaces the prose only in JSON mode, where `main.rs` renders it via `cloud::output::print_error`.
- Exit codes: `0` success, else `Error::exit_code()` — `1` error, `3` cancelled, `4` auth required, and
  `ChildExit(code)` passes a spawned child's status through. Clap uses `2` for usage errors.

### Telemetry failure classification (#450)

- `CloudError` also carries `failure: Option<ApiFailure>` (`src/failure.rs`). `failure::classify_api_error` is the
  single place a library error variant becomes a `FailureKind`; reach it through `CloudClient::convert_error`,
  `convert_error_for_organization`, or `convert_error_for_lookup`, so classification is inherited by conversion.
- A handler adds the *stage* with `error.at_stage(FailureStage::…)` in `map_err` at the boundary that owns it.
  Recording is first-write-wins: a coarse outer fallback never overwrites a precise inner one.
- Never derive a category from message text. Never widen a vocabulary to anything but a `&'static str` from an enum
  or an allowlisted status — those types are what make SQL, identifiers, response bodies and credentials
  structurally unable to reach telemetry.
- A boundary that rewrites a message must carry `failure` across (`..error` in a struct literal, or `with_failure`).

## Adding a command

Local clap definitions live in `src/local/cli.rs`. Cloud clap definitions, handlers, builders, wrapper methods,
dispatch, and tests are co-located in the owning `src/cloud/<domain>.rs`; `src/cloud/cli.rs` owns the top-level
cloud arguments, command enum, domain re-exports, delegation, and top-level tests.

**Local:** 1. Add a variant to the relevant enum in `src/local/cli.rs` using clap derive macros. 2. Add the match
arm in `run()` in `src/local/mod.rs`; `main.rs` delegates to that boundary. 3. Implement the handler in a dedicated
module under `src/local/` (e.g. `server.rs`, `postgres.rs`) — don't pile new logic into `main.rs`.

**Cloud:**

1. Make sure `clickhouse-cloud-api` already supports the necessary endpoints and models.
2. Add the clap variant and argument structs to the owning `src/cloud/<domain>.rs`. Create a new domain module and
   privately re-export its command enum from `src/cloud/cli.rs` if the surface warrants its own grouping.
3. Classify the variant in the domain enum's exhaustive `is_write()` match. OAuth (Bearer) auth is read-only; write
   commands require API key auth and fail fast on OAuth + write. `CloudCommands::is_write_command()` in
   `src/cloud/cli.rs` exhaustively delegates to each domain. Add read/write tests next to the clap definitions.
4. Add the exhaustive command match to the domain's `run()` dispatcher. `dispatch()` in `src/cloud/mod.rs` (private;
   entered from `cloud::run`) delegates only at the top-level `CloudCommands` boundary — add an arm there only when
   introducing a new domain.
5. Add a thin wrapper method in the domain module's `impl CloudClient` block: delegate to `self.api().<lib_method>()`,
   map errors via `self.convert_error(e)` / `convert_error_for_organization(e, org_id)` /
   `convert_error_for_lookup(e, lookup)`, and unwrap with `Self::unwrap_response`. Use the library's types here.
6. If the command sends a body, extract `build_<name>_request(...)` in the same domain module returning the library's
   request struct. Cover it with minimal + maximal unit tests asserting on library request-struct fields.
7. Implement the handler in the same domain module. Body-sending handlers call the build helper, pass the result
   through the `CloudClient` wrapper, and print with
   `if json { println!("{}", serde_json::to_string_pretty(&data)?); } else { print_human(&data)?; }`.
   Drive every detail/get view through `print_human` so it shares serde's behaviour — deprecated-field hiding, and
   summarising a PEM-framed certificate or key instead of dumping its body; a `println!` or `tabled` cell bypasses
   both. List views stay `tabled`; short action confirmations stay plain `println!`. Every field of a library
   response type is `Option`, so never `unwrap()`/`expect()` one: render absence with
   `crate::cloud::output::or_absent` (`-`) or `ABSENT`, and have `--filter` predicates treat absence as non-matching.
8. Add `Cli::try_parse_from` coverage next to the domain command definition for the new command's body-related
   flags, asserting parsed values.

## Writing help text

- Help lives in `#[command(about/after_help)]` and arg doc comments in `src/cli.rs`, `src/local/cli.rs`,
  `src/cloud/cli.rs`, and `src/cloud/<domain>.rs`; one block is `const INSTALL_AFTER_HELP` in `src/local/cli.rs`.
- A help screen has only: one-line `about`, clap's `Usage:`, `Arguments:`/`Options:`, `Commands:`, and an optional
  trailing `CONTEXT FOR AGENTS:` block via `after_help`. No `long_about`; no other `after_help` header.
- `about`: imperative verb phrase, ≤ ~60 chars, no trailing period, no implementation detail; keep siblings parallel
  ("List X", "Get X details", "Create X", "Delete X"). Flag help: one line, ≤ ~70 chars, include units/format
  ("Interval in seconds"), and never repeat clap's `[default: …]` or `[possible values: …]` in prose.
- State cross-flag constraints on the flag itself ("only with `--replication-mode cdc_only`"). Add a second
  doc-comment paragraph (≤ ~3 lines) only for a constraint the flag's name and type cannot convey.
- Shared flags (`--api-key`, `--api-secret`, `--url`, `--org-id`, `--json`, `--debug`) read identically everywhere.
- `CONTEXT FOR AGENTS:` — hard cap 8 content lines, target 3-6, one fact per line. May hold: an auth requirement or
  precondition; where to get required inputs ("Service ID: `cloud service list`"); non-obvious runtime behaviour
  (timeouts, stdin handling, irreversibility, "must be stopped first"); an output note only when it changes what the
  agent does; a `Typical flow:` line; at most one docs URL.
  It must NOT hold implementation details, crates/files, HTTP or API mechanics, storage paths, history or
  compatibility notes, reassurance, or anything already in the flag list, `[default:]`, or the `about` line.
- Put shared context (auth model, how to find IDs, typical flow) on the parent (`cloud service`, `local server`);
  leaves add a block only for a leaf-specific gotcha. A plain `get`/`list` usually needs none.
- Do not write tests that pin help or README wording (`help.contains("some sentence")`, `include_str!` on
  `README.md`, whole-screen equality). They protect phrasing, not facts, and turn every rewording into a test edit.
  Test structure instead: `try_parse_from` outcomes, `ErrorKind`, defaults and value names clap renders, hidden
  flags staying hidden, every subcommand having an `about`, block size, and a flag reading identically everywhere.
  A fact that must not disappear from help is guarded by review against this section, not by a substring.
- Content users still need but help must not carry goes to `README.md` as a short example or ≤ 3-line note.

## Tests

Test coverage is non-negotiable.

- **Clap parsing** — `Cli::try_parse_from` tests next to each command definition (`src/cli.rs`, the owning
  `src/cloud/<domain>.rs`, `src/cloud/cli.rs`, `src/local/cli.rs`). Assert flag names, types, defaults, repeatability.
- **Request builders** — unit tests for `build_*_request` helpers next to the owning cloud domain code, asserting on
  library request-struct fields with minimal + maximal inputs.
- **Cloud subprocess + wiremock** — `tests/cli_request_shape_test.rs`. Spawn the real binary against a local mock
  server and assert on requests, auth, errors, and output; use it when handler runtime behavior is not covered by
  clap or request-builder tests.
- **Local subprocess** — one binary per concern under `crates/clickhousectl/tests/`: 21 `local_*` binaries
  (`local_server_*`, `local_postgres_*`, `local_docker_*`, `local_client_*`, `local_install_*`, `local_remove_*`,
  `local_init_json_test.rs`, `local_structured_errors_test.rs`, `local_version_error_test.rs`) plus
  `telemetry_test.rs`. Add a new file rather than growing `cli_request_shape_test.rs`, which is Cloud-only.
- **Pure logic** — inline `mod tests` blocks across `src/` for version resolution, auth precedence, output
  formatting, platform detection, and other module-local helpers.
- **Help and README text** — structural assertions only (see Writing help text). No wording pins.

## CI gates

- Pin all GitHub Actions deps to SHA hashes, not tags. Never populate secrets in Actions triggered by external PRs.
- Two path classifiers fail closed; **both** need an entry when a source or test file is added or renamed, or CI
  breaks. `scripts/classify-cloud-integration.py` maps API-library source/test paths to the `service`, `postgres`,
  `organization`, `clickpipes` suites (unknown paths select all suites);
  `scripts/classify-install-integration.py` holds `INSTALL_EXACT_PATHS`/`INSTALL_PREFIXES` for the live local install
  matrix, verified by `test-cli.yml` and `test-install.yml` on PRs that touch the classifier or the CLI
  (`scripts/tests/test_classify_install_integration.py`).
- Internal PRs classify the exact base-to-head diff on every push via a secret-free planner job; the
  `Cloud integration decision` check goes green automatically when no suites are affected. Affected suites only run
  after the `run-cloud-integration` label is applied (one-shot, bound to the labeled head SHA). Scheduled runs select
  all suites; manual runs use the requested scope. Label, override, and fork rules: `.github/CLOUD_INTEGRATION.md`
  (driven by `cloud-integration-decision.yml` + `scripts/cloud-integration-decision.py`).

## Dependencies

Use `cargo add` with the latest version and an explicit crate, e.g. `cargo add -p clickhouse-cloud-api url`.

## Releases

- Push a version tag (`git tag v0.2.3 && git push origin v0.2.3`) to run the release workflow.
- Bump in lockstep: `crates/clickhousectl/Cargo.toml` (`version` and the `clickhouse-cloud-api` dep version),
  `crates/clickhouse-cloud-api/Cargo.toml`, `npm/package.json`. `pypi/pyproject.toml` needs no manual bump — maturin
  takes the version from `crates/clickhousectl/Cargo.toml` via `dynamic = ["version"]`.
- `clickhouse-cloud-api` publishes to crates.io; `clickhousectl` to GitHub releases, crates.io, npm and PyPI from
  the same workflow in separate jobs (crates.io uses a token, npm and PyPI use OIDC).

## Git workflow and documentation

- Branch per feature/issue and use the PR workflow. PRs should have an associated issue.
- PRs should include `README.md` updates for functionality or behaviour users and developers must understand.
- Keep `AGENTS.md` up to date when development practice changes materially.
