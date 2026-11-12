use crate::cloud::clickpipes::cli::{
    ClickPipeCreateValidationArgs, DatabaseDestinationArgs, DestinationRoleArgs,
};
use crate::cloud::clickpipes::create::{
    apply_create_request_args, build_create_validation_args, print_created,
};
use crate::cloud::clickpipes::destination::{build_destination, build_destination_roles};
use crate::cloud::clickpipes::parse_enum;
use crate::cloud::clickpipes::sources::{
    AuthenticationValueParser, GcpAuthentication, read_gcp_service_account_file,
};
use crate::cloud::clickpipes::table_mapping::{
    parse_bigquery_table_mapping_json, parse_postgres_table_mapping,
    parse_postgres_table_mapping_parts,
};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::shared::resolve_org_id;
use clap::builder::{NonEmptyStringValueParser, PossibleValuesParser};
use clap::{ArgGroup, Args};
use clickhouse_cloud_api::models::ClickPipeBigQueryPipeSettingsReplicationmode;

fn parse_bigquery_authentication(value: &str) -> CloudResult<GcpAuthentication> {
    use clickhouse_cloud_api::models::{
        ClickPipePostBigQueryServiceAccountSourceAuthentication as ServiceAccountAuth,
        ClickPipePostBigQueryWorkloadIdentitySourceAuthentication as WorkloadIdentityAuth,
    };

    let json = serde_json::Value::String(value.to_string());
    match serde_json::from_value::<ServiceAccountAuth>(json.clone())? {
        ServiceAccountAuth::ServiceAccount => Ok(GcpAuthentication::ServiceAccount),
        ServiceAccountAuth::Unknown(_) => {
            match serde_json::from_value::<WorkloadIdentityAuth>(json)? {
                WorkloadIdentityAuth::ServiceAccountWorkloadIdentity => {
                    Ok(GcpAuthentication::WorkloadIdentity)
                }
                WorkloadIdentityAuth::Unknown(value) => Err(CloudError::usage(format!(
                    "unknown BigQuery authentication method '{value}'"
                ))),
            }
        }
    }
}

pub(in crate::cloud::clickpipes) fn parse_supported_bigquery_auth(
    value: &str,
) -> Result<String, String> {
    parse_bigquery_authentication(value)
        .map(|_| value.to_string())
        .map_err(|error| error.message)
}

#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("bigquery_table_mappings")
        .required(true)
        .multiple(true)
        .args(["table_mappings", "table_mappings_json"])
))]
pub struct BigQueryCreateArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub validation: ClickPipeCreateValidationArgs,

    /// Authentication method
    #[arg(
        long,
        default_value = "SERVICE_ACCOUNT",
        value_parser = AuthenticationValueParser { parse: parse_supported_bigquery_auth, values: &["SERVICE_ACCOUNT", "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"] },
    )]
    pub auth: String,

    /// Path to a GCP service account JSON key file, or - to read it from stdin
    ///
    /// Required with --auth SERVICE_ACCOUNT and invalid with workload identity.
    #[arg(long, value_name = "PATH")]
    pub service_account_file: Option<String>,

    /// GCP project ID that owns the BigQuery resources
    ///
    /// Required with --auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY.
    #[arg(long)]
    pub project_id: Option<String>,

    /// GCS staging path for snapshot data
    #[arg(long)]
    pub staging_path: String,

    /// Table mappings as dataset.table:target_table (repeatable)
    ///
    /// Leaves every other per-table option at the ClickPipes default.
    #[arg(
        long = "table-mapping",
        value_name = "DATASET.TABLE:TARGET_TABLE",
        value_parser = parse_postgres_table_mapping
    )]
    pub table_mappings: Vec<String>,

    /// Full table mapping as a JSON object (repeatable)
    ///
    /// Supports sorting, exclusions, engines and CDC watermark/event fields.
    /// Combinable with --table-mapping; unknown fields are rejected.
    #[arg(long = "table-mapping-json", value_name = "JSON")]
    pub table_mappings_json: Vec<String>,

    /// Replication mode
    #[arg(
        long,
        default_value = "snapshot",
        value_parser = PossibleValuesParser::new(ClickPipeBigQueryPipeSettingsReplicationmode::VALUES),
    )]
    pub replication_mode: String,

    /// CDC method (required with --replication-mode cdc or cdc_only)
    ///
    /// Each table needs queryCdcWatermarkColumn for query_based or
    /// eventsFunction for events_based in --table-mapping-json.
    #[arg(long, value_parser = ["query_based", "events_based"])]
    pub replication_method: Option<String>,

    /// CDC sync interval in seconds (not with snapshot replication)
    #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
    pub sync_interval_seconds: Option<i64>,

    /// CDC sync schedule as a cron expression (not with snapshot replication)
    ///
    /// 5-field cron or a descriptor such as @hourly, at most once a minute.
    /// UTC unless prefixed with CRON_TZ=<zone>. Replaces --sync-interval-seconds.
    #[arg(long, value_name = "CRON", conflicts_with = "sync_interval_seconds")]
    pub sync_cron: Option<String>,

    /// Parallel table pulls per CDC sync (not with snapshot replication)
    #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
    pub query_cdc_pull_sync_parallelism: Option<i64>,

    /// CDC visibility delay in seconds (not with snapshot replication)
    #[arg(long, value_parser = clap::value_parser!(i64).range(0..))]
    pub sync_delay_seconds: Option<i64>,

    /// Maximum CDC pull window in seconds (not with snapshot replication)
    #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
    pub pull_window_size_seconds: Option<i64>,

    /// Preserve BigQuery nullability in destination columns
    #[arg(long, value_name = "true|false")]
    pub allow_nullable_columns: Option<bool>,

    /// Parallel workers during the initial snapshot
    #[arg(
        long,
        value_name = "WORKERS",
        value_parser = parse_finite_f64,
        allow_hyphen_values = true
    )]
    pub initial_load_parallelism: Option<f64>,

    /// Rows per partition during the snapshot
    #[arg(
        long,
        value_name = "ROWS",
        value_parser = parse_finite_f64,
        allow_hyphen_values = true
    )]
    pub snapshot_rows_per_partition: Option<f64>,

    /// Tables to snapshot in parallel
    #[arg(
        long,
        value_name = "TABLES",
        value_parser = parse_finite_f64,
        allow_hyphen_values = true
    )]
    pub snapshot_parallel_tables: Option<f64>,

    #[command(flatten)]
    pub destination: DatabaseDestinationArgs,

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,
}

fn parse_finite_f64(value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|error| format!("expected a number: {error}"))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("expected a finite number".into())
    }
}

pub(in crate::cloud::clickpipes) fn validate_bigquery_create_args(
    args: &BigQueryCreateArgs,
) -> CloudResult<()> {
    for (flag, value) in [
        ("--initial-load-parallelism", args.initial_load_parallelism),
        (
            "--snapshot-rows-per-partition",
            args.snapshot_rows_per_partition,
        ),
        ("--snapshot-parallel-tables", args.snapshot_parallel_tables),
    ] {
        if value.is_some_and(|value| !value.is_finite()) {
            return Err(CloudError::usage(format!(
                "invalid {flag}: expected a finite number"
            )));
        }
    }

    for (flag, value, minimum) in [
        ("--sync-interval-seconds", args.sync_interval_seconds, 1),
        (
            "--query-cdc-pull-sync-parallelism",
            args.query_cdc_pull_sync_parallelism,
            1,
        ),
        ("--sync-delay-seconds", args.sync_delay_seconds, 0),
        (
            "--pull-window-size-seconds",
            args.pull_window_size_seconds,
            1,
        ),
    ] {
        if value.is_some_and(|value| value < minimum) {
            return Err(CloudError::usage(format!(
                "{flag} must be at least {minimum}"
            )));
        }
        if args.replication_mode == "snapshot" && value.is_some() {
            return Err(CloudError::usage(format!(
                "{flag} cannot be used with --replication-mode snapshot"
            )));
        }
    }
    if args.replication_mode == "snapshot" && args.sync_cron.is_some() {
        return Err(CloudError::usage(
            "--sync-cron cannot be used with --replication-mode snapshot",
        ));
    }
    match (
        args.replication_mode.as_str(),
        args.replication_method.as_deref(),
    ) {
        ("snapshot", Some(_)) => {
            return Err(CloudError::usage(
                "--replication-method cannot be used with --replication-mode snapshot",
            ));
        }
        ("cdc" | "cdc_only", None) => {
            return Err(CloudError::usage(
                "--replication-method is required with --replication-mode cdc or cdc_only",
            ));
        }
        (_, Some(method)) if !["query_based", "events_based"].contains(&method) => {
            return Err(CloudError::usage(
                "--replication-method must be query_based or events_based",
            ));
        }
        _ => {}
    }

    let authentication = parse_bigquery_authentication(&args.auth)?;
    match (
        authentication,
        args.service_account_file.as_deref(),
        args.project_id.as_deref(),
    ) {
        (GcpAuthentication::ServiceAccount, None, _) => {
            return Err(CloudError::usage(
                "--auth SERVICE_ACCOUNT requires --service-account-file",
            ));
        }
        (GcpAuthentication::WorkloadIdentity, Some(_), _) => {
            return Err(CloudError::usage(
                "--service-account-file cannot be used with --auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            ));
        }
        (GcpAuthentication::WorkloadIdentity, None, None) => {
            return Err(CloudError::usage(
                "--auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY requires --project-id",
            ));
        }
        _ => {}
    }

    resolve_bigquery_table_mappings(args)?;
    Ok(())
}

fn resolve_bigquery_table_mappings(
    args: &BigQueryCreateArgs,
) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeBigQueryPipeTableMapping>> {
    use clickhouse_cloud_api::models::ClickPipeBigQueryPipeTableMapping;

    if args.table_mappings.is_empty() && args.table_mappings_json.is_empty() {
        return Err(CloudError::usage(
            "at least one --table-mapping <DATASET.TABLE:TARGET_TABLE> or \
             --table-mapping-json <JSON> is required",
        ));
    }
    let mut mappings =
        Vec::with_capacity(args.table_mappings.len() + args.table_mappings_json.len());
    for mapping in &args.table_mappings {
        let (source_dataset_name, source_table, target_table) =
            parse_postgres_table_mapping_parts(mapping)?;
        mappings.push(ClickPipeBigQueryPipeTableMapping {
            source_dataset_name,
            source_table,
            target_table,
            excluded_columns: None,
            sorting_keys: None,
            table_engine: None,
            use_custom_sorting_key: None,
            events_function: None,
            query_cdc_watermark_column: None,
        });
    }
    for (position, mapping) in args.table_mappings_json.iter().enumerate() {
        mappings.push(parse_bigquery_table_mapping_json(position, mapping)?);
    }
    for (position, mapping) in mappings.iter().enumerate() {
        let watermark = mapping.query_cdc_watermark_column.is_some();
        let events = mapping.events_function.is_some();
        let valid = match args.replication_method.as_deref() {
            Some("query_based") => watermark && !events,
            Some("events_based") => events && !watermark,
            _ => !watermark && !events,
        };
        if !valid {
            return Err(CloudError::usage(format!(
                "table mapping #{}: query_based requires queryCdcWatermarkColumn only; events_based requires eventsFunction only; snapshot allows neither",
                position + 1,
            )));
        }
    }
    Ok(mappings)
}

fn build_bigquery_request(
    args: &BigQueryCreateArgs,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostRequest> {
    use clickhouse_cloud_api::models::{
        ClickPipeBigQueryPipeSettings, ClickPipePostBigQueryServiceAccountSource,
        ClickPipePostBigQueryWorkloadIdentitySource,
        ClickPipePostBigQueryWorkloadIdentitySourceAuthentication, ClickPipePostRequest,
        ClickPipePostSource, ServiceAccount,
    };

    let request_args = build_create_validation_args(&args.validation);
    validate_bigquery_create_args(args)?;
    let authentication = parse_bigquery_authentication(&args.auth)?;
    let table_mappings = resolve_bigquery_table_mappings(args)?;

    let settings = ClickPipeBigQueryPipeSettings {
        replication_mode: parse_enum(&args.replication_mode)?,
        allow_nullable_columns: args.allow_nullable_columns,
        initial_load_parallelism: args.initial_load_parallelism,
        snapshot_num_rows_per_partition: args.snapshot_rows_per_partition,
        snapshot_number_of_parallel_tables: args.snapshot_parallel_tables,
        replication_method: args.replication_method.clone(),
        sync_interval_seconds: args.sync_interval_seconds,
        sync_cron: args.sync_cron.clone(),
        query_cdc_pull_sync_parallelism: args.query_cdc_pull_sync_parallelism,
        sync_delay_seconds: args.sync_delay_seconds,
        pull_window_size_seconds: args.pull_window_size_seconds,
    };
    let source = match authentication {
        GcpAuthentication::ServiceAccount => {
            let path = args.service_account_file.as_deref().ok_or_else(|| {
                CloudError::usage("--auth SERVICE_ACCOUNT requires --service-account-file")
            })?;
            ClickPipePostBigQueryServiceAccountSource {
                authentication: None,
                project_id: args.project_id.clone(),
                credentials: ServiceAccount {
                    service_account_file: read_gcp_service_account_file(path)?,
                },
                snapshot_staging_path: args.staging_path.clone(),
                settings,
                table_mappings,
            }
            .into()
        }
        GcpAuthentication::WorkloadIdentity => ClickPipePostBigQueryWorkloadIdentitySource {
            authentication:
                ClickPipePostBigQueryWorkloadIdentitySourceAuthentication::ServiceAccountWorkloadIdentity,
            project_id: args.project_id.clone().ok_or_else(|| {
                CloudError::usage(
                    "--auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY requires --project-id",
                )
            })?,
            snapshot_staging_path: args.staging_path.clone(),
            settings,
            table_mappings,
        }
        .into(),
    };

    let mut request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            bigquery: Some(source),
            kafka: None,
            kinesis: None,
            mongodb: None,
            mysql: None,
            object_storage: None,
            postgres: None,
            pubsub: None,
            validate_samples: None,
        },
        destination: build_destination(
            &args.destination.destination_database,
            "",
            vec![],
            build_destination_roles(&args.destination_roles.roles),
        ),
        field_mappings: None,
        scaling: None,
        settings: None,
        start_paused: None,
    };
    apply_create_request_args(&mut request, request_args);

    Ok(request)
}

pub(in crate::cloud::clickpipes) async fn clickpipe_create_bigquery(
    client: &CloudClient,
    args: &BigQueryCreateArgs,
    json: bool,
) -> CloudResult<()> {
    let request = build_bigquery_request(args)?;
    let org_id = resolve_org_id(client).await?;

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::clickpipes::cli::{ClickPipeCommands, ClickPipeCreateCommands};
    use crate::cloud::clickpipes::test_support::{
        MAXIMAL_BIGQUERY_TABLE_MAPPING_JSON, assert_bigquery_value, assert_names, assert_rejected,
        clickpipe_parse_error, names_word, parse_clickpipe,
    };
    use clickhouse_cloud_api::models::ClickPipeBigQueryPipeTableMappingTableengine;

    #[test]
    fn parses_bigquery_flags_defaults_and_repeatability() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::BigQuery(args),
        } = parse_clickpipe(&[
            "create",
            "bigquery",
            "svc-1",
            "--name",
            "pipe-1",
            "--service-account-file",
            "/tmp/account.json",
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping",
            "dataset.one:one",
            "--table-mapping",
            "dataset.two:two",
            "--table-mapping-json",
            r#"{"sourceDatasetName":"dataset","sourceTable":"three","targetTable":"three"}"#,
            "--replication-mode",
            "snapshot",
            "--allow-nullable-columns",
            "false",
            "--initial-load-parallelism",
            "2.5",
            "--snapshot-rows-per-partition",
            "1000000",
            "--snapshot-parallel-tables",
            "3",
            "--destination-database",
            "analytics",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected bigquery create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.auth, "SERVICE_ACCOUNT");
        assert_eq!(
            args.service_account_file.as_deref(),
            Some("/tmp/account.json")
        );
        assert_eq!(args.project_id, None);
        assert_eq!(args.staging_path, "gs://bucket/staging");
        assert_eq!(args.table_mappings, ["dataset.one:one", "dataset.two:two"]);
        assert_eq!(args.table_mappings_json.len(), 1);
        assert_eq!(args.replication_mode, "snapshot");
        assert_eq!(args.allow_nullable_columns, Some(false));
        assert_eq!(args.initial_load_parallelism, Some(2.5));
        assert_eq!(args.snapshot_rows_per_partition, Some(1_000_000.0));
        assert_eq!(args.snapshot_parallel_tables, Some(3.0));
        assert_eq!(args.destination.destination_database, "analytics");

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::BigQuery(args),
        } = parse_clickpipe(&[
            "create",
            "bigquery",
            "svc-1",
            "--name",
            "pipe-1",
            "--service-account-file",
            "/tmp/account.json",
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping-json",
            r#"{"sourceDatasetName":"dataset","sourceTable":"events","targetTable":"events"}"#,
        ])
        else {
            panic!("expected bigquery create");
        };
        assert!(args.table_mappings.is_empty());
        assert_eq!(args.table_mappings_json.len(), 1);
        assert_eq!(args.replication_mode, "snapshot");
        assert_eq!(args.allow_nullable_columns, None);
        assert_eq!(args.initial_load_parallelism, None);
        assert_eq!(args.snapshot_rows_per_partition, None);
        assert_eq!(args.snapshot_parallel_tables, None);
        assert_eq!(args.destination.destination_database, "default");
    }

    #[test]
    fn bigquery_tuning_numbers_require_finite_values() {
        for (flag, value) in [
            ("--initial-load-parallelism", "NaN"),
            ("--snapshot-rows-per-partition", "inf"),
            ("--snapshot-parallel-tables", "-inf"),
        ] {
            let error = clickpipe_parse_error(&[
                "create",
                "bigquery",
                "svc-1",
                "--name",
                "pipe-1",
                "--service-account-file",
                "/tmp/account.json",
                "--staging-path",
                "gs://bucket/staging",
                "--table-mapping",
                "dataset.events:events",
                flag,
                value,
            ]);
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            let message = error.to_string();
            assert_names(&message, &[flag, &format!("'{value}'")]);
        }

        assert_eq!(parse_finite_f64("-1.5"), Ok(-1.5));
        assert_eq!(parse_finite_f64("2.5"), Ok(2.5));
        for flag in [
            "--initial-load-parallelism",
            "--snapshot-rows-per-partition",
            "--snapshot-parallel-tables",
        ] {
            assert_bigquery_value(flag, "-1.5");
        }
    }

    fn bigquery_builder_args(service_account_file: String) -> BigQueryCreateArgs {
        BigQueryCreateArgs {
            service_id: "svc-1".into(),
            name: "pipe-1".into(),
            validation: ClickPipeCreateValidationArgs::default(),
            auth: "SERVICE_ACCOUNT".into(),
            service_account_file: Some(service_account_file),
            project_id: None,
            staging_path: "gs://bucket/staging".into(),
            table_mappings: vec!["source.events:events".into()],
            table_mappings_json: vec![],
            replication_mode: "snapshot".into(),
            replication_method: None,
            sync_interval_seconds: None,
            sync_cron: None,
            query_cdc_pull_sync_parallelism: None,
            sync_delay_seconds: None,
            pull_window_size_seconds: None,
            allow_nullable_columns: None,
            initial_load_parallelism: None,
            snapshot_rows_per_partition: None,
            snapshot_parallel_tables: None,
            destination: DatabaseDestinationArgs {
                destination_database: "default".into(),
            },
            destination_roles: DestinationRoleArgs::default(),
        }
    }

    #[test]
    fn build_bigquery_request_supports_minimal_fields() {
        let directory = tempfile::tempdir().unwrap();
        let service_account = directory.path().join("service-account.json");
        std::fs::write(&service_account, "{}").unwrap();
        let args = bigquery_builder_args(service_account.to_string_lossy().into_owned());

        let request = build_bigquery_request(&args).unwrap();

        assert_eq!(request.destination.database, "default");
        assert_eq!(request.destination.table, None);
        assert_eq!(request.destination.roles, None);
        let source = request.source.bigquery.as_ref().expect("bigquery source");
        let clickhouse_cloud_api::models::ClickPipeMutateBigQuerySource::ClickPipePostBigQueryServiceAccountSource(source) = source else {
            panic!("expected service-account source");
        };
        assert_eq!(source.snapshot_staging_path, "gs://bucket/staging");
        assert_eq!(source.table_mappings.len(), 1);
        assert_eq!(source.table_mappings[0].source_dataset_name, "source");
        assert_eq!(source.table_mappings[0].source_table, "events");
        assert_eq!(source.table_mappings[0].target_table, "events");
        assert_eq!(source.settings.replication_mode.to_string(), "snapshot");
        assert_eq!(source.settings.allow_nullable_columns, None);
        assert_eq!(source.settings.initial_load_parallelism, None);
        assert_eq!(source.settings.snapshot_num_rows_per_partition, None);
        assert_eq!(source.settings.snapshot_number_of_parallel_tables, None);
        assert_eq!(source.settings.sync_cron, None);
    }

    #[test]
    fn build_bigquery_request_supports_maximal_fields() {
        let directory = tempfile::tempdir().unwrap();
        let service_account = directory.path().join("service-account.json");
        std::fs::write(&service_account, "{\"project_id\":\"source-project\"}").unwrap();
        let mut args = bigquery_builder_args(service_account.to_string_lossy().into_owned());
        args.name = "maximal-pipe".into();
        args.staging_path = "gs://other-bucket/snapshots".into();
        args.project_id = Some("source-project".into());
        args.table_mappings = vec!["audit.events:audit_events".into()];
        args.table_mappings_json = vec![MAXIMAL_BIGQUERY_TABLE_MAPPING_JSON.into()];
        args.destination.destination_database = "analytics".into();
        args.destination_roles.roles = vec!["analytics_reader".into()];
        args.allow_nullable_columns = Some(true);
        args.initial_load_parallelism = Some(2.5);
        args.snapshot_rows_per_partition = Some(1_000_000.0);
        args.snapshot_parallel_tables = Some(3.0);

        let request = build_bigquery_request(&args).unwrap();

        assert_eq!(request.name, "maximal-pipe");
        assert_eq!(request.destination.database, "analytics");
        assert_eq!(
            request.destination.roles,
            Some(vec!["analytics_reader".into()])
        );
        let source = request.source.bigquery.as_ref().expect("bigquery source");
        let clickhouse_cloud_api::models::ClickPipeMutateBigQuerySource::ClickPipePostBigQueryServiceAccountSource(source) = source else {
            panic!("expected service-account source");
        };
        assert_eq!(source.snapshot_staging_path, "gs://other-bucket/snapshots");
        assert_eq!(source.project_id.as_deref(), Some("source-project"));
        assert_eq!(
            source.credentials.service_account_file,
            "eyJwcm9qZWN0X2lkIjoic291cmNlLXByb2plY3QifQ=="
        );
        assert_eq!(source.table_mappings.len(), 2);
        let mapping = &source.table_mappings[1];
        assert_eq!(mapping.source_dataset_name, "sales");
        assert_eq!(mapping.source_table, "orders");
        assert_eq!(mapping.target_table, "orders_raw");
        assert_eq!(
            mapping.excluded_columns.as_deref(),
            Some(&["internal_id".into(), "temporary".into()][..])
        );
        assert_eq!(
            mapping.sorting_keys.as_deref(),
            Some(&["created_at".into(), "id".into()][..])
        );
        assert_eq!(mapping.use_custom_sorting_key, Some(true));
        assert_eq!(
            mapping.table_engine,
            Some(ClickPipeBigQueryPipeTableMappingTableengine::MergeTree)
        );
        assert_eq!(source.settings.replication_mode.to_string(), "snapshot");
        assert_eq!(source.settings.allow_nullable_columns, Some(true));
        assert_eq!(source.settings.initial_load_parallelism, Some(2.5));
        assert_eq!(
            source.settings.snapshot_num_rows_per_partition,
            Some(1_000_000.0)
        );
        assert_eq!(
            source.settings.snapshot_number_of_parallel_tables,
            Some(3.0)
        );
    }

    #[test]
    fn build_bigquery_request_rejects_non_finite_settings_before_key_file() {
        fn set_initial_load_parallelism(args: &mut BigQueryCreateArgs) {
            args.initial_load_parallelism = Some(f64::NAN);
        }
        fn set_snapshot_rows(args: &mut BigQueryCreateArgs) {
            args.snapshot_rows_per_partition = Some(f64::INFINITY);
        }
        fn set_snapshot_tables(args: &mut BigQueryCreateArgs) {
            args.snapshot_parallel_tables = Some(f64::NEG_INFINITY);
        }

        for (flag, set_value) in [
            (
                "--initial-load-parallelism",
                set_initial_load_parallelism as fn(&mut BigQueryCreateArgs),
            ),
            (
                "--snapshot-rows-per-partition",
                set_snapshot_rows as fn(&mut BigQueryCreateArgs),
            ),
            (
                "--snapshot-parallel-tables",
                set_snapshot_tables as fn(&mut BigQueryCreateArgs),
            ),
        ] {
            let mut args = bigquery_builder_args("/missing/service-account.json".into());
            set_value(&mut args);

            let error = build_bigquery_request(&args).unwrap_err().to_string();

            // Validation runs before the (missing) key file is read.
            assert_names(&error, &[flag]);
        }
    }

    #[test]
    fn bigquery_workload_identity_selects_the_typed_union_without_a_key() {
        let mut args = bigquery_builder_args("/missing/must-not-be-read.json".into());
        args.auth = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY".into();
        args.service_account_file = None;
        args.project_id = Some("project-1".into());
        let request = build_bigquery_request(&args).expect("BigQuery workload identity builds");
        let source = request.source.bigquery.expect("bigquery source");
        let clickhouse_cloud_api::models::ClickPipeMutateBigQuerySource::ClickPipePostBigQueryWorkloadIdentitySource(source) = source else {
            panic!("expected workload-identity source");
        };
        assert_eq!(source.project_id, "project-1");
        assert_eq!(source.table_mappings.len(), 1);
        let json = serde_json::to_value(source).expect("source serializes");
        assert!(json.get("credentials").is_none());
    }

    #[test]
    fn bigquery_authentication_validates_project_and_key_before_file_io() {
        let mut workload = bigquery_builder_args("/missing/must-not-be-read.json".into());
        workload.auth = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY".into();
        assert_names(
            &build_bigquery_request(&workload).unwrap_err().message,
            &[
                "--service-account-file",
                "--auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
            ],
        );

        workload.service_account_file = None;
        let message = build_bigquery_request(&workload).unwrap_err().message;
        assert_names(
            &message,
            &["--auth SERVICE_ACCOUNT_WORKLOAD_IDENTITY", "--project-id"],
        );
        assert!(!message.contains("--service-account-file"), "{message}");

        let service_account = bigquery_builder_args("/missing/key.json".into());
        let mut service_account_without_key = service_account;
        service_account_without_key.service_account_file = None;
        let message = build_bigquery_request(&service_account_without_key)
            .unwrap_err()
            .message;
        assert_names(
            &message,
            &["--auth SERVICE_ACCOUNT", "--service-account-file"],
        );
        assert!(!message.contains("WORKLOAD_IDENTITY"), "{message}");
    }
    #[test]
    fn parses_bigquery_cdc_controls_and_rejects_invalid_values() {
        let base = [
            "create",
            "bigquery",
            "svc-1",
            "--name",
            "cdc",
            "--service-account-file",
            "/tmp/account.json",
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping",
            "dataset.orders:orders",
        ];
        let mut command = base.to_vec();
        command.extend([
            "--replication-mode",
            "cdc_only",
            "--replication-method",
            "query_based",
            "--sync-interval-seconds",
            "60",
            "--query-cdc-pull-sync-parallelism",
            "4",
            "--sync-delay-seconds",
            "0",
            "--pull-window-size-seconds",
            "86400",
        ]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::BigQuery(args),
        } = parse_clickpipe(&command)
        else {
            panic!("expected BigQuery create")
        };
        assert_eq!(args.replication_method.as_deref(), Some("query_based"));
        assert_eq!(args.sync_interval_seconds, Some(60));
        assert_eq!(args.query_cdc_pull_sync_parallelism, Some(4));
        assert_eq!(args.sync_delay_seconds, Some(0));
        assert_eq!(args.pull_window_size_seconds, Some(86400));
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::BigQuery(defaults),
        } = parse_clickpipe(&base)
        else {
            panic!("expected BigQuery create")
        };
        assert_eq!(defaults.replication_method, None);
        assert_eq!(defaults.sync_interval_seconds, None);
        assert_eq!(defaults.sync_cron, None);
        assert_eq!(defaults.query_cdc_pull_sync_parallelism, None);
        assert_eq!(defaults.sync_delay_seconds, None);
        assert_eq!(defaults.pull_window_size_seconds, None);
        for (flag, value) in [
            ("--replication-method", "invalid"),
            ("--sync-interval-seconds", "0"),
            ("--query-cdc-pull-sync-parallelism", "0"),
            ("--sync-delay-seconds", "-1"),
            ("--pull-window-size-seconds", "0"),
        ] {
            let mut command = base.to_vec();
            command.extend([flag, value]);
            assert_rejected(&command);
        }
        let mut repeated = base.to_vec();
        repeated.extend(["--sync-delay-seconds", "0", "--sync-delay-seconds", "1"]);
        assert_rejected(&repeated);
    }

    #[test]
    fn parses_bigquery_sync_cron_and_rejects_it_with_sync_interval() {
        let base = [
            "create",
            "bigquery",
            "svc-1",
            "--name",
            "cdc",
            "--service-account-file",
            "/tmp/account.json",
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping",
            "dataset.orders:orders",
            "--replication-mode",
            "cdc_only",
            "--replication-method",
            "query_based",
        ];
        for value in ["0 * * * *", "@hourly", "CRON_TZ=Europe/London 30 2 * * *"] {
            let mut command = base.to_vec();
            command.extend(["--sync-cron", value]);
            let ClickPipeCommands::Create {
                command: ClickPipeCreateCommands::BigQuery(args),
            } = parse_clickpipe(&command)
            else {
                panic!("expected BigQuery create")
            };
            assert_eq!(args.sync_cron.as_deref(), Some(value));
            assert_eq!(args.sync_interval_seconds, None);
        }

        let mut conflicting = base.to_vec();
        conflicting.extend(["--sync-cron", "@hourly", "--sync-interval-seconds", "60"]);
        assert_eq!(
            clickpipe_parse_error(&conflicting).kind(),
            clap::error::ErrorKind::ArgumentConflict
        );

        let mut repeated = base.to_vec();
        repeated.extend(["--sync-cron", "@hourly", "--sync-cron", "@daily"]);
        assert_rejected(&repeated);
    }

    #[test]
    fn rejects_bigquery_sync_cron_in_snapshot_mode_before_reading_key_file() {
        let base = [
            "create",
            "bigquery",
            "svc-1",
            "--name",
            "snap",
            "--service-account-file",
            "/missing/key.json",
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping",
            "dataset.orders:orders",
            "--sync-cron",
            "@hourly",
        ];
        let mut explicit = base.to_vec();
        explicit.extend(["--replication-mode", "snapshot"]);
        for command in [base.to_vec(), explicit] {
            let ClickPipeCommands::Create {
                command: ClickPipeCreateCommands::BigQuery(args),
            } = parse_clickpipe(&command)
            else {
                panic!("expected BigQuery create")
            };
            assert_eq!(args.replication_mode, "snapshot");
            let error = build_bigquery_request(&args).unwrap_err();
            assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Usage);
            assert_names(
                &error.message,
                &["--sync-cron", "--replication-mode snapshot"],
            );
        }
    }

    #[test]
    fn builds_bigquery_sync_cron_in_place_of_the_sync_interval() {
        let mut args = bigquery_builder_args("/unused/key.json".into());
        args.auth = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY".into();
        args.service_account_file = None;
        args.project_id = Some("source-project".into());
        args.replication_mode = "cdc".into();
        args.replication_method = Some("query_based".into());
        args.sync_cron = Some("CRON_TZ=Europe/London 0 * * * *".into());
        args.table_mappings.clear();
        args.table_mappings_json = vec![
            r#"{"sourceDatasetName":"sales","sourceTable":"orders","targetTable":"orders_raw","queryCdcWatermarkColumn":"updated_at"}"#.into(),
        ];

        let request = build_bigquery_request(&args).unwrap();

        let source = request.source.bigquery.as_ref().expect("bigquery source");
        let clickhouse_cloud_api::models::ClickPipeMutateBigQuerySource::ClickPipePostBigQueryWorkloadIdentitySource(source) = source else {
            panic!("expected workload-identity source");
        };
        assert_eq!(
            source.settings.sync_cron.as_deref(),
            Some("CRON_TZ=Europe/London 0 * * * *")
        );
        assert_eq!(source.settings.sync_interval_seconds, None);
    }

    #[test]
    fn builds_bigquery_cdc_settings_and_method_specific_mappings() {
        for (method, field, value) in [
            ("query_based", "queryCdcWatermarkColumn", "updated_at"),
            ("events_based", "eventsFunction", "changes"),
        ] {
            let mut args = bigquery_builder_args("/unused/key.json".into());
            args.auth = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY".into();
            args.service_account_file = None;
            args.project_id = Some("source-project".into());
            args.replication_mode = "cdc_only".into();
            args.replication_method = Some(method.into());
            args.sync_interval_seconds = Some(60);
            args.query_cdc_pull_sync_parallelism = Some(4);
            args.sync_delay_seconds = Some(0);
            args.pull_window_size_seconds = Some(86400);
            args.table_mappings.clear();
            args.table_mappings_json = vec![serde_json::json!({
                "sourceDatasetName":"sales", "sourceTable":"orders", "targetTable":"orders_raw", field: value
            }).to_string()];
            let request = build_bigquery_request(&args).unwrap();
            let source = serde_json::to_value(request).unwrap()["source"]["bigquery"].clone();
            assert_eq!(
                source["settings"],
                serde_json::json!({"replicationMode":"cdc_only",
                "replicationMethod":method, "syncIntervalSeconds":60, "queryCdcPullSyncParallelism":4,
                "syncDelaySeconds":0, "pullWindowSizeSeconds":86400})
            );
            assert_eq!(source["tableMappings"][0][field], value);
            assert!(source.get("credentials").is_none());
        }
    }

    #[test]
    fn rejects_bigquery_cdc_mismatches_before_reading_key_file() {
        for (mode, method, mapping) in [
            ("snapshot", Some("query_based"), "{}"),
            ("cdc", None, "{}"),
            ("cdc_only", Some("invalid"), "{}"),
            (
                "cdc",
                Some("query_based"),
                r#"{"eventsFunction":"appends"}"#,
            ),
            (
                "cdc_only",
                Some("events_based"),
                r#"{"queryCdcWatermarkColumn":"updated_at"}"#,
            ),
            (
                "snapshot",
                None,
                r#"{"queryCdcWatermarkColumn":"updated_at"}"#,
            ),
            (
                "cdc",
                Some("query_based"),
                r#"{"queryCdcWatermarkColumn":" "}"#,
            ),
            (
                "cdc",
                Some("events_based"),
                r#"{"eventsFunction":"future"}"#,
            ),
        ] {
            let mut args = bigquery_builder_args("/missing/key.json".into());
            args.replication_mode = mode.into();
            args.replication_method = method.map(str::to_owned);
            args.table_mappings.clear();
            let mut mapping: serde_json::Value = serde_json::from_str(mapping).unwrap();
            mapping["sourceDatasetName"] = "sales".into();
            mapping["sourceTable"] = "orders".into();
            mapping["targetTable"] = "orders_raw".into();
            args.table_mappings_json = vec![mapping.to_string()];
            let error = build_bigquery_request(&args).unwrap_err();
            assert!(
                !error.message.contains("/missing/key.json"),
                "{}",
                error.message
            );
        }
        for (flag, minimum) in [
            ("--sync-interval-seconds", 1),
            ("--query-cdc-pull-sync-parallelism", 1),
            ("--sync-delay-seconds", 0),
            ("--pull-window-size-seconds", 1),
        ] {
            let mut args = bigquery_builder_args("/missing/key.json".into());
            match flag {
                "--sync-interval-seconds" => args.sync_interval_seconds = Some(minimum - 1),
                "--query-cdc-pull-sync-parallelism" => {
                    args.query_cdc_pull_sync_parallelism = Some(minimum - 1)
                }
                "--sync-delay-seconds" => args.sync_delay_seconds = Some(minimum - 1),
                _ => args.pull_window_size_seconds = Some(minimum - 1),
            }
            // The minimum, not the snapshot-mode rejection the same value
            // would also hit.
            let error = build_bigquery_request(&args).unwrap_err();
            assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Usage);
            assert_names(&error.message, &[flag]);
            assert!(
                names_word(&error.message, &minimum.to_string()),
                "{}",
                error.message
            );
            assert!(
                !error.message.contains("--replication-mode"),
                "{}",
                error.message
            );
        }
        let mut args = bigquery_builder_args("/missing/key.json".into());
        args.sync_delay_seconds = Some(0);
        assert_names(
            &build_bigquery_request(&args).unwrap_err().message,
            &["--sync-delay-seconds", "--replication-mode snapshot"],
        );
    }
}
