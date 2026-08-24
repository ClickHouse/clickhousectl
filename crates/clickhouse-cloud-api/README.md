# clickhouse-cloud-api

Typed Rust client for the [ClickHouse Cloud API](https://clickhouse.com/docs/en/cloud/manage/openapi).

## Development

### Structure

| Path | Purpose |
|------|---------|
| `src/client.rs`, `src/client/*.rs` | `Client`, shared HTTP machinery, and per-domain endpoint methods |
| `src/models.rs`, `src/models/*.rs` | Public facade/macro and private per-domain request/response models |
| `src/convert.rs`, `src/convert/*.rs` | Conversion error and per-domain response-to-request conversions |
| `src/error.rs` | Error types (`Http`, `Json`, `Api`) |
| `clickhouse_cloud_openapi.json` | Checked-in copy of the spec (used by tests) |
| `tests/spec_coverage_test.rs` | Thin snapshot/live-spec consumer of the shared drift analyzer |
| `../clickhouse-openapi-analyzer` | Canonical Rust/OpenAPI parsing, comparison, report, and exemptions |

### Field optionality

The OpenAPI spec uses two conventions for marking fields required vs optional:

- **Schemas with a `required` array** (newer/beta endpoints) use standard OpenAPI semantics.
- **Schemas without `required`** (GA/legacy endpoints) treat fields whose description starts with `"Optional"` as optional. Everything else is implicitly required.
- **Known partial `required` arrays** use the union of that array and the description heuristic, as configured by the analyzer.

Additional rules:

- **PATCH request schemas** (name contains `Patch` and ends with `Request`) are always all-optional.
- **Nullable fields** (`type: ["string", "null"]` or `oneOf` with null) are always `Option<T>`, even if required.

In `src/models/<domain>.rs`, request fields follow those rules: required non-nullable fields use `T`, while optional or nullable fields use `Option<T>`. Every response field uses `Option<T>` plus `skip_serializing_if`, so missing keys and explicit `null` deserialize natively to `None` and absent fields are omitted when serialized. `#[serde(default)]` is banned because it can fabricate required request values and is redundant for `Option` response fields.

### Deprecated fields

The OpenAPI spec marks some response fields as deprecated (e.g. `Service.tier`, `ApiKey.roles`, `Member.role`). In almost all cases, these are not needed. The Cloud API library disables them by default, gated by a Cargo feature flag `deprecated-fields`. Enable this feature if you need to consume deprecated fields.

### Scripts

```bash
# Show a JSON manifest of required/optional fields per schema
python3 scripts/resolve-field-requirements.py

# Regenerate the DEPRECATED_FIELDS constant from the snapshot
python3 scripts/regenerate-deprecated-fields.py

# Regenerate the BETA_OPERATIONS constant from the snapshot
python3 scripts/regenerate-beta-lists.py

# Check for drift between the live spec and the library (dry run)
python3 scripts/check-openapi-drift.py --dry-run
```

Field optionality is maintained by hand. Edit the owning model domain file when the drift check flags a mismatch, and add new domain types to the private module and facade re-exports in `models.rs`.

### Testing

```bash
cargo test -p clickhouse-cloud-api          # all tests
cargo test -p clickhouse-openapi-analyzer   # analyzer fixtures + executable parity
cargo test -p clickhouse-cloud-api --test spec_coverage_test # shared snapshot report
cargo test -p clickhouse-cloud-api --test client_test        # wiremock client tests
cargo test -p clickhouse-cloud-api --test models_test        # serde round trips
```

Live-API lifecycle suites are `#[ignore]`d by default (they provision real resources):

```bash
cargo test --test integration_test -- --ignored --nocapture           # ClickHouse service CRUD
cargo test --test integration_postgres_test -- --ignored --nocapture  # Postgres service CRUD
```

ClickPipes E2E binaries live under `tests/clickpipes/` and are declared as named `[[test]]` entries in `Cargo.toml`. Each per-source binary provisions its own ClickHouse Cloud service and exercises one source; `clickpipe_e2e_test` runs every stage in parallel against a single shared service:

```bash
cargo test --test clickpipe_e2e_test -- --ignored --nocapture            # all sources, one CHC service
cargo test --test clickpipe_s3_test -- --ignored --nocapture             # per-source: S3
cargo test --test clickpipe_kafka_test -- --ignored --nocapture          # per-source: Kafka (Redpanda)
cargo test --test clickpipe_kinesis_test -- --ignored --nocapture        # per-source: Kinesis
cargo test --test clickpipe_mongo_test -- --ignored --nocapture          # per-source: MongoDB
cargo test --test clickpipe_mysql_test -- --ignored --nocapture          # per-source: MySQL
cargo test --test clickpipe_postgres_ec2_test -- --ignored --nocapture   # per-source: Postgres-on-EC2
cargo build -p clickhousectl
CLICKHOUSECTL_TEST_BINARY=../../target/debug/clickhousectl cargo test --test clickpipe_postgres_cdc_test -- --ignored --nocapture # CHC-managed Postgres CDC through the CLI
cargo test --test clickpipe_smoke_test -- --ignored --nocapture          # create-only smoke against a shared service
```

All require `CLICKHOUSE_CLOUD_API_KEY`, `CLICKHOUSE_CLOUD_API_SECRET`, `CLICKHOUSE_CLOUD_TEST_ORG_ID`, `CLICKHOUSE_CLOUD_TEST_PROVIDER`, and `CLICKHOUSE_CLOUD_TEST_REGION` in the environment, and are wired into the scheduled `Cloud Integration` GitHub Actions workflow. The ClickPipes E2E suites additionally need AWS credentials and an `eu-west-1` region quota; `clickpipe_smoke_test` reads a pre-provisioned service ID from `CLICKHOUSE_CLOUD_TEST_CLICKPIPE_SERVICE_ID`.

The managed-Postgres CDC test fetches its private CA, writes it to a temporary file, and exercises failed CLI creation without `--ca-certificate` followed by successful snapshot and CDC with the flag. The mandatory CI fixtures do not currently include a PostgreSQL source with a publicly trusted certificate, so successful creation without an uploaded CA remains untested.

Applying `run-cloud-integration` to an eligible PR selects suites from that PR's merge-base-to-head diff; known changes without a live suite finish without entering the environment-bearing job, while unknown API source or test paths select all suites. Changes to the classifier, its tests, or either decision workflow also select all suites through an independent workflow guard. Manual `Cloud Integration` dispatches accept `scope=all`, `service`, `postgres`, `organization`, or `clickpipes`. The focused scopes run only their corresponding suite; `clickpipes` runs Postgres CDC plus the fixture-gated smoke test, while `all` runs all four mandatory suites plus that optional smoke test. Because stacked-PR diffs exclude inherited changes, manually run `scope=all` against the top stack branch for full-stack validation.

### Cloud integration merge decision

`Cloud integration decision` is the stable required check for the current PR head.
The trusted controller runs from the default branch, checks out only its own
revision, and never executes PR code. It accepts exactly one of these outcomes:

- The secretless planner and selected live suites succeeded for the current SHA.
- The planner succeeded for the current SHA and skipped the environment-bearing
  job because it selected no suites.
- A maintainer recorded an explicit one-shot override for the current SHA.

Opening a PR or pushing a new head creates a pending decision. To request live
testing, remove and reapply `run-cloud-integration`; leaving the label attached
does not make later pushes inherit the result. A failed planner or live job
fails the decision. Runs from unrelated labels, stale heads, forks, Dependabot,
scheduled events, or manual dispatches never automatically pass it. Fork PRs
need a same-repository mirror run or a maintainer override and never receive the
Cloud environment.

Only repository `maintain` and `admin` roles can override. The same maintainer
must first add an audit comment containing the exact current SHA and a reason or
link to the covering stack run, then apply `skip-cloud-integration`:

```bash
sha=$(gh pr view PR_NUMBER --json headRefOid --jq .headRefOid)
gh pr comment PR_NUMBER --body "/skip-cloud-integration $sha REASON_OR_RUN_URL"
gh pr edit PR_NUMBER --add-label skip-cloud-integration
```

The resulting check records the actor, SHA, UTC timestamp, reason, and comment
URL. An unauthorized actor, missing reason, different SHA, or unrelated label
cannot pass. The label remains attached after use. After any push, remove it,
add a new comment containing the new SHA, and reapply it. If a live run fails
after an override, a subsequent override is required to waive that failure.

#### Stacked PR policy

The selected policy is to validate the combined snapshot once at the top and
use explicit SHA-bound overrides on lower PRs. Run manual `scope=all` against
the exact top branch SHA. Before overriding a lower PR, verify its current head
is an ancestor of that successful top SHA, and put the successful Actions run
URL in the lower PR's override comment. Repeat the comment and label action for
every lower head; a top-stack run never silently creates lower-head decisions.
The top PR still needs an affected label run or an override linking the manual
run at that same top SHA. A changed lower head requires a new covering top run
and a new override.

#### Branch protection rollout

Do not require the decision until `.github/workflows/cloud-integration-decision.yml`
is on the default branch. After it merges, add the context without replacing
other required checks:

```bash
gh api --method POST \
  repos/ClickHouse/clickhousectl/branches/main/protection/required_status_checks/contexts \
  -f 'contexts[]=Cloud integration decision'
```

Verify that `Cloud integration decision` appears in the returned context list.
This is a one-time post-merge repository setting; it must not be applied from
the controller's own PR.

`spec_coverage_test` sends the checked-in sources and snapshot through the
private `clickhouse-openapi-analyzer` crate. The analyzer recursively traverses
the module trees rooted at `client.rs`, `models.rs`, and `meta.rs`, including
private per-domain files. That same analyzer powers the scheduled live-spec
issue, so operation, model, field, optionality, beta, deprecation, enum,
snapshot, and stale-exemption findings share one implementation. The single
ignored test runs the same report against the live spec.

### Optionality exemptions

Occasionally the spec cannot be followed literally because verified API
behavior differs. All such policy lives in
`crates/clickhouse-openapi-analyzer/src/config.rs`, including optionality,
extra-field, deprecated-field, extra-enum-value, non-OpenAPI-method, partial
required-schema, and unsupported-enum configuration.

Add an exemption only for a deliberate runtime behavior and document why the
spec cannot be followed. New unsupported-enum acknowledgements also require a
tracking issue. The analyzer reports stale field/enum exemptions and vanished
unsupported locations so obsolete entries are removed during normal drift
remediation. See the repository `AGENTS.md` for exact key formats and the full
remediation and verification procedure.
