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
    /// Request-position fields, as (Rust type, wire name), whose Rust type
    /// deliberately differs from the property schema's shape. Entries become
    /// stale when the field's shape check no longer reports a finding.
    pub request_shape_exemptions: BTreeSet<(String, String)>,
    pub extra_field_exemptions: BTreeSet<(String, String)>,
    pub deprecated_field_exemptions: BTreeSet<(String, String)>,
    pub extra_enum_value_exemptions: BTreeSet<(String, String)>,
    /// Schemas whose upstream `required[]` is non-exhaustive; requiredness
    /// resolution unions the array with the description heuristic. This is
    /// request-position-only semantics: response-position fields are all
    /// `Option<T>` by policy. Entries become stale when the schema is removed,
    /// response-only, or the override no longer changes requiredness.
    pub partial_required_schemas: BTreeSet<String>,
    /// Schemas without `required[]` whose every field is optional in a
    /// request, although the description heuristic marks some required; keyed
    /// by spec schema name. Requiredness resolution treats each listed schema
    /// as all-optional (request position only). Entries become stale when the
    /// schema is removed, response-only, gains `required[]`, or the override no
    /// longer changes requiredness. A schema may not also be partial-required.
    pub optional_request_schemas: BTreeSet<String>,
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
        ]),
        optionality_exemptions: pairs(OPTIONALITY_EXEMPTIONS),
        fractional_response_exemptions: pairs(FRACTIONAL_RESPONSE_EXEMPTIONS),
        request_shape_exemptions: pairs(REQUEST_SHAPE_EXEMPTIONS),
        extra_field_exemptions: BTreeSet::new(),
        deprecated_field_exemptions: BTreeSet::new(),
        extra_enum_value_exemptions: BTreeSet::new(),
        partial_required_schemas: BTreeSet::new(),
        optional_request_schemas: strings(OPTIONAL_REQUEST_SCHEMAS),
        acknowledged_unsupported_enum_pointers: strings(ACKNOWLEDGED_UNSUPPORTED_ENUM_POINTERS),
    }
}

// Request fields whose Rust shape deliberately differs from the spec.
const REQUEST_SHAPE_EXEMPTIONS: &[(&str, &str)] = &[
    // The spec types `level` as a number and lists 0.5/0.9/0.95/0.99 only in
    // prose; the inline enum serializes those as strings. Changing it changes
    // what is sent, so it needs the live API to settle which side is right.
    ("ClickStackSelectItem", "level"),
    // `settings` defaults to a JSON-value map for existing callers; the typed
    // spec contract is opt-in as `ServiceClickhouseSettingsPatchRequest<
    // ServiceClickhouseSettingsMap>`, so the default type argument is wider.
    ("ServiceClickhouseSettingsPatchRequest", "settings"),
];

// Live slow-query list/detail responses contain fractional microsecond durations,
// including avgDurationUs = 1190.8419405320817. The spec now declares the
// average and percentiles as numbers, but max/total durations and execution
// durations are still integers; rounding would destroy valid response data.
// https://github.com/ClickHouse/clickhousectl/issues/758
// Only duration measurements are widened; counts and CPU/JIT counters stay i64.
const FRACTIONAL_RESPONSE_EXEMPTIONS: &[(&str, &str)] = &[
    ("PostgresSlowQueryPattern", "maxDurationUs"),
    ("PostgresSlowQueryPattern", "totalDurationUs"),
    ("PostgresQueryExecution", "durationUs"),
];

// Request schemas with no `required[]` whose every field is optional on the
// wire. The description fallback marks most of their fields required;
// following it would make every valid request invalid.
const OPTIONAL_REQUEST_SCHEMAS: &[&str] = &[
    // The source wrappers carry one property per source kind and are mutually
    // exclusive: a request sends exactly one arm, plus `validateSamples`. Since
    // the spec dropped the `| null` unions, the description fallback marks
    // every arm required. Non-Postgres requests must be able to omit the
    // Postgres arm, and so on for every kind. Live object-storage creates and
    // PATCHes on 2026-09-05 omitted `validateSamples` and succeeded; omission
    // must stay distinct from explicit false.
    "ClickPipePatchSource",
    "ClickPipePostSource",
    "ClickPipeSchemaDiscoverySource",
    // The settings PUT is partial and its keys are source-specific
    // (`streaming_*` for streaming pipes, `object_storage_*` for object
    // storage). `kafka_read_committed` is Kafka-only: the PUT is rejected with
    // "Setting 'kafka_read_committed' is only supported for Kafka ClickPipes"
    // for every other source, so absence is the only way a non-Kafka update
    // can express "does not apply". `object_storage_max_file_size` is also a
    // private-preview setting that orgs must have enabled by support.
    "ClickPipeSettingsPutRequest",
    // Despite its non-Patch name and descriptions that omit the "Optional"
    // prefix, RoleUpdateRequest is used only by PATCH and the operation states
    // that all three fields are optional and only supplied fields are changed.
    "RoleUpdateRequest",
    // Postgres configuration is partial: zero-value defaults fail, and omission
    // selects the server default.
    "pgConfig",
];

const OPTIONALITY_EXEMPTIONS: &[(&str, &str)] = &[
    // The server feature-gates iamRoleExternalId on AWS backup-bucket create:
    // orgs without the feature get "iamRoleExternalId is not available for this
    // organization" for any string and "is invalid" for "". The schema has no
    // required[], so the description fallback marks it required, which would
    // make AWS create impossible in those orgs. Omission is the only valid input.
    // https://github.com/ClickHouse/clickhousectl/issues/1245
    ("AwsBackupBucketPostRequestV1", "iamRoleExternalId"),
    // Kinesis requires protobufSchema only for Protobuf and forbids it for
    // other formats; the schema has no required[], and the legacy requiredness
    // heuristic misses that condition.
    ("ClickPipePostKinesisSource", "protobufSchema"),
    // skipFirstLines applies only to TabSeparated and TabSeparatedWithNames
    // (per its own description); the schema has no required[] and the field is
    // not nullable, so the description fallback marks it required for every
    // format. Omission is how a JSON, CSV or Parquet create leaves it out.
    ("ClickPipePostObjectStorageSource", "skipFirstLines"),
    // The official Terraform provider's merged #618 sends Postgres mapping-only
    // PATCHes with credentials/settings omitted and with only the non-empty
    // add/remove delta. Its maintainers verified this shape against the API:
    // https://github.com/ClickHouse/terraform-provider-clickhouse/pull/618
    ("ClickPipePatchPostgresSource", "credentials"),
    ("ClickPipePatchPostgresSource", "settings"),
    ("ClickPipePatchPostgresSource", "tableMappingsToAdd"),
    ("ClickPipePatchPostgresSource", "tableMappingsToRemove"),
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
    // The official Terraform provider's released Kafka update path omits
    // unchanged credentials and reversePrivateEndpointIds, including for a
    // CA-only update. The API schema also has authentication modes IAM_ROLE
    // (using iamRole) and workload identity with no credentials-union arm:
    // https://github.com/ClickHouse/terraform-provider-clickhouse/blob/7c1d20d485f260f410698a17936cf7793bb5d9e1/internal/service/clickhouse/resource/clickpipe_kafka_update_payload_test.go
    ("ClickPipePatchKafkaSource", "credentials"),
    ("ClickPipePatchKafkaSource", "reversePrivateEndpointIds"),
    // ClickPipeSettingsPutRequest's source-conditional contract also applies to
    // settings nested in a create request: on 2026-09-05 an object-storage create with the key set
    // to false was rejected, while omitting it succeeded.
    ("ClickPipeSettings", "kafka_read_committed"),
    // object_storage_max_file_size is object-storage-only and private preview
    // ("Contact support to enable it"), and can only lower the server's limit,
    // so a create must be able to leave it out. The schema has no required[]
    // and the field is not nullable, unlike its nullable neighbours.
    ("ClickPipeSettings", "object_storage_max_file_size"),
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
];

// The deprecated API-key `roles` fields are feature-gated, frozen legacy API
// replaced by assigned role IDs. They deliberately remain strings to avoid a
// source-breaking change for deprecated-fields consumers.
const ACKNOWLEDGED_UNSUPPORTED_ENUM_POINTERS: &[&str] = &[
    "/components/schemas/ApiKey/properties/roles/items",
    "/components/schemas/ApiKeyPatchRequest/properties/roles/items",
    "/components/schemas/ApiKeyPostRequest/properties/roles/items",
];
