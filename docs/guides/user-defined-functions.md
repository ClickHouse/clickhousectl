# Develop and deploy a user-defined function

[All documentation](../README.md)

`local udf` deploys [executable UDFs](https://clickhouse.com/docs/sql-reference/functions/udf#executable-user-defined-functions) to a local server, and `cloud udf` manages organization-scoped executable UDFs, versions, and service attachments in Cloud. Both read the same source directory: a `udf.json` definition in the Cloud API's field names next to the function's files. Cloud UDF operations are beta. Cloud reads support OAuth; Cloud writes require API key authentication.

The examples below upload a definition file and a ZIP you built. To upload a scaffolded directory instead, see [Deploy a source directory](#deploy-a-source-directory).

Save the definition below as `udf.json` and prepare a [source ZIP archive](https://clickhouse.com/docs/products/cloud/features/sql-console-features/user-defined-functions#manage-udfs-with-the-cloud-api). `--file` accepts a file or `-` for stdin. The definition uses the API's field names and excludes `uploadId`, which the CLI obtains from a fresh upload session:

```json
{
  "functionName": "my_udf",
  "type": "executable",
  "runtime": "native",
  "arguments": [{"name": "x", "type": "UInt64"}],
  "returnType": "UInt64",
  "memoryLimitMib": 128,
  "deterministic": false
}
```

## Scaffold a function

`local udf init` creates `clickhouse/udfs/<name>/` with a `udf.json` in the shape `--file` accepts, plus the function's sources:

```bash
clickhousectl local udf init my_udf                    # udf.json and an executable main.py
clickhousectl local udf init my_udf --runtime native   # udf.json, amd64/ and arm64/
```

For runtime `native`, build a Linux `main` binary into each architecture directory. On a host other than Linux amd64/arm64, `init` warns that the UDF can be deployed to Cloud but not to a local server. Pass `--type executable_pool` for a pooled function. Re-running keeps existing files and reports only the ones it created.

## Test on a local server

```bash
clickhousectl local server start
clickhousectl local udf deploy my_udf
clickhousectl local client --query "SELECT my_udf(1)"
clickhousectl local udf list
clickhousectl local udf remove my_udf
```

Every command except `init` takes `--server NAME` (default `default`). The server must already exist; `deploy`, `list` and `remove` work whether or not it is running, and `reload` needs it running. Like every local command, `local udf` looks for servers only in the current directory's project and never in parent directories, so run it from the project root. `deploy NAME` reads `clickhouse/udfs/NAME/` (`--dir PATH` selects another parent directory), and `functionName` must equal `NAME`. Local names are at most 235 characters, so every file named after the function fits the file-name limit. Redeploying replaces the function's files and definition.

`deploy` validates `udf.json` exactly as `cloud udf create --file` does and rejects symbolic links, as Cloud does. Runtime `python3.11` needs `main.py` at the root; runtime `native` runs only on Linux amd64/arm64 hosts and needs only the host's binary (`amd64/main` or `arm64/main`); the other architecture's directory is reported in `ignored_files` and not copied. `--python` with a native UDF is a usage error. Files are written under `.clickhouse/servers/<name>/data/`:

| Path | Contents |
| --- | --- |
| `config.d/chctl-udf.xml` | Managed overlay that points `user_defined_executable_functions_config` and `user_scripts_path` at the directories below. Rewritten on every `server start`; these two settings win over a `--config` overlay. |
| `user_defined_functions/<name>_function.xml` | The rendered `<function>` block, plus the `<name>.json` copy of the definition that `list` reads |
| `user_scripts/<name>/` | The source directory without `udf.json`, hidden entries and `__pycache__` (for `native`, only the host's `amd64/` or `arm64/`) |

On a running server, `deploy` first refuses (exit 2, nothing written) a name the server already uses for a built-in function, an alias, or a SQL function created with `CREATE FUNCTION`; built-in names that are case-insensitive clash in any case, so `LOWER` is refused like `lower`. A stopped server cannot be checked. `deploy` then reloads functions and confirms the function appears in `system.functions`. If ClickHouse does not load it, `deploy` exits 1 with `udf_not_loaded` and the path of the server log that records why. If ClickHouse rejects the reload because the function is broken, `deploy` exits 1 with `udf_rejected`. Both keep the deployed files. ClickHouse loads each function file on its own, so other functions still load, but `SYSTEM RELOAD FUNCTIONS` (sent by `deploy`, `reload` and `remove`) keeps failing on that server until you fix and redeploy the broken function or remove it. When the deployed function loads but another deployed UDF is broken, `deploy` succeeds and adds a warning naming the broken one. The broken function is named from ClickHouse's per-function load status (26.2 and later); older servers name a staged function they do not list, or one a name clash names. Older servers cannot show whether a function that was already loaded now runs its new definition, so such a redeploy exits 1 with `udf_rejected` naming the broken UDF; fix or remove it and redeploy. After a rejected redeploy the earlier definition stays loaded: `list` shows `yes (stale: last deploy rejected)` (JSON `last_deploy_rejected: true`) until a deploy, `reload` or `remove` reloads successfully. A stopped server loads the files on its next start. ClickHouse also rescans the function directory every few seconds, so `reload` is rarely needed outside scripts; a rejected `reload` exits 1 with `udf_reload_blocked`, naming the broken function when ClickHouse identifies it.

Runtime `python3.11` runs `main.py` through an absolute interpreter path, so no shebang or execute bit is needed. The interpreter is `--python PATH`, else `python3.11`, else `python3` on `PATH`, resolved to an absolute path at deploy time: redeploy after moving the project or the interpreter. Cloud runs Python 3.11, so `deploy` warns when the interpreter reports another version or none. `requirements.txt` is copied but not installed; install its packages into the interpreter you deploy with, such as a virtualenv passed with `--python`.

The definition maps to ClickHouse's `<function>` XML with the same units as Cloud:

| `udf.json` | `<function>` element |
| --- | --- |
| `functionName`, `type`, `arguments[].name/type`, `returnType`, `returnName` | `name`, `type`, `argument/name`, `argument/type`, `return_type`, `return_name` |
| `format` | `format` (`TabSeparated` when omitted) |
| `commandReadTimeout`, `commandWriteTimeout` (ms) | `command_read_timeout`, `command_write_timeout` |
| `poolSize`, `maxCommandExecutionTime` (s) | `pool_size`, `max_command_execution_time` (`executable_pool` only; on `executable` they are reported in `ignored_fields`) |
| `sendChunkHeader`, `deterministic` | `send_chunk_header`, `deterministic` |
| `runtime: python3.11` | `execute_direct` `0`; `command` is `exec` with the interpreter and `main.py` paths, so each worker is one Python process |
| `runtime: native` | `execute_direct` `1`; `command` is `<name>/<arch>/main` for the host CPU |
| `memoryLimitMib`, `sandboxType`, `sandboxVersion` | No local equivalent; accepted and reported as ignored |

## Deploy to Cloud in one step

`cloud udf deploy` uploads the same directory `local udf deploy` tested and waits until the function is usable:

```bash
clickhousectl cloud udf deploy my_udf --service <service-id>
clickhousectl cloud service query <service-id> -q "SELECT my_udf(1)"
```

It creates the UDF, or adds a version when the name already exists, waits for that build, attaches exactly that version, and waits until the attachment is `deployed`. Omitted fields follow the create defaults, never the previous version's values. An idle service is woken unless `--no-wake`; a stopped service is an error. `--timeout` bounds each wait, and a timeout exits 1 with the `timeout` error code and the command to keep polling. If attaching fails after the build, the error names the built version so you can resume with `cloud udf attach <name> <service-id> --version <n>`. JSON output is one object with `action` (`created` or `version_created`), `udf`, and `attachment`. It is the only `cloud udf` command that waits for the build and the deployment; the steps below run each part separately.

## Create and attach in Cloud

```bash
clickhousectl cloud udf create --file udf.json --artifact source.zip
clickhousectl cloud udf get my_udf
```

`create`, `version create` and `attach` return as soon as the API accepts them. Poll `cloud udf get` until `status` is `ready` (or `error`), then attach:

```bash
clickhousectl cloud udf attach my_udf <service-id> --wake
clickhousectl cloud udf attachment get my_udf <service-id>
```

Poll `attachment get` until it is `deployed`. The target service must be running. `--wake` wakes an idle service, waits up to ten minutes for it to reach `running`, then attaches once; without it, attaching to an idle service fails with the service state and the `cloud service wake` command. A stopped service must be started first. With `--json`, the error code is `service_idle`, `service_stopped` or `service_not_running`.

Attachment replaces the version already attached to that service. Without `--version`, it selects the latest ready version. Any other dependency error (HTTP 424) requires inspecting the UDF and service before retrying.

## Deploy a source directory

Pass a name instead of `--file` and `--artifact` to read `clickhouse/udfs/<name>/udf.json` and archive the rest of that directory, the same one `local udf deploy` uses:

```bash
clickhousectl cloud udf create my_udf
# After editing udf.json or the code
clickhousectl cloud udf version create my_udf
```

`--dir PATH` selects another parent directory, as for `deploy`. `--file` and `--artifact` go together and combine with neither `--dir` nor, for `create`, a name. The directory must not be a symbolic link and its `udf.json` must name the function; `version create` drops `functionName` from the request. The archive is deterministic and excludes `udf.json`, hidden entries and `__pycache__`; symbolic links inside are rejected. Runtime `python3.11` needs `main.py` at the root. Runtime `native` uploads only `amd64/main` and `arm64/main`, Linux binaries you build (see the [Cloud UDF docs](https://clickhouse.com/docs/products/cloud/features/sql-console-features/user-defined-functions)).

## Create a new version

`version.json` contains the complete desired definition without `uploadId`. It may keep a `functionName` equal to the command's name, so a `udf.json` can be reused; the CLI drops it from the request:

```bash
clickhousectl cloud udf version create my_udf --file version.json --artifact source-v2.zip
clickhousectl cloud udf version list my_udf
# After the returned version is ready, attach it; 2 is an example version number.
clickhousectl cloud udf attach my_udf <service-id> --version 2
```

Required fields are `type`, `runtime`, `arguments`, and `returnType`; initial creation also needs `functionName`. Types are `executable` and `executable_pool`, with runtimes `native` and `python3.11`. Read/write timeouts are milliseconds; maximum execution time is seconds. Use a complete definition for each new version and set `deterministic` only when identical arguments always produce identical results.

Version creation uses defaults for omitted options, without inheriting the previous version's configuration. Supply a complete request definition; GET output includes response-only fields and cannot be used directly as a request. Unknown fields, unsupported enum values, missing required fields and invalid limits fail before upload. Nullable options may be omitted or set to null; both use the API's default behavior.

Creation and version creation obtain a fresh upload session and stream the archive before submitting it. A failed upload never submits creation; uploads time out after five minutes, and retrying the upload requires a fresh command invocation. If failure occurs after submission, inspect the UDF/version before retrying so you do not create an unintended extra version.

## Detach and remove

Detach a version from every service before deleting it individually. The latest version and a version still building cannot be deleted individually.

```bash
clickhousectl cloud udf attachment list my_udf
clickhousectl cloud udf detach my_udf <service-id>
clickhousectl cloud udf version delete my_udf 1
```

**Deleting the whole UDF deletes every version and detaches it from all services.** Wait for all versions to finish building first; removal from services completes asynchronously.

```bash
clickhousectl cloud udf delete my_udf
```

All three list commands expose `--cursor` and `--limit` (1–100). JSON output retains the API's pagination object unchanged; human output summarizes the total record count, page limit, and available cursors. Detail and list output tolerate missing fields and new response status values.
