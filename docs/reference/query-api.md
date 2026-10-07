# SQL and Query API behavior

[All documentation](../README.md)

`cloud service query` runs ClickHouse SQL over HTTP without a local binary or service password. For the separate OAuth-only Postgres route, see [ClickHouse Managed Postgres](../guides/managed-postgres.md#run-read-only-sql-with-oauth).

## Authentication and network access

OAuth queries run as your identity with read-only SQL access. They do not bind a key or need a configured service Query API endpoint.

With API-key authentication and no stored per-service key, the CLI uses the active API key. If needed, it adds that key to the service's Query API endpoint, creating an endpoint with `sql_console_admin` roles when none exists. Existing endpoint roles and allowed origins are preserved, and other live key bindings remain. No new key is created or saved locally. Use `--no-auto-enable` to fail instead of making this change.

Endpoint roles apply to every bound key. Inspect the endpoint's roles and use appropriate key permissions before allowing a first-use bind. The command's help lists the conditional permissions it requires. A key already listed in the endpoint is not rebound: the CLI waits for acceptance, then reports `query_key_bound_rejected` if still refused.

```bash
clickhousectl cloud service query-endpoint get <service-id>
clickhousectl cloud service query <service-id> --no-auto-enable --query "SELECT 1"
```

The Query API proxies requests from inside Cloud. The service's IP allowlist restricts direct connections, not that proxied route. API-key queries still enforce the key's own IP allowlist, endpoint binding, and database role; OAuth remains read-only.

`query-endpoint create` merges and deduplicates key bindings while preserving existing browser origins. Its required `--role` replaces the roles for **all** bound keys. `--replace-open-api-keys` deliberately replaces the complete key list. Avoid concurrent endpoint updates, repairs, or binds across projects: they can overwrite each other's changes. Removing a binding does not prevent the next query from restoring it unless `--no-auto-enable` is used.

## SQL input and output

Supply one UTF-8 statement through `--query`, `--queries-file PATH`, or stdin. The two flags conflict; `--queries-file -` reads stdin explicitly. The Query API rejects multi-statement scripts. Use the native client for scripts.

```bash
clickhousectl cloud service query <service-id> --queries-file query.sql
clickhousectl cloud service query <service-id> --query "SELECT 1" --format JSONEachRow
```

`--query` never reads stdin. Combining it with a separate CSV stream is refused. Put the INSERT statement and data in one stream instead, after creating the destination table:

```bash
printf 'INSERT INTO trips FORMAT CSV\n' | cat - data.csv | \
  clickhousectl cloud service query <service-id>
```

`--format` overrides automatic agent JSON but cannot be combined with explicit `--json`. Defaults are `PrettyCompact` on a terminal, `TabSeparated` when piped, and `JSONEachRow` under explicit or automatic JSON mode.

## Timeouts and long-running queries

The gateway can stop waiting after about 30 seconds. **A timeout does not prove that SQL failed or never ran.** The CLI does not retry timed-out SQL. Verify the result before resending an INSERT or another write; it may duplicate work. Check active queries through `system.processes`, but absence there alone does not prove a statement never executed.

An idle service can be woken in either auth mode. If the gateway requests wake confirmation, the CLI resends with it. A wake-related timeout still leaves SQL completion uncertain. Check the service state and the statement's result before retrying. A stopped service is not woken: start it with `cloud service start`.

Use a native client for bulk loads, binary formats such as `RowBinary`/`Native`, or long-running queries:

```bash
clickhousectl local use stable
clickhousectl cloud service get <service-id>
clickhouse client --host <nativesecure-host> --secure --port 9440 \
  --user default --password --queries-file backfill.sql
```

Use the `nativesecure` host and port from `get`; `--password` prompts for the database password. Use the password saved at creation, or deliberately reset it with `cloud service reset-password`. Direct connections must satisfy the service's IP allowlist.

In JSON mode a timeout reports `error.code: query_timeout` on stderr. Optional `command`, `host`, and `port` fields suggest recovery; SQL and password placeholders are never filled with your input. A possible wake delay instead suggests a service-state check. Always check the exit status even if stdout already contains rows.

## Stored keys and deliberate repair

Earlier CLI versions created per-service keys in `.clickhouse/credentials.json` under `service_query_keys`. Such a record still takes precedence over the active API key for that service. A rejected stored key is diagnosed without rotating, enabling, or rebinding it automatically.

| JSON error code | Next step |
| --- | --- |
| `query_key_deleted` | Review and deliberately replace the deleted key |
| `query_key_disabled` / `query_key_expired` | Check the administrator's intent before enabling or replacing it |
| `query_key_unbound` | Review the endpoint's access policy before replacement |
| `query_key_rejected` | Check the reported IP allowlist and stored secret |
| `query_key_unverified` | Restore management-key read access or resolve the failed lookup |

When replacement is intended:

```bash
clickhousectl cloud service repair-query-key <service-id>
```

This requires API-key authentication and a stored record with exact ownership metadata. It creates and saves a replacement, preserves other bindings and credentials, then retires the old key. Legacy or incomplete records are refused. Failed old-key deletions are reported under `pendingCleanupApiKeyIds` and retried by a later query; only exactly identified CLI-created keys are eligible for cleanup.

Repair reports `verification` as `verified`, `skipped`, or `failed`. A skipped probe or failure unrelated to readiness can exit 0 with a notice. If acceptance still fails after the readiness window (about two minutes), repair exits 1 with `query_key_repair_unverified`, **but the replacement remains in place**. Do not rerun repair just for slow propagation: check the key and endpoint, then try the query when safe. Do not delete the credentials file while cleanup IDs are still needed.

## Named SQL endpoints and saved queries

`cloud query-api-endpoint` manages named SQL endpoints, separate from the service-level binding above. A definition includes `name`, `sql`, `database`, `apiKeyIds`, and `roles`; `parameters` and `allowedOrigins` are optional. For example, save this as `endpoint.json` using an existing table and a management key ID from `cloud key list`:

```json
{
  "name": "Order count",
  "sql": "SELECT count() FROM orders WHERE status = {status:String}",
  "database": "default",
  "apiKeyIds": ["11111111-1111-4111-8111-111111111111"],
  "roles": ["sql_console_read_only"],
  "parameters": {"status": "paid"},
  "allowedOrigins": ["https://example.com"]
}
```

```bash
clickhousectl cloud query-api-endpoint create <service-id> --file endpoint.json
clickhousectl cloud query-api-endpoint get <service-id> <endpoint-id>
```

Update replaces the complete definition. Omitted parameters/origins become empty; GET output has response-only fields and cannot be resubmitted unchanged. User-owned endpoints can be read but not updated/deleted through this command. List returns one page; continue with `pagination.nextCursor` and `--cursor` (`--limit` is 1–100).

Call the returned `url` using the bound key's authentication `keyId` and `keySecret`, not its management ID. Supply every SQL placeholder explicitly: stored parameter defaults were not applied during live endpoint validation.

```bash
curl --user "$QUERY_KEY_ID:$QUERY_KEY_SECRET" "$ENDPOINT_URL" \
  --header 'Content-Type: application/json' \
  --data '{"queryVariables":{"status":"paid"},"format":"JSONEachRow"}'
```

`allowedOrigins` controls browser CORS, not access by authenticated non-browser clients. Empty means no cross-origin browser access; `["*"]` allows every origin.

Saved queries store SQL for a service:

```bash
clickhousectl cloud saved-query create <service-id> --name "Order count" \
  --sql 'SELECT count() FROM orders WHERE status = {status:String}' \
  --database default --param status=paid
clickhousectl cloud saved-query get <service-id> <query-id>
```

Saved-query update also replaces the complete definition; pass every field again, including parameters to retain. `--sql-file -` reads stdin. List is paginated with `nextCursor`. Named endpoints and saved queries are beta; reads support OAuth and writes require API keys.
