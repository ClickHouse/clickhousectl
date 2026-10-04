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

- Add `Cli::try_parse_from` tests next to the clap definition for each new flag. Check the parsed value, the
  default, whether it can repeat, and any conflicts with other flags.
- For each new source or test file, add an entry to both `scripts/classify-cloud-integration.py` and
  `scripts/classify-install-integration.py`. CI fails on a file that is in neither.
- Add a short example to the root `README.md` when users will see the change.
- Run the CLI commands from "Commands" in the root `AGENTS.md`. Both clippy configurations must pass.

Copy this checklist and tick it off as you go:

```
- [ ] Procedure steps (local.md or cloud.md)
- [ ] Help text follows the cli-help-text skill
- [ ] try_parse_from tests for new flags
- [ ] Both classifiers map new files
- [ ] README example
- [ ] fmt, both clippy configurations, tests
```
