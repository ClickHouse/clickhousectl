use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct AnalyzerConfig {
    pub non_openapi_client_methods: BTreeSet<String>,
    pub optionality_exemptions: BTreeSet<(String, String)>,
    pub extra_field_exemptions: BTreeSet<(String, String)>,
    pub deprecated_field_exemptions: BTreeSet<(String, String)>,
    pub extra_enum_value_exemptions: BTreeSet<(String, String)>,
    pub partial_required_schemas: BTreeSet<String>,
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
        non_openapi_client_methods: strings(&["run_query", "run_query_bearer"]),
        optionality_exemptions: pairs(OPTIONALITY_EXEMPTIONS),
        extra_field_exemptions: BTreeSet::new(),
        deprecated_field_exemptions: BTreeSet::new(),
        extra_enum_value_exemptions: BTreeSet::new(),
        partial_required_schemas: strings(&["Service", "ServiceScalingPatchResponse"]),
        acknowledged_unsupported_enum_pointers: strings(ACKNOWLEDGED_UNSUPPORTED_ENUM_POINTERS),
    }
}

const OPTIONALITY_EXEMPTIONS: &[(&str, &str)] = &[
    // The legacy service-create schema marks almost every property required,
    // but the API requires only name/provider/region and rejects many defaults.
    ("ServicePostRequest", "autoscalingMode"),
    ("ServicePostRequest", "byocId"),
    ("ServicePostRequest", "complianceType"),
    ("ServicePostRequest", "dataWarehouseId"),
    ("ServicePostRequest", "enableCoreDumps"),
    ("ServicePostRequest", "endpoints"),
    ("ServicePostRequest", "hasTransparentDataEncryption"),
    ("ServicePostRequest", "idleScaling"),
    ("ServicePostRequest", "idleTimeoutMinutes"),
    ("ServicePostRequest", "isReadonly"),
    ("ServicePostRequest", "maxReplicaMemoryGb"),
    ("ServicePostRequest", "maxTotalMemoryGb"),
    ("ServicePostRequest", "minReplicaMemoryGb"),
    ("ServicePostRequest", "minTotalMemoryGb"),
    ("ServicePostRequest", "numReplicas"),
    ("ServicePostRequest", "maxReplicas"),
    ("ServicePostRequest", "minReplicas"),
    ("ServicePostRequest", "privateEndpointIds"),
    ("ServicePostRequest", "privatePreviewTermsChecked"),
    ("ServicePostRequest", "profile"),
    ("ServicePostRequest", "releaseChannel"),
    ("ServicePostRequest", "tags"),
    ("ServicePostRequest", "tier"),
    // Non-Postgres pipe requests must be able to omit the Postgres union arm.
    ("ClickPipePostSource", "postgres"),
    ("ClickPipePatchSource", "postgres"),
    // Empty/default scaling and settings objects fail server-side validation.
    ("ClickPipePostRequest", "scaling"),
    ("ClickPipePostRequest", "settings"),
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
    // Database pipes do not have table-shaped destinations.
    ("ClickPipeMutateDestination", "table"),
    ("ClickPipeMutateDestination", "managedTable"),
    ("ClickPipeMutateDestination", "tableDefinition"),
    // Empty TLS/IAM strings fail validation for sources that do not use them.
    ("ClickPipeMutatePostgresSource", "caCertificate"),
    ("ClickPipeMutatePostgresSource", "iamRole"),
    ("ClickPipeMutatePostgresSource", "tlsHost"),
    // Deprecated roles and opt-in pre-hashed keys must stay off the wire when unused.
    ("ApiKeyPostRequest", "roles"),
    ("ApiKeyPostRequest", "hashData"),
    // Deprecated request fields are feature-gated out in favour of replacements.
    ("InvitationPostRequest", "role"),
    ("OrganizationPrivateEndpointsPatch", "add"),
    // PgConfig is partial: zero-value defaults fail, omission selects server defaults.
    ("PgConfig", "autovacuum_analyze_scale_factor"),
    ("PgConfig", "autovacuum_max_workers"),
    ("PgConfig", "autovacuum_naptime"),
    ("PgConfig", "autovacuum_vacuum_cost_delay"),
    ("PgConfig", "autovacuum_vacuum_cost_limit"),
    ("PgConfig", "autovacuum_vacuum_insert_scale_factor"),
    ("PgConfig", "autovacuum_vacuum_scale_factor"),
    ("PgConfig", "autovacuum_work_mem"),
    ("PgConfig", "default_transaction_isolation"),
    ("PgConfig", "effective_cache_size"),
    ("PgConfig", "effective_io_concurrency"),
    ("PgConfig", "idle_in_transaction_session_timeout"),
    ("PgConfig", "idle_session_timeout"),
    ("PgConfig", "lock_timeout"),
    ("PgConfig", "maintenance_work_mem"),
    ("PgConfig", "max_connections"),
    ("PgConfig", "max_parallel_maintenance_workers"),
    ("PgConfig", "max_parallel_workers"),
    ("PgConfig", "max_parallel_workers_per_gather"),
    ("PgConfig", "max_slot_wal_keep_size"),
    ("PgConfig", "max_wal_size"),
    ("PgConfig", "max_worker_processes"),
    ("PgConfig", "min_wal_size"),
    ("PgConfig", "random_page_cost"),
    ("PgConfig", "ssl_min_protocol_version"),
    ("PgConfig", "statement_timeout"),
    ("PgConfig", "transaction_timeout"),
    ("PgConfig", "wal_compression"),
    ("PgConfig", "wal_keep_size"),
    ("PgConfig", "wal_sender_timeout"),
    ("PgConfig", "work_mem"),
];

// Follow-up remediation is tracked in #296. Remove entries as their Rust API
// types become checkable; stale acknowledgements are actionable findings.
const ACKNOWLEDGED_UNSUPPORTED_ENUM_POINTERS: &[&str] = &[
    "/components/schemas/ApiKey/properties/roles/items",
    "/components/schemas/ApiKeyPatchRequest/properties/roles/items",
    "/components/schemas/ApiKeyPostRequest/properties/roles/items",
    "/components/schemas/ByocInfrastructurePostRequest/properties/availabilityZoneSuffixes/items",
    "/components/schemas/InstanceServiceQueryApiEndpointsPostRequest/properties/roles/items",
    "/components/schemas/ServiceQueryAPIEndpoint/properties/roles/items",
    "/components/schemas/UpgradeWindow/properties/duration",
    "/components/schemas/UpgradeWindow/properties/startHourUtc",
    "/components/schemas/UpgradeWindowPutRequest/properties/startHourUtc",
    "/paths/~1v1~1organizations~1{organizationId}~1postgres~1{postgresId}~1slowQueryPatterns/get/parameters/8/schema",
    "/paths/~1v1~1organizations~1{organizationId}~1postgres~1{postgresId}~1slowQueryPatterns/get/parameters/9/schema",
];
