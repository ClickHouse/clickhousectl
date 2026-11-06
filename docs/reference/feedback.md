# Feedback

[All documentation](../README.md)

`clickhousectl feedback` sends a short free-text message to the clickhousectl team. Use it to say what the CLI could not do, what you expected, or what you would like to see. Coding agents are invited to send feedback on their own when the CLI lacked a capability, flag, or output they needed, or when its help or errors misled them.

```bash
clickhousectl feedback "cloud service create should accept a --wait flag"
clickhousectl feedback local server start cannot pick a free port automatically
clickhousectl feedback --json "postgres client should accept a database name"
```

Separate words are joined with single spaces, so quotes are optional. Every word from the first message word on is message text, even one that looks like a flag, so put options such as `--json` before the message. There is no prompt or confirmation: running the command sends the message.

## What is sent

Each message is one request to `https://chctl.clickhouse.com/v1/feedback` carrying:

- the message, trimmed
- the CLI version, operating system and architecture
- whether the `CI` environment variable is set
- whether a coding agent was detected, and which one
- the request headers every clickhousectl request carries: the `clickhousectl/<version>` User-Agent and, when a coding agent exposes them, its `agent-session-id` and `traceparent`
- `cloud_auth`, the kind of ClickHouse Cloud credential in use: `none`, `api_key`, or `oauth`
- `org_ids`, the ClickHouse Cloud organization IDs those credentials belong to

The receiving service records the time and the country and Cloudflare edge location derived from the connection. It does not store your IP address.

Feedback does not send your name, email, user ID, API key or key ID, organization names, Cloud credentials, command history, or an installation or device ID. Messages are kept permanently. They are not linked to [telemetry](telemetry.md) events, which carry no session headers.

## Cloud organizations

With Cloud credentials, feedback is not anonymous at the organization level: it names your organizations, so the team can see which organizations are asking. The credentials are the ones any Cloud command would use, in the usual order: a project API key, `CLICKHOUSE_CLOUD_API_KEY` and `CLICKHOUSE_CLOUD_API_SECRET` (also read from `.env`), then the OAuth login saved in `~/.clickhouse/tokens.json`. A login that an older version saved under the project's `.clickhouse/` directory is ignored, and feedback never moves, rewrites or removes a saved login. Before sending, the command asks the Cloud API who the credentials belong to, and keeps only the organization IDs:

| Credentials | `cloud_auth` | `org_ids` |
| --- | --- | --- |
| None, or an expired OAuth login | `none` | empty; no Cloud request is made |
| API key | `api_key` | the organization that owns the key |
| OAuth login | `oauth` | every organization you belong to |

`org_ids` lists each organization once, and at most 100 of them.

The lookup waits at most 5 seconds and is not retried. If it fails for any reason (offline, timeout, rejected credentials, an API error), `org_ids` is empty, `cloud_auth` still names the credential kind, and the feedback is sent anyway. The lookup never prompts you to log in, never refreshes an expired login, and prints nothing.

## What to write

Describe the gap and what you expected, not your workload. The message is stored as plain text after the service strips control and invisible characters. Do not include credentials, passwords, SQL, hostnames, connection strings, service IDs, or personal data. Send one message per gap rather than one per retry.

## Limits and errors

A message must contain at most 500 characters (Unicode scalar values) after trimming and Unicode NFC normalisation, the form the service stores, so a few characters count as more than one. Control characters other than newline and tab are rejected. An invalid message is a usage error: the command exits `2` and sends nothing.

| Result | Exit code | Output |
| --- | --- | --- |
| Sent | `0` | A confirmation; with `--json` (or a detected coding agent), `{"sent": true, "length": N}`, where `length` is the message length in characters |
| Invalid message | `2` | A usage error on stderr; nothing is sent |
| Not delivered | `1` | An error on stderr naming the reason: the service rejected the message, the request was too large, too many messages were sent recently, the service was unavailable, or it could not be reached within 10 seconds |

Errors in JSON mode keep the same text diagnostics as the other [management commands](automation.md#management-commands). A failed send is not retried automatically.

## Telemetry settings

Feedback is an explicit action, so `clickhousectl telemetry disable` and `DO_NOT_TRACK` do not block it; they control passive [telemetry](telemetry.md) only. When telemetry is enabled, the `feedback` invocation records an ordinary telemetry event with the command and the flags given before the message, and nothing from the message itself.

Distribution packagers can remove the command, together with telemetry, with `cargo build -p clickhousectl --no-default-features`.
