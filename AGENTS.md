# AGENTS.md

`CLAUDE.md` is a symlink to this file. Edit `AGENTS.md`; never replace the symlink.

clickhousectl (`chctl`) is the CLI for ClickHouse and Postgres, local and in ClickHouse Cloud. Use `--help` to
learn the current command surface. Root `README.md` introduces the CLI; `docs/` holds user guides, references,
and development docs. See `CONTRIBUTING.md` for documentation placement. The API library has its own README.
Do not duplicate user-facing documentation here.

## Commands

- `cargo fmt --all` before every commit (`fmt.yml` runs `cargo fmt --all --check`; bulk formatting commits are in `.git-blame-ignore-revs`; add the squash SHA on main in a follow-up, as branch SHAs don't survive squash).
- `cargo clippy -p clickhousectl --all-targets -- -D warnings` && `cargo test -p clickhousectl`.
- Keep telemetry-compiled-out building and linting, as CI does: `cargo check -p clickhousectl --no-default-features`
  && `cargo clippy -p clickhousectl --all-targets --no-default-features -- -D warnings`.
- Library crates: `cargo clippy -p clickhouse-cloud-api -p clickhouse-openapi-analyzer --all-targets -- -D warnings`
  && `cargo test -p clickhouse-cloud-api -p clickhouse-openapi-analyzer`; `python3 -m unittest discover -s scripts/tests -p 'test_*.py'`.
  CI also runs `cargo clippy --workspace --all-targets --all-features -- -D warnings` and
  `cargo test -p clickhouse-cloud-api --features deprecated-fields`; run both if `deprecated-fields` changed.

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
  The project root is found git-style, once per run (`init::project_root`): the nearest ancestor with a `.clickhouse/`
  directory owned by the user, stopping at home, whose `.clickhouse/` is global; `local init` uses the cwd.
  `CLICKHOUSECTL_PROJECT_DIR` replaces discovery (and `local init`'s cwd) with a named root.

## CLI invariants

- New Cloud handlers go through `CloudClient` wrapper methods co-located in each domain module, not
  `clickhouse_cloud_api::Client` directly. `src/cloud/client.rs` owns the core client, credential precedence,
  error conversion, and response unwrapping.
- One deliberate exception to wrapper error conversion: `run_service_query{,_bearer}` in `src/cloud/services/client.rs`
  return the raw library error and an unread `reqwest::Response`, because the query paths stream the body and
  classify the error themselves before `convert_query_error`. Do not copy that shape for other endpoints.
- Cloud handlers support `--json` unless there is good reason not to. JSON is emitted automatically when `--json`
  is passed or a coding agent is detected — `json_output()` in `main.rs` wraps `is_ai_agent::detect()` and is the
  only output-mode decision (`CLICKHOUSECTL_AUTO_JSON=0` turns detection off).
- `CloudError.kind` decides the top-level error: `Auth` (401, missing credentials; a 403 is not) → exit 4; `Usage` (input
  validation found at runtime) → clap-rendered, exit 2; else `Generic`. JSON mode prints every Cloud runtime failure,
  including auth and cancellation, via `cloud::output::print_error`; usage failures keep clap's text. Output mode
  never changes the exit code or the human message.
- CLI-owned stdout uses `src/stdout.rs`: crate-scoped `print!`/`println!` and `stdout::stdout()` suppress only typed
  `BrokenPipe` errors. Keep new manual writers on that handle; never suppress command/API failures or child exits.
  Crate-scoped `eprint!`/`eprintln!` there discard every stderr write failure, so use them, not hand-written
  `writeln!(stderr)`; clippy denies `std::`-qualified print macros.
- Single exit (#320): every invocation ends in `main`'s telemetry tail. The only exemptions are shell completion
  (`CHCTL_COMPLETE`, the first statement of `main`: no telemetry, update check, network or writes) and the hidden
  `telemetry send` child. Completion value candidates read local state only, never the Cloud API.
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
domain keeps its definitions, handlers and tests together in `src/cloud/<domain>.rs`, or `src/cloud/<domain>/` for
large domains (command enums and shared arg groups in `cli.rs`, `CloudClient` wrappers in `client.rs`, one file
per command family, tests beside the code they test; a source-specific arg struct may sit with its builder).

## Writing help text

Use the `cli-help-text` skill (`.agents/skills/cli-help-text/SKILL.md`) whenever you add or change a command, a
flag, or any help text. It holds the help standard and its test rule.

## Tests

Test coverage is non-negotiable. Test each behaviour in **one** layer: the cheapest one that reaches it, which is
the unit layer by default. Where a unit test and a subprocess test overlap, keep the unit test.

- **Clap parsing** — `Cli::try_parse_from` tests next to each command definition (`src/cli.rs`, the owning
  `src/cloud/<domain>.rs` or `src/cloud/<domain>/cli.rs`, `src/cloud/cli.rs`, `src/local/cli.rs`). Assert flag
  names, types, defaults, repeatability.
- **Request builders** — unit tests for `build_*_request` helpers next to the owning cloud domain code, asserting on
  library request-struct fields with minimal + maximal inputs.
- **Cloud subprocess + wiremock** — `tests/cli_request_shape_test.rs`. Spawn the real binary against a local mock
  server. It is the only layer that runs handlers, so a test belongs here only if it exercises: a handler making
  two or more calls or a read-modify-write; a distinct error path's exit code or JSON error; process behaviour
  (stdout/stderr, closed pipes, files, env, credentials); auth-mode wiring, once rather than per command; or one
  wiring test per command family (method, path, org, builder body on the wire). Not here: input validation
  (including "fails before HTTP" when clap rejects), field-by-field bodies, per-command repeats of shared
  behaviour (`convert_error`, the OAuth write check), or wording.
- **Local subprocess** — one `local_*` binary per concern under `crates/clickhousectl/tests/`, for what needs the
  real binary: files, processes, locks, signals, exit codes, JSON on stdout/stderr. Add a new file rather than
  growing `cli_request_shape_test.rs`, which is Cloud-only.
- **Live Cloud** — `#[ignore]`d `live_*_test.rs` under `crates/clickhousectl/tests/`, for CLI behaviour only real
  Cloud shows. They reuse the library's harness via `#[path]` and run in `cloud-integration.yml`
  (`docs/development.md`).
- **Pure logic** — inline `mod tests` blocks across `src/` for version resolution, auth precedence, output
  formatting, platform detection, and other module-local helpers.
- **No wording pins** — anywhere: help, README, error messages or human output. Assert the exit code, error code
  or kind, stream, JSON fields, and the identifiers a message must carry (the offending flag or value, an ID, a
  suggested command, re-parsed through clap). Asserting that a secret is absent is fine. Help structure: see the
  `cli-help-text` skill.
- **Not worth a test** — clap, serde or std behaviour on its own (a derive parsing, a `PossibleValuesParser`
  rejecting); flags that don't exist; assertions that cannot fail. Subprocess tests never wait out a production
  timeout; use the overrides in `src/timing.rs` (`docs/development.md`).

## CI gates

- Pin all GitHub Actions deps to SHA hashes, not tags. Never populate secrets in Actions triggered by external PRs.
- `audit.yml` runs `cargo audit --deny warnings` on manifest/lockfile PRs and every weekday; fix or bump an advisory
  rather than ignoring it. Dependabot (`.github/dependabot.yml`) opens weekly grouped Cargo and Actions updates.
- Path classifiers: add or rename a file under `crates/clickhouse-cloud-api/` and `scripts/classify-cloud-integration.py`
  needs an entry (unknown paths select all live suites); add or rename a `crates/clickhousectl/src/**/*.rs` outside
  `src/cloud/`, or a file in `crates/clickhousectl/tests/`, and `scripts/classify-install-integration.py` does. Install-relevant
  entries also go in `test-install.yml` `paths:` (an inventory test checks). Other paths need neither.
- Affected Cloud integration suites run only after the `run-cloud-integration` label is applied; the
  `Cloud integration decision` check passes by itself when none are affected. Rules: `.github/CLOUD_INTEGRATION.md`.

## Dependencies

Use `cargo add` with the latest version and an explicit crate, e.g. `cargo add -p clickhouse-cloud-api url`.
The MSRV is `rust-version` in the root `[workspace.package]` (`msrv.yml` reads it); raise it there and in both READMEs.

## Releases

Use the `release` skill (`.agents/skills/release/SKILL.md`); a release needs a manual step in another repo.

## Git workflow and documentation

- Branch per feature/issue and use the PR workflow. PRs should have an associated issue.
- Keep root `README.md` to product capabilities, installation, and representative examples. Put user tasks in
  `docs/guides/`, complex behavior and output contracts in `docs/reference/`, and development workflows in
  `CONTRIBUTING.md`, `docs/development.md`, or the relevant repository skill. Update the existing document;
  do not add every new flag or command to the README. Keep safety constraints at the point of use and link
  new guides from `docs/README.md`. See `CONTRIBUTING.md` for documentation checks and writing conventions.
- API-library changes and Rust caller migrations go under `## Unreleased` in `crates/clickhouse-cloud-api/CHANGELOG.md`
  (capabilities and usage in its `README.md`, analyzer work in `docs/development.md`); they do not require a root
  README or CLI user-guide change.
- Keep `AGENTS.md` up to date when development practice changes materially.
