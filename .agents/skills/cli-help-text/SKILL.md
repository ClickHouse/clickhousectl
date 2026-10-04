---
name: cli-help-text
description: Writes and reviews `clickhousectl` `--help` text — command `about` lines, flag help, option order, and the `CONTEXT FOR AGENTS:` block — so people and coding agents can learn the CLI from help alone. Use when adding or changing a command or flag, editing clap `about`/`after_help`/arg doc comments, reviewing help in a PR, or fixing a help-structure test failure.
---

# CLI help text

Help is how people and coding agents learn the CLI. An agent reads `--help` the way it reads a tool description,
so every line should help it pick the right command, pass the right inputs, and avoid a surprise. Anything else
is noise that costs the reader time and tokens.

## Where help lives

- `#[command(about = ..., after_help = ...)]` and arg doc comments in `src/cli.rs`, `src/local/cli.rs`,
  `src/cloud/cli.rs` and `src/cloud/<domain>.rs` (all under `crates/clickhousectl/`).
- `const INSTALL_AFTER_HELP` in `src/local/cli.rs`.
- `src/cloud/permissions.rs` appends API-key permission lines to every executable Cloud command, built from each
  domain's declarations. Don't write those lines by hand.

## Screen shape

A help screen has only these parts, in this order:

1. A one-line `about`.
2. clap's `Usage:`, `Arguments:`/`Options:` and `Commands:` sections, with the standard headings.
3. Optionally, a trailing `CONTEXT FOR AGENTS:` block set with `after_help`.

No `long_about`, `before_help`, `after_long_help`, or any other `after_help` header.

## `about`

- An imperative verb phrase, up to about 60 characters, no trailing period.
- Say what the command does, not how it does it.
- Keep siblings parallel: "List X", "Get X details", "Create X", "Delete X".
- Mark beta commands with `(Beta)`.

## Flags and arguments

- One line, up to about 70 characters. Include units or format: "Interval in seconds", "RFC 3339 timestamp".
- Don't restate what clap renders: `[default: …]` and `[possible values: …]`.
- Put a cross-flag constraint on the flag it limits: "Only with `--replication-mode cdc_only`".
- Add a second doc-comment paragraph (up to about 3 lines) only for a constraint the flag's name and type
  can't convey.
- Use enums (`value_enum`) rather than describing valid strings in prose, so clap lists and checks them.
- Shared flags (`--org-id`, `--org-name`, `--api-key`, `--api-secret`, `--url`, `--json`, `--debug`) read the same
  everywhere. Copy the wording from an existing declaration.
- Resource names stay positional, under `Arguments`. Compatibility flags stay hidden.

## Option order

- Command-specific flags first, with display ranks below 900.
- Then the shared block, in this order: `--org-id`, `--org-name` (where it exists), `--api-key`, `--api-secret`,
  `--url`, `--json`, `--debug`. Use the `help_order` ranks in `src/cli.rs` (900–906). clap adds `--help` at 999.
- Set the rank at every declaration, including local `--json` and auth flags, so inherited flags keep the block
  together.
- Both local clients order common arguments as name, host, port, version, query, queries-file.
- Keep release-only URL hiding as it is.

## `CONTEXT FOR AGENTS:`

At most 8 content lines, ideally 3 to 6, one fact per line. Permission lines added by `permissions.rs` don't
count toward the 8.

Include only what changes what the agent does next:

- An auth requirement or precondition ("must be stopped first").
- Credential precedence, without storage paths.
- Where to get a required input: "Service ID: `cloud service list`".
- Runtime behaviour the flags don't show: timeouts, stdin handling, irreversibility, long waits, retry safety.
- An output note, only if it changes what the agent does with the output.
- A `Typical flow:` line.
- At most one docs URL.

Leave out implementation details, crate or file names, HTTP and API mechanics, storage paths, history or
compatibility notes, reassurance, and anything already in the `about` line, the flag list or `[default:]`.

Put shared context (auth model, how to find IDs, typical flow) on the parent command, such as `cloud service` or
`local server`. A leaf gets a block only for its own gotcha. A plain `get` or `list` usually needs none.

Write it plainly. State the fact and, where it isn't obvious, the reason. Don't use capitals or "MUST" for
emphasis: current models follow plain instructions closely, and emphasis makes them over-apply a rule.

Before (states mechanics, not consequences):

```
CONTEXT FOR AGENTS:
Calls DELETE /v1/organizations/{id}/services/{id} via clickhouse-cloud-api.
IMPORTANT: you MUST stop the service first!!
```

After:

```
CONTEXT FOR AGENTS:
Irreversible. Running service must be stopped first (--force or `cloud service stop`).
```

## When help can't hold it

Content users still need but help mustn't carry goes in the root `README.md`, as a short example or a note of
up to 3 lines.

## Tests

Test structure, not wording. Never pin help or README phrasing (`help.contains("some sentence")`, `include_str!`
on `README.md`, whole-screen equality): those tests break on every rewording and protect nothing.

Do test: `try_parse_from` outcomes, `ErrorKind`, defaults and value names clap renders, hidden flags staying
hidden, every subcommand having an `about`, block size, and shared flags reading the same everywhere. The
tree-wide checks are in `src/cli.rs` tests (`whole_command_tree_follows_help_structure` and its neighbours). A fact
that must stay in help is guarded by review against this skill.

## Checklist

1. Edit the clap definitions.
2. Build, then read each changed screen as an agent would see it:
   `cargo run -q -p clickhousectl -- <command path> --help`. Check the parent's screen too.
3. Check each line against the rules above. For each `CONTEXT FOR AGENTS:` line, ask whether an agent would act
   differently without it. If not, delete it.
4. Run `cargo test -p clickhousectl` to catch structure failures, then the gates in the root `AGENTS.md`.
