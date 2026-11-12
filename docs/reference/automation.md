# Automation and output contracts

[All documentation](../README.md)

## Select output explicitly

Use `--json` for commands that return structured data. Detected coding agents select JSON automatically. Use explicit flags in scripts rather than depending on detection.

```bash
clickhousectl cloud service list --json
clickhousectl cloud key list --all --json
clickhousectl local clickhouse list --json
```

Cloud structured output follows each command's API-shaped contract, usually camelCase but sometimes snake_case. Missing/null optional response fields are generally omitted. Do not assume all lists are arrays, all fields have the same casing, or every timestamp has the same type.

| Command | Successful output |
| --- | --- |
| `cloud service list --json` | Bare array |
| `cloud auth login --json` | Object with `saved` (file path), `identity`, `verification` (`verified` or `unverified`), and optional `warning`; the OAuth device code and URL go to stderr |
| `cloud auth logout --json` | Object with `oauth_tokens` and `api_keys`, each `cleared` or `unchanged` |
| `cloud auth signup --json` | Object with `url` and `browser_opened` |
| `cloud service create --json` | The API's `service` and one-time `password`, plus `next` (see [Wait for completion](#wait-for-completion-and-retry-deliberately)); with `--dotenv`, `password` is omitted and `dotenv` has `file` and the written `keys` |
| `cloud service dotenv --json` | Object with `file`, `service` (name), and `vars` of `key` and `value`; never a password |
| `cloud key list --json` | Object with `result` and optional pagination metadata; `--all` gives a combined array |
| `cloud service settings list --json` | Object with `settings`; values retain string/number types |
| `cloud clickpipe settings get --json` | Object with snake_case setting names |
| `local init --json` | Object with `paths` this run created (`.clickhouse/`, `clickhouse/`, `postgres/`) and, when the directory was inside another project, `enclosing_project` with that project's path |
| `local clickhouse list --json`, `local postgres list --json`, `local status --json` | Object with `servers`, `total_servers`, `total_running_servers`, and `project_scope` (`kind` and `path`, below); each server has an `engine` (`clickhouse` or `postgres`). A stopped ClickHouse server's `version` is the one it was last seen ready on, which a restart without `--version` reuses; a start that never became ready does not change it. An engine's `list` holds only that engine; `status` holds both |
| `local udf deploy --json` | Object with `name`, `server`, `type`, `runtime`, `reloaded` (the server was running, so functions were reloaded), `loaded` (`null` when the server is stopped), `interpreter`, `ignored_fields`, `ignored_files`, `warnings` (such as an interpreter that is not Python 3.11, or another deployed UDF that is broken), `function_config`, and `scripts_dir` |
| `local udf list --json` | Object with `server`, `server_running`, and a `udfs` array of `name`, `type`, `runtime`, `loaded` (`null` when the server is stopped), and `last_deploy_rejected` (the running server rejected the last deploy of these files; a loaded function runs an earlier definition) |
| `cloud udf deploy --json` | Object with `action` (`created` or `version_created`), `udf`, and `attachment` |
| `cloud udf list`, `cloud udf version list`, `cloud udf attachment list` with `--json` | Object with a `result` array and optional outer `limit`, `totalCount`, and `nextCursor`; missing/null metadata is omitted |
| `cloud postgres metrics --json` | API-shaped metrics; data-point timestamps are epoch seconds |
| `cloud service query` | ClickHouse format; `--format` overrides agent auto-JSON and conflicts with explicit `--json` |
| `cloud postgres query --json` | JSON array lines: names, types, rows; success may be empty |
| `local client`, `local postgres client` | Native client output, even in JSON mode |
| `cloud service prometheus`, legacy `cloud org prometheus` | Raw Prometheus text, even in JSON mode |
| `cloud postgres prometheus service/org --json` | Prometheus text encoded as one JSON string |

Human detail views summarize PEM blocks; JSON returns their original contents. `postgres certs get` deliberately emits PEM. Treat JSON results and files as potentially sensitive.

Paginated commands require command-specific handling. `cloud org list`, `cloud service snapshot list` and `cloud clickpipe list` read every page themselves, as do organization-name, auto-detected organization and ClickPipe `--name` lookups. For keys, an absent `nextCursor` ends pagination; an empty token is still valid. Other endpoints may return `pagination.nextCursor`. Use the specific guide/help and preserve opaque cursors unchanged.

## Check status and stderr

| Exit code | Meaning |
| --- | --- |
| `0` | Success (which may mean an asynchronous request was accepted) |
| `1` | Runtime error |
| `2` | Usage error, including invalid command-line values such as an empty Cloud resource ID (often an unset shell variable); nothing is sent |
| `3` | An interactive prompt was cancelled (the `skills` agent or scope picker); Ctrl-C elsewhere ends the process by signal |
| `4` | Authentication required/rejected (HTTP 401, missing credentials), including OAuth-only writes |

An HTTP 403 is a runtime error (`1`, code `http_4xx`), not `4`: it refuses the request, for example a quota, a precondition such as ClickStack not being set up, an organization policy or a missing grant, and logging in again does not fix it. The exception is a credential check backed by the identity (whoami) endpoint: `cloud auth login`, `cloud auth whoami`, and the key bind in `cloud service query`. There a 403 rejects the credentials themselves and exits `4`.

Exit `2` also covers input that is checked after parsing but can never succeed, such as an unknown `--provider`, a malformed `--tag`, `--ip-allow` or `--role-id` entry, `--min-replicas` without `--max-replicas`, empty SQL, a `--file` body that is not valid JSON or has unknown fields, or `cloud auth login --interactive` without a TTY. A file that cannot be read is a runtime error (`1`, code `io`), and so is a check that depends on the resource's current state. When valid JSON does not fit the request, the error names the file, the JSON path of the offending value, such as `policies[0].permissions[0]`, and the kind of JSON value expected; only JSON syntax errors give a line and column.

Spawned native clients retain their own output and exit status. A successful wrapper handoff does not prove SQL succeeded.

Local and Cloud runtime errors in JSON mode write an envelope to stderr:

```json
{"error":{"code":"server_not_found","message":"The selected server was not found"}}
```

`error.code` and `error.message` are always present; `command`, `details`, and command-specific recovery fields are optional. `details` carries the underlying error text (from ClickHouse, a managed server's output, Docker, the HTTP client, the OS, or a parser) when `message` is a summary. Branch on the stable code, not English wording. Debug, progress, or cleanup diagnostics can precede the object, so stderr as a whole is not guaranteed to be one JSON document. Clap usage errors remain text and exit 2. `update` and `skills` failures use the same envelope with the generic codes `transport`, `timeout`, `io`, and `other` (`update` also reports `http_4xx`, `http_5xx`, or `rate_limited` when the release server answers with an error status, and `package_managed`, with the package manager's upgrade command in `command`, for an npm, pip, uv, or pipx install it will not replace, and `verification_failed` when the release's `SHA256SUMS` does not list the downloaded archive or lists a different digest); `telemetry` and `feedback` failures keep their text diagnostics.

Error rendering adds nothing to stdout, but a streaming query or a command that already committed a change may have emitted a result before failing. Do not accept partial output as success or assume nonzero exit means no side effects.

Cloud generic codes are `auth_required`, `cancelled`, `http_4xx`, `http_5xx`, `rate_limited`, `transport`, `timeout`, `sql_error`, `service_stopped`, `io`, and `other`, with specific recovery codes taking precedence. `resource_not_found` means the API answered 404 for an identifier, or a `--name`/`--email` matched nothing; `command` lists what does exist in that scope. For a resource inside a service, the CLI reads the service to decide which one is missing. A malformed identifier keeps the API's own diagnostic. ClickStack and Postgres subresources and service-wide singletons (such as the scaling schedule, upgrade window, and backup bucket or configuration) can still report `http_4xx`. Treat a not-found from a delete as "already gone" only when `command` lists the kind you meant to delete, not its parent. A delete the API rejects for a service that still exists, such as one still provisioning, reports `http_4xx`, never `resource_not_found`; for a ClickHouse service the message names its current state.

`cloud udf attach` reports the service's state when it cannot attach: `service_idle` (rerun with `--wake`, or wake it; `command` is the wake command), `service_stopped` (start it first; `command` is the start command), or `service_not_running` (starting, or another non-running state: wait for `running`, or start it). A `--wake` that finds the service stopped or failed while waiting reports `service_stopped` or `service_not_running`.

`cloud clickpipe start` exits 1 with `clickpipe_still_pausing` when the API answers with `Pausing`: a CDC pipe that is still pausing accepts the start but does not apply it. `command` is the `clickpipe get` to repeat until `state` is `Paused`; then run start again. It exits 1 with `clickpipe_start_not_applied` when the API answers with `Paused`: a pipe that has only just finished pausing drops the start too. `command` is the same start; wait a few seconds and rerun it until it returns `Running`. It exits 1 with `clickpipe_start_no_effect`, with no `command` and no `next`, when the API answers with `Completed`: start does not re-run a completed pipe, so there is nothing to retry. Only start is checked: a resync answered with `Paused` or `Completed` is applied.

When a local error's text comes from another program, `message` is clickhousectl's own summary and that text is in `details`: the server's startup output or log tail (`startup_exit`, `startup_timeout`), the Docker daemon's (`docker_error`), the HTTP client's, downloader's, or extractor's (`network_error`, `download_failed`), the OS's or serializer's (`io_error`, `sql_input_open_failed`, `sql_input_read_failed`), a failed subprocess's (`local_error`), and ClickHouse's, the HTTP client's, or the JSON parser's for the UDF codes `udf_definition_invalid` (invalid JSON), `udf_rejected`, `udf_reload_blocked`, `udf_query_failed`, and `udf_server_unreachable`. Human mode prints the same text. Telemetry never records it. Managed-client failures include `project_scope.path`, `server.selection`/`name`, and ordered `guidance`; project-local stop/remove errors can also carry scope and guidance instead of a top-level recovery command. `project_scope.path` is the project root the command acted on: the nearest directory holding `.clickhouse/` at or above the current directory (never the home directory for its subdirectories), else the current directory. Where `project_scope` has a `kind`, it is `project_root` when that `.clickhouse/` was found and `current_directory` when none was; only `current_directory` guidance suggests changing directory. New optional fields and codes may be added compatibly.

## Wait for completion and retry deliberately

| Operation | Completion check or retry constraint |
| --- | --- |
| ClickHouse create/start/stop | Poll `service get` for the intended state |
| ClickHouse delete | Poll `service get` until `state` is `terminated` or `softdeleted`, or it fails with `resource_not_found` |
| Postgres create/restore/replica | Poll `postgres get`; verify connectivity and credentials separately |
| Postgres resize (`update --size`) | Poll `postgres get` until `size` shows the requested value; this can take several minutes |
| Postgres HA change (`update --ha-type`) | No field signal: `haType` shows the new value as soon as the change is accepted, minutes before the standby exists; check `pg_stat_replication` on the primary |
| Postgres config/restart | Re-read configuration, compare `pg_postmaster_start_time()` before/after restart, then `SHOW` the changed setting |
| Postgres promote | Poll the replica until `isPrimary: true` |
| Postgres switchover | No completion signal: it may wait for the maintenance window, and service-level state, `isPrimary` and readiness do not confirm it |
| CDC stop/update/start | Wait for `Paused`, update, explicitly restart, then verify ingestion |
| CDC scaling (`clickpipe cdc-scaling update`) | Poll `cdc-scaling get` until `replicaCpuMillicores` shows the requested value; another update meanwhile fails with a conflict |
| Reverse private endpoint | Wait for `Ready`; resolve source-side acceptance if needed |
| Query API timeout | SQL may have completed; check the result before repeating a write |
| UDF creation/attachment | Wait for the version to be ready and inspect the service attachment; `udf deploy` waits for both |

The asynchronous commands add a `next` object to their `--json` output that names the poll. The CLI still returns as soon as the API accepts the request:

```json
"next": {
  "command": "clickhousectl cloud service get 1234-... --org-id 5678-... --json",
  "field": "state",
  "until": ["running"],
  "fail_on": ["failed"],
  "interval_seconds": 5
}
```

Run `command` as is: it carries the resolved IDs and `--org-id`, but not credentials. Stop when the top-level `field` of its output holds one of the `until` values. A value in `fail_on` means the operation will not finish. `until_error_code`, when present, is an error code from `command` that also means finished, such as `resource_not_found` once a deleted service is gone. `interval_seconds` is a suggested delay between polls.

| Command | Polls | `field` | `until` | `fail_on` |
| --- | --- | --- | --- | --- |
| `service create`, `service start`, `service wake` | `service get` | `state` | `running` | `failed` |
| `service stop` | `service get` | `state` | `stopped`, `idle` | `failed`, `terminated` |
| `service delete` | `service get` | `state` | `terminated`, `softdeleted` (or `resource_not_found`) | `failed` |
| `postgres create`, `postgres restore`, `postgres read-replica create` | `postgres get` on the new service | `state` | `running` | `deleting` |
| `postgres update --size` | `postgres get` | `size` | The requested value | None |
| `postgres promote` | `postgres get` | `isPrimary` | `true` | None |
| `clickpipe start`, `clickpipe resync` | `clickpipe get` | `state` | `Running`, `Completed` | `Failed`, `InternalError` |
| `clickpipe stop` | `clickpipe get` | `state` | `Stopped`, `Paused` | `Failed`, `InternalError` |
| `clickpipe cdc-scaling update` | `clickpipe cdc-scaling get` | `replicaCpuMillicores` | The requested value | None |
| `clickpipe reverse-private-endpoint create` | `clickpipe reverse-private-endpoint get` | `status` | `Ready` | `Failed`, `Rejected`, `Expired` |

Some commands have no `next`, because no single field confirms them. `postgres restart` can show `running` before the restart begins, so compare `pg_postmaster_start_time()` instead. Nothing that `postgres get` returns confirms a `postgres switchover`. A `postgres update --ha-type`, alone or with `--size`, has none because `haType` shows the new value on acceptance, minutes before the standby is replicating, so check `pg_stat_replication` on the primary instead. Any other `postgres update` applies at once. `next` is also omitted when the API response has no ID to poll. An endpoint in `PendingAcceptance` stays there until the source-side owner accepts it.

See the relevant [task guide](../README.md) for the full sequence. Resource creation, resync, password reset, and SQL writes are not safe generic retry targets. Reconcile the observed state before deciding what to repeat.

## Management commands

Successful JSON output from each management invocation is one object:

| Command | Fields |
| --- | --- |
| `skills` | `scope`, skill names, and per-agent paths plus created/updated/unchanged file counts |
| `telemetry status/enable/disable` | `action`, saved `preference`, effective `enabled`, `reason`, and available `config_path` |
| `update --check` | `current_version`, `latest_version`, `action` (`up_to_date` or `update_available`) |
| `update` | Version fields and `action` (`up_to_date` or `updated`); current version is the pre-replacement version |
| `feedback` | `sent` (always `true`) and `length`, the sent message's length in characters |

Unattended `skills` requires `--agent`, `--all`, or `--detected-only`; omission without a TTY exits 2. Scope defaults to the current project; `--global` uses the home directory. The common `.agents/skills/` path is always included. Telemetry's saved enabled preference can still be overridden by `DO_NOT_TRACK`.

## Update check

Every command except `update`, `--version` included, checks for a newer release when its cached result is stale, then prints a one-line notice to stderr after the command's output if one exists. The notice is suppressed in JSON mode (`--json` or a detected coding agent). The check reads `https://builds.clickhouse.com/clickhousectl/latest.txt`, falling back to the GitHub latest-release redirect, with a 400 ms budget, sending only the `User-Agent`, never the agent correlation headers. A completed check holds for 24 hours and a failed one is retried after an hour; the result is cached in `~/.clickhouse/last_update_check`. Set `CLICKHOUSECTL_NO_UPDATE_CHECK=1` (any value other than empty, `0`, or `false`) to turn off both the check and the notice. `update` and `update --check` always contact the release server.

## Local error codes

| Code | Meaning |
| ---- | ------- |
| `server_not_found` | The selected local server does not exist |
| `udf_definition_invalid` | `udf.json` is not valid JSON (message names the file, line and column; `details` has the parser's text) or fails validation (message carries the reason). Cloud `udf create` and `udf version create` report it too, without `details` |
| `udf_source_invalid` | The UDF directory does not exist (the message and `command` suggest `udf init`), is not a directory or is itself a symbolic link, has no `udf.json` or entrypoint, or contains a symbolic link; or the staged script path contains a single quote. Cloud `udf create` and `udf version create` report it for a missing `udf.json` or entrypoint or a symbolic link inside the directory; a missing or symlinked directory there is a usage error |
| `udf_runtime_unsupported` | Runtime `native` needs a Linux amd64/arm64 host |
| `udf_not_loaded` | The running server did not load the deployed function; see the log path in the message |
| `udf_rejected` | The running server rejected the function reload after a deploy, and the deployed function is broken or (before ClickHouse 26.2) could not be confirmed to run its new definition; the message and `command` name the broken function when identified, which may not be the one deployed; `details` has ClickHouse's error. Other functions still load, but every function reload fails until the broken one is fixed or removed |
| `udf_reload_blocked` | `udf reload` was rejected, or `udf remove` deleted the files but the reload still fails, because a deployed function is broken; `command` removes it when identified; `details` has ClickHouse's error |
| `udf_not_found` | No UDF of that name is deployed to the selected server |
| `udf_interpreter_not_found` | No `python3.11` or `python3` on `PATH` and no usable `--python`, or the interpreter path contains a single quote |
| `udf_query_failed` | The local server rejected a UDF statement; the server's text is in `details` |
| `udf_server_unreachable` | The local server's HTTP port did not answer; the client's text is in `details` |
| `managed_client_server_not_found` | Managed client lookup did not find the selected server in the current project |
| `managed_client_server_not_running` | The managed client server exists in the current project but is stopped |
| `managed_client_binary_not_found` | The client binary selected by managed server metadata is not installed |
| `managed_client_project_state_unavailable` | Managed client lookup could not read or lock current-project server state |
| `server_selection_required` | A server name (or, for `clickhouse stop --global`, a `--project`) is required because the selection is ambiguous or unsafe |
| `server_not_running` | The selected local server exists but is stopped |
| `server_running` | The operation requires a stopped server, or a running server is using the version |
| `invalid_server_name` | A stored server name contains path separators or `..`; a name given on the command line is rejected as a usage error, including a Postgres name with a character other than letters, digits, `_`, `.`, and `-` |
| `unsupported_argument` | An argument was rejected because it would break the managed server lifecycle (`--http-port`/`--tcp-port` `0`); a pass-through `--config` or `-C` is a usage error |
| `config_not_found` | The named `clickhouse start --config` file does not exist, or the name is ambiguous |
| `invalid_config_name` | The config name is a path rather than a file in the configs dir; `clickhouse start --config` rejects one as a usage error |
| `invalid_version` | The version selector is invalid |
| `version_not_installed` | The requested or configured version is not installed locally |
| `binary_not_launchable` | The version is installed but its binary cannot be launched (missing, not a regular file, or not executable) |
| `version_selection_required` | A version must be chosen because no default is set or the choice is ambiguous |
| `version_already_installed` | The requested version is already installed |
| `version_unavailable` | The requested version could not be resolved or downloaded |
| `version_is_default` | The version is the current default and `--force` was not passed |
| `unsupported_client_version` | The installed client does not support the requested operation |
| `unsupported_platform` | No ClickHouse build exists for this OS and architecture |
| `port_in_use` | A requested or stored port is occupied, Docker did not publish it, or no managed port is available |
| `startup_exit` | A managed server exited before it became ready |
| `startup_timeout` | A managed server did not become ready before its deadline |
| `download_failed` | An artifact or image download or extraction failed |
| `network_error` | An HTTP request failed |
| `docker_unavailable` | Docker could not be reached (the message names the cause and the platform fix) |
| `docker_error` | A Docker operation failed |
| `container_name_conflict` | The container name is held by a container clickhousectl does not manage |
| `postgres_error` | A Postgres validation or state error; the message carries its recovery guidance |
| `server_metadata_invalid` | A managed server metadata file contains invalid JSON; the path and conservative recovery guidance are included |
| `io_error` | A local filesystem, metadata, or serialization operation failed |
| `local_error` | A fallback for other failures, such as a subprocess that could not run; `details` has its text when there is any |
