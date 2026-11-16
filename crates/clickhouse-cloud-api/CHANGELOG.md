# Changelog

Changes to the `clickhouse-cloud-api` crate, newest first, with what a Rust caller
must change to upgrade.

## Versioning policy

`clickhouse-cloud-api` is released in lockstep with `clickhousectl`: every release
publishes both crates at the same version, even when this crate has not changed.
Before 1.0, a minor release (0.x to 0.x+1) may contain breaking changes. The crate
follows the ClickHouse Cloud OpenAPI spec, and many spec changes break Rust callers:
a field that becomes required, a new method parameter, a renamed or added enum
variant. Patch releases (0.x.y) do not contain breaking changes. Each entry says what
changed and how to migrate. A Cargo requirement such as `0.6` already stays within one
minor version; read the entries before raising it.

## Unreleased

Structs with a required field no longer implement `Default`: request bodies
such as `ServicePostRequest`, `ClickPipePostRequest` and
`ClickStackCreateAlertRequest`, their nested required types, the request
unions that defaulted to one of them (for example `ClickStackAlertChannel` and
`BackupBucketPostRequest`), and the string enums used only as required request
fields (for example `ServicePostRequestProvider`). `..Default::default()` used
to fill a field the API had just made required with `""`, `0`, `false` or the
first enum variant, and the request still compiled. Callers of these types
write every field: `None` for the optional ones and a real value for each
required one. All-`Option` structs, such as PATCH bodies, and every response
type keep `Default`. Older entries below that offer `..Default::default()`
apply only to types that still implement it.

The six request structs without `Default` that carry a `deprecated-fields`
field gain a `new` constructor that takes only the required fields and leaves
every other field `None`: `ApiKeyPostRequest::new(name)`,
`ClickPipeScaling::new(replicas, replica_cpu_millicores, replica_memory_gb)`,
`ClickStackTileInput::new(name, x, y, w, h)`,
`InvitationPostRequest::new(email)`,
`OrganizationPrivateEndpointsPatch::new(remove)` and
`ServicePostRequest::new(name, provider, region, ip_access_list)`. A struct
literal must name every field, including the gated ones, so it stops compiling
when any crate in the build enables `deprecated-fields` (Cargo unifies features).
Build these structs with `new` and set optional fields with struct update syntax,
for example `ServicePostRequest { num_replicas: Some(3), ..ServicePostRequest::new(...) }`.
For the all-`Option` PATCH bodies with a gated field (`ApiKeyPatchRequest`,
`ClickPipeScalingPatchRequest`, `MemberPatchRequest`), use `..Default::default()`.

With `deprecated-fields` enabled, `ClickPipeScaling.concurrency` is now
`Option<i64>` and is omitted when `None`. It used to be a required `i64`, so
every scaling request sent `"concurrency": 0`, which the default build never
sent. Replace `concurrency: 0` with `concurrency: None`, or use
`ClickPipeScaling::new`. The analyzer no longer treats a deprecated property
as required.

The crate re-exports `reqwest` as `clickhouse_cloud_api::reqwest`, so a
prebuilt client passed to `Client::with_http_client` or
`Client::with_http_client_bearer` uses the same version as this crate. No
change is needed; callers can drop their own `reqwest` dependency if they only
used it for this.

ClickStack chart configs gain a PromQL variant that queries a `promql` source.
Each of the line, stacked-bar, stacked-line, categorical-bar, table, number,
pie and heatmap chart config unions (and their `*Response` unions) adds a
`ClickStack{Kind}PromqlChartConfig` arm, selected by `configType: "promql"`, and
the four alert chart config unions add `ClickStackAlert{Kind}PromqlChartConfig`
with the alert overlay's `name`, `where` and `whereLanguage`. A PromQL config
carries `expressions` instead of `select`: `ClickStackPromqlRangeExpression`
for time series and heatmaps, `ClickStackPromqlInstantExpression` (with
`query_type`) for tables, and `ClickStackPromqlReducibleExpression` (with
`query_type` and `reducer`) for number, pie and categorical-bar charts. A
payload without `configType` that carries `expressions` no longer reaches a
builder arm; it lands in `Unknown`, as a raw-SQL payload without `configType`
already did. The heatmap union now dispatches on `configType` before
`heatmapMode`, so any other `configType` value is `Unknown`. Exhaustive matches
on these unions without a wildcard arm need the new arms.

The beta service JWT provider methods are `jwt_provider_get_list`,
`jwt_provider_create`, `jwt_provider_get`, `jwt_provider_update` and
`jwt_provider_delete`. A JWT provider lets users connect to a service with a
token from an external identity provider; they sign in as `default` with the
roles in the token's roles claim. Create takes `JwtProviderPostRequest`
(`name`, `issuer`, `audience`, `jwks_url`, optional `roles_claim`) and is safe
to retry with the same configuration; update takes `JwtProviderPutRequest`,
which replaces the whole configuration, so `roles_claim: None` removes an
existing claim. Reads return `JwtProvider`, and delete returns no `result`. The
server validates that the JWKS URL serves an RS256 key set, a service has at
most 5 providers, and every method needs `control-plane:service:manage` and
the Enterprise tier.

`ServiceReplicaScalingPatchRequest` gains optional `profile`, a custom instance
profile name such as `v1-highmem-s`; `None` leaves the profile unchanged.
Struct-literal callers need `profile: None` or `..Default::default()`.

`click_pipe_get_list` now takes `limit: Option<i64>` and `cursor: Option<&str>`
after the organization and service IDs, and still returns
`ApiResponse<Vec<ClickPipe>>`. Pass `None, None` to keep listing every ClickPipe
in one response, as before. Either argument requests a page, most recent first
(`limit` at least 1, server default 50); the envelope's `limit`, `total_count`
and `next_cursor` carry the page metadata, and `next_cursor` is `None` on the
last page. `total_count` is best-effort and may be `None`.

`ClickPipePostKafkaSource.credentials` is now `Option<serde_json::Value>` and
`reverse_private_endpoint_ids` is `Option<Vec<String>>`, because the spec no
longer requires either. `None` omits the key; `Default` no longer sends
`"reversePrivateEndpointIds": []`. To send the previous body, wrap the values
in `Some`: `Some(serde_json::Value::Null)` still sends `"credentials": null`
(IAM_ROLE), and `Some(vec![])` still sends an empty list. The spec now lists
`protobufSchema` as optional itself, so its optionality exemption is removed.

`ClickPipePostKafkaSource.authentication` is now
`ClickPipePostKafkaSourceAuthentication` instead of an `Option`, because the
spec now requires it. Replace `Some(auth)` with `auth`; a source without
authentication can no longer be built, and deserializing a body without it fails.

`ClickPipeSchemaDiscoveryField` gains optional `is_virtual` (metadata added by
ClickPipes, such as `_topic` or `_path`) and `is_virtual_disabled_default`
(left unselected by default when building destination columns).

`postgres_logs_get_list` now takes `severity: Option<&PostgresLogsGetListSeverity>`
instead of `Option<&str>`, because the spec now lists the accepted levels. The
enum has `Debug`, `Log`, `Info`, `Notice`, `Warning`, `Error`, `Fatal` and
`Panic`, a `VALUES` const, and an `Unknown(String)` catch-all. Migrate
`Some("ERROR")` to `Some(&PostgresLogsGetListSeverity::Error)`; the query string
sent is unchanged.

`PostgresService`, `PostgresServiceListItem` and `PostgresServicePatchRequest`
gain optional `deletion_protection` (`PgDeletionProtectionProperty`, a `bool`).
When it is `true`, the service cannot be deleted until it is turned off; it is
only available to organizations with deletion protection enabled. Struct-literal
callers need `deletion_protection: None` or `..Default::default()`.

`PgSize` adds 49 GCP local-SSD sizes: C3 (`C3_standard_lssd_4` /
`c3-standard-lssd-4`), C3D (`C3d_highmem_lssd_8`, `C3d_standard_lssd_8`) and N2
(`N2_highmem_2xlssd_4` / `n2-highmem-2xlssd-4`, `N2_standard_2xlssd_4`, and the
`4xlssd`, `8xlssd` and `16xlssd` highmem shapes). Exhaustive matches without a
wildcard arm need the new variants.

The spec now declares slow-query `avgDurationUs` and the `p50`/`p95`/`p99`
durations as numbers, so their fractional-response analyzer exemptions are
removed. The fields stay `Option<f64>`.

`ClickStackLogSource` and `ClickStackLogSourceResponse` gain optional
`event_name_expression` and `severity_number_expression`, the expressions for
the OpenTelemetry log `EventName` and numeric `SeverityNumber` (1-24). `TryFrom`
carries them back to the request type. Struct-literal callers need the new
fields as `None` or `..Default::default()`.

`ClickPipeMongoDBPipeTableMapping` gains optional `mongo_config`
(`ClickPipeMongoDBTableConfig`), per-collection MongoDB options. Its one field,
the required `persist_deleted_documents`, keeps the full document as it was
right before a delete instead of only its `_id`; it requires
`changeStreamPreAndPostImages` on the source collection. Omitting `mongo_config`
leaves the API default (`false`). Responses use the all-`Option`
`ClickPipeMongoDBTableConfigResponse`. Struct-literal callers of
the table mapping need `mongo_config: None` or `..Default::default()`.

The beta `member_notification_settings_get_list` method returns one page of a member's own notification overrides as `ApiResponse<Vec<NotificationSetting>>` (`type`, `resource_type`, `resource_id`, `channel_type`, `effect`), with envelope `limit` and `next_cursor`; optional filters cover notification type and resource ID/kind, and the page size defaults to 100 (1 to 250). The beta `member_notification_settings_update` method takes `MemberNotificationSettingsPatchRequest`, whose `rules` (`NotificationSettingRule`: required `effect`, `notifications` and `resources`, optional `channels`) are applied in order, and returns the affected settings as `MemberNotificationSettingsPatchResponse`. A rule with effect `inherit` removes the setting, so the role default applies again. Effects, channels, notification types and resource kinds stay open strings.

The beta `organization_public_previews_get_list` method lists the organization's preview features as `PublicPreview` (`id`, `name`, `description`, `docs_url`, `enabled`, `can_toggle`), and the beta `organization_public_preview_patch` method switches one on or off by `id` with `PublicPreviewPatchRequest { enabled }`. The beta `organization_byoc_infrastructure_customer_authorized_access_get` and `organization_byoc_infrastructure_customer_authorized_access_update` methods read and change when ClickHouse personnel may access a BYOC infrastructure as `ByocCustomerAuthorizedAccess` (`mode`, `allowed`, and for a window `start_timestamp`, `duration_seconds`, `window_end_timestamp`). Update takes `ByocCustomerAuthorizedAccessPatchRequest`: `mode` is an open string (currently `always`, `never` or `window`), and `start_timestamp` with `duration_seconds` (1 to 172800) are required for `window` and rejected otherwise. As PATCH bodies, all three request types keep their fields `Option`; the server still requires `rules`, `enabled` and `mode`. `ActivityType` adds `Byoc_infrastructure_update_customer_authorized_access`.

`ClickPipePostObjectStorageSource` and `ClickPipeObjectStorageSource` gain optional `skip_first_lines` (`skipFirstLines`, 0 to 100), the number of lines to skip at the start of each file; it applies only to `TabSeparated` and `TabSeparatedWithNames` and can only be set at creation. `ClickPipeSettings`, `ClickPipeSettingsPutRequest` and `ClickPipeSettingsResponse` gain optional `object_storage_max_file_size` (`i64`, 1048576 to 536870912000 bytes): files larger than this are skipped and not retried. It can only lower the limit and is a private preview that support must enable for the organization. Both stay `Option` and are omitted when `None`, although the spec does not mark them optional. Struct-literal callers need the new fields as `None` or `..Default::default()`.

Every `Client` method sends each path parameter (organization, service, key, pipe or provider ID, UDF or setting name) as one percent-encoded URL segment, so `/`, `?`, `#` or `%` in an ID can no longer change the request path; the API answers 400 or 404 for a malformed ID instead. The same applies to the service ID in the Query API URLs. A parameter that is empty, `.` or `..` cannot be one segment, because URL parsing drops or resolves it, so the method returns `Error::InvalidPathParameter` without sending a request. Each method takes its HTTP method and path template from its generated `meta::operations` descriptor.

## 0.5.1

No changes to this crate; released in lockstep with `clickhousectl` 0.5.1.

## 0.5.0

The beta `click_stack_validate_webhook` method checks a `ClickStackWebhookInput`
against the rules used when a webhook is created, without saving it; name
uniqueness is not checked. It returns `ClickStackValidateWebhookResponse`, with
`valid`, `errors` (`ClickStackValidateWebhookResponseErrorsItem` path and
message pairs), and `normalized`, a free-form `serde_json::Value`. ClickStack
sources gain optional expressions: `ClickStackLogSource` and
`ClickStackTraceSource` add `order_by_expression`, `ClickStackSessionSource`
adds `resource_attributes_expression`, and `ClickStackTraceSource` also adds
`displayed_timestamp_value_expression`, `sample_rate_expression`, and
`span_links_value_expression`. Each `*Response` variant has the same fields, and
`TryFrom` carries them back to the request type. Struct-literal callers need the
new fields as `None` or `..Default::default()`. The unused
`ClickStackValidationError`, `ClickStackValidationErrorItem`, and
`ClickStackValidationErrorItemErrors` types are removed because the API no
longer defines them; no method returned them.

ClickStack role permissions are now a union, a breaking change for Rust callers.
`ClickStackCASLPermission` (in `ClickStackCreateRoleRequest` and
`ClickStackUpdateRoleRequest`) and `ClickStackCASLPermissionResponse` (in
`ClickStackRole`) were structs; they are now enums with a
`ClickStackClickHouseCASLPermission` arm, a `ClickStackMongoDBCASLPermission` arm and an
`Unknown(serde_json::Value)` catch-all, dispatched on the required `integration` field
(`clickhouse` or `mongodb`). A payload without a known `integration`, or one that does
not fit the arm it selects, lands in `Unknown` verbatim. Wrap a struct literal in its arm,
for example `ClickStackCASLPermission::ClickStackMongoDBCASLPermission(ClickStackMongoDBCASLPermission { .. })`,
and `match` a response arm before reading its fields. Each arm adds `fields` (a CASL field
restriction) and replaces the `String` fields `action` and `subject` with the shared
`ClickStackCASLPermissionAction` and `ClickStackCASLPermissionSubject` enums;
`integration` is the arm's single-value enum and is required in requests. ClickHouse
`conditions` are a typed row policy (`ClickStackClickHouseCASLPermissionConditions`:
`connection`, `from` with `database_name`/`table_name`, `where`, and `where_language`,
`sql` or `lucene`). MongoDB `conditions` map field names to a
`ClickStackMongoDBCASLPermissionConditionValue`, either `String` or a
`Regex(ClickStackMongoDBCASLPermissionRegexCondition)` with `$regex` and optional
`$options`; in responses each value stays raw `serde_json::Value`. The
`ClickStackCASLPermissionConditions` alias is removed because the API no longer defines it.
Each new request type has an all-`Option` `*Response` variant.
`TryFrom<ClickStackCASLPermissionResponse> for ClickStackCASLPermission`, and the
same conversion for each arm and its nested ClickHouse conditions, converts a fetched
permission for write-back, naming any missing required field by wire name and passing
`Unknown` through verbatim. A MongoDB condition value that is neither a string nor a
`$regex` object is never dropped: the arm conversion reports it as `conditions`, and the
union conversion writes the permission back verbatim as `Unknown`.

ClickStack dashboard filters are now unions, a breaking change for Rust callers.
`ClickStackFilterInput` (in `ClickStackCreateDashboardRequest`), `ClickStackFilter` (in
`ClickStackUpdateDashboardRequest`) and `ClickStackFilterResponse` (in
`ClickStackDashboardResponse`) were structs; they are now enums dispatched on the required
`type` field, with an `Unknown(serde_json::Value)` catch-all for a `type` this crate does
not know or a payload that does not fit the arm it selects. The arms are the query-expression
filter (`QUERY_EXPRESSION`, the old struct's shape), the static-list filter (`STATIC_LIST`,
a fixed `options` list), the Prometheus-label filter (`PROMETHEUS_LABEL`, a `label` and
optional `match` selector) and the source filter (`SOURCE_FILTER`, a column the source's
`filterSettings` declares, with a required `variable_name`). Create-request arms hold the
`ClickStack*FilterInput` structs; update-request arms hold `ClickStack*Filter`, which adds
the required `id`; response arms hold the all-`Option` `ClickStack*FilterResponse`
structs. Every kind gains `is_global_requirement`, and `min_selections`/`max_selections`
typed as the integer enums `ClickStackFilterMinSelections` (`0` or `1`) and
`ClickStackFilterMaxSelections` (`1`), whose `Unknown(i64)` keeps any other value. The
`ClickStackFilterType`, `ClickStackFilterSourcemetrictype` and `ClickStackFilterWherelanguage`
enums and their `ClickStackFilterInput*` twins are replaced by
`ClickStackQueryExpressionFilterInputType`, `...Sourcemetrictype` and `...Wherelanguage`,
shared by the query-expression input, request and response structs. Wrap a struct literal
in its arm, for example
`ClickStackFilter::ClickStackQueryExpressionFilter(ClickStackQueryExpressionFilter { .. })`,
and `match` a response arm before reading its fields. `TryFrom<ClickStackFilterResponse>
for ClickStackFilter` converts a fetched filter for write-back, naming any missing
required field by wire name and passing `Unknown` through verbatim.

ClickStack heatmap tiles are now a union too: `ClickStackHeatmapChartConfig` and
`ClickStackHeatmapChartConfigResponse` were structs and are now enums dispatched on
`heatmapMode`. The `ClickStackHeatmapDistributionChartConfig` arm keeps the old struct's
fields, renames its enums to `ClickStackHeatmapDistributionChartConfigDisplaytype` and
`...Wherelanguage`, and adds an optional `heatmap_mode`; an absent `heatmapMode` selects
it. The new `ClickStackHeatmapSeriesChartConfig` arm (`heatmapMode: "series"`, required)
colors one row per `group_by` value from ordinary `ClickStackSelectItem`s. A payload
without a `heatmapMode` that carries the series-only `groupBy` lands in `Unknown`. Tile
configs gain a `stacked_line` display type: the `ClickStackStackedLineChartConfig` arm of
`ClickStackTileConfig` (and its `*Response`) is a builder/raw-SQL union over
`ClickStackStackedLineBuilderChartConfig` and `ClickStackStackedLineRawSqlChartConfig`,
dispatched on `configType` like the line chart. `ClickStackTimeChartSeriesDisplaytype`
gains `Stacked_line`. The line builder and raw-SQL charts gain `line_interpolation`
(`smooth`, `linear` or `step`), the raw-SQL number chart gains `color_rules`, tiles
(`ClickStackTileInput`, `ClickStackTileOutput`) gain a `description` tooltip, and
`ClickStackVariableSavedFilterValue` gains `all`, which `TryFrom` carries back. Each new
type has an all-`Option` `*Response` variant; struct-literal callers need the new fields
as `None` or `..Default::default()`.

ClickStack alerts gain inline chart configs, a breaking change for Rust callers. The spec
no longer requires `source`, `channel` or `channels` on `ClickStackCreateAlertRequest` and
`ClickStackUpdateAlertRequest`, so those fields are now `Option<ClickStackCreateAlertRequestSource>`
(or `...UpdateAlertRequestSource`), `Option<ClickStackAlertChannel>` and
`Option<ClickStackAlertChannels>`, and an unset value is omitted from the request instead
of being sent with its default. Wrap existing values in `Some(..)`. The three alert source
enums gain `Inline` (`inline`), for an alert that queries its own `chart_config` instead of
a saved search or dashboard tile. Requests gain `chart_config`
(`Option<ClickStackAlertChartConfig>`), `display_name` and `tags`; `ClickStackAlertResponse`
gains the same fields, with `chart_config` as `ClickStackAlertChartConfigResponse`.
`ClickStackAlertChartConfig` is an enum dispatched on `displayType` (`line`, `stacked_bar`,
`stacked_line` or `number`) whose arms, `ClickStackAlertLineChartConfig`,
`ClickStackAlertBarChartConfig`, `ClickStackAlertStackedLineChartConfig` and
`ClickStackAlertNumberChartConfig`, are builder/raw-SQL unions dispatched on `configType`
like the tile chart configs. Each `ClickStackAlert{Line,Bar,StackedLine,Number}{Builder,RawSql}ChartConfig`
struct carries every field of the matching tile chart config plus the alert overlay's
optional `name`, `where` and `where_language` (`ClickStackAlertChartConfigOverlayWherelanguage`,
`sql` or `lucene`); `ClickStackAlertChartConfigOverlay` models the overlay on its own. Any
other `displayType`, or a payload that does not fit the arm it selects, lands in `Unknown`
verbatim. Each new type has an all-`Option` `*Response` variant.

Postgres services gain IPv4 access lists. `PostgresServicePostRequest.ip_access_list`
takes `Option<Vec<PostgresIpAccessListEntry>>`: `None` omits the field and allows
all addresses on Postgres (5432) and PgBouncer (6432), while `Some(vec![])` sends
`[]` and blocks public access. `PostgresServicePatchRequest.ip_access_list` takes a
`PostgresIpAccessListPatch`, whose `remove` list (`PostgresIpAccessListRuleReference`
IDs) is applied before its `add` list; an `add` entry with an `id` edits that rule
and one without creates a rule. Both lists are required, like `IpAccessListPatch`,
so leave the unused one empty. `PostgresService` reports `ip_access_list` as
all-`Option` `PostgresIpAccessListEntryResponse` entries, which convert back with
`TryFrom` (missing `portRange` or `source` is reported by wire name), and gains
`earliest_restore_time`, the earliest `postgres_instance_restore` target.
`PostgresBackup` gains `status`, a string the API documents as always `ready`.
Struct-literal callers of these types need the new fields as `None` or
`..Default::default()`.

`PgConfig` and `PgConfigResponse` gain 22 Postgres settings: `bgwriter_delay`,
`bgwriter_flush_after`, `bgwriter_lru_maxpages`, `bgwriter_lru_multiplier`,
`default_toast_compression`, `enable_parallel_hash`, `hash_mem_multiplier`,
`io_method`, `io_workers`, `log_lock_waits`, `log_min_duration_statement`,
`log_parameter_max_length`, `log_temp_files`, `logical_decoding_work_mem`,
`maintenance_io_concurrency`, `max_logical_replication_workers`,
`max_replication_slots`, `max_sync_workers_per_subscription`, `max_wal_senders`,
`temp_buffers`, `track_activity_query_size`, and `vacuum_cost_delay`. Numeric
settings are `serde_json::Value`, like the existing ones, because the API accepts a
number or a string with a unit. The four enumerated settings use new value enums,
each with a `VALUES` const and an `Unknown(String)` catch-all:
`PgConfigDefaultToastCompression` (`pglz`, `lz4`), `PgConfigEnableParallelHash`
and `PgConfigLogLockWaits` (`on`, `off`), and `PgConfigIoMethod` (`sync`,
`worker`, `io_uring`). Struct-literal callers of `PgConfig` and
`PgConfigResponse` need the new fields as `None` or `..Default::default()`.

`PgSize` renames its 61 GCP variants to the API's local-SSD size names, a breaking
change for Rust callers. C4A, C4 and C4D sizes gain `lssd` before the vCPU count
(`C4_standard_4` / `c4-standard-4` is now `C4_standard_lssd_4` /
`c4-standard-lssd-4`), and Z3 sizes gain `highmem` (`Z3_highlssd_8` /
`z3-highlssd-8` is now `Z3_highmem_highlssd_8` / `z3-highmem-highlssd-8`). The old
variants are removed without aliases: rename them in code, and a former wire name
now deserializes as `PgSize::Unknown`. AWS sizes are unchanged.

`ServicePostRequest` no longer has `encryption_key` or
`encryption_assumed_role_identifier`, because the spec no longer documents these
customer-managed disk-key fields on the create request. Remove both fields from struct
literals and assignments; no replacement request field exists. `Service` still
reports `encryption_key`, `encryption_assumed_role_identifier`, and
`encryption_role_id` for existing services.

`ClickPipeBigQueryPipeSettings` and `ClickPipeBigQueryPipeSettingsResponse` gain
optional `sync_cron`, a cron expression that schedules CDC syncs in place of
`sync_interval_seconds`; the API rejects a request that sets both, and snapshot
pipes omit it. `TryFrom` carries it back to the request type. Struct-literal
callers need `sync_cron: None` or `..Default::default()`.

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

AWS backup-bucket models now carry `iam_role_external_id` (`iamRoleExternalId`). `AwsBackupBucket` responses tolerate missing and null IDs. `AwsBackupBucketPostRequestV1.iam_role_external_id` is `Option<String>` and is omitted when `None`: the server rejects the field in organizations without the external-ID feature, so it carries an optionality exemption although the spec marks it required. Callers that built it with a `String` now wrap it in `Some`. `AwsBackupBucketProperties` still requires a string. `AwsBackupBucketPatchRequestV1.iam_role_external_id` distinguishes omission (`None`, preserve), a value (`Some(Some(id))`, replace), and explicit null (`Some(None)`, clear). Its deserializer preserves all three states. Regenerated operation metadata also adds `control-plane:service:manage` to `INSTANCE_CREATE`, alongside `control-plane:organization:create-service`.

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

### ClickStack list pagination

`click_stack_list_alerts`, `click_stack_list_webhooks`, and
`click_stack_list_saved_searches` take `limit: Option<i64>` and
`offset: Option<i64>` after the organization and service IDs. Pass `None, None`
for the server defaults (1,000 records, offset zero). For a complete inventory,
request pages with an explicit limit from 1 to 1,000 and advance the offset
until the returned page is shorter than that limit. Existing callers upgrading
to 0.5.0 should add `None, None` to preserve their current request behavior.

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

## Earlier versions

Versions before 0.5.0 did not record library changes.
