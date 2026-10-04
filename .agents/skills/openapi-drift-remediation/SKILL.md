---
name: openapi-drift-remediation
description: Remediates drift between the live ClickHouse Cloud OpenAPI spec and this repo — plans the split into API-library and CLI pull requests, fixes `clickhouse-cloud-api` against the spec, then exposes the change in `clickhousectl`. Use when working an OpenAPI drift issue, when `scripts/check-openapi-drift.py` or `spec_coverage_test` reports findings, or when the Cloud API spec has changed.
---

# OpenAPI drift remediation

Drift is fixed in two kinds of pull request, never one:

- **API PR** — `clickhouse-cloud-api` (and the analyzer config) only. It is published to crates.io and reviewed
  against the spec, so it should read as a faithful translation of the spec and nothing more.
- **CLI PR** — `clickhousectl` exposure of what the API PR added. Exposure is a product decision (which flags,
  which commands, what to leave out), reviewed on its own terms and revertable without touching the library.

An API PR may touch CLI code only to keep it compiling after a breaking library type change, with no new
surface and an unchanged wire request (pin it with a test). A CLI PR never changes the library.

Read `crates/clickhouse-cloud-api/AGENTS.md` (model policy, analyzer configuration) and the root `AGENTS.md`
(help text, tests, gates) before editing. This skill is the workflow; those files are the rules.

## 1. Reproduce and inventory

1. Start from the branch the work will stack on (usually `main`, or the head of an open drift stack) and run
   `python3 scripts/check-openapi-drift.py --dry-run`. It prints the issue body and does not touch the snapshot.
2. Work from the typed findings: `spec_pointer` is an RFC 6901 location in the live spec, `rust_item` the intended
   Rust location. The analyzer exits 0 even when it finds drift — read `has_drift`/`actionable_count`.
3. Group the findings by library domain (`src/client/<domain>.rs`, `src/models/<domain>.rs`) and by the CLI command
   group they would appear in (`cloud service`, `cloud postgres`, `cloud clickpipe`, `cloud org`, ...). Note the
   cross-cutting ones separately: enum values, deprecations, beta lists, stale exemptions, the snapshot itself.

## 2. Plan the PR pairs

- One pair (an API PR and a CLI PR) is enough when the changes affect a single CLI command group, such as
  `cloud postgres`.
- Use one pair per command group when the changes affect several groups or add a new library module.
- The first API PR also refreshes the spec snapshot and fixes findings that don't belong to one group, such as
  new enum values.
- In each pair, the API PR lands before the CLI PR.
- `spec_coverage_test` fails until the last API PR lands, listing the drift not yet fixed. Run the library tests
  with `cargo test --no-fail-fast` so this expected failure doesn't hide a real one.
- Issues: the last API PR fixes the drift issue, and earlier ones reference it. Open one issue per CLI PR listing
  the planned commands, including any new library methods you chose not to expose and why.
- Write the plan down before starting, and check it with the user if the split isn't obvious.

## 3. API PR

1. Replace `crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json` with the same live document being remediated
   (first API PR only). Never hand-edit the spec; snapshot operation/schema findings mean this file is stale.
2. Fix each finding at its `spec_pointer` and `rust_item`:
   - Missing/extra operations: add or remove the `Client` method in the owning `src/client/<domain>.rs`. Only
     intentional non-OpenAPI helpers belong in `non_openapi_client_methods`.
   - Missing models/fields, extra fields: update structs/enums/aliases and their explicit
     `#[serde(rename = "...")]` wire names (never `rename_all`) in the owning `src/models/<domain>.rs`, then
     re-export from the `models.rs` facade. A new schema needs one Rust type per position it is used in (`{Name}`, `{Name}Response`,
     or both) with the same fields on each. An undefined `$ref` (`missing_schema_definition`) is an upstream-spec
     defect, not a model to invent locally.
   - Optionality: request fields are `T` when the resolved spec requires them, else `Option<T>`; every response
     field is `Option<T>`; all `Option` fields carry `skip_serializing_if`. Never add `#[serde(default)]`.
   - Enum values: update the variant, its wire name, its `Display`, and any `VALUES` const. Keep data-carrying
     catch-alls.
   - Beta/deprecation: rerun `python3 scripts/regenerate-beta-lists.py` and
     `python3 scripts/regenerate-deprecated-fields.py` in every API PR that adds operations or deprecated fields.
     A deprecated field on a split schema needs its `{Name}Response` entry and `cfg` marker added by hand.
   - Operation permissions: regenerate `src/meta/operations.rs` with
     `cargo run -p clickhouse-openapi-analyzer --bin openapi-drift-analyzer -- --spec crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json --generate-operations crates/clickhouse-cloud-api/src/meta/operations.rs`,
     then `cargo fmt --all`. Model unsupported security deliberately; never flatten it into an empty permission list.
   - Stale exemptions: remove or narrow the entry in the analyzer `config.rs`. Add an exemption only for verified
     runtime behaviour, with a comment saying why the spec cannot be followed — never to make CI green.
   - Unsupported enum constraints: prefer changing the Rust field to a concrete value enum. Otherwise
     acknowledge the pointer in `acknowledged_unsupported_enum_pointers` with a tracking issue.
3. Add library tests for what changed: missing-key → `None` and explicit-`null` → `None` for each new response type;
   request strictness and the `TryFrom` write-back for each split pair; wiremock coverage for each new method; facade
   paths for new exports. Add live integration steps for new operations to the owning suite.
4. Map any added or renamed file in both classifiers (`scripts/classify-cloud-integration.py`,
   `scripts/classify-install-integration.py`) and their fixtures.
5. Document the change, and any Rust caller migration, in `crates/clickhouse-cloud-api/README.md`. The root README
   does not change in an API PR.
6. Run the library gates from the root `AGENTS.md`, plus `cargo check --workspace --all-features` and both
   `clickhousectl` clippy configurations (a library change can break the CLI build). Re-run the dry run: the last
   API PR must report no actionable drift.

## 4. CLI PR

Use the `add-cli-command` skill (`.agents/skills/add-cli-command/SKILL.md`) for each new command or flag.
