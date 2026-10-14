# AGENTS.md

`CLAUDE.md` is a symlink to this file. Edit `AGENTS.md`; never replace the symlink.

clickhousectl (`chctl`) is the CLI for ClickHouse and Postgres, local and in ClickHouse Cloud. Use `--help` to
learn the current command surface. Root `README.md` introduces the CLI; `docs/` holds user guides, references,
and development docs. See `CONTRIBUTING.md` for documentation placement. The API library has its own README.
Do not duplicate user-facing documentation here.

## Commands

- `cargo fmt --all` before every commit (`fmt.yml` runs `cargo fmt --all --check`; bulk formatting commits are in `.git-blame-ignore-revs`).
- `cargo clippy -p clickhousectl -- -D warnings` && `cargo test -p clickhousectl`.
- Keep telemetry-compiled-out building and linting, as CI does: `cargo check -p clickhousectl --no-default-features`
  && `cargo clippy -p clickhousectl --all-targets --no-default-features -- -D warnings`.
- Library crates: `cargo clippy -p clickhouse-cloud-api -p clickhouse-openapi-analyzer --all-targets -- -D warnings`
  && `cargo test -p clickhouse-cloud-api -p clickhouse-openapi-analyzer`; `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`.
  If `deprecated-fields` changed, also `cargo check --workspace --all-features`.

**Done** means: `cargo fmt --all`; both clippy configurations clean; tests pass for every crate touched;
classifier mappings updated if a file was added or renamed; the relevant guide/reference updated for user-visible behaviour;
work on a branch, with an associated issue and a PR.

## Workspace

- `crates/clickhousectl/` — the CLI. All local logic; wraps `clickhouse-cloud-api` for cloud.
- `crates/clickhouse-cloud-api/` — typed Cloud API client, published to crates.io.
- `crates/clickhouse-openapi-analyzer/` — private OpenAPI/Rust drift tooling.
  Both library crates are governed by `crates/clickhouse-cloud-api/AGENTS.md`; read it before touching either.
- To resolve OpenAPI drift for the ClickHouse Cloud API, use the `openapi-drift-remediation` skill
  (`.agents/skills/openapi-drift-remediation/SKILL.md`).
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
- `CloudError.kind` decides the top-level error: `Auth` (401/403, missing credentials) → exit 4; `Usage` (input
  validation found at runtime) → clap-rendered, exit 2; else `Generic`. JSON mode prints every Cloud runtime failure,
  including auth and cancellation, via `cloud::output::print_error`; usage failures keep clap's text. Output mode
  never changes the exit code or the human message.
- CLI-owned stdout uses `src/stdout.rs`: crate-scoped `print!`/`println!` and `stdout::stdout()` suppress only typed
  `BrokenPipe` errors. Keep new manual writers on that handle; never suppress command/API failures or child exits.
- Exit codes: `0` success, else `Error::exit_code()` — `1` error, `3` cancelled, `4` auth required, and
  `ChildExit(code)` passes a spawned child's status through. Clap uses `2` for usage errors.

### Telemetry failure classification (#450)

- `CloudError.failure` (`src/failure.rs`) is classified only by `failure::classify_api_error`, reached through the
  `CloudClient::convert_error*` methods. Add the stage with `error.at_stage(FailureStage::…)` at the boundary that
  owns it; first write wins, so an outer fallback never overwrites an inner stage.
- Never derive a category from message text. Vocabularies are only `&'static str` from an enum or an allowlisted
  status, which keeps SQL, identifiers, response bodies and credentials out of telemetry.
- A boundary that rewrites a message must carry `failure` across (`..error` in a struct literal, or `with_failure`).

## Adding a command

To add or change a local or Cloud command or flag, use the `add-cli-command` skill
(`.agents/skills/add-cli-command/SKILL.md`). Local clap definitions live in `src/local/cli.rs`. Each Cloud
domain keeps its definitions, handlers and tests together in `src/cloud/<domain>.rs`.

## Writing help text

Use the `cli-help-text` skill (`.agents/skills/cli-help-text/SKILL.md`) whenever you add or change a command, a
flag, or any help text. It holds the help standard and its test rule.

## Tests

Test coverage is non-negotiable.

- **Clap parsing** — `Cli::try_parse_from` tests next to each command definition (`src/cli.rs`, the owning
  `src/cloud/<domain>.rs`, `src/cloud/cli.rs`, `src/local/cli.rs`). Assert flag names, types, defaults, repeatability.
- **Request builders** — unit tests for `build_*_request` helpers next to the owning cloud domain code, asserting on
  library request-struct fields with minimal + maximal inputs.
- **Cloud subprocess + wiremock** — `tests/cli_request_shape_test.rs`. Spawn the real binary against a local mock
  server and assert on requests, auth, errors, and output; use it when handler runtime behavior is not covered by
  clap or request-builder tests.
- **Local subprocess** — one `local_*` binary per concern under `crates/clickhousectl/tests/`. Add a new file
  rather than growing `cli_request_shape_test.rs`, which is Cloud-only.
- **Pure logic** — inline `mod tests` blocks across `src/` for version resolution, auth precedence, output
  formatting, platform detection, and other module-local helpers.
- **Help and README text** — structural assertions only (see the `cli-help-text` skill). No wording pins.

## CI gates

- Pin all GitHub Actions deps to SHA hashes, not tags. Never populate secrets in Actions triggered by external PRs.
- Two path classifiers fail closed; **both** need an entry when a source or test file is added or renamed, or CI
  breaks. `scripts/classify-cloud-integration.py` maps API-library source/test paths to the `service`, `postgres`,
  `organization`, `clickpipes` suites (unknown paths select all suites);
  `scripts/classify-install-integration.py` holds `INSTALL_EXACT_PATHS`/`INSTALL_PREFIXES` for the live local install
  matrix, verified by `test-cli.yml` and `test-install.yml` on PRs that touch the classifier or the CLI
  (`scripts/tests/test_classify_install_integration.py`).
- Affected Cloud integration suites run only after the `run-cloud-integration` label is applied; the
  `Cloud integration decision` check passes by itself when none are affected. Rules: `.github/CLOUD_INTEGRATION.md`.

## Dependencies

Use `cargo add` with the latest version and an explicit crate, e.g. `cargo add -p clickhouse-cloud-api url`.
Every crate declares the same `rust-version`; raise it in all three manifests, both READMEs and `msrv.yml` together.

## Releases

Use the `release` skill (`.agents/skills/release/SKILL.md`); a release needs a manual step in another repo.

## Git workflow and documentation

- Branch per feature/issue and use the PR workflow. PRs should have an associated issue.
- Keep root `README.md` to product capabilities, installation, and representative examples. Put user tasks in
  `docs/guides/`, complex behavior and output contracts in `docs/reference/`, and development workflows in
  `CONTRIBUTING.md`, `docs/development.md`, or the relevant repository skill. Update the existing document;
  do not add every new flag or command to the README. Keep safety constraints at the point of use and link
  new guides from `docs/README.md`. See `CONTRIBUTING.md` for documentation checks and writing conventions.
- API-library-only changes, Rust caller migrations, and analyzer work belong in
  `crates/clickhouse-cloud-api/README.md`; they do not require a root README or CLI user-guide change.
- Keep `AGENTS.md` up to date when development practice changes materially.
