use crate::cloud::clickpipes::cli::{
    ClickPipeCreateValidationArgs, DatabaseDestinationArgs, DestinationRoleArgs,
};
use crate::cloud::clickpipes::create::{
    apply_create_request_args, build_create_validation_args, print_created,
};
use crate::cloud::clickpipes::destination::{build_destination, build_destination_roles};
use crate::cloud::clickpipes::parse_enum;
use crate::cloud::clickpipes::sources::{DB_AUTHS, REPLICATION_MODES};
use crate::cloud::clickpipes::table_mapping::{
    parse_postgres_table_mapping, parse_postgres_table_mapping_json,
    parse_postgres_table_mapping_parts,
};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult, read_flag_file};
use crate::cloud::shared::resolve_org_id;
use clap::builder::{NonEmptyStringValueParser, PossibleValuesParser};
use clap::{ArgGroup, Args};
use clickhouse_cloud_api::models::{
    ClickPipePostgresPipeTableMapping, ClickPipePostgresPipeTableMappingTableengine,
};

pub(in crate::cloud::clickpipes) const POSTGRES_TYPES: &[&str] = &[
    "postgres",
    "supabase",
    "neon",
    "alloydb",
    "planetscale",
    "rdspostgres",
    "aurorapostgres",
    "cloudsqlpostgres",
    "azurepostgres",
    "crunchybridge",
    "tigerdata",
];

/// The two table-mapping flags are one required "at least one of" group, so
/// clap names both in the missing-argument error and either alone is enough.
#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("postgres_table_mappings")
        .required(true)
        .multiple(true)
        .args(["table_mappings", "table_mappings_json"])
))]
pub struct PostgresCreateArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub validation: ClickPipeCreateValidationArgs,

    /// PostgreSQL host
    #[arg(long)]
    pub host: String,

    /// PostgreSQL port (1-65535)
    #[arg(
        long,
        default_value = "5432",
        value_parser = clap::value_parser!(u16).range(1..=65535)
    )]
    pub port: u16,

    /// Source database name
    #[arg(long)]
    pub pg_database: String,

    /// Username (required with --auth basic; invalid with --auth IAM_ROLE)
    #[arg(long, requires = "password")]
    pub username: Option<String>,

    /// Password (required with --auth basic; invalid with --auth IAM_ROLE)
    #[arg(long, requires = "username")]
    pub password: Option<String>,

    /// Table mappings as schema.table:target_table (repeatable)
    ///
    /// Uses ReplacingMergeTree; override tableEngine with --table-mapping-json.
    #[arg(
        long = "table-mapping",
        value_name = "SCHEMA.TABLE:TARGET_TABLE",
        value_parser = parse_postgres_table_mapping
    )]
    pub table_mappings: Vec<String>,

    /// Full table mapping as a JSON object (repeatable)
    ///
    /// Supports per-table options, including an explicit tableEngine override.
    /// Omitted tableEngine uses ReplacingMergeTree. Combinable with
    /// --table-mapping; unknown fields are rejected.
    #[arg(long = "table-mapping-json", value_name = "JSON")]
    pub table_mappings_json: Vec<String>,

    /// Postgres type
    #[arg(
        long,
        default_value = "postgres",
        value_parser = PossibleValuesParser::new(POSTGRES_TYPES),
    )]
    pub postgres_type: String,

    /// Replication mode
    #[arg(
        long,
        default_value = "cdc",
        value_parser = PossibleValuesParser::new(REPLICATION_MODES),
    )]
    pub replication_mode: String,

    /// Authentication method
    #[arg(
        long,
        default_value = "basic",
        value_parser = PossibleValuesParser::new(DB_AUTHS),
    )]
    pub auth: String,

    /// IAM role ARN (required with --auth IAM_ROLE; invalid with basic auth)
    #[arg(long, required_if_eq("auth", "IAM_ROLE"))]
    pub iam_role: Option<String>,

    /// Certificate hostname override (defaults to --host)
    #[arg(long, value_name = "HOSTNAME")]
    pub tls_host: Option<String>,

    /// Path to a PEM CA bundle for a private or self-signed source certificate
    #[arg(long, value_name = "PATH")]
    pub ca_certificate: Option<String>,

    /// Disable TLS and send source traffic unencrypted (unsafe)
    #[arg(
        long,
        conflicts_with_all = ["tls_host", "ca_certificate", "skip_cert_verification"]
    )]
    pub disable_tls: bool,

    /// Skip certificate verification (unsafe; prefer --ca-certificate)
    #[arg(long)]
    pub skip_cert_verification: bool,

    /// Postgres publication name
    #[arg(long)]
    pub publication_name: Option<String>,

    /// Replication slot name (only with --replication-mode cdc_only)
    #[arg(long)]
    pub replication_slot_name: Option<String>,

    /// CDC sync interval in seconds (at least 1)
    #[arg(long, value_name = "SECONDS", allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(1..))]
    pub sync_interval_seconds: Option<i64>,

    /// Rows to pull per CDC batch (at least 1)
    #[arg(long, value_name = "ROWS", allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(1..))]
    pub pull_batch_size: Option<i64>,

    /// Snapshot workers per table (at least 1; create-time only)
    #[arg(long, value_name = "WORKERS", allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(1..))]
    pub initial_load_parallelism: Option<i64>,

    /// Snapshot rows per partition (at least 1,000; create-time only)
    #[arg(long, value_name = "ROWS", allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(1000..))]
    pub snapshot_rows_per_partition: Option<i64>,

    /// Tables to snapshot in parallel (at least 1; create-time only)
    #[arg(long, value_name = "TABLES", allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(1..))]
    pub snapshot_parallel_tables: Option<i64>,

    /// Preserve Postgres nullability; defaults to false (create-time only)
    #[arg(long, value_name = "true|false")]
    pub allow_nullable_columns: Option<bool>,

    /// Enable PG17+ slot failover; defaults to false (create-time only)
    ///
    /// Applies only when ClickPipes creates the slot.
    #[arg(long, value_name = "true|false")]
    pub enable_failover_slots: Option<bool>,

    /// Hard-delete Postgres DELETEs; defaults to false (create-time only)
    #[arg(long, value_name = "true|false")]
    pub delete_on_merge: Option<bool>,

    #[command(flatten)]
    pub destination: DatabaseDestinationArgs,

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,
}

/// Validate the cross-flag rules and resolve every table mapping, from both
/// `--table-mapping` and `--table-mapping-json`, before any request is made.
pub(in crate::cloud::clickpipes) fn validate_postgres_create_args(
    args: &PostgresCreateArgs,
) -> CloudResult<Vec<ClickPipePostgresPipeTableMapping>> {
    if args.port == 0 {
        return Err(CloudError::usage("--port must be in the range 1..=65535"));
    }
    if args.table_mappings.is_empty() && args.table_mappings_json.is_empty() {
        return Err(CloudError::usage(
            "at least one --table-mapping <SCHEMA.TABLE:TARGET_TABLE> or \
             --table-mapping-json <JSON> is required",
        ));
    }
    if args.auth == "IAM_ROLE" && args.iam_role.is_none() {
        return Err(CloudError::usage(
            "--auth IAM_ROLE requires --iam-role <IAM_ROLE>",
        ));
    }
    if args.auth == "basic" && args.iam_role.is_some() {
        return Err(CloudError::usage(
            "--iam-role cannot be used with --auth basic; use --auth IAM_ROLE",
        ));
    }
    // IAM_ROLE authentication has no username or password: the role ARN is the
    // whole credential. Reject the pair rather than silently dropping it, the
    // same way basic auth rejects --iam-role.
    if args.auth == "IAM_ROLE" && (args.username.is_some() || args.password.is_some()) {
        return Err(CloudError::usage(
            "--username and --password cannot be used with --auth IAM_ROLE; use --auth basic",
        ));
    }
    // clap joins --username and --password with `requires`, so half a pair is
    // already a usage error; this owns the "basic auth needs the pair at all"
    // rule, which clap cannot express because it depends on --auth's value.
    if args.auth == "basic" && (args.username.is_none() || args.password.is_none()) {
        return Err(CloudError::usage(
            "--auth basic requires --username <USERNAME> and --password <PASSWORD>",
        ));
    }
    if args.replication_slot_name.is_some() && args.replication_mode != "cdc_only" {
        return Err(CloudError::usage(
            "--replication-slot-name can only be used with --replication-mode cdc_only",
        ));
    }
    if args.disable_tls && args.tls_host.is_some() {
        return Err(CloudError::usage(
            "--tls-host cannot be used with --disable-tls",
        ));
    }
    if args.disable_tls && args.ca_certificate.is_some() {
        return Err(CloudError::usage(
            "--ca-certificate cannot be used with --disable-tls",
        ));
    }
    if args.disable_tls && args.skip_cert_verification {
        return Err(CloudError::usage(
            "--skip-cert-verification cannot be used with --disable-tls",
        ));
    }

    // The simple mappings are sent first, then the JSON ones, each in the
    // order given: clap's derive API does not expose argv indices, so
    // interleaving the two flags is not observable here.
    let mut mappings =
        Vec::with_capacity(args.table_mappings.len() + args.table_mappings_json.len());
    for mapping in &args.table_mappings {
        let (source_schema_name, source_table, target_table) =
            parse_postgres_table_mapping_parts(mapping)?;
        mappings.push(ClickPipePostgresPipeTableMapping {
            source_schema_name,
            source_table,
            target_table,
            table_engine: ClickPipePostgresPipeTableMappingTableengine::ReplacingMergeTree,
            excluded_columns: Vec::new(),
            partition_by_expr: String::new(),
            partition_key: String::new(),
            sorting_keys: Vec::new(),
            use_custom_sorting_key: false,
        });
    }
    for (position, mapping) in args.table_mappings_json.iter().enumerate() {
        mappings.push(parse_postgres_table_mapping_json(position, mapping)?);
    }

    Ok(mappings)
}

/// Build the Postgres pipe settings sent at create time.
///
/// Every field the API models is wired here, because the settings are
/// create-time decisions: `ClickPipePatchPostgresPipeSettings` can only patch
/// `syncIntervalSeconds` and `pullBatchSize`, so anything else not set now can
/// never be applied to the pipe.
///
/// `allowNullableColumns`, `deleteOnMerge` and `enableFailoverSlots` are
/// required by the schema and therefore always serialized: absence is not
/// representable on the wire, so an omitted flag sends `false` — the request
/// shape the CLI has always sent. Every other setting is omitted when its flag
/// is omitted, via `skip_serializing_if`.
fn build_postgres_pipe_settings(
    args: &PostgresCreateArgs,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostgresPipeSettings> {
    Ok(
        clickhouse_cloud_api::models::ClickPipePostgresPipeSettings {
            replication_mode: parse_enum(&args.replication_mode)?,
            publication_name: args.publication_name.clone(),
            replication_slot_name: args.replication_slot_name.clone(),
            sync_interval_seconds: args.sync_interval_seconds,
            pull_batch_size: args.pull_batch_size,
            initial_load_parallelism: args.initial_load_parallelism,
            snapshot_num_rows_per_partition: args.snapshot_rows_per_partition,
            snapshot_number_of_parallel_tables: args.snapshot_parallel_tables,
            allow_nullable_columns: args.allow_nullable_columns.unwrap_or(false),
            delete_on_merge: args.delete_on_merge.unwrap_or(false),
            enable_failover_slots: args.enable_failover_slots.unwrap_or(false),
        },
    )
}

fn build_postgres_request(
    args: &PostgresCreateArgs,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostRequest> {
    use clickhouse_cloud_api::models::{
        ClickPipeMutatePostgresSource, ClickPipeMutatePostgresSourceAuthentication,
        ClickPipePostRequest, ClickPipePostSource, PLAIN,
    };

    let request_args = build_create_validation_args(&args.validation);
    let table_mappings = validate_postgres_create_args(args)?;
    let ca_certificate = args
        .ca_certificate
        .as_deref()
        .map(|path| read_flag_file("--ca-certificate", path))
        .transpose()?;
    // Match the parsed authentication mode, not the raw `--auth` string, so a
    // new mode added to the library enum is a compile error here rather than a
    // silent credential-less create.
    let authentication: ClickPipeMutatePostgresSourceAuthentication = parse_enum(&args.auth)?;
    let credentials = match &authentication {
        // `validate_postgres_create_args` has already required the pair for
        // basic auth, so `zip` yields `Some` for every invocation that reaches
        // here.
        ClickPipeMutatePostgresSourceAuthentication::Basic => args
            .username
            .as_deref()
            .zip(args.password.as_deref())
            .map(|(username, password)| PLAIN {
                username: username.to_string(),
                password: password.to_string(),
            }),
        // The role ARN is the whole credential: the `credentials` object must
        // stay off the wire entirely.
        ClickPipeMutatePostgresSourceAuthentication::IAM_ROLE => None,
        // Unreachable for parsed input, since --auth is restricted to
        // DB_AUTHS; these modes have no credential shape to send.
        ClickPipeMutatePostgresSourceAuthentication::ServiceAccountWorkloadIdentity
        | ClickPipeMutatePostgresSourceAuthentication::Unknown(_) => None,
    };
    let source = ClickPipeMutatePostgresSource {
        r#type: Some(parse_enum(&args.postgres_type)?),
        credentials,
        host: args.host.clone(),
        port: i64::from(args.port),
        database: args.pg_database.clone(),
        disable_tls: args.disable_tls,
        skip_cert_verification: args.skip_cert_verification,
        authentication,
        iam_role: args.iam_role.clone(),
        tls_host: args.tls_host.clone(),
        ca_certificate,
        settings: build_postgres_pipe_settings(args)?,
        table_mappings,
    };

    let mut request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            postgres: Some(source),
            bigquery: None,
            kafka: None,
            kinesis: None,
            mongodb: None,
            mysql: None,
            object_storage: None,
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

fn postgres_tls_error_hint(message: &str) -> Option<&'static str> {
    let message = message.to_ascii_lowercase();

    if message.contains("x509: certificate signed by unknown authority") {
        return Some(
            "The source certificate chain is not publicly trusted. For a private or \
             self-signed source CA, pass its PEM CA bundle with \
             `--ca-certificate <PATH>`.",
        );
    }

    if (message.contains("x509: certificate is valid for ") && message.contains(", not "))
        || (message.contains("x509: cannot validate certificate for ")
            && message.contains("because it doesn't contain any ip sans"))
    {
        return Some(
            "The source certificate does not match `--host`. Pass the certificate's \
             hostname with `--tls-host <HOSTNAME>`.",
        );
    }

    None
}

fn add_postgres_tls_error_hint(mut error: CloudError) -> CloudError {
    if let Some(hint) = postgres_tls_error_hint(&error.message) {
        error.message.push_str("\n\nHint: ");
        error.message.push_str(hint);
    }
    error
}

pub(in crate::cloud::clickpipes) async fn clickpipe_create_postgres(
    client: &CloudClient,
    args: &PostgresCreateArgs,
    json: bool,
) -> CloudResult<()> {
    let request = build_postgres_request(args)?;
    let org_id = resolve_org_id(client).await?;

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await
        .map_err(add_postgres_tls_error_hint)?;
    print_created(&clickpipe, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::clickpipes::cli::{ClickPipeCommands, ClickPipeCreateCommands};
    use crate::cloud::clickpipes::test_support::{
        MAXIMAL_TABLE_MAPPING_JSON, assert_names, clickpipe_parse_error,
        clickpipe_validation_message, parse_clickpipe, postgres_cli_args,
    };

    #[test]
    fn parses_postgres_flags_defaults_and_repeatability() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&[
            "create",
            "postgres",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "postgres.example",
            "--port",
            "5433",
            "--pg-database",
            "source-db",
            "--username",
            "user",
            "--password",
            "password",
            "--table-mapping",
            "public.one:one",
            "--table-mapping",
            "public.two:two",
            "--postgres-type",
            "neon",
            "--replication-mode",
            "cdc_only",
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:role",
            "--tls-host",
            "tls.example",
            "--ca-certificate",
            "/tmp/ca.pem",
            "--skip-cert-verification",
            "--publication-name",
            "publication",
            "--replication-slot-name",
            "slot",
            "--destination-database",
            "analytics",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected postgres create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.host, "postgres.example");
        assert_eq!(args.port, 5433);
        assert_eq!(args.pg_database, "source-db");
        assert_eq!(args.username.as_deref(), Some("user"));
        assert_eq!(args.password.as_deref(), Some("password"));
        assert_eq!(args.table_mappings, ["public.one:one", "public.two:two"]);
        assert_eq!(args.postgres_type, "neon");
        assert_eq!(args.replication_mode, "cdc_only");
        assert_eq!(args.auth, "IAM_ROLE");
        assert_eq!(args.iam_role.as_deref(), Some("arn:role"));
        assert_eq!(args.tls_host.as_deref(), Some("tls.example"));
        assert_eq!(args.ca_certificate.as_deref(), Some("/tmp/ca.pem"));
        assert!(!args.disable_tls);
        assert!(args.skip_cert_verification);
        assert_eq!(args.publication_name.as_deref(), Some("publication"));
        assert_eq!(args.replication_slot_name.as_deref(), Some("slot"));
        assert_eq!(args.destination.destination_database, "analytics");

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&[
            "create",
            "postgres",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "postgres.example",
            "--pg-database",
            "source-db",
            "--username",
            "user",
            "--password",
            "password",
            "--table-mapping",
            "public.events:events",
        ])
        else {
            panic!("expected postgres create");
        };
        assert_eq!(args.port, 5432);
        assert_eq!(args.table_mappings, ["public.events:events"]);
        assert_eq!(args.postgres_type, "postgres");
        assert_eq!(args.replication_mode, "cdc");
        assert_eq!(args.auth, "basic");
        assert_eq!(args.iam_role, None);
        assert_eq!(args.tls_host, None);
        assert_eq!(args.ca_certificate, None);
        assert!(!args.disable_tls);
        assert!(!args.skip_cert_verification);
        assert_eq!(args.publication_name, None);
        assert_eq!(args.replication_slot_name, None);
        assert_eq!(args.sync_interval_seconds, None);
        assert_eq!(args.pull_batch_size, None);
        assert_eq!(args.initial_load_parallelism, None);
        assert_eq!(args.snapshot_rows_per_partition, None);
        assert_eq!(args.snapshot_parallel_tables, None);
        assert_eq!(args.allow_nullable_columns, None);
        assert_eq!(args.enable_failover_slots, None);
        assert_eq!(args.delete_on_merge, None);
        assert_eq!(args.destination.destination_database, "default");
    }

    #[test]
    fn parses_postgres_cdc_settings_flags() {
        let mut cli_args = postgres_cli_args(Some("public.events:events"));
        cli_args.extend([
            "--sync-interval-seconds",
            "30",
            "--pull-batch-size",
            "50000",
            "--initial-load-parallelism",
            "4",
            "--snapshot-rows-per-partition",
            "1000000",
            "--snapshot-parallel-tables",
            "3",
            "--allow-nullable-columns",
            "true",
            "--enable-failover-slots",
            "false",
            "--delete-on-merge",
            "true",
        ]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&cli_args)
        else {
            panic!("expected postgres create");
        };

        assert_eq!(args.sync_interval_seconds, Some(30));
        assert_eq!(args.pull_batch_size, Some(50_000));
        assert_eq!(args.initial_load_parallelism, Some(4));
        assert_eq!(args.snapshot_rows_per_partition, Some(1_000_000));
        assert_eq!(args.snapshot_parallel_tables, Some(3));
        // Tri-state booleans: an explicit `false` is distinguishable from an
        // omitted flag at the clap layer.
        assert_eq!(args.allow_nullable_columns, Some(true));
        assert_eq!(args.enable_failover_slots, Some(false));
        assert_eq!(args.delete_on_merge, Some(true));
    }

    #[test]
    fn postgres_cdc_settings_booleans_require_an_explicit_value() {
        let mut cli_args = postgres_cli_args(Some("public.events:events"));
        cli_args.push("--allow-nullable-columns");
        let error = clickpipe_parse_error(&cli_args);
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
        let message = error.to_string();
        assert!(message.contains("--allow-nullable-columns"), "{message}");
    }

    #[test]
    fn postgres_numeric_settings_reject_values_below_their_minimums() {
        for (flag, values) in [
            ("--sync-interval-seconds", vec!["0", "-1"]),
            ("--pull-batch-size", vec!["0", "-1"]),
            ("--initial-load-parallelism", vec!["0", "-1"]),
            ("--snapshot-rows-per-partition", vec!["0", "-1", "999"]),
            ("--snapshot-parallel-tables", vec!["0", "-1"]),
        ] {
            for value in values {
                let mut args = postgres_cli_args(Some("public.events:events"));
                args.extend([flag, value]);
                let error = clickpipe_parse_error(&args);
                assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
                assert!(error.to_string().contains(flag), "{error}");
            }
        }
    }

    #[test]
    fn postgres_numeric_settings_parse_and_build_valid_minimums() {
        let mut cli_args = postgres_cli_args(Some("public.events:events"));
        cli_args.extend([
            "--sync-interval-seconds",
            "1",
            "--pull-batch-size",
            "1",
            "--initial-load-parallelism",
            "1",
            "--snapshot-rows-per-partition",
            "1000",
            "--snapshot-parallel-tables",
            "1",
        ]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&cli_args)
        else {
            panic!("expected postgres create");
        };
        assert_eq!(args.sync_interval_seconds, Some(1));
        assert_eq!(args.pull_batch_size, Some(1));
        assert_eq!(args.initial_load_parallelism, Some(1));
        assert_eq!(args.snapshot_rows_per_partition, Some(1000));
        assert_eq!(args.snapshot_parallel_tables, Some(1));

        let request = build_postgres_request(&args).unwrap();
        let settings = request.source.postgres.unwrap().settings;
        assert_eq!(settings.sync_interval_seconds, Some(1));
        assert_eq!(settings.pull_batch_size, Some(1));
        assert_eq!(settings.initial_load_parallelism, Some(1));
        assert_eq!(settings.snapshot_num_rows_per_partition, Some(1000));
        assert_eq!(settings.snapshot_number_of_parallel_tables, Some(1));
    }

    #[test]
    fn postgres_port_accepts_boundaries_and_rejects_out_of_range_values() {
        for port in ["1", "65535"] {
            let mut args = postgres_cli_args(Some("public.events:events"));
            args.extend(["--port", port]);
            parse_clickpipe(&args);
        }

        for port in ["0", "65536"] {
            let mut args = postgres_cli_args(Some("public.events:events"));
            args.extend(["--port", port]);
            let error = clickpipe_parse_error(&args);
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            let message = error.to_string();
            assert!(message.contains("--port"), "{message}");
        }
    }

    #[test]
    fn postgres_requires_at_least_one_complete_table_mapping() {
        let error = clickpipe_parse_error(&postgres_cli_args(None));
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        assert!(error.to_string().contains("--table-mapping"));

        // Each malformed part fails on its own input; the error names the
        // flag, the rejected value and, for a shape error, the expected syntax.
        for (mapping, syntax) in [
            ("public.events", Some("schema.table:target_table")),
            ("events:target", Some("schema.table:target_table")),
            (".events:target", None),
            ("public.:target", None),
            ("public.events:", None),
            (" .events:target", None),
            ("public. :target", None),
            ("public.events: ", None),
        ] {
            let error = clickpipe_parse_error(&postgres_cli_args(Some(mapping)));
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            let message = error.to_string();
            assert_names(&message, &["--table-mapping", &format!("'{mapping}'")]);
            assert_eq!(
                message.contains("schema.table:target_table"),
                syntax.is_some(),
                "{message}"
            );
        }
    }

    #[test]
    fn parses_postgres_table_mapping_json_flag() {
        let second =
            r#"{"sourceSchemaName":"audit","sourceTable":"events","targetTable":"audit_events"}"#;

        // JSON only: the simple form is not required when the JSON form is given.
        let mut cli_args = postgres_cli_args(None);
        cli_args.extend([
            "--table-mapping-json",
            MAXIMAL_TABLE_MAPPING_JSON,
            "--table-mapping-json",
            second,
        ]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&cli_args)
        else {
            panic!("expected postgres create");
        };
        assert!(args.table_mappings.is_empty());
        assert_eq!(
            args.table_mappings_json,
            [MAXIMAL_TABLE_MAPPING_JSON, second]
        );

        // Both forms together: each flag keeps its own values, verbatim.
        let mut cli_args = postgres_cli_args(Some("public.events:events"));
        cli_args.extend(["--table-mapping-json", second]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&cli_args)
        else {
            panic!("expected postgres create");
        };
        assert_eq!(args.table_mappings, ["public.events:events"]);
        assert_eq!(args.table_mappings_json, [second]);

        // Simple form only: the JSON list stays empty.
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&postgres_cli_args(Some("public.events:events")))
        else {
            panic!("expected postgres create");
        };
        assert!(args.table_mappings_json.is_empty());
    }

    #[test]
    fn postgres_missing_both_table_mapping_flags_names_both() {
        let error = clickpipe_parse_error(&postgres_cli_args(None));
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        let message = error.to_string();
        assert!(message.contains("--table-mapping "), "{message}");
        assert!(message.contains("--table-mapping-json"), "{message}");
    }

    #[test]
    fn postgres_table_mapping_json_is_not_parsed_by_clap() {
        // Content validation belongs to `validate_postgres_create_args`, which
        // runs before any request; clap only collects the raw strings.
        let mut cli_args = postgres_cli_args(None);
        cli_args.extend(["--table-mapping-json", "{ not json"]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(args),
        } = parse_clickpipe(&cli_args)
        else {
            panic!("expected postgres create");
        };
        assert_eq!(args.table_mappings_json, ["{ not json"]);

        let command = parse_clickpipe(&cli_args);
        assert_eq!(
            command
                .clickpipe_create_validation_error()
                .map(|(source, message)| (
                    source,
                    message.starts_with("--table-mapping-json #1: ")
                )),
            Some(("postgres", true))
        );
    }

    #[test]
    fn postgres_iam_role_auth_requires_role_arn() {
        let mut args = postgres_cli_args(Some("public.events:events"));
        args.extend(["--auth", "IAM_ROLE"]);
        let error = clickpipe_parse_error(&args);
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        let message = error.to_string();
        assert!(message.contains("--iam-role"), "{message}");

        args.extend(["--iam-role", "arn:aws:iam::123456789012:role/clickpipe"]);
        parse_clickpipe(&args);
    }

    #[test]
    fn postgres_iam_role_auth_does_not_require_username_or_password() {
        // IAM_ROLE authentication has no username or password: the role ARN is
        // the whole credential, so clap must not demand the pair.
        let mut args = vec![
            "create",
            "postgres",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "postgres.example",
            "--pg-database",
            "source-db",
            "--table-mapping",
            "public.events:events",
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:aws:iam::123456789012:role/clickpipe",
        ];
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(parsed),
        } = parse_clickpipe(&args)
        else {
            panic!("expected postgres create");
        };
        assert_eq!(parsed.username, None);
        assert_eq!(parsed.password, None);
        assert_eq!(parsed.auth, "IAM_ROLE");

        // The pair is rejected rather than silently dropped, the same way
        // basic auth rejects --iam-role.
        args.extend(["--username", "user", "--password", "password"]);
        let command = parse_clickpipe(&args);
        assert_names(
            &clickpipe_validation_message(&command).expect("pair is rejected"),
            &[
                "--username",
                "--password",
                "--auth IAM_ROLE",
                "--auth basic",
            ],
        );
    }

    #[test]
    fn postgres_iam_role_only_error_names_the_role_flag_and_not_the_pair() {
        // --username/--password used to be `required_unless_present =
        // "iam_role"`, so `--auth IAM_ROLE` on its own made clap demand all
        // three flags, and following that advice landed on the "cannot be
        // used with --auth IAM_ROLE" rejection. Only --iam-role is missing.
        let args = vec![
            "create",
            "postgres",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "postgres.example",
            "--pg-database",
            "source-db",
            "--table-mapping",
            "public.events:events",
            "--auth",
            "IAM_ROLE",
        ];
        let error = clickpipe_parse_error(&args);
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        let message = error.to_string();
        assert!(message.contains("--iam-role"), "{message}");
        assert!(!message.contains("--username"), "{message}");
        assert!(!message.contains("--password"), "{message}");
    }

    #[test]
    fn postgres_basic_auth_without_any_credentials_is_a_validation_error() {
        // clap only pairs the two flags now, so neither being present parses
        // cleanly and `validate_postgres_create_args` owns the basic-auth
        // rule: it depends on --auth's value, which clap cannot express.
        let args = vec![
            "create",
            "postgres",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "postgres.example",
            "--pg-database",
            "source-db",
            "--table-mapping",
            "public.events:events",
        ];
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(parsed),
        } = parse_clickpipe(&args)
        else {
            panic!("expected postgres create");
        };
        assert_eq!(parsed.auth, "basic");
        assert_eq!(parsed.username, None);
        assert_eq!(parsed.password, None);

        let command = parse_clickpipe(&args);
        let message = clickpipe_validation_message(&command).expect("basic auth needs the pair");
        assert_names(
            &message,
            &[
                "--auth basic",
                "--username <USERNAME>",
                "--password <PASSWORD>",
            ],
        );
        assert!(!message.contains("IAM_ROLE"), "{message}");
    }

    #[test]
    fn postgres_basic_auth_still_requires_username_and_password() {
        // The two flags are joined with clap `requires`, the same pairing the
        // Kafka credential flags use (issue #606), so half a pair is a usage
        // error naming the missing half rather than the whole set.
        for omitted in ["--username", "--password"] {
            let args: Vec<&str> = [
                "create",
                "postgres",
                "svc-1",
                "--name",
                "pipe-1",
                "--host",
                "postgres.example",
                "--pg-database",
                "source-db",
                "--table-mapping",
                "public.events:events",
                "--username",
                "user",
                "--password",
                "password",
            ]
            .into_iter()
            .collect();
            let position = args.iter().position(|arg| *arg == omitted).unwrap();
            let mut args = args;
            args.drain(position..position + 2);

            let error = clickpipe_parse_error(&args);
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "omitting {omitted} should be a usage error"
            );
            assert!(error.to_string().contains(omitted), "{error}");
        }
    }

    #[test]
    fn postgres_cross_value_relationships_have_specific_validation_errors() {
        let mut basic_with_role = postgres_cli_args(Some("public.events:events"));
        basic_with_role.extend(["--iam-role", "arn:aws:iam::123456789012:role/clickpipe"]);
        let command = parse_clickpipe(&basic_with_role);
        let message = clickpipe_validation_message(&command).expect("role on basic auth");
        assert_names(&message, &["--iam-role", "--auth basic", "--auth IAM_ROLE"]);
        assert!(!message.contains("--username"), "{message}");

        for mode in ["cdc", "snapshot"] {
            let mut args = postgres_cli_args(Some("public.events:events"));
            args.extend([
                "--replication-mode",
                mode,
                "--replication-slot-name",
                "existing_slot",
            ]);
            let command = parse_clickpipe(&args);
            assert_names(
                &clickpipe_validation_message(&command).expect("slot outside cdc_only"),
                &["--replication-slot-name", "--replication-mode cdc_only"],
            );
        }

        for tls_only_flag in [
            vec!["--tls-host", "postgres.internal.example"],
            vec!["--ca-certificate", "/tmp/postgres-ca.pem"],
            vec!["--skip-cert-verification"],
        ] {
            let mut args = postgres_cli_args(Some("public.events:events"));
            args.push("--disable-tls");
            args.extend(tls_only_flag);
            let error = clickpipe_parse_error(&args);
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
    }

    #[test]
    fn postgres_tls_error_hints_are_narrow_and_preserve_api_detail() {
        let unknown_authority = "BAD_REQUEST: failed to establish connection: tls: failed to verify certificate: \
             x509: certificate signed by unknown authority";
        let error = add_postgres_tls_error_hint(CloudError::new(unknown_authority));
        assert!(error.message.starts_with(unknown_authority));
        assert!(error.message.contains("--ca-certificate <PATH>"));

        let hostname = postgres_tls_error_hint(
            "x509: certificate is valid for postgres.internal.example.com, not 10.0.0.8",
        )
        .expect("hostname mismatch hint");
        assert!(hostname.contains("--tls-host <HOSTNAME>"));
        let ip_sans = postgres_tls_error_hint(
            "x509: cannot validate certificate for 10.0.0.8 because it doesn't contain any IP Sans",
        )
        .expect("IP SAN hostname mismatch hint");
        assert!(ip_sans.contains("--tls-host <HOSTNAME>"));
        assert!(
            postgres_tls_error_hint(
                "BAD_REQUEST: failed to establish connection: connection refused"
            )
            .is_none()
        );
        assert!(
            postgres_tls_error_hint("tls: failed to verify certificate: certificate expired")
                .is_none()
        );
    }

    fn postgres_builder_args() -> PostgresCreateArgs {
        PostgresCreateArgs {
            service_id: "svc-1".into(),
            name: "pipe-1".into(),
            validation: ClickPipeCreateValidationArgs::default(),
            host: "postgres.example".into(),
            port: 5432,
            pg_database: "source-db".into(),
            username: Some("user".into()),
            password: Some("password".into()),
            table_mappings: vec!["public.events:events".into()],
            table_mappings_json: vec![],
            postgres_type: "postgres".into(),
            replication_mode: "cdc".into(),
            auth: "basic".into(),
            iam_role: None,
            tls_host: None,
            ca_certificate: None,
            disable_tls: false,
            skip_cert_verification: false,
            publication_name: None,
            replication_slot_name: None,
            sync_interval_seconds: None,
            pull_batch_size: None,
            initial_load_parallelism: None,
            snapshot_rows_per_partition: None,
            snapshot_parallel_tables: None,
            allow_nullable_columns: None,
            enable_failover_slots: None,
            delete_on_merge: None,
            destination: DatabaseDestinationArgs {
                destination_database: "default".into(),
            },
            destination_roles: DestinationRoleArgs::default(),
        }
    }

    #[test]
    fn build_postgres_request_supports_minimal_fields() {
        let request = build_postgres_request(&postgres_builder_args()).unwrap();

        assert_eq!(request.name, "pipe-1");
        assert_eq!(request.field_mappings, None);
        assert_eq!(request.scaling, None);
        assert_eq!(request.settings, None);
        assert_eq!(request.destination.database, "default");
        assert_eq!(request.destination.columns, None);
        assert_eq!(request.destination.table, None);
        assert_eq!(request.destination.managed_table, None);
        assert_eq!(request.destination.roles, None);
        assert_eq!(request.destination.table_definition, None);
        assert!(request.source.bigquery.is_none());
        assert!(request.source.kafka.is_none());
        assert!(request.source.kinesis.is_none());
        assert!(request.source.mongodb.is_none());
        assert!(request.source.mysql.is_none());
        assert!(request.source.object_storage.is_none());
        assert!(request.source.pubsub.is_none());
        assert_eq!(request.source.validate_samples, None);

        let source = request.source.postgres.as_ref().expect("postgres source");
        assert_eq!(source.r#type.as_ref().unwrap().to_string(), "postgres");
        assert_eq!(source.authentication.to_string(), "basic");
        let credentials = source.credentials.as_ref().expect("basic credentials");
        assert_eq!(credentials.username, "user");
        assert_eq!(credentials.password, "password");
        assert_eq!(source.host, "postgres.example");
        assert_eq!(source.port, 5432);
        assert_eq!(source.database, "source-db");
        assert!(!source.disable_tls);
        assert!(!source.skip_cert_verification);
        assert_eq!(source.iam_role, None);
        assert_eq!(source.tls_host, None);
        assert_eq!(source.ca_certificate, None);
        assert_eq!(source.settings.replication_mode.to_string(), "cdc");
        assert_eq!(source.settings.publication_name, None);
        assert_eq!(source.settings.replication_slot_name, None);
        assert!(!source.settings.allow_nullable_columns);
        assert!(!source.settings.delete_on_merge);
        assert!(!source.settings.enable_failover_slots);
        assert_eq!(source.settings.initial_load_parallelism, None);
        assert_eq!(source.settings.pull_batch_size, None);
        assert_eq!(source.settings.snapshot_num_rows_per_partition, None);
        assert_eq!(source.settings.snapshot_number_of_parallel_tables, None);
        assert_eq!(source.settings.sync_interval_seconds, None);
        assert_eq!(source.table_mappings.len(), 1);
        let mapping = &source.table_mappings[0];
        assert_eq!(mapping.source_schema_name, "public");
        assert_eq!(mapping.source_table, "events");
        assert_eq!(mapping.target_table, "events");
        assert!(mapping.excluded_columns.is_empty());
        assert_eq!(mapping.partition_by_expr, "");
        assert_eq!(mapping.partition_key, "");
        assert!(mapping.sorting_keys.is_empty());
        assert!(!mapping.use_custom_sorting_key);
    }

    #[test]
    fn build_postgres_request_supports_maximal_fields_and_certificate_content() {
        let directory = tempfile::tempdir().unwrap();
        let ca_certificate = directory.path().join("postgres-ca.pem");
        std::fs::write(&ca_certificate, "POSTGRES_CA").unwrap();
        let mut args = postgres_builder_args();
        args.name = "maximal-pipe".into();
        args.host = "rds.example".into();
        args.port = 65535;
        args.pg_database = "production".into();
        // IAM_ROLE auth has no username or password: the role ARN is the whole
        // credential, so the `credentials` object is omitted entirely.
        args.username = None;
        args.password = None;
        args.table_mappings = vec![
            "public.users:users_raw".into(),
            "audit.events:audit_events".into(),
        ];
        args.postgres_type = "rdspostgres".into();
        args.replication_mode = "cdc_only".into();
        args.auth = "IAM_ROLE".into();
        args.iam_role = Some("arn:aws:iam::123456789012:role/clickpipe".into());
        args.tls_host = Some("database.internal".into());
        args.ca_certificate = Some(ca_certificate.to_string_lossy().into_owned());
        args.skip_cert_verification = true;
        args.publication_name = Some("clickpipe_publication".into());
        args.replication_slot_name = Some("clickpipe_slot".into());
        args.sync_interval_seconds = Some(30);
        args.pull_batch_size = Some(50_000);
        args.initial_load_parallelism = Some(4);
        args.snapshot_rows_per_partition = Some(1_000_000);
        args.snapshot_parallel_tables = Some(3);
        args.allow_nullable_columns = Some(true);
        args.enable_failover_slots = Some(true);
        args.delete_on_merge = Some(true);
        args.destination.destination_database = "analytics".into();
        args.destination_roles = DestinationRoleArgs {
            roles: vec!["analytics_reader".into(), "analytics_writer".into()],
        };

        let request = build_postgres_request(&args).unwrap();
        assert_eq!(request.name, "maximal-pipe");
        assert_eq!(request.destination.database, "analytics");
        assert_eq!(request.destination.table, None);
        assert_eq!(
            request.destination.roles,
            Some(vec![
                "analytics_reader".to_string(),
                "analytics_writer".to_string()
            ])
        );
        let source = request.source.postgres.as_ref().expect("postgres source");
        assert_eq!(source.r#type.as_ref().unwrap().to_string(), "rdspostgres");
        assert_eq!(source.authentication.to_string(), "IAM_ROLE");
        assert_eq!(source.credentials, None);
        assert_eq!(source.host, "rds.example");
        assert_eq!(source.port, 65535);
        assert_eq!(source.database, "production");
        assert_eq!(
            source.iam_role.as_deref(),
            Some("arn:aws:iam::123456789012:role/clickpipe")
        );
        assert_eq!(source.tls_host.as_deref(), Some("database.internal"));
        assert_eq!(source.ca_certificate.as_deref(), Some("POSTGRES_CA"));
        assert!(!source.disable_tls);
        assert!(source.skip_cert_verification);
        assert_eq!(source.settings.replication_mode.to_string(), "cdc_only");
        assert_eq!(
            source.settings.publication_name.as_deref(),
            Some("clickpipe_publication")
        );
        assert_eq!(
            source.settings.replication_slot_name.as_deref(),
            Some("clickpipe_slot")
        );
        assert_eq!(source.settings.sync_interval_seconds, Some(30));
        assert_eq!(source.settings.pull_batch_size, Some(50_000));
        assert_eq!(source.settings.initial_load_parallelism, Some(4));
        assert_eq!(
            source.settings.snapshot_num_rows_per_partition,
            Some(1_000_000)
        );
        assert_eq!(source.settings.snapshot_number_of_parallel_tables, Some(3));
        assert!(source.settings.allow_nullable_columns);
        assert!(source.settings.enable_failover_slots);
        assert!(source.settings.delete_on_merge);
        assert_eq!(source.table_mappings.len(), 2);
        assert_eq!(source.table_mappings[0].source_schema_name, "public");
        assert_eq!(source.table_mappings[0].source_table, "users");
        assert_eq!(source.table_mappings[0].target_table, "users_raw");
        assert_eq!(source.table_mappings[1].source_schema_name, "audit");
        assert_eq!(source.table_mappings[1].source_table, "events");
        assert_eq!(source.table_mappings[1].target_table, "audit_events");
    }

    #[test]
    fn build_postgres_request_supports_explicit_tls_opt_outs() {
        let mut args = postgres_builder_args();
        args.skip_cert_verification = true;
        let request = build_postgres_request(&args).unwrap();
        let source = request.source.postgres.as_ref().expect("postgres source");
        assert!(!source.disable_tls);
        assert!(source.skip_cert_verification);

        args.skip_cert_verification = false;
        args.disable_tls = true;
        let request = build_postgres_request(&args).unwrap();
        let source = request.source.postgres.as_ref().expect("postgres source");
        assert!(source.disable_tls);
        assert!(!source.skip_cert_verification);
    }

    #[test]
    fn build_postgres_pipe_settings_omits_unset_settings_and_sends_explicit_false() {
        let mut args = postgres_builder_args();
        args.sync_interval_seconds = Some(15);
        args.allow_nullable_columns = Some(false);
        args.delete_on_merge = Some(false);
        args.enable_failover_slots = Some(false);

        let settings = build_postgres_pipe_settings(&args).unwrap();
        assert_eq!(settings.sync_interval_seconds, Some(15));
        assert!(!settings.allow_nullable_columns);
        assert!(!settings.delete_on_merge);
        assert!(!settings.enable_failover_slots);
        assert_eq!(settings.pull_batch_size, None);
        assert_eq!(settings.initial_load_parallelism, None);
        assert_eq!(settings.snapshot_num_rows_per_partition, None);
        assert_eq!(settings.snapshot_number_of_parallel_tables, None);

        // The three schema-required booleans always serialize; every other
        // unset setting is omitted rather than sent as a zero value.
        let value = serde_json::to_value(&settings).unwrap();
        let mut keys: Vec<&str> = value
            .as_object()
            .expect("settings object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "allowNullableColumns",
                "deleteOnMerge",
                "enableFailoverSlots",
                "replicationMode",
                "syncIntervalSeconds",
            ]
        );
    }

    #[test]
    fn build_postgres_pipe_settings_defaults_optional_booleans_to_false() {
        let settings = build_postgres_pipe_settings(&postgres_builder_args()).unwrap();

        assert!(!settings.allow_nullable_columns);
        assert!(!settings.delete_on_merge);
        assert!(!settings.enable_failover_slots);
    }

    #[test]
    fn build_postgres_request_trims_table_mapping_components() {
        let mut args = postgres_builder_args();
        args.table_mappings = vec![" public.events : events_raw ".into()];

        let request = build_postgres_request(&args).unwrap();
        let source = request.source.postgres.as_ref().expect("postgres source");
        let mapping = &source.table_mappings[0];
        assert_eq!(mapping.source_schema_name, "public");
        assert_eq!(mapping.source_table, "events");
        assert_eq!(mapping.target_table, "events_raw");
    }

    #[test]
    fn build_postgres_request_combines_both_table_mapping_flags() {
        let mut args = postgres_builder_args();
        args.table_mappings = vec!["public.events:events".into()];
        args.table_mappings_json = vec![
            MAXIMAL_TABLE_MAPPING_JSON.to_string(),
            r#"{"sourceSchemaName":"audit","sourceTable":"events","targetTable":"audit_events","tableEngine":"Null"}"#
                .to_string(),
        ];

        let request = build_postgres_request(&args).unwrap();
        let source = request.source.postgres.as_ref().expect("postgres source");

        // The simple mappings come first, then the JSON ones in order.
        assert_eq!(source.table_mappings.len(), 3);
        assert_eq!(
            source.table_mappings[0],
            ClickPipePostgresPipeTableMapping {
                source_schema_name: "public".into(),
                source_table: "events".into(),
                target_table: "events".into(),
                table_engine: ClickPipePostgresPipeTableMappingTableengine::ReplacingMergeTree,
                excluded_columns: Vec::new(),
                partition_by_expr: String::new(),
                partition_key: String::new(),
                sorting_keys: Vec::new(),
                use_custom_sorting_key: false,
            }
        );
        assert_eq!(
            source.table_mappings[1],
            ClickPipePostgresPipeTableMapping {
                source_schema_name: "public".into(),
                source_table: "users".into(),
                target_table: "users_raw".into(),
                excluded_columns: vec!["ssn".into(), "dob".into()],
                sorting_keys: vec!["created_at".into(), "id".into()],
                use_custom_sorting_key: true,
                partition_by_expr: "toYYYYMM(created_at)".into(),
                partition_key: "id".into(),
                table_engine: ClickPipePostgresPipeTableMappingTableengine::ReplacingMergeTree,
            }
        );
        assert_eq!(
            source.table_mappings[2].table_engine,
            ClickPipePostgresPipeTableMappingTableengine::Null
        );
    }

    #[test]
    fn build_postgres_request_accepts_json_mappings_alone() {
        let mut args = postgres_builder_args();
        args.table_mappings.clear();
        args.table_mappings_json = vec![MAXIMAL_TABLE_MAPPING_JSON.to_string()];

        let request = build_postgres_request(&args).unwrap();
        let source = request.source.postgres.as_ref().expect("postgres source");
        assert_eq!(source.table_mappings.len(), 1);
        assert_eq!(source.table_mappings[0].target_table, "users_raw");
    }

    #[test]
    fn build_postgres_request_preserves_basic_auth_replication_modes() {
        for mode in REPLICATION_MODES {
            let mut args = postgres_builder_args();
            args.replication_mode = (*mode).into();
            if *mode == "cdc_only" {
                args.publication_name = Some("publication".into());
                args.replication_slot_name = Some("slot".into());
            }

            let request = build_postgres_request(&args).unwrap();
            let source = request.source.postgres.as_ref().expect("postgres source");
            assert_eq!(source.authentication.to_string(), "basic");
            assert_eq!(source.iam_role, None);
            assert_eq!(source.settings.replication_mode.to_string(), *mode);
        }
    }

    /// Every `--postgres-type` value maps to a named library variant rather
    /// than falling through to `Unknown`, and serializes back to the same
    /// string, so the server routes the connection to the type the user chose.
    #[test]
    fn build_postgres_request_maps_every_postgres_type_to_a_known_variant() {
        use clickhouse_cloud_api::models::ClickPipeMutatePostgresSourceType;

        for postgres_type in POSTGRES_TYPES {
            let mut args = postgres_builder_args();
            args.postgres_type = (*postgres_type).into();

            let request = build_postgres_request(&args).unwrap();
            let source_type = request
                .source
                .postgres
                .as_ref()
                .and_then(|source| source.r#type.as_ref())
                .expect("postgres source type");
            assert!(
                !matches!(source_type, ClickPipeMutatePostgresSourceType::Unknown(_)),
                "{postgres_type} fell through to Unknown"
            );
            assert_eq!(
                serde_json::to_value(source_type).unwrap(),
                serde_json::json!(postgres_type)
            );
        }
    }

    #[test]
    fn build_postgres_request_defensively_rejects_invalid_inputs() {
        let cases = [
            {
                let mut args = postgres_builder_args();
                args.port = 0;
                (args, &["--port", "1..=65535"][..])
            },
            {
                let mut args = postgres_builder_args();
                args.table_mappings.clear();
                (
                    args,
                    &[
                        "--table-mapping <SCHEMA.TABLE:TARGET_TABLE>",
                        "--table-mapping-json <JSON>",
                    ][..],
                )
            },
            {
                let mut args = postgres_builder_args();
                args.table_mappings_json = vec!["{}".into()];
                (args, &["--table-mapping-json #1: ", "sourceSchemaName"][..])
            },
            {
                let mut args = postgres_builder_args();
                args.table_mappings = vec![".events:events".into()];
                (args, &["'.events:events'"][..])
            },
            {
                let mut args = postgres_builder_args();
                args.auth = "IAM_ROLE".into();
                (args, &["--auth IAM_ROLE", "--iam-role <IAM_ROLE>"][..])
            },
            {
                let mut args = postgres_builder_args();
                args.iam_role = Some("arn:role".into());
                (args, &["--iam-role", "--auth basic"][..])
            },
            {
                let mut args = postgres_builder_args();
                args.replication_slot_name = Some("slot".into());
                (
                    args,
                    &["--replication-slot-name", "--replication-mode cdc_only"][..],
                )
            },
            {
                let mut args = postgres_builder_args();
                args.disable_tls = true;
                args.tls_host = Some("postgres.internal.example".into());
                (args, &["--tls-host", "--disable-tls"][..])
            },
            {
                let mut args = postgres_builder_args();
                args.disable_tls = true;
                args.ca_certificate = Some("secret-ca-path".into());
                (args, &["--ca-certificate", "--disable-tls"][..])
            },
            {
                let mut args = postgres_builder_args();
                args.disable_tls = true;
                args.skip_cert_verification = true;
                (args, &["--skip-cert-verification", "--disable-tls"][..])
            },
        ];

        // Each case breaks one rule; the error names the flags involved.
        for (args, names) in cases {
            let error = build_postgres_request(&args).unwrap_err();
            assert_names(&error.message, names);
        }
    }
}
