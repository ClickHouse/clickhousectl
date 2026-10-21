# clickhouse-cloud-api

Typed Rust client for the [ClickHouse Cloud API](https://clickhouse.com/docs/en/cloud/manage/openapi).

Requires Rust 1.94 or later.

## Updated Cloud API surface

BigQuery replication modes now include `Cdc` and `CdcOnly`, alongside `Snapshot`,
matching the modes documented by the API. BigQuery settings add optional `replication_method`, `sync_interval_seconds`,
`query_cdc_pull_sync_parallelism`, `sync_delay_seconds`, and
`pull_window_size_seconds`; table mappings add `query_cdc_watermark_column` and
`events_function`. These strings remain open as specified by the API. Responses
preserve absent and null values as `None`, and `TryFrom` converts settings and
mappings back to request types after checking required fields. Struct-literal
callers must supply the new fields or use `..Default::default()`. The API requires
CDC settings and table mapping fields to match the replication method; snapshot
requests omit them.

`openapi_key_get_list(organization_id, limit, cursor)` returns one page of API keys.
Migrate existing calls by passing `None, None` for the new arguments. The server
accepts limits from 1–250 and defaults to 250; continue with the response's
`next_cursor` until it is `None` to read all keys, including empty intermediate
pages. Cursors are opaque: pass even an empty string unchanged. `ApiResponse<T>`
now preserves optional `limit`, `total_count`, and `next_cursor` envelope metadata.
Missing or null metadata becomes `None` and is omitted when serialized. Struct
literal callers must supply the new fields or use `..Default::default()` when
available. Callers that iterate pages should detect repeated cursors to avoid loops.

`service_profiles_list` now takes `region_id: Option<&str>`: wrap existing region
arguments in `Some(...)`, or pass `None` with `byoc_id` to use the infrastructure's
region. When both are supplied, the region must match that infrastructure.

The beta `snapshot_get_list` and `snapshot_get` methods return service `Snapshot`
records, including the `throttled` status and full snapshot type. Snapshot response
fields tolerate missing and null values; provider-specific bucket properties and
unknown status/type values remain lossless.

The beta `snapshot_configuration_get` and `snapshot_configuration_update` methods
read and update scheduled snapshots using `SnapshotConfiguration` and
`SnapshotConfigurationPatchRequest`. `gap` and `time_frame` are measured in
minutes. Updates require an ADMIN API key and at least one field; `None` omits a
field to leave it unchanged, while `Some(false)` explicitly disables scheduling.
The API rejects null and validates supported enabled cadence pairs: gap/time-frame
values of `(30, 1440)` or `(60, 2880)`. Response fields tolerate absence and null.

ClickPipes requests now include destination-table `ttl`, MongoDB
`initial_load_parallelism`, and `start_paused`. An empty TTL is omitted to preserve
existing create behavior; set a nonempty TTL SQL expression or
`start_paused: Some(true)` to send them. Starting paused is unsupported
for database ClickPipes. `UdfArgumentOutput` matches the newly named UDF response
schema; `UdfArgumentResponse` remains a compatible alias.

The live snapshot adds `credit_balances_get` (trial and prepaid credit balances), `service_profiles_list` (region and optional BYOC infrastructure), and `click_pipes_service_context_get` (GCP workload identity readiness and principal).

BigQuery and Pub/Sub source models now distinguish service-account and workload-identity authentication with typed unions. Build `ClickPipePostBigQueryServiceAccountSource` or `ClickPipePostPubSubServiceAccountSource` and call `.into()` for existing service-account flows; workload-identity variants omit customer credentials. BigQuery settings and table mappings now permit the optional fields the API accepts. Kafka requests gain optional `protobuf_schema`, which is only supported for Protobuf without a schema registry; MySQL table mappings gain `partition_by_expr`.

`ServiceProfile` now represents the profile discovery response (`profile`, `cpu_cores`, `memory_gi`); the former `Service.profile` value enum is named `ServiceProfileName`. Dynamic profile names remain lossless through its `Unknown(String)` variant.

ClickStack models now include alert channel lists, 30-second alert intervals, query-timeout errors, chart formulas and series limits, dashboard variables and broadcast filters, service-version expressions, and typed SQL/variable saved-filter unions. New formula and saved-filter response/request pairs support explicit fallible write-back through `TryFrom`; absent nested required fields return their wire names. UDF responses include `deterministic`. UDF request models preserve `deterministic` and nullable `memoryLimitMib` in both executable variants, including version creation.

The current alert request schemas have no `required` array or optional marker on either `channel` or `channels`, so both fields remain strict in the Rust request models. This mirrors the documented requiredness policy; it does not establish whether the server accepts a channels-only request. Supply the channel list explicitly rather than relying on the empty `Default` value (the API specifies 1–10 channels).

Postgres slow-query aggregate durations (`*DurationUs`) and execution `durationUs` use `Option<f64>` to preserve fractional microseconds returned by the API. Counts remain integral. The analyzer tracks the upstream integer-schema discrepancy with response-only, stale-checked exceptions ([#758](https://github.com/ClickHouse/clickhousectl/issues/758)).

Kinesis source format enums now include `Protobuf`. Set `ClickPipePostKinesisSource.protobuf_schema` to the base64-encoded `.proto` source or serialized `FileDescriptorSet` for that format; omit it for other formats. Organization Prometheus discovery has graduated from beta and is no longer listed in `BETA_OPERATIONS`.

`Organization.capabilities.snapshots` reports snapshot eligibility as an optional boolean. Kinesis create requests accept `ClickPipeKinesisSchemaRegistry` for AWS Glue; supply `type`, `glue_region`, and `glue_registry_name`, and optionally `glue_role_arn` to assume a different role from the Kinesis source. Kinesis responses use `ClickPipeKinesisSchemaRegistryResponse`, whose fields tolerate absence and null; convert it with `TryFrom` before writing it back. Kafka create requests accept `tombstone_mode: Some(Delete)` to delete matching destination rows for tombstone records. This requires exactly-once delivery and is set only at creation. Struct-literal callers of `Organization`, `ClickPipeKinesisSource`, `ClickPipeKafkaSource`, `ClickPipePostKinesisSource`, and `ClickPipePostKafkaSource` need to supply the new optional fields as `None` or use `..Default::default()`.

`ByocConfig.account_id` reports the cloud account the BYOC infrastructure is bound to (AWS account ID, GCP project ID, or Azure subscription ID). `ByocConfig.account_name` is deprecated in the spec and is now available only with the `deprecated-fields` feature; read `account_id` instead. `ByocConfigState` adds `Infra_terminating`, `Infra_degraded`, and `Infra_upgrading`; `ActivityType` adds the approved-domain, public-preview, and SAML query-ownership-migration activity types; and both Kafka tombstone-mode enums add `Soft_delete`. Exhaustive matches without a wildcard arm need the new variants, and struct-literal callers of `ByocConfig` need `account_id` or `..Default::default()`.

`ServicePostRequest.backup_encryption_config` restores a service from a backup stored in your organization's own bucket (private preview, transparent-data-encryption services only). `BackupEncryptionConfig` is a string-keyed map of JSON values: deserialize the backup's `encryption_config.json` into it verbatim rather than constructing it; omit the field for any other restore. `ActivityType` adds `Postgres_action`. Struct-literal callers of `ServicePostRequest` need `backup_encryption_config: None` or `..Default::default()`.

The beta BYOC infrastructure methods `organization_byoc_infrastructure_get` (returns `ByocInfrastructureDetails`, including BYO-VPC and load-balancer settings), `organization_byoc_infrastructure_progress_get` (returns `ByocInfrastructureProgress`, whose `ByocInfrastructureProgressStage` entries nest through `sub_stages`), and `organization_byoc_infrastructure_validate` (takes `ByocInfrastructureValidatePostRequest` and returns `ByocInfrastructureValidation` permission checks without creating infrastructure) are available. `ByocInfrastructurePostRequest` now requires only `account_id` and `region_id`: `availability_zone_suffixes`, `display_name`, and `vpc_cidr_range` are `Option`, so wrap existing values in `Some(...)`. It also gains optional BYO-VPC (`vpc_id`, `private_subnet_ids`, `public_subnet_ids`, `gcp_pod_cidr_range_names`, `gcp_shared_vpc_host_project_id`), AWS `external_id`, Azure `tenant_id` and `service_principal_client_id`, and `tags` (`ByocInfrastructureTags`, a string map of at most 50 entries) fields; struct-literal callers supply them as `None` or use `..Default::default()`.

The beta `organization_byoc_infrastructure_private_endpoint_config_get` method returns a BYOC infrastructure's private link endpoint service as `ByocInfrastructurePrivateEndpointConfig` (`endpoint_name`, `private_dns_hostname`); for BYOC organizations it replaces the deprecated `organization_private_endpoint_config_get_list`. The beta `organization_byoc_infrastructure_tags_get` method returns the infrastructure's current additional tags as `ByocInfrastructureTagsResponse`. `ByocInfrastructurePatchRequest` gains optional `enable_private_link`, `enable_private_load_balancer`, `enable_public_load_balancer`, `gcp_psc_subnet_id` (GCP BYO-VPC only, accepted only with `enable_private_link: Some(true)`), and `tags`, which replaces the tag set as a whole. Struct-literal callers of `ByocInfrastructurePatchRequest` need the new fields as `None` or `..Default::default()`.

The beta `notification_get_list` method returns one page as `ApiResponse<Vec<Notification>>`, including envelope `limit` and `next_cursor`. Pass the opaque cursor unchanged to retrieve the next page, stopping when it is absent. Optional filters cover severity, notification type, resource ID/kind, and inclusive RFC 3339 date bounds. The server defaults the page size to 100 and clamps it to 250; notifications are retained for 180 days and restricted to resources the caller can access. All `Notification` fields tolerate absence and null; type, severity and resource kind stay open strings.

The beta `notification_types_get_list` method returns the organization's notification types as `NotificationType` entries (`name`, `title`, `description`, `category`, `severity`, `resource_type`, `is_configurable`, `is_available`); `severity` is `NotificationTypeSeverity` (`Info`, `Warning`, `Critical`, or `Unknown(String)`), while `category` and `resource_type` are open-ended strings. Types the caller lacks permission to use are returned with `is_available: Some(false)`.

The beta `replica_aware_routing_get` and `replica_aware_routing_update` service methods read and change replica aware (sticky) routing as `ReplicaAwareRouting` (`http_enabled`, `native_enabled`). Update takes `ReplicaAwareRoutingPatchRequest`; set at least one field, and an omitted field is left unchanged. Enabling HTTP routing requires clients to send the `X-ClickHouse-Replica-Tag` header.

`PrometheusDiscoveryLabels` gains `meta_clickhouse_region` and `meta_clickhouse_provider` (`__meta_clickhouse_region` and `__meta_clickhouse_provider`), plus `additional_labels`, a flattened `Option<BTreeMap<String, String>>` holding any string label the API adds beyond the named ones (`None` when there are none; non-string extras are ignored). `PgConfig` and `PgConfigResponse` gain `autovacuum_analyze_threshold`, `autovacuum_freeze_max_age`, `autovacuum_vacuum_insert_threshold`, `autovacuum_vacuum_threshold`, `checkpoint_timeout`, `shared_memory_percent`, and `wal_buffers`. Struct-literal callers of these types need the new fields as `None` or `..Default::default()`. Snapshot operations now require the `control-plane:service:view-snapshots` and `control-plane:service:manage-snapshots` permissions in `meta::operations`.

The MySQL and Postgres ClickPipe source authentication enums (`ClickPipeMutateMySQLSourceAuthentication`, `ClickPipeMySQLSourceAuthentication`, `ClickPipePatchMySQLSourceAuthentication`, `ClickPipeMutatePostgresSourceAuthentication`, `ClickPipePostgresSourceAuthentication`) gain `ServiceAccountWorkloadIdentity` (`SERVICE_ACCOUNT_WORKLOAD_IDENTITY`), and the MySQL source type enums gain `Cloudsqlmysql` (`cloudsqlmysql`); exhaustive `match`es over them need the new arm. `ClickPipeMongoDBPipeTableMapping` gains `structured_ingestion` (`ClickPipeMongoDBStructuredIngestion`: required `enabled`, plus optional `columns` of `ClickPipeMongoDBPipeColumn` `name`/`type` pairs, `drop_unexpected_values`, `excluded_columns`, `sorting_keys`, and `partition_by_expr`) to replicate a collection into typed columns instead of one JSON column; responses use the all-`Option` `ClickPipeMongoDBStructuredIngestionResponse` and `ClickPipeMongoDBPipeColumnResponse`. Struct-literal callers of the table mapping need `structured_ingestion: None` or `..Default::default()`.

AWS backup-bucket models now carry `iam_role_external_id` (`iamRoleExternalId`). `AwsBackupBucket` responses tolerate missing and null IDs. The current spec requires a string on `AwsBackupBucketPostRequestV1` and `AwsBackupBucketProperties`; supply it when constructing or deserializing these request types. `AwsBackupBucketPatchRequestV1.iam_role_external_id` distinguishes omission (`None`, preserve), a value (`Some(Some(id))`, replace), and explicit null (`Some(None)`, clear). Its deserializer preserves all three states. Regenerated operation metadata also adds `control-plane:service:manage` to `INSTANCE_CREATE`, alongside `control-plane:organization:create-service`.

The Query API endpoint management methods are `query_api_endpoint_create`, `query_api_endpoint_get`, `query_api_endpoint_list`, `query_api_endpoint_update`, and `query_api_endpoint_delete`. Create and update take `PublicQueryApiEndpointRequest`; list accepts an optional cursor and limit (1–100) and returns `ApiResponse<Vec<PublicQueryApiEndpointListItem>>`. Read entries from `result` and pagination from the envelope's `next_cursor`, `limit`, and `total_count`. The earlier `QueryApiEndpointListResponse` container remains available for existing callers, but is no longer the list method's return payload. User-owned endpoints can be listed and read, but cannot be updated or deleted through this API.

The saved query methods (no longer beta) are `saved_query_create`, `saved_query_get`, `saved_query_list`, `saved_query_update`, and `saved_query_delete`, scoped to a service. Create and update take `PublicSavedQueryRequest` (update replaces the whole query); get returns `PublicSavedQuery` and list returns `PublicSavedQueryListItem` entries. List accepts an optional cursor and limit (1–100) and reports pagination on the envelope: `ApiResponse::next_cursor` (`None` on the last page), `limit`, and `total_count`.

The beta `postgres_service_backup_get_list` method lists a Postgres service's retained base backups as `PostgresBackup` entries (`key`, `last_modified`), newest first. It accepts an optional cursor and limit (1–100) and reports pagination on the envelope like `saved_query_list`. Restore a Postgres service to a point in time with `postgres_instance_restore`; backup keys are not restore targets.

The beta `whoami_get` method resolves the authenticated caller without an organization ID and needs no API-key permission. It returns `Whoami`, selected by `actorType`: `Whoami::WhoamiUser` (`user_id`, `email`, `name`, and the `WhoamiOrganization` entries the user belongs to) for OAuth or JWT callers, or `Whoami::WhoamiApiKey` (`key_id`, `name`, `organization_id`) for an organization API key. An unrecognized or absent `actorType` is kept verbatim in `Whoami::Unknown`.

Kafka schema registries are now typed unions selected by `type`, because the API also accepts the AWS Glue Schema Registry. This is a breaking change for Rust callers. `ClickPipePostKafkaSource.schema_registry` takes `ClickPipeMutateKafkaSchemaRegistry`: build the former struct's fields as `ClickPipeMutateKafkaConfluentSchemaRegistry` (adding `r#type: None` to keep the API's Confluent default) and call `.into()`, or build `ClickPipeKafkaGlueSchemaRegistry` (`type`, `glue_region`, `glue_registry_name`, optional `glue_role_arn`) and call `.into()`. `ClickPipeKafkaSource.schema_registry` returns `ClickPipeKafkaSchemaRegistry`; match `ClickPipeKafkaConfluentSchemaRegistry` (including payloads without `type`) or `ClickPipeKafkaGlueSchemaRegistryResponse`, which converts to its request type with `TryFrom`. The authentication enums are renamed `ClickPipeKafkaConfluentSchemaRegistryAuthentication` and `ClickPipeMutateKafkaConfluentSchemaRegistryAuthentication`.

`ServicePostRequestTier` now accepts only `Development` and `Production`, matching the spec: the nine `Dedicated_*` variants are removed from the service-create request enum (response enums `ServiceTier` and `ServiceScalingPatchResponseTier` keep them). The field is deprecated; prefer `min_replica_memory_gb`, `max_replica_memory_gb`, and `num_replicas`. A caller that still needs a dedicated value can send it as `ServicePostRequestTier::Unknown("dedicated_...".into())`. `ActivityType` adds `Organization_security_contact_add`, `Organization_security_contact_remove`, and `Organization_security_contact_replace`. The Query API endpoint (`query_api_endpoint_*`), scaling schedule (`scaling_schedule_*`), and UDF (`udf_*`) methods have graduated from beta and are no longer listed in `BETA_OPERATIONS`.

`organization_get_list(limit, cursor)` and the beta `snapshot_get_list(organization_id, service_id, limit, cursor)` now return one page, like `openapi_key_get_list`: pass `None, None` to keep the previous request, and continue with the response's `next_cursor` until it is `None` to read every page. `limit` must be at least 1 and defaults to 250 on the server.

Request fields the spec no longer requires are now `Option<T>` and omitted when `None`: `ApiKeyPostRequest.assigned_role_ids`, `ip_access_list` and `state`; `ClickPipeMutateDestination.columns`; `ClickPipePostRequest.field_mappings` and `start_paused`; `InstanceServiceQueryApiEndpointsPostRequest.allowed_origins`; `InvitationPostRequest.assigned_role_ids`; and `ServicPrivateEndpointePostRequest.description`. Wrap existing values in `Some(...)` to send the same request. `columns` and `field_mappings` previously skipped empty lists, and `start_paused` skipped `false`; `Some(vec![])` and `Some(false)` are now sent as written, so map those to `None` to keep omitting them. `ScimGroupPostRequest`, `ScimUserPostRequest`, and `ScimPatchOp` gain optional `id` and `meta`, which the IdP echoes back and the server ignores on write; `ScimPatchOp.meta` is `ScimPatchOpMeta`, selected by `resourceType` (`User` or `Group`) with a lossless `Unknown` fallback. Struct-literal callers supply them as `None` or use `..Default::default()`. The ClickPipe source wrappers (`ClickPipePostSource`, `ClickPipePatchSource`, `ClickPipeSchemaDiscoverySource`) and `ClickPipeSettingsPutRequest` keep every field `Option`: their arms are mutually exclusive or source-specific, so the analyzer exempts them from the description-based requiredness fallback.

The spec no longer accepts `sandboxVersion` when creating a UDF or a UDF version, so `sandbox_version` is removed from `UdfCreateRequestV1`, `UdfCreateRequestV2`, `UdfVersionCreateRequestV1`, and `UdfVersionCreateRequestV2`; drop it from struct literals. `Udf.sandbox_version` and the `UdfSandboxVersion` enum remain for responses.

`ReversePrivateEndpoint` adds read-only `dns_targets` (`ReversePrivateEndpointDnsTarget`: `id`, `kind`, `internal_dns_name`) and `private_dns_mappings` (`ReversePrivateEndpointPrivateDnsMapping`: `private_dns_name`, `internal_dns_name`). `CustomPrivateDnsMapping` and `CustomPrivateDnsMappingResponse` gain optional `target_id`, which routes a VPC_RESOURCE custom DNS name to one of the endpoint's `dns_targets`; leave it `None` to keep the default target. Struct-literal callers supply the new fields as `None` or use `..Default::default()`.

### ClickHouse settings models

`ServiceClickhouseSettingsPatchRequest` uses a map of setting names to JSON values, and `ServiceClickhouseSettingsPatchResponse.settings` returns the applied map. The published OpenAPI now describes both fields as nonempty objects with string or integer values. Use `ServiceClickhouseSettingsPatchRequest<ServiceClickhouseSettingsMap>` and `ServiceClickhouseSettingValue` for typed requests; the default JSON-value map remains source-compatible with existing callers. Explicit `ServiceClickhouseSettingsPatchRequest<String>` callers remain supported: encoded objects are validated and serialized as objects before sending.

`ServiceClickhouseSetting.value` is `Option<serde_json::Value>`: the published contract permits strings and integers. The response aliases `ServiceClickhouseSettingValueResponse` and `ServiceClickhouseSettingsMapResponse` retain arbitrary JSON to tolerate future response types. Values retain their JSON types; missing and null values remain absent.

## Postgres Query API

`Client::run_postgres_query_bearer(org_id, service_id, &RunPostgresQueryRequest)`
runs read-only SQL over HTTPS with the client's OAuth Bearer token. The request
requires `sql` and accepts an optional `database`; omission uses `postgres`.
Basic/API-key clients receive `Error::AuthMismatch` before any network request. The method neither provisions query credentials nor wakes the
service or retries queries.

This console Query API route is outside the published OpenAPI. It accepts
`clickhousectl` audience OAuth tokens with read-only database access; Cloud API
keys are unsupported.

The returned `reqwest::Response` supports streaming and automatically decompresses
gzip using the default HTTP client. Its fixed format is
`JSONCompactEachRowWithNamesAndTypes`: a JSON array of column names, then an array
of type names, followed by one array per row, all newline-delimited. An empty
result can have an empty body. SQL errors retain their status, server `code`
(currently `POSTGRES_ERROR`), and `details` in `Error::Sql`; other HTTP errors retain
their status and body.
Query host selection follows `with_query_host`, `CLICKHOUSE_CLOUD_QUERY_HOST`,
and the management API environment, in that order.

The ignored smoke test checks a read and an empty result against an existing
service without creating resources. Supply
`CLICKHOUSE_CLOUD_TEST_BEARER_TOKEN` (a `clickhousectl` audience OAuth token),
`CLICKHOUSE_CLOUD_TEST_ORG_ID`, and `CLICKHOUSE_CLOUD_TEST_POSTGRES_SERVICE_ID`.
Optionally set `CLICKHOUSE_CLOUD_TEST_POSTGRES_DATABASE`,
`CLICKHOUSE_CLOUD_API_BASE_URL`, or `CLICKHOUSE_CLOUD_QUERY_HOST` for another database
or environment. Run:

```bash
cargo test -p clickhouse-cloud-api --test run_query_test live_postgres_query_bearer_smoke -- --ignored --nocapture
```

For additional live coverage, verify that the route rejects write statements,
invalid tokens, and tokens without access to the requested organization or service,
and repeat the smoke test with an explicit non-default database.

## Development

### Structure

| Path | Purpose |
|------|---------|
| `src/client.rs`, `src/client/*.rs` | `Client`, shared HTTP machinery, and per-domain endpoint methods |
| `src/models.rs`, `src/models/*.rs` | Public facade/macro and private per-domain request/response models |
| `src/convert.rs`, `src/convert/*.rs` | Conversion error and per-domain response-to-request conversions |
| `src/error.rs` | Error types (`Http`, `Json`, `Api`) |
| `clickhouse_cloud_openapi.json` | Checked-in copy of the spec (used by tests) |
| `tests/spec_coverage_test.rs` | Thin snapshot/live-spec consumer of the shared drift analyzer |
| `../clickhouse-openapi-analyzer` | Canonical Rust/OpenAPI parsing, comparison, report, and exemptions |

### Field optionality

The OpenAPI spec uses two conventions for marking fields required vs optional:

- **Schemas with a `required` array** (newer/beta endpoints) use standard OpenAPI semantics.
- **Schemas without `required`** (GA/legacy endpoints) treat fields whose description starts with `"Optional"` as optional. Everything else is implicitly required.
- **Known partial `required` arrays** use the union of that array and the description heuristic, as configured by the analyzer.

Additional rules:

- **PATCH request schemas** (name contains `Patch` and ends with `Request`) are always all-optional.
- **Nullable fields** (`type: ["string", "null"]` or `oneOf` with null) are always `Option<T>`, even if required.

In `src/models/<domain>.rs`, request fields follow those rules: required non-nullable fields use `T`, while optional or nullable fields use `Option<T>`. Every response field uses `Option<T>` plus `skip_serializing_if`, so missing keys and explicit `null` deserialize natively to `None` and absent fields are omitted when serialized. `#[serde(default)]` is banned because it can fabricate required request values and is redundant for `Option` response fields.

API key PATCH expiry uses `Option<Option<DateTime<Utc>>>`: `None` leaves the current expiry unchanged, `Some(Some(time))` sets it, and `Some(None)` removes it. These three states survive serialization and deserialization. API key creation still uses `Option<DateTime<Utc>>`, where omission creates a key without an expiry.

### Deprecated fields

The OpenAPI spec marks some response fields as deprecated (e.g. `Service.tier`, `ApiKey.roles`, `Member.role`). In almost all cases, these are not needed. The Cloud API library disables them by default, gated by a Cargo feature flag `deprecated-fields`. Enable this feature if you need to consume deprecated fields.

### Scripts

```bash
# Show a JSON manifest of required/optional fields per schema
python3 scripts/resolve-field-requirements.py

# Regenerate the DEPRECATED_FIELDS constant from the snapshot
python3 scripts/regenerate-deprecated-fields.py

# Regenerate the BETA_OPERATIONS constant from the snapshot
python3 scripts/regenerate-beta-lists.py

# Check for drift between the live spec and the library (dry run)
python3 scripts/check-openapi-drift.py --dry-run
```

Field optionality is maintained by hand. Edit the owning model domain file when the drift check flags a mismatch, and add new domain types to the private module and facade re-exports in `models.rs`.

#### Automated drift remediation

When the nightly `openapi-drift.yml` run opens a new drift issue, it launches a Cursor Cloud Agent through the
[Cloud Agents API](https://cursor.com/docs/cloud-agent/api/endpoints.md) to remediate it.
`scripts/launch-drift-remediation-agent.py` sends the request. The agent follows the `openapi-drift-remediation`
skill and opens draft pull requests for review. Later runs only edit the open issue, so they never launch another
agent. Each issue maps to a fixed agent ID, so a retry can't start a second agent for the same issue.

Configure it in the repository settings:

| Setting | Kind | Purpose |
| --- | --- | --- |
| `CURSOR_API_KEY` | Secret, required | Cursor user or service-account API key. Without it the step logs a warning and skips. |
| `CURSOR_DRIFT_AGENT_MODEL` | Variable, optional | Model ID from `GET https://api.cursor.com/v1/models`. When unset, the key owner's default model is used. |
| `CURSOR_DRIFT_AGENT_MODEL_PARAMS` | Variable, optional | JSON array of model parameters, such as `[{"id": "thinking", "value": "high"}]`. |
| `CURSOR_DRIFT_AGENT_ENVIRONMENT` | Variable, optional | Name of a saved Cursor cloud environment. When unset, Cursor picks the repository's saved environment. |

To start an agent for a drift issue that is already open, run the workflow manually with **Launch a Cursor
remediation agent** checked. Preview the request without calling the API:

```bash
python3 scripts/launch-drift-remediation-agent.py --dry-run \
  --issue-url https://github.com/ClickHouse/clickhousectl/issues/NUMBER \
  --repo-url https://github.com/ClickHouse/clickhousectl
```

### Testing

```bash
cargo test -p clickhouse-cloud-api          # all tests
cargo test -p clickhouse-openapi-analyzer   # analyzer fixtures + executable parity
cargo test -p clickhouse-cloud-api --test spec_coverage_test # shared snapshot report
cargo test -p clickhouse-cloud-api --test client_test        # wiremock client tests
cargo test -p clickhouse-cloud-api --test models_test        # serde round trips
```

Live-API lifecycle suites are `#[ignore]`d by default (they provision real resources):

```bash
cargo test --test integration_test -- --ignored --nocapture           # ClickHouse service CRUD
cargo test --test integration_postgres_test -- --ignored --nocapture  # Postgres service CRUD
```

ClickPipes E2E binaries live under `tests/clickpipes/` and are declared as named `[[test]]` entries in `Cargo.toml`. Each per-source binary provisions its own ClickHouse Cloud service and exercises one source; `clickpipe_e2e_test` runs every stage in parallel against a single shared service:

```bash
cargo test --test clickpipe_e2e_test -- --ignored --nocapture            # all sources, one CHC service
cargo test --test clickpipe_s3_test -- --ignored --nocapture             # per-source: S3
cargo test --test clickpipe_kafka_test -- --ignored --nocapture          # per-source: Kafka (Redpanda)
cargo test --test clickpipe_kinesis_test -- --ignored --nocapture        # per-source: Kinesis
cargo test --test clickpipe_mongo_test -- --ignored --nocapture          # per-source: MongoDB
cargo test --test clickpipe_mysql_test -- --ignored --nocapture          # per-source: MySQL
cargo test --test clickpipe_postgres_ec2_test -- --ignored --nocapture   # per-source: Postgres-on-EC2
cargo test --test clickpipe_postgres_cli_cdc_test -- --ignored --nocapture # CHC-managed Postgres CDC via CLI
cargo test --test clickpipe_smoke_test -- --ignored --nocapture          # create-only smoke against a shared service
```

All require `CLICKHOUSE_CLOUD_API_KEY`, `CLICKHOUSE_CLOUD_API_SECRET`, `CLICKHOUSE_CLOUD_TEST_ORG_ID`, `CLICKHOUSE_CLOUD_TEST_PROVIDER`, and `CLICKHOUSE_CLOUD_TEST_REGION` in the environment, and are wired into the scheduled `Cloud Integration` GitHub Actions workflow. The ClickPipes E2E suites additionally need AWS credentials and an `eu-west-1` region quota; `clickpipe_smoke_test` reads a pre-provisioned service ID from `CLICKHOUSE_CLOUD_TEST_CLICKPIPE_SERVICE_ID`.

The managed-Postgres target uses the CLI's default table mapping and compares exact source and destination IDs and values after snapshot, INSERT, UPDATE, and DELETE. Destination reads use `FINAL WHERE _peerdb_is_deleted = 0`. It requires `CLICKHOUSE_CLOUD_TEST_CLICKHOUSECTL_BIN` to point to a built `clickhousectl` binary. From the workspace root, build and run it with:

```bash
cargo build -p clickhousectl
CLICKHOUSE_CLOUD_TEST_CLICKHOUSECTL_BIN=target/debug/clickhousectl \
  cargo test -p clickhouse-cloud-api --test clickpipe_postgres_cli_cdc_test -- --ignored --nocapture
```

The trusted planner runs automatically on every same-repository PR push and selects suites from that PR's merge-base-to-head diff; it is read-only and never receives Cloud secrets. When it selects no live suites, the `Cloud integration decision` check turns green with no action needed. When live suites are selected, the decision check names them and stays `action_required` until `run-cloud-integration` is applied — the labeled event is the one-shot, exact-SHA authorization that admits the environment-bearing job, so re-apply the label after any push. Unknown API source or test paths select all suites, and changes to the classifier, its tests, or the workflow also select all suites through an independent workflow guard. Manual `Cloud Integration` dispatches accept `scope=all`, `service`, `postgres`, `organization`, or `clickpipes`. The focused scopes run only their corresponding suite; `clickpipes` runs Postgres CDC plus the fixture-gated smoke test, while `all` runs all four mandatory suites plus that optional smoke test. Because stacked-PR diffs exclude inherited changes, manually run `scope=all` against the top stack branch for full-stack validation.

`spec_coverage_test` sends the checked-in sources and snapshot through the
private `clickhouse-openapi-analyzer` crate. The analyzer recursively traverses
the module trees rooted at `client.rs`, `models.rs`, and `meta.rs`, including
private per-domain files. That same analyzer powers the scheduled live-spec
issue, so operation, model, field, optionality, beta, deprecation, enum,
snapshot, and stale-exemption findings share one implementation. The single
ignored test runs the same report against the live spec. The analyzer also checks
inline union payload fields and request requiredness. Report schema version 10 also
compares effective operation parameters against the snapshot (additions, removals,
requiredness, and schema constraints), resolving local references and ignoring
prose/example changes. Missing Rust arguments and incompatible optionality or
scalar/array shapes remain actionable even after a snapshot refresh. Array
arguments may use a pluralized name and represent omission with an empty slice.

Successful JSON envelopes are checked against the method's actual returned
struct, including generic `ApiResponse<T>`, aliases, and flattened structs. Missing
envelope fields retain exact definition pointers through response/schema refs
and `allOf`; unmappable envelopes and unsupported inline unions are actionable.
These checks cover envelope field presence, not generic payload type substitution
or HTTP serialization behavior. Parameter defaults, bounds, and other constraints
are snapshot comparisons; scalar/array shape checks compare Rust argument types.
External or unresolved contract references fail analysis rather than report clean.

### Optionality exemptions

Occasionally the spec cannot be followed literally because verified API
behavior differs. All such policy lives in
`crates/clickhouse-openapi-analyzer/src/config.rs`, including optionality,
extra-field, deprecated-field, extra-enum-value, non-OpenAPI-method, partial
required-schema, and unsupported-enum configuration.

Add an exemption only for a deliberate runtime behavior and document why the
spec cannot be followed. New unsupported-enum acknowledgements also require a
tracking issue. The analyzer reports stale field/enum exemptions and vanished
unsupported locations so obsolete entries are removed during normal drift
remediation. Acknowledged locations also report changed enum values against
the snapshot, including numeric and mixed values; unchanged sets remain
acknowledged. See the repository `AGENTS.md` for exact key formats and the full
remediation and verification procedure.

### ClickStack list pagination

`click_stack_list_alerts`, `click_stack_list_webhooks`, and
`click_stack_list_saved_searches` take `limit: Option<i64>` and
`offset: Option<i64>` after the organization and service IDs. Pass `None, None`
for the server defaults (1,000 records, offset zero). For a complete inventory,
request pages with an explicit limit from 1 to 1,000 and advance the offset
until the returned page is shorter than that limit. Existing callers upgrading
to 0.5.0 should add `None, None` to preserve their current request behavior.

### UDF attachment errors

Rust callers receive `Error::UdfAttachmentUnavailable` for a structured attachment failure (HTTP 424); its `UdfAttachResponse424` payload preserves the error code, service state, wake eligibility, and request ID. Fields tolerate absence and null, and enums retain unknown values. Malformed responses remain `Error::Api` with the original error message.

### OpenAPI response coverage

The OpenAPI analyzer checks inline union payload fields and request requiredness, plus inline JSON response objects named `{PascalizedOperationId}Response{Status}` and reachable through client return types or error payloads. It reports obsolete helper exclusions and requiredness overrides as stale exemptions. Acknowledged unsupported enum locations are checked against the snapshot: changed value sets are actionable, while reordering is ignored. Deprecated API-key `roles` fields remain strings behind `deprecated-fields` for source compatibility. Its report format is version 11.


Success response checking follows each operation's actual Rust return type. Generic
applications and aliases retain their type arguments, which are substituted through
response fields, flattened wrappers, arrays, and typed maps. An incompatible payload
kind or referenced element model produces `response_shape_mismatch` with an operation,
Rust field, and exact specification pointer. These checks use the target specification
independently of the snapshot; removing a component and refreshing the snapshot cannot
hide an obsolete return type. Response `Option` fields remain legitimate regardless of
specification requiredness. Nullable 3.0 and 3.1 forms are supported.

Resolved JSON success contracts are also compared between specifications, including
referenced schemas. Added, removed, and changed contracts have separate snapshot
findings with the previous/current contracts and locations. Prose, examples, and
ordering of property/required sets do not cause contract drift. These findings describe
specification changes, not necessarily a Rust incompatibility or a backend rollout.

This is structural coverage, not a proof of every possible live value: arbitrary JSON
payloads remain lossless catch-alls, existing union checks own named/inline variant
coverage, and numeric ranges, string patterns, custom deserializers, non-JSON responses,
and undocumented backend behavior are not validated by the shape checker. Unsupported
compositions, unresolved generic bindings, alias cycles, and analysis depth limits are
reported explicitly rather than counted as successful analysis.

### UDF list response migration

`udf_list` and `udf_version_list` now return `ApiResponse<Vec<Udf>>`;
`udf_attachment_list` returns `ApiResponse<Vec<UdfAttachment>>`. Read rows from
`response.result` and pagination from `response.limit`, `response.total_count`, and
`response.next_cursor`. `total_count` is best-effort and can be absent or null; follow
`next_cursor`, not the count, to find subsequent pages. The exported legacy
`UdfListResponse`, `UdfVersionListResponse`, `UdfAttachmentListResponse`, and `Pagination`
models remain available for callers that explicitly use them, but these three methods
no longer return nested `items`/`pagination` objects.

The historical implementation matched the earlier beta specification. An October
snapshot refresh adopted array results and outer pagination without migrating the
method returns. Removed-component warnings were a breadcrumb, but the old analyzer
lost generic payload arguments and checked only outer field presence. UAT-01's
regression reconstructs those returns against the current snapshot and requires all
three mismatches to remain actionable after refresh.

`integration_udf_test` creates two disposable UDFs, multiple versions, and a disposable
service. It checks empty/nonempty lists, real continuation pages at `limit=1`, and
attachment/deletion visibility, then cleans up on both success and failure. UDF client,
model, and CLI paths select the service integration suite, which runs this test.


### Operation permissions and metadata

`meta::operations` exposes offline endpoint descriptors alongside the existing typed `Client` methods:

```rust
use clickhouse_cloud_api::meta::operations;

let endpoint = &operations::INSTANCE_GET;
assert_eq!(endpoint.operation_id, "instanceGet");
assert_eq!(endpoint.rust_method, "instance_get");
assert_eq!(endpoint.required_permissions, &["control-plane:service:view"]);
assert_eq!(operations::by_operation_id("instanceGet"), Some(endpoint));
```

Each descriptor includes the exact operation ID, Rust method name, HTTP method, path template, and required Cloud API-key permission IDs. **All listed permissions are required.** An empty slice means the spec requires a valid key without additional named permissions. Unknown operation IDs return `None`; they never appear as an empty permission list. `operations::ALL` iterates every descriptor. These requirements describe API-key authorization; they do not replace OAuth restrictions or SQL privileges. A caller making several API requests can union the permission slices of those operations.

The committed catalog is generated by the shared analyzer, without runtime parsing or network access:

```sh
cargo run -p clickhouse-openapi-analyzer --bin openapi-drift-analyzer -- \
  --spec crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json \
  --generate-operations crates/clickhouse-cloud-api/src/meta/operations.rs
cargo fmt --all
```

The analyzer's `clickhouse_cloud_config()` enables `check_operation_permissions`; generic `AnalyzerConfig::default()` leaves this Cloud-specific convention disabled. Enabled analysis resolves operation security overrides and root inheritance, normalizes permission sets, and reports unknown, malformed, anonymous, alternative, or multiple-scheme requirements as actionable findings. Generation refuses these unsupported forms. Permission changes are compared both against the snapshot and against the Rust descriptors, so refreshing the snapshot alone cannot hide stale permissions. Descriptor identity, matching client methods, coverage, and the complete sorted catalog are checked too.
