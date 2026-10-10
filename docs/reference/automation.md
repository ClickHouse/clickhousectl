# Automation and output contracts

[All documentation](../README.md)

## Select output explicitly

Use `--json` for commands that return structured data. Detected coding agents select JSON automatically. Use explicit flags in scripts rather than depending on detection.

```bash
clickhousectl cloud service list --json
clickhousectl cloud key list --all --json
clickhousectl local server list --json
```

Cloud structured output follows each command's API-shaped contract, usually camelCase but sometimes snake_case. Missing/null optional response fields are generally omitted. Do not assume all lists are arrays, all fields have the same casing, or every timestamp has the same type.

| Command | Successful output |
| --- | --- |
| `cloud service list --json` | Bare array |
| `cloud key list --json` | Object with `result` and optional pagination metadata; `--all` gives a combined array |
| `cloud service settings list --json` | Object with `settings`; values retain string/number types |
| `cloud clickpipe settings get --json` | Object with snake_case setting names |
| `local server list --json` | Object with `servers`, `total_servers`, and `project_scope` |
| `local udf deploy --json` | Object with `name`, `server`, `type`, `runtime`, `reloaded` (the server was running, so functions were reloaded), `loaded` (`null` when the server is stopped), `interpreter`, `ignored_fields`, `ignored_files`, `warnings` (such as an interpreter that is not Python 3.11, or another deployed UDF that is broken), `function_config`, and `scripts_dir` |
| `local udf list --json` | Object with `server`, `server_running`, and a `udfs` array of `name`, `type`, `runtime`, `loaded` (`null` when the server is stopped), and `last_deploy_rejected` (the running server rejected the last deploy of these files; a loaded function runs an earlier definition) |
| `cloud postgres metrics --json` | API-shaped metrics; data-point timestamps are epoch seconds |
| `cloud service query` | ClickHouse format; `--format` overrides agent auto-JSON and conflicts with explicit `--json` |
| `cloud postgres query --json` | JSON array lines: names, types, rows; success may be empty |
| `local client`, `local postgres client` | Native client output, even in JSON mode |
| `cloud service prometheus`, legacy `cloud org prometheus` | Raw Prometheus text, even in JSON mode |
| `cloud postgres prometheus service/org --json` | Prometheus text encoded as one JSON string |

Human detail views summarize PEM blocks; JSON returns their original contents. `postgres certs get` deliberately emits PEM. Treat JSON results and files as potentially sensitive.

Paginated commands require command-specific handling. For keys, an absent `nextCursor` ends pagination; an empty token is still valid. Other endpoints may return `pagination.nextCursor`. Use the specific guide/help and preserve opaque cursors unchanged.

## Check status and stderr

| Exit code | Meaning |
| --- | --- |
| `0` | Success (which may mean an asynchronous request was accepted) |
| `1` | Runtime error |
| `2` | Usage error, including invalid command-line values |
| `3` | Cancellation |
| `4` | Authentication required/rejected, including OAuth-only writes |

Spawned native clients retain their own output and exit status. A successful wrapper handoff does not prove SQL succeeded.

Local and Cloud runtime errors in JSON mode write an envelope to stderr:

```json
{"error":{"code":"server_not_found","message":"The selected server was not found"}}
```

`error.code` and `error.message` are always present; `command`, `details`, and command-specific recovery fields are optional. `details` carries the underlying ClickHouse or parser error text when `message` is a summary. Branch on the stable code, not English wording. Debug, progress, or cleanup diagnostics can precede the object, so stderr as a whole is not guaranteed to be one JSON document. Clap usage errors remain text and exit 2. Management-command failures (`skills`, `telemetry`, `update`) keep their existing diagnostics rather than sharing this runtime envelope.

Error rendering adds nothing to stdout, but a streaming query or a command that already committed a change may have emitted a result before failing. Do not accept partial output as success or assume nonzero exit means no side effects.

Cloud generic codes are `auth_required`, `cancelled`, `http_4xx`, `http_5xx`, `rate_limited`, `transport`, `timeout`, `sql_error`, `service_stopped`, `io`, and `other`, with specific recovery codes taking precedence. `resource_not_found` applies to supported service/Postgres/organization lookups, not every resource: check the organization scope and use the relevant list command. A malformed identifier or a missing ClickPipe/key can carry the API's own diagnostic instead.

Local errors redact external logs, subprocess output, and OS details where needed. The UDF codes `udf_definition_invalid` (invalid JSON), `udf_rejected`, `udf_reload_blocked`, `udf_query_failed`, and `udf_server_unreachable` keep ClickHouse's, the HTTP client's, or the JSON parser's text out of `message` and return it in `details`. Managed-client failures include `project_scope.path`, `server.selection`/`name`, and ordered `guidance`; project-local stop/remove errors can also carry scope and guidance instead of a top-level recovery command. These identify the exact directory inspected, without searching parent projects. New optional fields and codes may be added compatibly.

## Wait for completion and retry deliberately

| Operation | Completion check or retry constraint |
| --- | --- |
| ClickHouse create/start/stop | Poll `service get` for the intended state |
| Postgres create/restore/replica | Poll `postgres get`; verify connectivity and credentials separately |
| Postgres config/restart | Re-read configuration, compare `pg_postmaster_start_time()` before/after restart, then `SHOW` the changed setting |
| Postgres promote | Poll the replica until `isPrimary: true` |
| Postgres HA switchover | Service-level state and readiness do not prove the internal node swap |
| CDC stop/update/start | Wait for `Paused`, update, explicitly restart, then verify ingestion |
| Reverse private endpoint | Wait for `Ready`; resolve source-side acceptance if needed |
| Query API timeout | SQL may have completed; check the result before repeating a write |
| Query-key repair | Replacement can persist despite failed verification; do not blindly rotate again |
| UDF creation/attachment | Wait for the version to be ready and inspect the service attachment |

See the relevant [task guide](../README.md) for the full sequence. Resource creation, resync, password reset, and SQL writes are not safe generic retry targets. Reconcile the observed state before deciding what to repeat.

## Management commands

Successful JSON output from each management invocation is one object:

| Command | Fields |
| --- | --- |
| `skills` | `scope`, skill names, and per-agent paths plus created/updated/unchanged file counts |
| `telemetry status/enable/disable` | `action`, saved `preference`, effective `enabled`, `reason`, and available `config_path` |
| `update --check` | `current_version`, `latest_version`, `action` (`up_to_date` or `update_available`) |
| `update` | Version fields and `action` (`up_to_date` or `updated`); current version is the pre-replacement version |

Unattended `skills` requires `--agent`, `--all`, or `--detected-only`; omission without a TTY exits 2. Scope defaults to the current project; `--global` uses the home directory. The common `.agents/skills/` path is always included. Telemetry's saved enabled preference can still be overridden by `DO_NOT_TRACK`.

## Local error codes

| Code | Meaning |
| ---- | ------- |
| `server_not_found` | The selected local server does not exist, or `--server` names a local Postgres instance |
| `udf_definition_invalid` | `udf.json` is not valid JSON (message names the file, line and column; `details` has the parser's text) or fails validation (message carries the reason) |
| `udf_source_invalid` | The UDF directory does not exist (the message and `command` suggest `udf init`), is not a directory or is itself a symbolic link, has no `udf.json` or entrypoint, or contains a symbolic link; or the staged script path contains a single quote |
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
| `server_selection_required` | A server name (or, for `server stop --global`, a `--project`) is required because the selection is ambiguous or unsafe |
| `server_not_running` | The selected local server exists but is stopped |
| `server_running` | The operation requires a stopped server, or a running server is using the version |
| `invalid_server_name` | The server name contains path separators or `..` |
| `unsupported_argument` | An argument was rejected because it would break the managed server lifecycle (a pass-through `--config`, or `--http-port`/`--tcp-port` `0`) |
| `config_not_found` | The named `server start --config` file does not exist, or the name is ambiguous |
| `invalid_config_name` | The config name is a path rather than a file in the configs dir |
| `invalid_version` | The version selector is invalid |
| `version_not_installed` | The requested or configured version is not installed locally |
| `binary_not_launchable` | The version is installed but its binary cannot be launched (missing, not a regular file, or not executable) |
| `version_selection_required` | A version must be chosen because no default is set or the choice is ambiguous |
| `version_already_installed` | The requested version is already installed |
| `version_unavailable` | The requested version could not be resolved or downloaded |
| `version_is_default` | The version is the current default and `--force` was not passed |
| `unsupported_client_version` | The installed client does not support the requested operation |
| `unsupported_platform` | No ClickHouse build exists for this OS and architecture |
| `port_in_use` | A requested port is occupied or no managed port is available |
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
| `local_error` | A redacted fallback for failures whose text cannot be rendered safely |
