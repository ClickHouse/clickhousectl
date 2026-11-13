---
name: add-cli-command
description: Adds or changes a clickhousectl command, subcommand or flag, for local commands (`local ...`) or ClickHouse Cloud commands (`cloud ...`). Covers the clap definition, dispatch, handler, output, Cloud permissions, tests and help. Use when adding a command or flag, exposing a `clickhouse-cloud-api` method in the CLI, or changing what a command sends or prints.
---

# Add a CLI command

Paths are relative to `crates/clickhousectl/`.

1. Follow the procedure for the kind of command:
   - Local (`local ...`, runs on this machine): [references/local.md](references/local.md)
   - Cloud (`cloud ...`, calls the Cloud API): [references/cloud.md](references/cloud.md)
2. Write help text with the `cli-help-text` skill (`.agents/skills/cli-help-text/SKILL.md`).
3. Finish with the steps below.

## Finish

- Add `Cli::try_parse_from` tests next to the clap definition for a new flag that has a default, a custom
  parser or value range, repetition, or conflicts and requirements. Skip a flag the derive alone parses.
- Classifier entries: a new `crates/clickhousectl/src/**/*.rs` outside `src/cloud/`, or a new file in
  `crates/clickhousectl/tests/`, needs one in `scripts/classify-install-integration.py` (plus
  `test-install.yml` `paths:` if install-relevant). A `src/cloud/` file needs none. Add a
  `scripts/classify-cloud-integration.py` entry only if a CLI file should select a live Cloud suite.
- Update the relevant user guide or reference under `docs/` when users will see the change, following
  `CONTRIBUTING.md`. Keep safety and behavioral details beside the affected task. Change the root `README.md`
  only when its capability overview or representative examples need updating; do not add a flag catalog.
- Check documentation links/anchors and example commands against this branch's help and request definitions.
- Run the CLI commands from "Commands" in the root `AGENTS.md`. Both clippy configurations must pass.

Copy this checklist and tick it off as you go:

```
- [ ] Procedure steps (local.md or cloud.md)
- [ ] Help text follows the cli-help-text skill
- [ ] try_parse_from tests for flags with their own parsing rules
- [ ] Install classifier lists new CLI files outside `src/cloud/` and `tests/`
- [ ] Relevant guide/reference and any affected README example
- [ ] Documentation links and command examples checked
- [ ] fmt, both clippy configurations, tests
```
