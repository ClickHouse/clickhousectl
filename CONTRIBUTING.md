# Contributing

Read [AGENTS.md](AGENTS.md) for repository invariants and [CLI development](docs/development.md) for code layout and detailed test workflows. Work on a branch associated with an issue and open a pull request for review.

## Build and test

Install a current stable Rust toolchain. Run commands from the repository root:

```bash
cargo build -p clickhousectl
cargo run -p clickhousectl -- --help
```

To install your checkout:

```bash
cargo install --path crates/clickhousectl
```

For CLI code changes, run the required checks:

```bash
cargo fmt --all
cargo clippy -p clickhousectl -- -D warnings
cargo test -p clickhousectl
cargo check -p clickhousectl --no-default-features
cargo clippy -p clickhousectl --all-targets --no-default-features -- -D warnings
```

For API/analyzer changes, read the [crate guidance](crates/clickhouse-cloud-api/AGENTS.md) and run its gates from root `AGENTS.md`. Keep API-library work separate from CLI exposure. Live tests create resources; use the [integration procedure](docs/development.md#live-cloud-integration), not ordinary development credentials.

## Put documentation where readers need it

| Content | Destination |
| --- | --- |
| Product/capability overview, install, representative quickstarts | [Root README](README.md) |
| End-to-end user tasks with prerequisites and verification | [Guides](docs/README.md#task-guides) |
| Complex behavior, safety constraints, machine-readable contracts | [References](docs/README.md#reference) |
| Flags, defaults, allowed values, immediate command warnings | CLI `--help` |
| Build/test workflows, architecture, contributor procedures | This guide, [development docs](docs/development.md), and repository skills |
| Rust API capabilities, caller migrations, drift/analyzer changes | [API library README](crates/clickhouse-cloud-api/README.md) |

Update the existing guide/reference when behavior changes; a README edit is needed only if its overview or examples change. Do not add an exhaustive flag catalog or duplicate every new command in the README. Delete redundant explanations instead of moving them into another long document.

Keep authentication, network defaults, destructive effects, asynchronous completion, and retry constraints beside the step they affect. Link from a short example to any deeper contract. Use straightforward technical language, sentence casing, and no em dashes. Do not document a CLI capability until it is exposed in the CLI.

For documentation changes:

- Check commands and JSON bodies against this branch's help and request definitions, not an older website reference. Do not run create/delete examples against live services just to validate syntax.
- Check relative links and heading anchors, including links into moved sections. Add new documents to [the documentation index](docs/README.md).
- Render the relevant help and run the [structural help checks](docs/development.md#local-tests) when examples or command guidance change. Do not add tests that pin documentation wording.
- Run `git diff --check` and inspect the full diff, including new files, for lost safety details and unintended changes.

## Add or change a command

Follow the repository's [add-cli-command skill](.agents/skills/add-cli-command/SKILL.md) and [help-text rules](.agents/skills/cli-help-text/SKILL.md). Test parsing and runtime behavior, including read/write authorization and JSON errors. New or renamed source/test files need explicit classification in both integration classifiers.

Use the [OpenAPI drift workflow](.agents/skills/openapi-drift-remediation/SKILL.md) for API drift and the [release workflow](.agents/skills/release/SKILL.md) for releases. These are task-specific procedures; ordinary documentation changes do not require release work.
