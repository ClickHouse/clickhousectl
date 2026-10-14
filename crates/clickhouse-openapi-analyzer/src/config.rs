use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct AnalyzerConfig {
    /// Enforce ClickHouse Cloud's basicAuth permission convention and the public
    /// operation catalog. Generic OpenAPI analysis leaves this disabled.
    pub check_operation_permissions: bool,
    /// Intentional helpers without an OpenAPI operation. Removed helpers and
    /// helpers that gain a matching operation are reported as stale.
    pub non_openapi_client_methods: BTreeSet<String>,
    /// Fields deliberately kept optional despite the resolved spec. Only
    /// meaningful for request-position usage: optionality findings are
    /// suppressed entirely in response position, so a response-only entry can
    /// never be hit and would surface as a stale exemption.
    pub optionality_exemptions: BTreeSet<(String, String)>,
    /// Response-only integer properties whose verified runtime values require f64.
    /// Entries become stale when the spec, Rust type, or response reachability changes.
    pub fractional_response_exemptions: BTreeSet<(String, String)>,
    pub extra_field_exemptions: BTreeSet<(String, String)>,
    pub deprecated_field_exemptions: BTreeSet<(String, String)>,
    pub extra_enum_value_exemptions: BTreeSet<(String, String)>,
    /// Schemas whose upstream `required[]` is non-exhaustive; requiredness
    /// resolution unions the array with the description heuristic. This is
    /// request-position-only semantics: response-position fields are all
    /// `Option<T>` by policy. Entries become stale when the schema is removed,
    /// response-only, or the override no longer changes requiredness.
    pub partial_required_schemas: BTreeSet<String>,
    /// Unsupported locations whose value sets are acknowledged in the snapshot.
    /// Changes to those sets still produce actionable drift.
    pub acknowledged_unsupported_enum_pointers: BTreeSet<String>,
}

fn strings(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn pairs(values: &[(&str, &str)]) -> BTreeSet<(String, String)> {
    values
        .iter()
        .map(|(left, right)| ((*left).to_string(), (*right).to_string()))
        .collect()
}

/// Canonical comparison policy for the ClickHouse Cloud API library.
pub fn clickhouse_cloud_config() -> AnalyzerConfig {
    AnalyzerConfig {
        check_operation_permissions: true,
        // These query-host routes are implemented by the SQL console service,
        // independently of the management API's published OpenAPI specification.
        non_openapi_client_methods: strings(&[
            "run_query",
            "run_query_bearer",
            "run_postgres_query_bearer",
            // Explicit verified-CLI auth lane of organizationRolesGetList; the
            // administrative method keeps its legacy RBACRole response contract.
            "organization_assignable_roles_get_list",
        ]),
        optionality_exemptions: pairs(OPTIONALITY_EXEMPTIONS),
        fractional_response_exemptions: pairs(FRACTIONAL_RESPONSE_EXEMPTIONS),
        extra_field_exemptions: BTreeSet::new(),
        deprecated_field_exemptions: BTreeSet::new(),
        extra_enum_value_exemptions: BTreeSet::new(),
        partial_required_schemas: BTreeSet::new(),
        acknowledged_unsupported_enum_pointers: strings(ACKNOWLEDGED_UNSUPPORTED_ENUM_POINTERS),
    }
}

// Live slow-query list/detail responses contain fractional microsecond durations,
// including avgDurationUs = 1190.8419405320817. The public spec still declares
// integers (verified 2026-09-08); rounding would destroy valid response data.
// https://github.com/ClickHouse/clickhousectl/issues/758
// Only duration measurements are widened; counts and CPU/JIT counters stay i64.
const FRACTIONAL_RESPONSE_EXEMPTIONS: &[(&str, &str)] = &[
    ("PostgresSlowQueryPattern", "avgDurationUs"),
    ("PostgresSlowQueryPattern", "maxDurationUs"),
    ("PostgresSlowQueryPattern", "p50DurationUs"),
    ("PostgresSlowQueryPattern", "p95DurationUs"),
    ("PostgresSlowQueryPattern", "p99DurationUs"),
    ("PostgresSlowQueryPattern", "totalDurationUs"),
    ("PostgresQueryExecution", "durationUs"),
];

const OPTIONALITY_EXEMPTIONS: &[(&str, &str)] = &[
    // The spec allows protobufSchema only for Protobuf without schemaRegistry;
    // requiring it would reject JSON/Avro and schema-registry Kafka requests.
    ("ClickPipePostKafkaSource", "protobufSchema"),
    // Kinesis likewise requires protobufSchema only for Protobuf and forbids it
    // for other formats; the legacy requiredness heuristic misses that condition.
    ("ClickPipePostKinesisSource", "protobufSchema"),
    // Non-Postgres pipe requests must be able to omit the Postgres union arm.
    ("ClickPipePostSource", "postgres"),
    ("ClickPipePatchSource", "postgres"),
    // A live object-storage PATCH on 2026-09-05 omitted validateSamples and
    // succeeded without changing source or destination state. The nested
    // schema has no required[], but the description fallback marks the field
    // required because it cannot inherit the parent PATCH method.
    ("ClickPipePatchSource", "validateSamples"),
    // The official Terraform provider's merged #618 sends Postgres mapping-only
    // PATCHes with credentials/settings omitted and with only the non-empty
    // add/remove delta. Its maintainers verified this shape against the API:
    // https://github.com/ClickHouse/terraform-provider-clickhouse/pull/618
    ("ClickPipePatchPostgresSource", "credentials"),
    ("ClickPipePatchPostgresSource", "settings"),
    ("ClickPipePatchPostgresSource", "tableMappingsToAdd"),
    ("ClickPipePatchPostgresSource", "tableMappingsToRemove"),
    // The control plane accepted an object-storage create with validateSamples
    // absent on 2026-09-05. Omission must stay distinct from explicit false.
    ("ClickPipePostSource", "validateSamples"),
    // CDC mode and numeric ranges require absence unless meaningful values exist.
    ("ClickPipePostgresPipeSettings", "publicationName"),
    ("ClickPipePostgresPipeSettings", "replicationSlotName"),
    ("ClickPipePostgresPipeSettings", "syncIntervalSeconds"),
    ("ClickPipePostgresPipeSettings", "pullBatchSize"),
    ("ClickPipePostgresPipeSettings", "initialLoadParallelism"),
    (
        "ClickPipePostgresPipeSettings",
        "snapshotNumRowsPerPartition",
    ),
    (
        "ClickPipePostgresPipeSettings",
        "snapshotNumberOfParallelTables",
    ),
    // A Kafka broker that requires no authentication has no representation in
    // the spec enum (PLAIN..MUTUAL_TLS only) and the schema carries no
    // `required[]`, so requiredness resolves from the description heuristic.
    // Absence is the wire representation of "no auth": the control plane's own
    // field is `omitempty` and the equivalent PATCH property is nullable.
    // Keeping this `T` would force every request to claim an auth mechanism.
    ("ClickPipePostKafkaSource", "authentication"),
    // The official Terraform provider's released Kafka update path omits
    // unchanged credentials and reversePrivateEndpointIds, including for a
    // CA-only update. The API schema also has authentication modes IAM_ROLE
    // (using iamRole) and workload identity with no credentials-union arm:
    // https://github.com/ClickHouse/terraform-provider-clickhouse/blob/7c1d20d485f260f410698a17936cf7793bb5d9e1/internal/service/clickhouse/resource/clickpipe_kafka_update_payload_test.go
    ("ClickPipePatchKafkaSource", "credentials"),
    ("ClickPipePatchKafkaSource", "reversePrivateEndpointIds"),
    // `kafka_read_committed` is Kafka-only: the settings PUT is rejected with
    // "Setting 'kafka_read_committed' is only supported for Kafka ClickPipes"
    // for every other source, and the schema carries no `required[]`, so
    // requiredness resolves from the description heuristic. Absence is the only
    // way a non-Kafka settings update can express "does not apply"; keeping this
    // `T` would put the key on every request and break all non-Kafka pipes.
    ("ClickPipeSettingsPutRequest", "kafka_read_committed"),
    // The source wrappers below carry one property per source kind and are
    // mutually exclusive: a request sends exactly one arm (plus
    // `validateSamples`). The schemas have no `required[]` and, since the spec
    // dropped the `| null` unions, the description fallback marks every arm
    // required — following it would make every valid request invalid.
    ("ClickPipePostSource", "bigquery"),
    ("ClickPipePostSource", "kafka"),
    ("ClickPipePostSource", "kinesis"),
    ("ClickPipePostSource", "mongodb"),
    ("ClickPipePostSource", "mysql"),
    ("ClickPipePostSource", "objectStorage"),
    ("ClickPipePostSource", "pubsub"),
    ("ClickPipePatchSource", "kafka"),
    ("ClickPipePatchSource", "kinesis"),
    ("ClickPipePatchSource", "mongodb"),
    ("ClickPipePatchSource", "mysql"),
    ("ClickPipePatchSource", "objectStorage"),
    ("ClickPipePatchSource", "pubsub"),
    ("ClickPipeSchemaDiscoverySource", "kafka"),
    ("ClickPipeSchemaDiscoverySource", "kinesis"),
    ("ClickPipeSchemaDiscoverySource", "objectStorage"),
    ("ClickPipeSchemaDiscoverySource", "pubsub"),
    // The settings PUT is partial and its keys are source-specific
    // (`streaming_*` for streaming pipes, `object_storage_*` for object
    // storage), as with `kafka_read_committed` above. The schema has no
    // `required[]`, so the description fallback marks every key required;
    // requiring all of them would make every valid settings update invalid.
    (
        "ClickPipeSettingsPutRequest",
        "clickhouse_max_download_threads",
    ),
    (
        "ClickPipeSettingsPutRequest",
        "clickhouse_max_insert_threads",
    ),
    ("ClickPipeSettingsPutRequest", "clickhouse_max_threads"),
    (
        "ClickPipeSettingsPutRequest",
        "clickhouse_min_insert_block_size_bytes",
    ),
    (
        "ClickPipeSettingsPutRequest",
        "clickhouse_parallel_distributed_insert_select",
    ),
    (
        "ClickPipeSettingsPutRequest",
        "clickhouse_parallel_view_processing",
    ),
    ("ClickPipeSettingsPutRequest", "object_storage_concurrency"),
    (
        "ClickPipeSettingsPutRequest",
        "object_storage_max_file_count",
    ),
    (
        "ClickPipeSettingsPutRequest",
        "object_storage_max_insert_bytes",
    ),
    (
        "ClickPipeSettingsPutRequest",
        "object_storage_polling_interval_ms",
    ),
    (
        "ClickPipeSettingsPutRequest",
        "object_storage_use_cluster_function",
    ),
    (
        "ClickPipeSettingsPutRequest",
        "streaming_max_insert_wait_ms",
    ),
    // The same source-conditional contract applies to settings nested in a
    // create request: on 2026-09-05 an object-storage create with the key set
    // to false was rejected, while omitting it succeeded.
    ("ClickPipeSettings", "kafka_read_committed"),
    // Empty TLS/IAM strings fail validation for sources that do not use them.
    ("ClickPipeMutatePostgresSource", "caCertificate"),
    ("ClickPipeMutatePostgresSource", "iamRole"),
    ("ClickPipeMutatePostgresSource", "tlsHost"),
    // IAM_ROLE authentication has no username or password: `iamRole` is the
    // whole credential. The schema carries no `required[]`, so requiredness
    // resolves from the description heuristic and `credentials` reads as
    // required, but an IAM_ROLE pipe has nothing to put in it and the sibling
    // MySQL schema (which does carry `required[]`) leaves it out. Keeping this
    // `T` would force every IAM_ROLE request to send an empty username and
    // password pair.
    ("ClickPipeMutatePostgresSource", "credentials"),
    // ClickStackAlertChartConfigOverlay is an `allOf` mixin that every alert
    // chart config variant adds to its chart config; the spec never uses it on
    // its own. It carries no `required[]`, so the description fallback marks
    // all three fields required, but the composed variants require only their
    // chart config's fields, and the schema's own `where` description says the
    // field is rejected on raw-SQL variants, so it cannot be required. The
    // standalone struct keeps the optionality the variants carry.
    ("ClickStackAlertChartConfigOverlay", "name"),
    ("ClickStackAlertChartConfigOverlay", "where"),
    ("ClickStackAlertChartConfigOverlay", "whereLanguage"),
    // Deprecated request fields are feature-gated out in favour of replacements.
    ("OrganizationPrivateEndpointsPatch", "add"),
    // Despite its non-Patch name and descriptions that omit the "Optional"
    // prefix, RoleUpdateRequest is used only by PATCH and the operation states
    // that all three fields are optional and only supplied fields are changed.
    ("RoleUpdateRequest", "actors"),
    ("RoleUpdateRequest", "name"),
    ("RoleUpdateRequest", "policies"),
    // PgConfig is partial: zero-value defaults fail, omission selects server defaults.
    ("PgConfig", "autovacuum_analyze_scale_factor"),
    ("PgConfig", "autovacuum_analyze_threshold"),
    ("PgConfig", "autovacuum_freeze_max_age"),
    ("PgConfig", "autovacuum_max_workers"),
    ("PgConfig", "autovacuum_naptime"),
    ("PgConfig", "autovacuum_vacuum_cost_delay"),
    ("PgConfig", "autovacuum_vacuum_cost_limit"),
    ("PgConfig", "autovacuum_vacuum_insert_scale_factor"),
    ("PgConfig", "autovacuum_vacuum_insert_threshold"),
    ("PgConfig", "autovacuum_vacuum_scale_factor"),
    ("PgConfig", "autovacuum_vacuum_threshold"),
    ("PgConfig", "autovacuum_work_mem"),
    ("PgConfig", "bgwriter_delay"),
    ("PgConfig", "bgwriter_flush_after"),
    ("PgConfig", "bgwriter_lru_maxpages"),
    ("PgConfig", "bgwriter_lru_multiplier"),
    ("PgConfig", "checkpoint_timeout"),
    ("PgConfig", "default_toast_compression"),
    ("PgConfig", "default_transaction_isolation"),
    ("PgConfig", "effective_cache_size"),
    ("PgConfig", "effective_io_concurrency"),
    ("PgConfig", "enable_parallel_hash"),
    ("PgConfig", "hash_mem_multiplier"),
    ("PgConfig", "idle_in_transaction_session_timeout"),
    ("PgConfig", "idle_session_timeout"),
    ("PgConfig", "io_method"),
    ("PgConfig", "io_workers"),
    ("PgConfig", "lock_timeout"),
    ("PgConfig", "log_lock_waits"),
    ("PgConfig", "log_min_duration_statement"),
    ("PgConfig", "log_parameter_max_length"),
    ("PgConfig", "log_temp_files"),
    ("PgConfig", "logical_decoding_work_mem"),
    ("PgConfig", "maintenance_io_concurrency"),
    ("PgConfig", "maintenance_work_mem"),
    ("PgConfig", "max_connections"),
    ("PgConfig", "max_logical_replication_workers"),
    ("PgConfig", "max_parallel_maintenance_workers"),
    ("PgConfig", "max_parallel_workers"),
    ("PgConfig", "max_parallel_workers_per_gather"),
    ("PgConfig", "max_replication_slots"),
    ("PgConfig", "max_slot_wal_keep_size"),
    ("PgConfig", "max_sync_workers_per_subscription"),
    ("PgConfig", "max_wal_senders"),
    ("PgConfig", "max_wal_size"),
    ("PgConfig", "max_worker_processes"),
    ("PgConfig", "min_wal_size"),
    ("PgConfig", "random_page_cost"),
    ("PgConfig", "shared_memory_percent"),
    ("PgConfig", "ssl_min_protocol_version"),
    ("PgConfig", "statement_timeout"),
    ("PgConfig", "temp_buffers"),
    ("PgConfig", "track_activity_query_size"),
    ("PgConfig", "transaction_timeout"),
    ("PgConfig", "vacuum_cost_delay"),
    ("PgConfig", "wal_buffers"),
    ("PgConfig", "wal_compression"),
    ("PgConfig", "wal_keep_size"),
    ("PgConfig", "wal_sender_timeout"),
    ("PgConfig", "work_mem"),
];

// The deprecated API-key `roles` fields are feature-gated, frozen legacy API
// replaced by assigned role IDs. They deliberately remain strings to avoid a
// source-breaking change for deprecated-fields consumers.
const ACKNOWLEDGED_UNSUPPORTED_ENUM_POINTERS: &[&str] = &[
    "/components/schemas/ApiKey/properties/roles/items",
    "/components/schemas/ApiKeyPatchRequest/properties/roles/items",
    "/components/schemas/ApiKeyPostRequest/properties/roles/items",
];
