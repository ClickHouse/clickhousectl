# ClickPipe configuration

[All documentation](../README.md) · [Ingestion guide](../guides/clickpipes.md)

Use `cloud clickpipe create <source> --help` for flag names, allowed values, defaults, and bounds. This reference covers choices and update semantics that need more context than a flag list.

## Source credentials and connectivity

| Source | Setup and authentication |
| --- | --- |
| Kafka | Omit all credential flags only for an unauthenticated broker. Auth is inferred from a complete username/password, IAM access-key pair, IAM role, or certificate/private-key pair. Partial pairs fail. Avro uses a schema registry or AWS Glue; Protobuf accepts a schema file. |
| Kinesis | Use IAM role or a complete access-key pair. Choose stream and region, then initial position/format. |
| Object storage | Grant read access through IAM, access keys, an Azure connection string, or a GCP service account. Schema discovery needs a running destination service. |
| Postgres / MySQL | Basic auth requires both username and password; `IAM_ROLE` on supported RDS/Aurora sources requires a role ARN and rejects username/password. |
| MongoDB | CDC requires MongoDB 5.1+ with a replica set or sharded cluster and Change Streams/oplog access. |
| BigQuery | Snapshot loads require a pre-created GCS staging bucket and permissions for reads, export jobs, and staging objects. Only snapshot replication is supported. |
| Pub/Sub | Service-account key or enabled workload identity; this source is limited preview. Choose `earliest`, `latest`, or `timestamp` deliberately. |

For MySQL CDC, use `ROW` binlogs with `FULL` row images and at least 72 hours of retention. Default `GTID` requires GTID replication; choose `FILE_POS` only for a matching setup. Grant source `SELECT`, `REPLICATION CLIENT`, and `REPLICATION SLAVE`. See [MySQL setup](https://clickhouse.com/docs/integrations/clickpipes/mysql).

For MongoDB, retain at least 24 hours of oplog history (72+ helps a long snapshot). Generic sources use `readAnyDatabase` and `clusterMonitor`; see [MongoDB setup](https://clickhouse.com/docs/integrations/clickpipes/mongodb/source/generic). For BigQuery permissions, see the [connector overview](https://clickhouse.com/blog/bigquery-clickpipe-private-preview).

GCP service-account flags take files, not inline key JSON. `--service-account-file -` reads stdin; the contents are encoded, not the path. GCP workload identity is private preview for enabled organizations with GCP-hosted services. Run `clickpipe context get`, grant the returned principal source access, then use `--auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY` without credential flags. This works for GCS, GCMK Kafka, Pub/Sub, and BigQuery; BigQuery still needs project, staging path, and mappings. The CLI never grants IAM permissions for you.

TLS flags for database sources default to verification. `--ca-certificate` reads PEM contents; `--tls-host` names the certificate host when different from the connection host. Disabling verification weakens authentication; disabling TLS removes encryption. See the [Postgres flow](../guides/clickpipes.md#replicate-postgres-changes).

## Choose table mappings

Database creates require at least one mapping. Repeat or combine the simple and JSON forms:

| Source | Simple `--table-mapping` | Required JSON source fields, plus `targetTable` |
| --- | --- | --- |
| Postgres / MySQL | `schema.table:target_table` | `sourceSchemaName`, `sourceTable` |
| MongoDB | `database.collection:target_table` | `sourceDatabaseName`, `sourceCollection` |
| BigQuery | `dataset.table:target_table` | `sourceDatasetName`, `sourceTable` |

`--table-mapping-json` takes one object. For Postgres, for example:

```json
{
  "sourceSchemaName": "public",
  "sourceTable": "orders",
  "targetTable": "orders",
  "excludedColumns": ["private_note"],
  "sortingKeys": ["id"],
  "partitionByExpr": "toYYYYMM(created_at)",
  "tableEngine": "ReplacingMergeTree"
}
```

Postgres, MySQL, and BigQuery support excluded columns and custom sorting keys; MongoDB supports the names and engine. Nonempty `sortingKeys` enables `useCustomSortingKey` automatically. Explicit `false` with keys, or `true` without keys, fails. Unknown fields/engines and missing names fail locally.

`partitionByExpr` defines destination `PARTITION BY`; `partitionKey` selects a source column for parallel snapshotting. They serve different purposes; Postgres/MySQL support both. Choices shaping a destination table are made at creation, not changed later on an existing mapping.

Supported engines are `MergeTree`, `ReplacingMergeTree`, and `Null`. Postgres defaults to `ReplacingMergeTree`; simple mappings for the other sources leave optional settings to the service. A raw SELECT is not a current-state CDC view: with `ReplacingMergeTree`, use `FINAL` and exclude `_peerdb_is_deleted = 1`. `MergeTree` retains event versions and does not support `FINAL`. See [CDC deduplication](https://clickhouse.com/docs/integrations/clickpipes/postgres/deduplication).

## Choose create-time ingestion controls

Kafka, Kinesis, object storage, and Pub/Sub support `--start-paused`, repeatable JSON `--field-mapping`, and initial scaling. Supply all three of `--replicas`, `--cpu-millicores`, and `--memory-gb` together for an allocation. Database CDC scaling is service-wide via `clickpipe cdc-scaling`, not per-pipe allocation.

For Postgres, snapshot worker/partition settings, nullability, failover slots, and delete-on-merge are create-time choices. Only `syncIntervalSeconds` and `pullBatchSize` can be patched later. `--replication-slot-name` requires `--replication-mode cdc_only`; PG17+ failover slots apply when ClickPipes creates the slot, not when reusing one. MongoDB `--initial-load-parallelism` is per collection; `--snapshot-parallel-collections` controls concurrent collections.

Kafka/Kinesis Protobuf schema inputs accept `.proto` or serialized `FileDescriptorSet`; use `--protobuf-schema-file -` for stdin. The encoded limit is 1 MiB. Kafka Protobuf input conflicts with registry flags; `--exactly-once` is create-only. Pub/Sub timestamp seek requires `--seek-timestamp`; `earliest` reads backlog and `latest` reads new messages only.

Object storage `--skip-initial-load` and `--start-after` cannot be combined. Schema discovery probes the source without creating a pipe and requires an API key, including for workload identity. `--validate-samples` is optional on creates; the API documents it as having no effect for Postgres/MySQL.

## Update a pipe

Use `clickpipe update --file patch.json` or `--file -`. For CDC, [stop and wait for `Paused`](../guides/clickpipes.md#pause-update-and-resume-cdc) first. GET responses contain fields that cannot be patched; build a request from the writable fields below.

Omitted and null top-level fields are not sent. Explicit `false`, `0`, empty strings, and arrays remain explicit. Empty `{}`, unknown fields, and unsupported enum values fail before the request. A source patch selects at most one source arm.

| Object | Writable fields |
| --- | --- |
| Root | `name`, `source`, `destination`, `fieldMappings`, `settings` |
| `destination` | `columns` |
| `source.kafka` | `authentication`, `iamRole`, `caCertificate`, `reversePrivateEndpointIds`, `credentials` |
| `source.kinesis` | `authentication`, `iamRole`, `accessKey` |
| `source.objectStorage` | `skipInitialLoad`, `startAfter`, `authentication`, `iamRole`, `connectionString`, `path`, `azureContainerName`, `accessKey`, `serviceAccountKey` |
| `source.pubsub` | `authentication`, `ackDeadline`, `serviceAccountKey` |
| `source.postgres` | `credentials`, `host`, `port`, `database`, TLS fields, `settings`, `tableMappingsToAdd`, `tableMappingsToRemove` |
| `source.mysql` | `credentials`, `host`, `port`, `authentication`, `iamRole`, TLS fields, `serverId`, `settings`, `tableMappingsToAdd`, `tableMappingsToRemove` |
| `source.mongodb` | `credentials`, `uri`, `readPreference`, TLS fields, `settings`, `tableMappingsToAdd`, `tableMappingsToRemove` |

TLS fields are `tlsHost`, `caCertificate`, `disableTls`, and `skipCertVerification`. The source object also accepts `validateSamples`. BigQuery has no source PATCH arm; only applicable root fields can be updated.

A rename leaves unrelated fields alone:

```json
{"name":"events-v2"}
```

For object storage, root `fieldMappings` requires `destination.columns` in the same patch. Send complete arrays with a mapping for every column; the patch changes pipe configuration, not the destination table schema. Omission preserves mappings. An empty mapping array is not a supported way to clear mappings while keeping configured columns. Do not send `database`, `table`, `managedTable`, or `tableDefinition` inside a destination patch.

Kafka credentials must match the authentication mode. IAM role carries `iamRole` without a credentials object; workload identity carries neither. Unrelated credential/CA/endpoint fields may be omitted. For CDC table changes use `tableMappingsToAdd`/`tableMappingsToRemove`; these are distinct from root field mappings. Omitting a Postgres source port preserves it.

`clickpipe settings get/update` is for streaming and object-storage pipes, including Pub/Sub. For database-source settings, use `clickpipe get` and the source-specific PATCH instead. Omitted settings preserve stored values; explicit `0`/`false` are retained. For Kafka isolation, pass `--kafka-read-committed` explicitly if the current API response omits it. To change `object_storage_max_insert_bytes`, use `settings update`: the whole-pipe PATCH currently validates but does not apply that value.

## Connect a private source

Create a reverse private endpoint on the destination service, then wait for `Ready`:

```bash
clickhousectl cloud clickpipe reverse-private-endpoint create <service-id> \
  --type VPC_ENDPOINT_SERVICE --description 'analytics source' \
  --vpc-endpoint-service-name com.amazonaws.vpce.us-east-1.vpce-svc-12345678901234567
clickhousectl cloud clickpipe reverse-private-endpoint get <service-id> <endpoint-id>
```

AWS `PendingAcceptance` means the source account must accept the connection. Only use the endpoint after it is `Ready`. Kafka takes repeatable `--reverse-private-endpoint-id`; Postgres/MySQL use an endpoint DNS name as `--host`.

Other types are `VPC_RESOURCE`, `MSK_MULTI_VPC`, and private-preview `GCP_PSC_SERVICE_ATTACHMENT`; help lists their required fields. Custom private DNS mappings use exact or leading-wildcard names. They are unsupported for MSK and require support enablement for AWS PrivateLink. Update replaces the entire custom DNS list, so repeat every mapping to retain, or deliberately use `--clear-custom-private-dns-mappings`.
