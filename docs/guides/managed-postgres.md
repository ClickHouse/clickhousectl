# Operate ClickHouse Managed Postgres

[All documentation](../README.md)

These beta commands manage Postgres in ClickHouse Cloud. Use [an API key](../reference/authentication.md) for changes; read-only SQL has its own OAuth-only route.

## Create and connect with verified TLS

```bash
clickhousectl cloud postgres create --name app-db \
  --provider aws --region us-east-1 --size c6gd.xlarge --pg-version 18
clickhousectl cloud postgres get <postgres-id>
clickhousectl cloud postgres certs get <postgres-id> --output ca.pem
```

Save the initial password and any connection string: later `get` responses do not return credentials. If creation omits both, use `postgres reset-password <postgres-id> --generate`. Creation returns before provisioning completes; repeat `get` until `state=running`.

Use the host and username from `get`, the saved CA, and a password prompt:

```bash
psql "host=<host> port=5432 dbname=postgres user=<username> sslmode=verify-full sslrootcert=ca.pem" \
  --command "SELECT version()"
```

Use `sslmode=verify-full` for certificate and hostname verification. If you embed credentials in an application URI, percent-encode the username/password as URI components; shell quoting is insufficient.

GCP is available in private preview: use `--provider gcp` with GCP region and instance-size names. Inspect `postgres create --help` for the available options.

## Restrict network access

**Omitting access flags on create allows all IPv4 and IPv6 addresses on Postgres (5432) and PgBouncer (6432).** To restrict it, pass one `--ip-allow IP_OR_CIDR:PORTS[=DESCRIPTION]` per IPv4 rule. `PORTS` is a port or an inclusive range such as `5432..6432`, with no default. Once you pass a rule, only the listed rules apply, so add one for each port you use. `--block-public-access` creates the service with no rules, which blocks public access:

```bash
clickhousectl cloud postgres create --name app-db \
  --provider aws --region us-east-1 --size c6gd.xlarge \
  --ip-allow '203.0.113.0/24:5432=office' --ip-allow '203.0.113.0/24:6432=office'
```

`get` lists the IPv4 rules with their IDs; IPv6 rules are not shown. Change them with `update`: `--add-ip-allow` creates a rule and `--remove-ip-allow <rule-id>` removes one. Removals apply before additions:

```bash
clickhousectl cloud postgres get <postgres-id>
clickhousectl cloud postgres update <postgres-id> \
  --remove-ip-allow <rule-id> --add-ip-allow '198.51.100.7:5432..6432'
```

A rule update can partially succeed. After a failure, run `get` and compare the rules before retrying, so a retry does not repeat a change that already applied.

## Run read-only SQL with OAuth

This route needs no database password or local `psql`, but accepts **only OAuth**. Remove higher-priority key flags, project credentials, environment variables, and `.env` key values before logging in. Clearing the credentials file does not delete keys in the organization; see [authentication](../reference/authentication.md).

```bash
clickhousectl cloud auth login
clickhousectl cloud auth status
clickhousectl cloud postgres query <postgres-id> --query "SELECT version()"
```

Supply `--query`, `--queries-file PATH`, or stdin (`--queries-file -` is explicit stdin). Input must be nonempty UTF-8. `--database` defaults to the server's `postgres` database. Scripts return only the final statement's result. Queries are sent once without automatic retries. Use `psql` for writes and interactive sessions.

Human output is tab-separated with a header. JSON mode streams an array of column names, an array of types, then an array per row. It is a sequence of JSON values, not one document. Success can be empty; failures can leave partial rows. Check the exit status before consuming the result.

## Change configuration and verify it took effect

With API-key authentication, inspect the current configuration, make a targeted change, and read it back:

```bash
clickhousectl cloud postgres config get <postgres-id> --json
clickhousectl cloud postgres config patch <postgres-id> --set log_min_duration_statement=1000
clickhousectl cloud postgres config get <postgres-id> --json
```

Exit 0 means the API accepted the change, not that it is active. If the response requires a restart, record `SELECT pg_postmaster_start_time()` through `psql`, request the restart, and wait for connections to recover:

```bash
clickhousectl cloud postgres restart <postgres-id>
```

Run `SELECT pg_postmaster_start_time()` again: only a later timestamp confirms a restart after your baseline. Readiness or `state=running` alone does not. Use `SHOW log_min_duration_statement` to confirm the setting is active.

`config replace --file` replaces the whole configuration. Start from `config get --json` and retain both sections and all settings you want to keep. `config patch` and `config replace` read the document from stdin with `--file -`. Files for both patch and replace must contain `pgConfig` and `pgBouncerConfig`, even if one is `{}`. PgBouncer values are strings, including numeric values:

```json
{"pgConfig":{},"pgBouncerConfig":{"default_pool_size":"16"}}
```

`pgConfig` uses supported Cloud GUC names; unknown names, null values, and values outside a setting's documented set (for example `io_method` or `wal_compression`) are rejected before any request. On create, restore, or read-replica creation, `--pg-bouncer-config-file` takes just the string-valued PgBouncer map, without the two-section wrapper.

`update --size` and `--ha-type` return once the API accepts the change, which then applies in the background and can take several minutes. After a resize, the printed service still shows the old size; poll `get` until `size` shows the new one (in JSON mode, `next` names the poll). After an HA change, `haType` shows the new value at once while the standby is still being added or removed, so `get` cannot confirm it; check `pg_stat_replication` on the primary through `psql`, where an added standby appears once it is replicating.

Tag updates fetch and replace the complete tag snapshot. Omitted flags preserve it; `--clear-tags` empties it. Avoid concurrent tag edits because one snapshot can overwrite another.

## Monitor and recover

```bash
clickhousectl cloud postgres logs <postgres-id> \
  --from-date <RFC3339-start> --to-date <RFC3339-end> --limit 100
clickhousectl cloud postgres metrics <postgres-id> \
  --from-date <RFC3339-start> --to-date <RFC3339-end>
clickhousectl cloud postgres backup list <postgres-id>
```

`logs`, `metrics`, `slow-queries list` and `backup list` also take `--name <postgres-name>` in place of the service ID. Logs accept an inclusive window of at most 30 days; use limit/offset pagination. `--severity` matches one level exactly: `DEBUG`, `LOG`, `INFO`, `NOTICE`, `WARNING`, `ERROR`, `FATAL`, or `PANIC`, in any case. Metrics and `slow-queries` require ordered RFC 3339 bounds with at most millisecond precision; `slow-queries list` treats the end bound as inclusive at minute granularity, and `slow-queries get --timestamp` is currently ignored by the API (results cover the most recent 24 hours). `postgres prometheus service/org` emits Prometheus text, encoded as one JSON string in JSON mode.

Backup listing shows completed base backups and their status, with cursor pagination. Restore targets a point in time within retention, not a backup key. `get` shows the earliest restorable time once a primary's first backup has completed:

```bash
clickhousectl cloud postgres restore <postgres-id> \
  --name restored-app --restore-target <RFC3339-time>
clickhousectl cloud postgres get <restored-postgres-id>
```

**The restore endpoint gives every restored service allow-all IPv4 rules on 5432 and 6432**, so `restore` then replaces them with the source's current IPv4 rules (not the rules at the restore target). Pass `--ip-allow` rules or `--block-public-access` to set the restored rules instead, as on create. If the source's rules cannot be read completely, nothing is restored. If they cannot be applied after the restore, the command fails and names the restored service, which still exists with the allow-all rules: remove them with `update --remove-ip-allow`. The API does not expose IPv6 rules, so they are neither copied nor removed; check them in the Cloud console.

Restore and read-replica creation return a new service ID without a password. A restored service starts with the source's password as it was when the restore was requested, not the password at the restore target. A read replica uses the source's password and follows later resets. Run `reset-password` on the primary; a read replica rejects it with a 400.

`promote` and `switchover` also return after acceptance. Promotion makes a read replica an independent primary; confirm `isPrimary: true` on that replica, while the source stays primary. Switchover replaces the primary's server: it moves onto the standby if one exists, otherwise onto a newly provisioned server, so it also works on a service without HA. The replacement catches up, a planned failover moves the service to it, and the old server is removed; this may wait for the maintenance window, and clients reconnect at failover. Service-level state, `isPrimary`, and readiness do not show its progress, so the CLI cannot confirm that a switchover completed.

## Delete a service

**Deletion permanently removes the service and its data, including when running. No stop is required.** Verify the service ID before proceeding:

```bash
clickhousectl cloud postgres delete <postgres-id>
```

If your organization has deletion protection enabled, `update --deletion-protection true` makes the API reject `delete` for that service until you turn it off with `--deletion-protection false`. `get` and `list` show the current setting:

```bash
clickhousectl cloud postgres update <postgres-id> --deletion-protection true
```
