# CLI development

[Contributing](../CONTRIBUTING.md) · [All documentation](README.md)

## Code layout

| Path | Responsibility |
| --- | --- |
| `crates/clickhousectl/src/main.rs`, `cli.rs` | Entrypoint, common CLI options, output mode |
| `crates/clickhousectl/src/local/` | Local command definitions, dispatch, servers, Docker/Postgres, output |
| `crates/clickhousectl/src/cloud/<domain>.rs` | Cloud domain definitions, handlers, builders, wrappers, tests |
| `crates/clickhousectl/src/cloud/client.rs` | Credential resolution and shared Cloud client/error handling |
| `crates/clickhousectl/src/stdout.rs` | CLI-owned stdout and typed broken-pipe handling |
| `crates/clickhouse-cloud-api/` | Published typed API client |
| `crates/clickhouse-openapi-analyzer/` | Private OpenAPI/Rust drift tooling |

The API/analyzer crates share [crate-level instructions](../crates/clickhouse-cloud-api/AGENTS.md). For command work, follow [add-cli-command](../.agents/skills/add-cli-command/SKILL.md) and its local/Cloud references. [AGENTS.md](../AGENTS.md) owns the invariants; avoid duplicating them in each procedure.

## Local tests

Run the standard CLI gates from [Contributing](../CONTRIBUTING.md#build-and-test). Pick focused checks during development:

- Clap parsing and request builders have tests beside command definitions.
- `crates/clickhousectl/tests/cli_request_shape_test.rs` spawns the binary against wiremock for Cloud requests, authentication, errors, and output.
- Local subprocess tests use separate `local_*` binaries for each concern.
- Pure version/auth/output logic uses inline tests.

Structural help tests check the full command tree, descriptions, shared flags, and agent context limits in both feature configurations:

```bash
cargo test -p clickhousectl --bin clickhousectl cli::tests::
cargo test -p clickhousectl --no-default-features --bin clickhousectl cli::tests::
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
```

Never pin help or README wording. Verify commands with `cargo run -q -p clickhousectl -- <command> --help` and inspect the parent screen too.

The Cloud and local-install classifiers fail closed for unclassified source/test files. Update both when adding or renaming one. Documentation paths alone do not select live suites. See `scripts/classify-cloud-integration.py` and `scripts/classify-install-integration.py`.

## Live Cloud integration

Use a dedicated test workspace: these tests create and change real Cloud resources. Maintainer labels, exact-SHA overrides, stacked-PR policy, and required checks are documented in [the CI guide](../.github/CLOUD_INTEGRATION.md). Affected suites run with `run-cloud-integration`; the decision check passes without the label when no suites are selected.

The library's suites cover services (`integration_test`), Postgres (`integration_postgres_test`), organization access (`integration_org_test`), and ClickPipes (`tests/clickpipes/`; only Postgres CDC runs in CI).

Provide API credentials through your secret manager and set the test scope:

```bash
export CLICKHOUSE_CLOUD_TEST_ORG_ID=<test-org-id>
export CLICKHOUSE_CLOUD_TEST_PROVIDER=aws
export CLICKHOUSE_CLOUD_TEST_REGION=eu-west-1
```

`CLICKHOUSE_CLOUD_API_KEY` and `CLICKHOUSE_CLOUD_API_SECRET` are required. The organization suite also requires `CLICKHOUSE_CLOUD_TEST_SECONDARY_USER_ID` for a second user in the test organization.

```bash
cargo test -p clickhouse-cloud-api --test integration_test -- --ignored --nocapture
cargo test -p clickhouse-cloud-api --test integration_postgres_test -- --ignored --nocapture
cargo test -p clickhouse-cloud-api --test integration_org_test -- --ignored --nocapture
```

Failures fail the run by default. `CONTINUE_ON_NON_BLOCKING_FAILURES=1` collects non-blocking capability failures for a final summary while continuing.

## Debugging endpoints

Cloud `--debug` prints the selected credential source and API URL. For a non-production control plane, `--url` overrides the Cloud API base URL; release builds accept it but hide it from help. `CLICKHOUSE_CLOUD_QUERY_HOST` separately overrides the Query API host.

Keep tests isolated from real credentials and telemetry. Use the existing temporary-project and local-mock patterns in subprocess tests and set `DO_NOT_TRACK=1` for development checks.

## Telemetry changes

The [user telemetry reference](reference/telemetry.md) describes what is recorded and how to disable it. `src/telemetry.rs` builds events from argument definitions, not raw values; `src/failure.rs` defines the bounded failure vocabulary. Paths here are relative to `crates/clickhousectl/`.

Classify library errors through `CloudClient::convert_error*`. Add a stage only at the boundary that owns it; the first stage wins. Preserve the failure object when rewriting diagnostics. Never derive classification from free text or introduce arbitrary values into telemetry fields. Validate both default and telemetry-compiled-out configurations when changing this code.
