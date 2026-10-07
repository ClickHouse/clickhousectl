# Ingest and verify data with ClickPipes

[All documentation](../README.md)

ClickPipes supports object storage, Kafka, Kinesis, Pub/Sub, Postgres, MySQL, MongoDB, and BigQuery. These examples walk through object storage and Postgres CDC. Use each source's `create --help` for its flags and [configuration reference](../reference/clickpipe-configuration.md) for update, mapping, and authentication rules.

## Prepare the destination

Use [API-key authentication](../reference/authentication.md) for creation and schema discovery. Select a ClickHouse Cloud service and wait for it to be running:

```bash
clickhousectl cloud service list
clickhousectl cloud service get <service-id>
clickhousectl cloud clickpipe context get <service-id>
```

The context reports source capabilities and, where enabled, the GCP workload-identity principal. The CLI configures ingestion but does not grant source IAM permissions or make your source reachable. Set those up first.

## Discover and load object storage

This example uses a private S3 bucket with an IAM role. Replace the URL and role with your source, and make sure the role can read the objects. Keep source secrets out of committed scripts; command-line credential values can be visible in process listings even when expanded from environment variables.

```bash
clickhousectl cloud clickpipe schema-discover object-storage <service-id> \
  --storage-type s3 --source-url 'https://example-bucket.s3.amazonaws.com/events/**' \
  --format JSONEachRow --iam-role "$S3_IAM_ROLE_ARN"
```

Discovery returns inferred fields without creating a pipe. Review them, then choose destination columns. Object storage requires at least one `--column name:type`; this example assumes the source has `event_id` and `event_type` fields:

```bash
clickhousectl cloud clickpipe create object-storage <service-id> \
  --name events-snapshot --storage-type s3 \
  --source-url 'https://example-bucket.s3.amazonaws.com/events/**' \
  --format JSONEachRow --iam-role "$S3_IAM_ROLE_ARN" \
  --database default --table events \
  --column 'event_id:Int64' --column 'event_type:String' --start-paused
clickhousectl cloud clickpipe get <service-id> <clickpipe-id>
clickhousectl cloud clickpipe start <service-id> <clickpipe-id>
```

Use the ID returned by create, or find it with `clickpipe list <service-id>`. `--start-paused` creates this pipe in `Stopped` state for review before ingestion; omission starts ingestion immediately. That flag is supported for object storage, Kafka, Kinesis, and Pub/Sub, not database sources. For continuous S3 ingestion, configure the SQS queue options shown in `create object-storage --help`.

Inspect the pipe and query the destination to confirm data arrived:

```bash
clickhousectl cloud clickpipe get <service-id> <clickpipe-id>
clickhousectl cloud service query <service-id> \
  --query "SELECT count() FROM default.events"
```

An accepted create/start request is not proof ingestion finished. Verify expected rows as well as the pipe's state. SQL follows the [Query API authentication and timeout rules](../reference/query-api.md).

## Replicate Postgres changes

Before creating a PostgreSQL CDC pipe:

- Make the source reachable from ClickHouse Cloud using [ClickPipes egress IPs](https://clickhouse.com/docs/integrations/clickpipes/networking/static-ips) or supported private connectivity.
- Enable logical replication (`wal_level=logical`) and provision enough WAL senders and replication slots.
- Create a publication containing every mapped table. Each table needs a primary key or suitable replica identity.
- Grant the source user connection access, schema `USAGE`, table `SELECT`, and `REPLICATION`.

Follow the [PostgreSQL source setup guide](https://clickhouse.com/docs/integrations/clickpipes/postgres/source/generic) for your source. TLS and certificate verification are enabled by default. For ClickHouse Managed Postgres, export its CA first:

```bash
clickhousectl cloud postgres certs get <postgres-id> --output postgres-ca.pem
```

Then create the pipe using an existing publication and table:

```bash
clickhousectl cloud clickpipe create postgres <service-id> \
  --name app-cdc --host <postgres-host> --pg-database postgres \
  --username "$POSTGRES_USERNAME" --password "$POSTGRES_PASSWORD" \
  --ca-certificate postgres-ca.pem --publication-name clickpipes \
  --destination-database default --table-mapping public.orders:orders
clickhousectl cloud clickpipe get <service-id> <clickpipe-id>
```

For a publicly trusted source certificate, omit `--ca-certificate`. Use `--tls-host` only if the certificate names a different host. `--skip-cert-verification` accepts untrusted/mismatched certificates and is for diagnosis; `--disable-tls` sends unencrypted traffic and conflicts with certificate/TLS flags.

Simple Postgres mappings use `ReplacingMergeTree`. For a current-state view, deduplicate versions and exclude deletion markers:

```bash
clickhousectl cloud service query <service-id> \
  --query "SELECT count() FROM default.orders FINAL WHERE _peerdb_is_deleted = 0"
```

Compare expected source data after the snapshot and ongoing replication have caught up. Raw row counts include versions and deletion markers. See [mapping choices](../reference/clickpipe-configuration.md#choose-table-mappings) before changing engine, sorting, or partition settings.

## Monitor and diagnose

```bash
clickhousectl cloud service prometheus <service-id> --filtered-metrics true | \
  grep 'clickpipe_id="<clickpipe-id>"'
```

Metrics are included in the destination service's raw Prometheus output. Available metrics vary by source; see [ClickPipes monitoring](https://clickhouse.com/docs/integrations/clickpipes/monitoring).

Streaming and object-storage operational errors appear in `system.clickpipes_log` (seven-day retention). Query a bounded recent window; malformed-record/schema errors go to `<destination_table_name>_clickpipes_error`. Postgres, MySQL, and MongoDB operational/error logs are viewed in the Cloud console instead. See [error reporting](https://clickhouse.com/docs/integrations/clickpipes/home#error-reporting).

## Pause, update, and resume CDC

Postgres, MySQL, and MongoDB CDC pipes must be `Paused` before an update. Save a minimal patch such as `{"name":"app-cdc-v2"}` as `patch.json`:

```bash
clickhousectl cloud clickpipe stop <service-id> <clickpipe-id>
clickhousectl cloud clickpipe get <service-id> <clickpipe-id> --json
# Repeat get until the state is Paused, not Pausing.
clickhousectl cloud clickpipe update <service-id> <clickpipe-id> --file patch.json
clickhousectl cloud clickpipe start <service-id> <clickpipe-id>
clickhousectl cloud clickpipe get <service-id> <clickpipe-id>
```

Update does not stop, wait, or restart automatically. If it fails, leave the pipe paused until you have inspected the error and can safely proceed. [Partial updates and mapping changes](../reference/clickpipe-configuration.md#update-a-pipe) have different semantics from full resource replacement.

Deletion removes the ingestion pipeline; treat it as destructive and confirm the selected pipe first. Resync is a separate operation that re-ingests data: inspect `clickpipe resync --help` before using it, rather than retrying it as a diagnostic.
