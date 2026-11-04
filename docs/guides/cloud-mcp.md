# Connect a coding agent to the Cloud MCP server

[All documentation](../README.md)

The [ClickHouse Cloud MCP server](https://clickhouse.com/docs/products/cloud/features/ai-ml/mcp/remote-mcp) gives an agent read-only access to your organizations, services and data at `https://mcp.clickhouse.cloud/mcp`. `cloud mcp add` registers it in a coding agent and starts that agent's sign-in in one step.

You need the agent's CLI on your `PATH` and a ClickHouse Cloud account. You don't need to sign in to clickhousectl: the agent signs in to the MCP server itself and keeps its own token. Cloud-wide flags such as `--org-id` and `--api-key` are rejected, because clickhousectl doesn't authenticate to the MCP server.

## Register and sign in

```bash
clickhousectl cloud mcp add claude
```

The server is registered for your user, so it's available in every project. Sign-in then opens your browser and waits until you finish it. If you're already signed in to ClickHouse Cloud in that browser, it can finish in a few seconds. Start a new agent session to use the server's tools.

| Agent | Registers with | Signs in with |
| --- | --- | --- |
| `claude` | `claude mcp add --transport http --scope user` | `claude mcp login` |
| `codex` | `codex mcp add --url` | `codex mcp add`, which signs in itself |
| `opencode` | `opencode mcp add --url --global` | `opencode mcp auth` |
| `pi` | `pi mcp add --url` | `pi mcp login` |
| `gemini` | `gemini mcp add --scope user --transport http` | None. Run `/mcp auth clickhouse-cloud` inside Gemini CLI |

The server is named `clickhouse-cloud`; use `--name` to choose another name.

## Rerun or remove

Rerunning is safe and signs in again, for example after you sign out or a token expires. Claude Code keeps an existing entry with the same name; the other agents replace it.

To remove the server, use the agent's own command, such as `claude mcp remove clickhouse-cloud -s user` or `codex mcp remove clickhouse-cloud`.

## Use it from scripts and agents

With `--json`, or when clickhousectl detects a coding agent, stdout holds one object with `agent`, `name`, `url`, `registration` (`added` or `already_registered`) and `signed_in`. For Gemini CLI, `sign_in_command` holds the command to run in a session. The agent CLI's own messages, including any sign-in URL, go to stderr. If the agent CLI fails, clickhousectl exits with its exit code.

Sign-in blocks until a person finishes it in the browser, including when a coding agent runs the command.
