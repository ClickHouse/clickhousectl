use crate::cloud::clickpipes::cli::{
    ClickPipeCreateValidationArgs, DatabaseDestinationArgs, DestinationRoleArgs,
};
use crate::cloud::clickpipes::create::{
    apply_create_request_args, build_create_validation_args, print_created,
};
use crate::cloud::clickpipes::destination::{build_destination, build_destination_roles};
use crate::cloud::clickpipes::sources::DB_AUTHS;
use crate::cloud::clickpipes::table_mapping::{
    parse_mysql_table_mapping_json, parse_postgres_table_mapping,
    parse_postgres_table_mapping_parts,
};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult, read_flag_file};
use crate::cloud::shared::{parse_serde_enum, resolve_org_id};
use clap::builder::{NonEmptyStringValueParser, PossibleValuesParser};
use clap::{ArgGroup, Args};
use clickhouse_cloud_api::models::{
    ClickPipeMutateMySQLSourceType, ClickPipeMySQLPipeSettingsReplicationmechanism,
    ClickPipeMySQLPipeSettingsReplicationmode,
};

#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("mysql_table_mappings")
        .required(true)
        .multiple(true)
        .args(["table_mappings", "table_mappings_json"])
))]
pub struct MySqlCreateArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub validation: ClickPipeCreateValidationArgs,

    /// MySQL host
    #[arg(long)]
    pub host: String,

    /// MySQL port (1-65535)
    #[arg(
        long,
        default_value = "3306",
        value_parser = clap::value_parser!(u16).range(1..=65535)
    )]
    pub port: u16,

    /// Username (required with --auth basic; invalid with --auth IAM_ROLE)
    #[arg(long, requires = "password")]
    pub username: Option<String>,

    /// Password (required with --auth basic; invalid with --auth IAM_ROLE)
    #[arg(long, requires = "username")]
    pub password: Option<String>,

    /// Table mappings as schema.table:target_table (repeatable)
    ///
    /// Leaves every other per-table option at the ClickPipes default.
    #[arg(
        long = "table-mapping",
        value_name = "SCHEMA.TABLE:TARGET_TABLE",
        value_parser = parse_postgres_table_mapping
    )]
    pub table_mappings: Vec<String>,

    /// Full table mapping as a JSON object (repeatable)
    ///
    /// Supports excludedColumns, sortingKeys, useCustomSortingKey,
    /// partitionKey, partitionByExpr and tableEngine. Combinable with
    /// --table-mapping; unknown fields are rejected.
    #[arg(long = "table-mapping-json", value_name = "JSON")]
    pub table_mappings_json: Vec<String>,

    /// MySQL type
    #[arg(
        long,
        default_value = "mysql",
        value_parser = PossibleValuesParser::new(ClickPipeMutateMySQLSourceType::VALUES),
    )]
    pub mysql_type: String,

    /// Replication mode
    #[arg(
        long,
        default_value = "cdc",
        value_parser = PossibleValuesParser::new(ClickPipeMySQLPipeSettingsReplicationmode::VALUES),
    )]
    pub replication_mode: String,

    /// Replication mechanism
    #[arg(
        long,
        default_value = "GTID",
        value_parser = PossibleValuesParser::new(ClickPipeMySQLPipeSettingsReplicationmechanism::VALUES),
    )]
    pub replication_mechanism: String,

    /// Authentication method
    #[arg(
        long,
        default_value = "basic",
        value_parser = PossibleValuesParser::new(DB_AUTHS),
    )]
    pub auth: String,

    /// IAM role ARN (required with --auth IAM_ROLE; invalid with basic auth)
    ///
    /// IAM_ROLE applies to RDS and Aurora MySQL sources only.
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

    /// MySQL server_id used in the replication topology (1-4294967295)
    ///
    /// Must be unique across every replica connected to the source. Assigned
    /// automatically when omitted.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..=4294967295))]
    pub server_id: Option<u64>,

    /// Interval in seconds to sync data during CDC replication
    #[arg(long, value_name = "SECONDS", value_parser = clap::value_parser!(i64).range(1..))]
    pub sync_interval_seconds: Option<i64>,

    /// Number of rows to pull in each CDC batch
    #[arg(long, value_name = "ROWS", value_parser = clap::value_parser!(i64).range(1..))]
    pub pull_batch_size: Option<i64>,

    /// Parallel workers per table in the initial snapshot phase
    #[arg(long, value_name = "WORKERS", value_parser = clap::value_parser!(i64).range(1..))]
    pub initial_load_parallelism: Option<i64>,

    /// Number of rows per partition during the snapshot phase
    #[arg(long, value_name = "ROWS", value_parser = clap::value_parser!(i64).range(1000..))]
    pub snapshot_rows_per_partition: Option<i64>,

    /// Tables to snapshot in parallel during the initial load phase
    #[arg(long, value_name = "TABLES", value_parser = clap::value_parser!(i64).range(1..))]
    pub snapshot_parallel_tables: Option<i64>,

    /// Preserve MySQL nullability in destination columns
    #[arg(long, value_name = "true|false")]
    pub allow_nullable_columns: Option<bool>,

    /// Enable hard deletes in ReplacingMergeTree for MySQL DELETEs
    #[arg(long, value_name = "true|false")]
    pub delete_on_merge: Option<bool>,

    /// Enable compression for the MySQL connection
    #[arg(long, value_name = "true|false")]
    pub use_compression: Option<bool>,

    #[command(flatten)]
    pub destination: DatabaseDestinationArgs,

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,
}

/// Check the `clickpipe create mysql` flag relationships clap cannot express,
/// because each one depends on the value of `--auth` rather than its presence.
pub(in crate::cloud::clickpipes) fn validate_mysql_create_args(
    args: &MySqlCreateArgs,
) -> CloudResult<()> {
    resolve_mysql_table_mappings(args)?;
    // Clap enforces this for parsed input via `required_if_eq`; hand-built
    // args reach it here.
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
    if args.disable_tls
        && (args.tls_host.is_some() || args.ca_certificate.is_some() || args.skip_cert_verification)
    {
        return Err(CloudError::usage(
            "--disable-tls cannot be combined with --tls-host, --ca-certificate, or --skip-cert-verification",
        ));
    }
    for (flag, value, minimum) in [
        ("--sync-interval-seconds", args.sync_interval_seconds, 1),
        ("--pull-batch-size", args.pull_batch_size, 1),
        (
            "--initial-load-parallelism",
            args.initial_load_parallelism,
            1,
        ),
        (
            "--snapshot-rows-per-partition",
            args.snapshot_rows_per_partition,
            1000,
        ),
        (
            "--snapshot-parallel-tables",
            args.snapshot_parallel_tables,
            1,
        ),
    ] {
        if value.is_some_and(|value| value < minimum) {
            return Err(CloudError::usage(format!(
                "{flag} must be at least {minimum}"
            )));
        }
    }

    Ok(())
}

fn resolve_mysql_table_mappings(
    args: &MySqlCreateArgs,
) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeMySQLPipeTableMapping>> {
    use clickhouse_cloud_api::models::ClickPipeMySQLPipeTableMapping;

    if args.table_mappings.is_empty() && args.table_mappings_json.is_empty() {
        return Err(CloudError::usage(
            "at least one --table-mapping <SCHEMA.TABLE:TARGET_TABLE> or \
             --table-mapping-json <JSON> is required",
        ));
    }
    let mut mappings =
        Vec::with_capacity(args.table_mappings.len() + args.table_mappings_json.len());
    for mapping in &args.table_mappings {
        let (source_schema_name, source_table, target_table) =
            parse_postgres_table_mapping_parts(mapping)?;
        mappings.push(ClickPipeMySQLPipeTableMapping {
            source_schema_name,
            source_table,
            target_table,
            partition_by_expr: None,
            excluded_columns: None,
            partition_key: None,
            sorting_keys: None,
            table_engine: None,
            use_custom_sorting_key: None,
        });
    }
    for (position, mapping) in args.table_mappings_json.iter().enumerate() {
        mappings.push(parse_mysql_table_mapping_json(position, mapping)?);
    }
    Ok(mappings)
}

fn build_mysql_request(
    args: &MySqlCreateArgs,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostRequest> {
    use clickhouse_cloud_api::models::{
        ClickPipeMutateMySQLSource, ClickPipeMutateMySQLSourceAuthentication,
        ClickPipeMySQLPipeSettings, ClickPipePostRequest, ClickPipePostSource, PLAIN,
    };

    let request_args = build_create_validation_args(&args.validation);
    validate_mysql_create_args(args)?;
    let source_type = parse_serde_enum(
        &args.mysql_type,
        "MySQL source type",
        ClickPipeMutateMySQLSourceType::VALUES,
    )?;
    let replication_mode = parse_serde_enum(
        &args.replication_mode,
        "MySQL replication mode",
        ClickPipeMySQLPipeSettingsReplicationmode::VALUES,
    )?;
    let replication_mechanism = parse_serde_enum(
        &args.replication_mechanism,
        "MySQL replication mechanism",
        ClickPipeMySQLPipeSettingsReplicationmechanism::VALUES,
    )?;
    let authentication: ClickPipeMutateMySQLSourceAuthentication =
        parse_serde_enum(&args.auth, "MySQL authentication", DB_AUTHS)?;
    let table_mappings = resolve_mysql_table_mappings(args)?;

    let ca_certificate = args
        .ca_certificate
        .as_deref()
        .map(|path| read_flag_file("--ca-certificate", path))
        .transpose()?;

    // Match the parsed authentication mode, not the raw `--auth` string, so a
    // new mode added to the library enum is a compile error here rather than a
    // silent credential-less create.
    let credentials = match &authentication {
        // `validate_mysql_create_args` has already required the pair for basic
        // auth, so `zip` yields `Some` for every invocation that reaches here.
        ClickPipeMutateMySQLSourceAuthentication::Basic => args
            .username
            .as_deref()
            .zip(args.password.as_deref())
            .map(|(username, password)| PLAIN {
                username: username.to_string(),
                password: password.to_string(),
            }),
        // The role ARN is the whole credential: the `credentials` object must
        // stay off the wire entirely.
        ClickPipeMutateMySQLSourceAuthentication::IAM_ROLE => None,
        // Unreachable for parsed input, since --auth is restricted to
        // DB_AUTHS; these modes have no credential shape to send.
        ClickPipeMutateMySQLSourceAuthentication::ServiceAccountWorkloadIdentity
        | ClickPipeMutateMySQLSourceAuthentication::Unknown(_) => None,
    };

    let source = ClickPipeMutateMySQLSource {
        r#type: Some(source_type),
        credentials,
        host: args.host.clone(),
        port: i64::from(args.port),
        authentication: Some(authentication),
        iam_role: args.iam_role.clone(),
        tls_host: args.tls_host.clone(),
        ca_certificate,
        disable_tls: if args.disable_tls { Some(true) } else { None },
        skip_cert_verification: if args.skip_cert_verification {
            Some(true)
        } else {
            None
        },
        server_id: args.server_id.map(|value| value as i64),
        settings: ClickPipeMySQLPipeSettings {
            replication_mode,
            replication_mechanism: Some(replication_mechanism),
            allow_nullable_columns: args.allow_nullable_columns,
            delete_on_merge: args.delete_on_merge,
            initial_load_parallelism: args.initial_load_parallelism,
            pull_batch_size: args.pull_batch_size,
            snapshot_num_rows_per_partition: args.snapshot_rows_per_partition,
            snapshot_number_of_parallel_tables: args.snapshot_parallel_tables,
            sync_interval_seconds: args.sync_interval_seconds,
            use_compression: args.use_compression,
        },
        table_mappings,
    };

    let mut request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            mysql: Some(source),
            bigquery: None,
            kafka: None,
            kinesis: None,
            mongodb: None,
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

pub(in crate::cloud::clickpipes) async fn clickpipe_create_mysql(
    client: &CloudClient,
    args: &MySqlCreateArgs,
    json: bool,
) -> CloudResult<()> {
    let request = build_mysql_request(args)?;
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
        MAXIMAL_MYSQL_TABLE_MAPPING_JSON, assert_names, assert_rejected, clickpipe_parse_error,
        clickpipe_validation_message, mysql_create_cli_args, names_word, parse_clickpipe,
    };
    use clickhouse_cloud_api::models::ClickPipeMySQLPipeTableMappingTableengine;

    #[test]
    fn parses_mysql_flags_defaults_repeatability_and_server_id_range() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::MySQL(args),
        } = parse_clickpipe(&[
            "create",
            "mysql",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "mysql.example",
            "--port",
            "3307",
            "--table-mapping",
            "source.one:one",
            "--table-mapping",
            "source.two:two",
            "--table-mapping-json",
            r#"{"sourceSchemaName":"source","sourceTable":"three","targetTable":"three"}"#,
            "--mysql-type",
            "mariadb",
            "--replication-mode",
            "cdc_only",
            "--replication-mechanism",
            "FILE_POS",
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:role",
            "--tls-host",
            "tls.example",
            "--ca-certificate",
            "/tmp/ca.pem",
            "--skip-cert-verification",
            "--server-id",
            "4294967295",
            "--sync-interval-seconds",
            "1",
            "--pull-batch-size",
            "2",
            "--initial-load-parallelism",
            "3",
            "--snapshot-rows-per-partition",
            "1000",
            "--snapshot-parallel-tables",
            "4",
            "--allow-nullable-columns",
            "false",
            "--delete-on-merge",
            "true",
            "--use-compression",
            "false",
            "--destination-database",
            "analytics",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected mysql create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.host, "mysql.example");
        assert_eq!(args.port, 3307);
        // IAM_ROLE authentication takes the role ARN, not a credential pair.
        assert_eq!(args.username, None);
        assert_eq!(args.password, None);
        assert_eq!(args.table_mappings, ["source.one:one", "source.two:two"]);
        assert_eq!(args.table_mappings_json.len(), 1);
        assert_eq!(args.mysql_type, "mariadb");
        assert_eq!(args.replication_mode, "cdc_only");
        assert_eq!(args.replication_mechanism, "FILE_POS");
        assert_eq!(args.auth, "IAM_ROLE");
        assert_eq!(args.iam_role.as_deref(), Some("arn:role"));
        assert_eq!(args.tls_host.as_deref(), Some("tls.example"));
        assert_eq!(args.ca_certificate.as_deref(), Some("/tmp/ca.pem"));
        assert!(!args.disable_tls);
        assert!(args.skip_cert_verification);
        assert_eq!(args.server_id, Some(4_294_967_295));
        assert_eq!(args.sync_interval_seconds, Some(1));
        assert_eq!(args.pull_batch_size, Some(2));
        assert_eq!(args.initial_load_parallelism, Some(3));
        assert_eq!(args.snapshot_rows_per_partition, Some(1000));
        assert_eq!(args.snapshot_parallel_tables, Some(4));
        assert_eq!(args.allow_nullable_columns, Some(false));
        assert_eq!(args.delete_on_merge, Some(true));
        assert_eq!(args.use_compression, Some(false));
        assert_eq!(args.destination.destination_database, "analytics");

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::MySQL(args),
        } = parse_clickpipe(&[
            "create",
            "mysql",
            "svc-1",
            "--name",
            "pipe-1",
            "--host",
            "mysql.example",
            "--username",
            "user",
            "--password",
            "password",
            "--table-mapping-json",
            r#"{"sourceSchemaName":"source","sourceTable":"events","targetTable":"events"}"#,
            "--server-id",
            "1",
        ])
        else {
            panic!("expected mysql create");
        };
        assert_eq!(args.port, 3306);
        assert_eq!(args.username.as_deref(), Some("user"));
        assert_eq!(args.password.as_deref(), Some("password"));
        assert!(args.table_mappings.is_empty());
        assert_eq!(args.table_mappings_json.len(), 1);
        assert_eq!(args.mysql_type, "mysql");
        assert_eq!(args.replication_mode, "cdc");
        assert_eq!(args.replication_mechanism, "GTID");
        assert_eq!(args.auth, "basic");
        assert_eq!(args.iam_role, None);
        assert_eq!(args.tls_host, None);
        assert_eq!(args.ca_certificate, None);
        assert!(!args.disable_tls);
        assert!(!args.skip_cert_verification);
        assert_eq!(args.server_id, Some(1));
        assert_eq!(args.sync_interval_seconds, None);
        assert_eq!(args.pull_batch_size, None);
        assert_eq!(args.initial_load_parallelism, None);
        assert_eq!(args.snapshot_rows_per_partition, None);
        assert_eq!(args.snapshot_parallel_tables, None);
        assert_eq!(args.allow_nullable_columns, None);
        assert_eq!(args.delete_on_merge, None);
        assert_eq!(args.use_compression, None);
        assert_eq!(args.destination.destination_database, "default");

        for invalid in ["0", "4294967296"] {
            assert_rejected(&[
                "create",
                "mysql",
                "svc-1",
                "--name",
                "pipe-1",
                "--host",
                "mysql.example",
                "--username",
                "user",
                "--password",
                "password",
                "--table-mapping",
                "source.events:events",
                "--server-id",
                invalid,
            ]);
        }

        for (flag, invalid) in [
            ("--sync-interval-seconds", "0"),
            ("--pull-batch-size", "0"),
            ("--initial-load-parallelism", "0"),
            ("--snapshot-rows-per-partition", "999"),
            ("--snapshot-parallel-tables", "0"),
        ] {
            let mut args = mysql_create_cli_args();
            args.extend([flag, invalid]);
            assert_rejected(&args);
        }
    }

    #[test]
    fn mysql_port_accepts_default_and_boundaries_and_rejects_out_of_range_values() {
        for (port, expected) in [(None, 3306), (Some("1"), 1), (Some("65535"), 65535)] {
            let mut args = mysql_create_cli_args();
            args.extend(["--username", "user", "--password", "password"]);
            if let Some(port) = port {
                args.extend(["--port", port]);
            }
            let ClickPipeCommands::Create {
                command: ClickPipeCreateCommands::MySQL(parsed),
            } = parse_clickpipe(&args)
            else {
                panic!("expected mysql create");
            };
            assert_eq!(parsed.port, expected);
        }

        for port in ["0", "65536"] {
            let mut args = mysql_create_cli_args();
            args.extend([
                "--username",
                "user",
                "--password",
                "password",
                "--port",
                port,
            ]);
            let error = clickpipe_parse_error(&args);
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            assert_eq!(error.exit_code(), 2);
        }
    }

    #[test]
    fn mysql_iam_role_auth_does_not_require_username_or_password() {
        // IAM_ROLE authentication has no username or password: the role ARN is
        // the whole credential, so clap must not demand the pair.
        let mut args = mysql_create_cli_args();
        args.extend([
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:aws:iam::123456789012:role/clickpipe",
        ]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::MySQL(parsed),
        } = parse_clickpipe(&args)
        else {
            panic!("expected mysql create");
        };
        assert_eq!(parsed.username, None);
        assert_eq!(parsed.password, None);
        assert_eq!(parsed.auth, "IAM_ROLE");
        assert_eq!(
            parsed.iam_role.as_deref(),
            Some("arn:aws:iam::123456789012:role/clickpipe")
        );
        assert_eq!(clickpipe_validation_message(&parse_clickpipe(&args)), None);

        // The pair is rejected rather than silently dropped, the same way
        // basic auth rejects --iam-role.
        args.extend(["--username", "user", "--password", "password"]);
        let (source, message) = parse_clickpipe(&args)
            .clickpipe_create_validation_error()
            .expect("pair is rejected");
        assert_eq!(source, "mysql");
        assert_names(
            &message,
            &[
                "--username",
                "--password",
                "--auth IAM_ROLE",
                "--auth basic",
            ],
        );
    }

    #[test]
    fn mysql_iam_role_only_error_names_the_role_flag_and_not_the_pair() {
        // `--auth IAM_ROLE` on its own must ask for --iam-role only: asking for
        // the credential pair would send the user into the "cannot be used with
        // --auth IAM_ROLE" rejection.
        let mut args = mysql_create_cli_args();
        args.extend(["--auth", "IAM_ROLE"]);
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
    fn mysql_basic_auth_without_any_credentials_is_a_validation_error() {
        // clap only pairs the two flags, so neither being present parses
        // cleanly and `validate_mysql_create_args` owns the basic-auth rule:
        // it depends on --auth's value, which clap cannot express.
        let args = mysql_create_cli_args();
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::MySQL(parsed),
        } = parse_clickpipe(&args)
        else {
            panic!("expected mysql create");
        };
        assert_eq!(parsed.auth, "basic");
        assert_eq!(parsed.username, None);
        assert_eq!(parsed.password, None);

        let (source, message) = parse_clickpipe(&args)
            .clickpipe_create_validation_error()
            .expect("basic auth needs the pair");
        assert_eq!(source, "mysql");
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
    fn mysql_basic_auth_still_requires_username_and_password() {
        // The two flags are joined with clap `requires`, so half a pair is a
        // usage error naming the missing half rather than the whole set.
        for omitted in ["--username", "--password"] {
            let mut args = mysql_create_cli_args();
            args.extend(["--username", "user", "--password", "password"]);
            let position = args.iter().position(|arg| *arg == omitted).unwrap();
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
    fn mysql_iam_role_with_basic_auth_is_a_validation_error() {
        // The reverse of the credential rule, and the guard MySQL previously
        // lacked entirely: basic auth has no use for a role ARN.
        let mut args = mysql_create_cli_args();
        args.extend([
            "--username",
            "user",
            "--password",
            "password",
            "--iam-role",
            "arn:aws:iam::123456789012:role/clickpipe",
        ]);
        let (source, message) = parse_clickpipe(&args)
            .clickpipe_create_validation_error()
            .expect("role on basic auth");
        assert_eq!(source, "mysql");
        assert_names(&message, &["--iam-role", "--auth basic", "--auth IAM_ROLE"]);
        assert!(!message.contains("--username"), "{message}");
    }

    fn mysql_builder_args() -> MySqlCreateArgs {
        MySqlCreateArgs {
            service_id: "svc-1".into(),
            name: "pipe-1".into(),
            validation: ClickPipeCreateValidationArgs::default(),
            host: "mysql.example".into(),
            port: 3306,
            username: Some("user".into()),
            password: Some("password".into()),
            table_mappings: vec!["source.events:events".into()],
            table_mappings_json: vec![],
            mysql_type: "mysql".into(),
            replication_mode: "cdc".into(),
            replication_mechanism: "GTID".into(),
            auth: "basic".into(),
            iam_role: None,
            tls_host: None,
            ca_certificate: None,
            disable_tls: false,
            skip_cert_verification: false,
            server_id: None,
            sync_interval_seconds: None,
            pull_batch_size: None,
            initial_load_parallelism: None,
            snapshot_rows_per_partition: None,
            snapshot_parallel_tables: None,
            allow_nullable_columns: None,
            delete_on_merge: None,
            use_compression: None,
            destination: DatabaseDestinationArgs {
                destination_database: "default".into(),
            },
            destination_roles: DestinationRoleArgs::default(),
        }
    }

    #[test]
    fn build_mysql_request_sends_basic_credentials() {
        let request = build_mysql_request(&mysql_builder_args()).unwrap();

        assert_eq!(request.name, "pipe-1");
        assert_eq!(request.destination.database, "default");
        assert_eq!(request.destination.table, None);
        assert_eq!(request.destination.roles, None);
        assert!(request.source.postgres.is_none());
        assert!(request.source.kafka.is_none());

        let source = request.source.mysql.as_ref().expect("mysql source");
        assert_eq!(source.r#type.as_ref().unwrap().to_string(), "mysql");
        assert_eq!(source.authentication.as_ref().unwrap().to_string(), "basic");
        let credentials = source.credentials.as_ref().expect("basic credentials");
        assert_eq!(credentials.username, "user");
        assert_eq!(credentials.password, "password");
        assert_eq!(source.host, "mysql.example");
        assert_eq!(source.port, 3306);
        assert_eq!(source.iam_role, None);
        assert_eq!(source.tls_host, None);
        assert_eq!(source.ca_certificate, None);
        assert_eq!(source.disable_tls, None);
        assert_eq!(source.skip_cert_verification, None);
        assert_eq!(source.server_id, None);
        assert_eq!(source.settings.replication_mode.to_string(), "cdc");
        assert_eq!(
            source
                .settings
                .replication_mechanism
                .as_ref()
                .unwrap()
                .to_string(),
            "GTID"
        );
        assert_eq!(source.settings.allow_nullable_columns, None);
        assert_eq!(source.settings.delete_on_merge, None);
        assert_eq!(source.settings.initial_load_parallelism, None);
        assert_eq!(source.settings.pull_batch_size, None);
        assert_eq!(source.settings.snapshot_num_rows_per_partition, None);
        assert_eq!(source.settings.snapshot_number_of_parallel_tables, None);
        assert_eq!(source.settings.sync_interval_seconds, None);
        assert_eq!(source.settings.use_compression, None);
        assert_eq!(source.table_mappings.len(), 1);
        assert_eq!(source.table_mappings[0].source_schema_name, "source");
        assert_eq!(source.table_mappings[0].source_table, "events");
        assert_eq!(source.table_mappings[0].target_table, "events");
    }

    #[test]
    fn build_mysql_request_omits_the_credentials_object_for_iam_role() {
        let directory = tempfile::tempdir().unwrap();
        let ca_certificate = directory.path().join("mysql-ca.pem");
        std::fs::write(&ca_certificate, "MYSQL_CA").unwrap();
        let mut args = mysql_builder_args();
        args.name = "maximal-pipe".into();
        args.host = "rds.example".into();
        args.port = 3307;
        // IAM_ROLE auth has no username or password: the role ARN is the whole
        // credential, so the `credentials` object is omitted entirely.
        args.username = None;
        args.password = None;
        args.table_mappings = vec!["source.users:users_raw".into()];
        args.table_mappings_json = vec![MAXIMAL_MYSQL_TABLE_MAPPING_JSON.into()];
        args.mysql_type = "rdsmysql".into();
        args.replication_mode = "cdc_only".into();
        args.replication_mechanism = "FILE_POS".into();
        args.auth = "IAM_ROLE".into();
        args.iam_role = Some("arn:aws:iam::123456789012:role/clickpipe".into());
        args.tls_host = Some("database.internal".into());
        args.ca_certificate = Some(ca_certificate.to_string_lossy().into_owned());
        args.disable_tls = false;
        args.skip_cert_verification = true;
        args.server_id = Some(4_294_967_295);
        args.sync_interval_seconds = Some(1);
        args.pull_batch_size = Some(2);
        args.initial_load_parallelism = Some(3);
        args.snapshot_rows_per_partition = Some(1000);
        args.snapshot_parallel_tables = Some(4);
        args.allow_nullable_columns = Some(false);
        args.delete_on_merge = Some(true);
        args.use_compression = Some(false);
        args.destination.destination_database = "analytics".into();
        args.destination_roles = DestinationRoleArgs {
            roles: vec!["analytics_reader".into()],
        };

        let request = build_mysql_request(&args).unwrap();
        assert_eq!(request.name, "maximal-pipe");
        assert_eq!(request.destination.database, "analytics");
        assert_eq!(
            request.destination.roles,
            Some(vec!["analytics_reader".to_string()])
        );

        let source = request.source.mysql.as_ref().expect("mysql source");
        assert_eq!(source.r#type.as_ref().unwrap().to_string(), "rdsmysql");
        assert_eq!(
            source.authentication.as_ref().unwrap().to_string(),
            "IAM_ROLE"
        );
        assert_eq!(source.credentials, None);
        assert_eq!(
            source.iam_role.as_deref(),
            Some("arn:aws:iam::123456789012:role/clickpipe")
        );
        assert_eq!(source.host, "rds.example");
        assert_eq!(source.port, 3307);
        assert_eq!(source.tls_host.as_deref(), Some("database.internal"));
        // The file contents are sent, not the path.
        assert_eq!(source.ca_certificate.as_deref(), Some("MYSQL_CA"));
        assert_eq!(source.disable_tls, None);
        assert_eq!(source.skip_cert_verification, Some(true));
        assert_eq!(source.server_id, Some(4_294_967_295));
        assert_eq!(source.settings.replication_mode.to_string(), "cdc_only");
        assert_eq!(
            source
                .settings
                .replication_mechanism
                .as_ref()
                .unwrap()
                .to_string(),
            "FILE_POS"
        );
        assert_eq!(source.settings.sync_interval_seconds, Some(1));
        assert_eq!(source.settings.pull_batch_size, Some(2));
        assert_eq!(source.settings.initial_load_parallelism, Some(3));
        assert_eq!(source.settings.snapshot_num_rows_per_partition, Some(1000));
        assert_eq!(source.settings.snapshot_number_of_parallel_tables, Some(4));
        assert_eq!(source.settings.allow_nullable_columns, Some(false));
        assert_eq!(source.settings.delete_on_merge, Some(true));
        assert_eq!(source.settings.use_compression, Some(false));
        assert_eq!(source.table_mappings.len(), 2);
        let mapping = &source.table_mappings[1];
        assert_eq!(mapping.source_schema_name, "sales");
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
        assert_eq!(mapping.partition_key.as_deref(), Some("id"));
        assert_eq!(
            mapping.partition_by_expr.as_deref(),
            Some("toYYYYMM(created_at)")
        );
        assert_eq!(
            mapping.table_engine,
            Some(ClickPipeMySQLPipeTableMappingTableengine::ReplacingMergeTree)
        );
    }

    #[test]
    fn build_mysql_request_defensively_rejects_invalid_credential_combinations() {
        let cases = [
            {
                let mut args = mysql_builder_args();
                args.auth = "IAM_ROLE".into();
                (args, &["--auth IAM_ROLE", "--iam-role <IAM_ROLE>"][..])
            },
            {
                let mut args = mysql_builder_args();
                args.iam_role = Some("arn:role".into());
                (args, &["--iam-role", "--auth basic"][..])
            },
            {
                let mut args = mysql_builder_args();
                args.auth = "IAM_ROLE".into();
                args.iam_role = Some("arn:role".into());
                (
                    args,
                    &[
                        "--username",
                        "--password",
                        "--auth IAM_ROLE",
                        "--auth basic",
                    ][..],
                )
            },
            {
                let mut args = mysql_builder_args();
                args.username = None;
                args.password = None;
                (
                    args,
                    &[
                        "--auth basic",
                        "--username <USERNAME>",
                        "--password <PASSWORD>",
                    ][..],
                )
            },
            {
                let mut args = mysql_builder_args();
                args.table_mappings = vec!["source.events".into()];
                (args, &["'source.events'"][..])
            },
            {
                let mut args = mysql_builder_args();
                args.sync_interval_seconds = Some(0);
                (args, &["--sync-interval-seconds"][..])
            },
            {
                let mut args = mysql_builder_args();
                args.disable_tls = true;
                args.skip_cert_verification = true;
                (args, &["--disable-tls", "--skip-cert-verification"][..])
            },
        ];

        // Each case breaks one rule; the error names the flags involved.
        for (args, names) in cases {
            let error = build_mysql_request(&args).unwrap_err();
            assert_names(&error.message, names);
        }

        // A minimum names its bound.
        let mut args = mysql_builder_args();
        args.sync_interval_seconds = Some(0);
        let message = build_mysql_request(&args).unwrap_err().message;
        assert!(names_word(&message, "1"), "{message}");
    }
}
