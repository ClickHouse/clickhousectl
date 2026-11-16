# SQL and Query API behavior

[All documentation](../README.md)

`cloud service query` runs ClickHouse SQL over HTTP without a local binary or service password. For the separate OAuth-only Postgres route, see [ClickHouse Managed Postgres](../guides/managed-postgres.md#run-read-only-sql-with-oauth).

## Authentication and network access

OAuth queries run as your identity with read-only SQL access. They do not bind a key or need a configured service Query API endpoint.

With API-key authentication, the CLI uses the active API key. If needed, it adds that key to the service's Query API endpoint, creating an endpoint with `sql_console_admin` roles when none exists. Existing endpoint roles and allowed origins are preserved, and other live key bindings remain. No new key is created or saved locally. Use `--no-auto-enable` to fail instead of making this change.

Binding is a write. A read-only API key cannot bind itself: the query exits 1 with `query_key_bind_forbidden`, whose `api_key_id` names the key, and nothing is changed. Bind the read-only key once with a key that has write access, adding `--allowed-origins '*'` when the service has no endpoint yet:

```bash
clickhousectl cloud service query-endpoint create <service-id> --role <role> --open-api-key <key-id>
```

Endpoint roles apply to every bound key. Inspect the endpoint's roles and use appropriate key permissions before allowing a first-use bind. The command's help lists the conditional permissions it requires. A key already listed in the endpoint is not rebound: the CLI waits for acceptance, then reports `query_key_bound_rejected` if still refused. When the endpoint refuses a statement with HTTP 403 but answers a `SELECT 1` probe with the same key, the endpoint role lacks a grant for that statement. The CLI reports the refusal as a query failure (exit 1) without binding or re-running the statement, with or without `--no-auto-enable`.

```bash
clickhousectl cloud service query-endpoint get <service-id>
clickhousectl cloud service query <service-id> --no-auto-enable --query "SELECT 1"
```

The Query API proxies requests from inside Cloud. The service's IP allowlist restricts direct connections, not that proxied route. API-key queries still enforce the key's own IP allowlist, endpoint binding, and database role; OAuth remains read-only.

`query-endpoint create` merges and deduplicates key bindings while preserving existing browser origins. Its required `--role` replaces the roles for **all** bound keys. `--replace-open-api-keys` deliberately replaces the complete key list. Avoid concurrent endpoint updates or binds across projects: they can overwrite each other's changes. Removing a binding does not prevent the next query from restoring it unless `--no-auto-enable` is used.

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
clickhousectl local use latest
clickhousectl cloud service client <service-id> -- --queries-file backfill.sql
```

`cloud service client` connects to the service's `nativesecure` endpoint and signs in through a browser as your Cloud user, so no service password is needed; it needs ClickHouse 25.12 or newer. See [Open an interactive client](../guides/cloud-service.md#open-an-interactive-client). Direct connections must satisfy the service's IP allowlist.

In JSON mode a timeout reports `error.code: query_timeout` on stderr. Optional `command`, `host`, and `port` fields suggest recovery; the SQL placeholder is never filled with your input. A possible wake delay instead suggests a service-state check. Always check the exit status even if stdout already contains rows.

## Keys stored by earlier versions

Earlier versions created a per-service key and stored it in `.clickhouse/credentials.json` under `service_query_keys`. The CLI no longer uses such a record; queries always bind the active API key. The first `cloud service query` or `cloud service delete` of that service removes the record and prints a notice to stderr that names the stored key's management ID. The key itself still exists in the organization, enabled and bound to the service's endpoint. Delete it when nothing else uses it:

```bash
clickhousectl cloud key list
clickhousectl cloud key delete <key-id>
```

Those keys are named `clickhousectl-query-<service-name>`. Once a key is deleted, the next first-use bind removes its stale binding from the endpoint.

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

Update replaces the complete definition. Omitted parameters/origins become empty; GET output has response-only fields and cannot be resubmitted unchanged. User-owned endpoints can be read but not updated/deleted through this command. List JSON contains `items` and `pagination`; continue with `pagination.nextCursor` and `--cursor` (`--limit` is 1–100). `pagination.currentCursor` reflects the supplied cursor; `pagination.totalRecords` reflects the API's optional total count.

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

Saved-query update also replaces the complete definition; pass every field again, including parameters to retain. `--sql-file -` reads stdin. List is paginated with `nextCursor`. Named endpoint and saved-query reads support OAuth; writes require API keys.
