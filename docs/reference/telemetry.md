# Telemetry

[All documentation](../README.md)

The CLI collects anonymous usage data to guide development. Events contain the command path; explicitly supplied flag and positional argument **names**, never values; outcome and exit code; CLI version, OS and architecture; CI presence; and detected coding-agent identity. There is no installation ID or device ID.

Argument values, SQL, credentials, resource identifiers, native arguments after `--`, and unrecognized input tokens are not recorded. Defaults and values supplied by the environment do not count as explicitly passed arguments. For example, `local server stop analytics-prod` records that `name` was supplied, not `analytics-prod`.

Runtime failures can add bounded fields: failure stage/kind, an allowlisted HTTP status, and retry, provisioning-state, and duration categories. These are fixed vocabularies, not error messages, response bodies, or exact timings. Successful runs have no failure classification. See the [failure definitions](../../crates/clickhousectl/src/failure.rs) for the current values and [development notes](../development.md#telemetry-changes) for the rules governing changes.

## Notice and controls

The first ordinary run shows a notice, records it in `~/.clickhouse/telemetry.json`, and sends nothing. Sending starts on a following run unless disabled. Explicit `telemetry enable` enables sending immediately and skips the notice.

```bash
clickhousectl telemetry disable
clickhousectl telemetry status
export DO_NOT_TRACK=1
```

`disable` persists the preference per machine. `DO_NOT_TRACK=1` overrides it for an environment. `status` is read-only: it does not create preferences, emit an event, or refresh the update cache. Before first-run setup, it reports unconfigured status.

To inspect a payload locally without sending it:

```bash
CHCTL_TELEMETRY_DEBUG=1 clickhousectl local list
```

The debug payload is printed to stderr and nothing is sent. Apart from read-only `telemetry status`, each invocation records one event when enabled. Native-client handoffs record `exec_attempt` with code 0 before execution; this means handoff was reached, not that the native client succeeded. Failures the wrapper detects before handoff retain their actual failure status.

Sending runs in a short-lived detached process. Distribution packagers can remove telemetry, including its subcommand, with `cargo build -p clickhousectl --no-default-features`.
