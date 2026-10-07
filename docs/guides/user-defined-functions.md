# Upload and attach a user-defined function

[All documentation](../README.md)

`cloud udf` manages organization-scoped executable UDFs, versions, and service attachments. All UDF operations are beta. Reads support OAuth; writes require API key authentication.

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

## Create and attach

```bash
clickhousectl cloud udf create --file udf.json --artifact source.zip
clickhousectl cloud udf get my_udf
```

Wait for `status=ready` and ensure the target service is running (wake it if idle), then attach:

```bash
clickhousectl cloud udf attach my_udf <service-id>
clickhousectl cloud udf attachment get my_udf <service-id>
```

Attachment replaces the version already attached to that service. Without `--version`, it selects the latest ready version. A dependency error (HTTP 424) requires inspecting the UDF and service before retrying.

## Create a new version

`version.json` contains the complete desired definition without `functionName` or `uploadId`:

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
