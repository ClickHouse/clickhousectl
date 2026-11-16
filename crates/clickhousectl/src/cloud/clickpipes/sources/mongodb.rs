use crate::cloud::clickpipes::cli::{
    ClickPipeCreateValidationArgs, DatabaseDestinationArgs, DestinationRoleArgs,
};
use crate::cloud::clickpipes::create::{
    apply_create_request_args, build_create_validation_args, print_created,
};
use crate::cloud::clickpipes::destination::{build_destination, build_destination_roles};
use crate::cloud::clickpipes::table_mapping::{
    parse_mongodb_table_mapping, parse_mongodb_table_mapping_json,
    parse_mongodb_table_mapping_parts,
};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult, read_flag_file};
use crate::cloud::shared::{parse_serde_enum, resolve_org_id};
use clap::builder::{NonEmptyStringValueParser, PossibleValuesParser};
use clap::{ArgGroup, Args};
use clickhouse_cloud_api::models::{
    ClickPipeMongoDBPipeSettingsReplicationmode, ClickPipeMutateMongoDBSourceReadpreference,
};

#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("mongodb_table_mappings")
        .required(true)
        .multiple(true)
        .args(["table_mappings", "table_mappings_json"])
))]
pub struct MongoDbCreateArgs {
    /// Service ID
    #[arg(value_parser = NonEmptyStringValueParser::new())]
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub validation: ClickPipeCreateValidationArgs,

    /// MongoDB connection URI (e.g., mongodb+srv://cluster0.example.mongodb.net/mydb)
    #[arg(long)]
    pub uri: String,

    /// Username
    #[arg(long)]
    pub username: String,

    /// Password
    #[arg(long)]
    pub password: String,

    /// Table mappings as database.collection:target_table (repeatable)
    #[arg(
        long = "table-mapping",
        value_name = "DATABASE.COLLECTION:TARGET_TABLE",
        value_parser = parse_mongodb_table_mapping
    )]
    pub table_mappings: Vec<String>,

    /// Full table mapping as a JSON object (repeatable)
    ///
    /// Supports tableEngine, structuredIngestion (typed columns) and mongoConfig.
    /// Combinable with --table-mapping; unknown fields are rejected.
    #[arg(long = "table-mapping-json", value_name = "JSON")]
    pub table_mappings_json: Vec<String>,

    /// Replication mode
    #[arg(
        long,
        default_value = "cdc",
        value_parser = PossibleValuesParser::new(ClickPipeMongoDBPipeSettingsReplicationmode::VALUES),
    )]
    pub replication_mode: String,

    /// Read preference
    #[arg(
        long,
        default_value = "secondaryPreferred",
        value_parser = PossibleValuesParser::new(ClickPipeMutateMongoDBSourceReadpreference::VALUES),
    )]
    pub read_preference: String,

    /// Certificate hostname override (defaults to the --uri host)
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

    /// Interval in seconds to sync data during CDC replication
    #[arg(long, value_name = "SECONDS", value_parser = clap::value_parser!(i64).range(1..))]
    pub sync_interval_seconds: Option<i64>,

    /// Number of rows to pull in each CDC batch
    #[arg(long, value_name = "ROWS", value_parser = clap::value_parser!(i64).range(1..))]
    pub pull_batch_size: Option<i64>,

    /// Parallel workers per collection during the initial snapshot
    #[arg(long, value_name = "WORKERS", value_parser = clap::value_parser!(i64).range(1..))]
    pub initial_load_parallelism: Option<i64>,

    /// Number of rows per partition during the snapshot phase
    #[arg(long, value_name = "ROWS", value_parser = clap::value_parser!(i64).range(1000..))]
    pub snapshot_rows_per_partition: Option<i64>,

    /// Collections to snapshot in parallel during the initial load phase
    #[arg(long, value_name = "COLLECTIONS", value_parser = clap::value_parser!(i64).range(1..))]
    pub snapshot_parallel_collections: Option<i64>,

    /// Enable hard deletes in ReplacingMergeTree for MongoDB DELETEs
    #[arg(long, value_name = "true|false")]
    pub delete_on_merge: Option<bool>,

    /// Store JSON values in the native ClickHouse JSON format
    #[arg(long, value_name = "true|false")]
    pub use_json_native_format: Option<bool>,

    #[command(flatten)]
    pub destination: DatabaseDestinationArgs,

    #[command(flatten)]
    pub destination_roles: DestinationRoleArgs,
}

pub(in crate::cloud::clickpipes) fn validate_mongodb_create_args(
    args: &MongoDbCreateArgs,
) -> CloudResult<()> {
    resolve_mongodb_table_mappings(args)?;
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
            "--snapshot-parallel-collections",
            args.snapshot_parallel_collections,
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

fn resolve_mongodb_table_mappings(
    args: &MongoDbCreateArgs,
) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeMongoDBPipeTableMapping>> {
    use clickhouse_cloud_api::models::ClickPipeMongoDBPipeTableMapping;

    if args.table_mappings.is_empty() && args.table_mappings_json.is_empty() {
        return Err(CloudError::usage(
            "at least one --table-mapping <DATABASE.COLLECTION:TARGET_TABLE> or \
             --table-mapping-json <JSON> is required",
        ));
    }
    let mut mappings =
        Vec::with_capacity(args.table_mappings.len() + args.table_mappings_json.len());
    for mapping in &args.table_mappings {
        let (source_database_name, source_collection, target_table) =
            parse_mongodb_table_mapping_parts(mapping)?;
        mappings.push(ClickPipeMongoDBPipeTableMapping {
            source_database_name,
            source_collection,
            target_table,
            mongo_config: None,
            structured_ingestion: None,
            table_engine: None,
        });
    }
    for (position, mapping) in args.table_mappings_json.iter().enumerate() {
        mappings.push(parse_mongodb_table_mapping_json(position, mapping)?);
    }
    Ok(mappings)
}

fn build_mongodb_request(
    args: &MongoDbCreateArgs,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePostRequest> {
    use clickhouse_cloud_api::models::{
        ClickPipeMongoDBPipeSettings, ClickPipeMutateMongoDBSource, ClickPipePostRequest,
        ClickPipePostSource, PLAIN,
    };

    let request_args = build_create_validation_args(&args.validation);
    validate_mongodb_create_args(args)?;
    let read_preference = parse_serde_enum(
        &args.read_preference,
        "MongoDB read preference",
        ClickPipeMutateMongoDBSourceReadpreference::VALUES,
    )?;
    let replication_mode = parse_serde_enum(
        &args.replication_mode,
        "MongoDB replication mode",
        ClickPipeMongoDBPipeSettingsReplicationmode::VALUES,
    )?;

    let table_mappings = resolve_mongodb_table_mappings(args)?;

    let ca_certificate = args
        .ca_certificate
        .as_deref()
        .map(|path| read_flag_file("--ca-certificate", path))
        .transpose()?;

    let source = ClickPipeMutateMongoDBSource {
        credentials: Some(PLAIN {
            username: args.username.clone(),
            password: args.password.clone(),
        }),
        uri: args.uri.clone(),
        read_preference,
        tls_host: args.tls_host.clone(),
        ca_certificate,
        disable_tls: if args.disable_tls { Some(true) } else { None },
        skip_cert_verification: if args.skip_cert_verification {
            Some(true)
        } else {
            None
        },
        settings: ClickPipeMongoDBPipeSettings {
            initial_load_parallelism: args.initial_load_parallelism,
            replication_mode,
            delete_on_merge: args.delete_on_merge,
            pull_batch_size: args.pull_batch_size,
            snapshot_num_rows_per_partition: args.snapshot_rows_per_partition,
            snapshot_number_of_parallel_tables: args.snapshot_parallel_collections,
            sync_interval_seconds: args.sync_interval_seconds,
            use_json_native_format: args.use_json_native_format,
        },
        table_mappings,
    };

    let mut request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            mongodb: Some(source),
            bigquery: None,
            kafka: None,
            kinesis: None,
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

pub(in crate::cloud::clickpipes) async fn clickpipe_create_mongodb(
    client: &CloudClient,
    args: &MongoDbCreateArgs,
    json: bool,
) -> CloudResult<()> {
    let request = build_mongodb_request(args)?;
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
        MAXIMAL_MONGODB_TABLE_MAPPING_JSON, assert_names, assert_rejected, names_word,
        parse_clickpipe,
    };
    use clickhouse_cloud_api::models::ClickPipeMongoDBPipeTableMappingTableengine;

    #[test]
    fn parses_mongodb_flags_defaults_and_repeatability() {
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::MongoDB(args),
        } = parse_clickpipe(&[
            "create",
            "mongodb",
            "svc-1",
            "--name",
            "pipe-1",
            "--uri",
            "mongodb://mongo.example/source",
            "--username",
            "user",
            "--password",
            "password",
            "--table-mapping",
            "source.one:one",
            "--table-mapping",
            "source.two:two",
            "--table-mapping-json",
            r#"{"sourceDatabaseName":"source","sourceCollection":"three","targetTable":"three"}"#,
            "--replication-mode",
            "snapshot",
            "--read-preference",
            "nearest",
            "--tls-host",
            "tls.example",
            "--ca-certificate",
            "/tmp/ca.pem",
            "--skip-cert-verification",
            "--sync-interval-seconds",
            "1",
            "--pull-batch-size",
            "2",
            "--snapshot-rows-per-partition",
            "1000",
            "--snapshot-parallel-collections",
            "3",
            "--delete-on-merge",
            "false",
            "--use-json-native-format",
            "true",
            "--destination-database",
            "analytics",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected mongodb create");
        };
        assert_eq!(args.service_id, "svc-1");
        assert_eq!(args.name, "pipe-1");
        assert_eq!(args.uri, "mongodb://mongo.example/source");
        assert_eq!(args.username, "user");
        assert_eq!(args.password, "password");
        assert_eq!(args.table_mappings, ["source.one:one", "source.two:two"]);
        assert_eq!(args.table_mappings_json.len(), 1);
        assert_eq!(args.replication_mode, "snapshot");
        assert_eq!(args.read_preference, "nearest");
        assert_eq!(args.tls_host.as_deref(), Some("tls.example"));
        assert_eq!(args.ca_certificate.as_deref(), Some("/tmp/ca.pem"));
        assert!(!args.disable_tls);
        assert!(args.skip_cert_verification);
        assert_eq!(args.sync_interval_seconds, Some(1));
        assert_eq!(args.pull_batch_size, Some(2));
        assert_eq!(args.snapshot_rows_per_partition, Some(1000));
        assert_eq!(args.snapshot_parallel_collections, Some(3));
        assert_eq!(args.delete_on_merge, Some(false));
        assert_eq!(args.use_json_native_format, Some(true));
        assert_eq!(args.destination.destination_database, "analytics");

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::MongoDB(args),
        } = parse_clickpipe(&[
            "create",
            "mongodb",
            "svc-1",
            "--name",
            "pipe-1",
            "--uri",
            "mongodb://mongo.example/source",
            "--username",
            "user",
            "--password",
            "password",
            "--table-mapping-json",
            r#"{"sourceDatabaseName":"source","sourceCollection":"events","targetTable":"events"}"#,
        ])
        else {
            panic!("expected mongodb create");
        };
        assert!(args.table_mappings.is_empty());
        assert_eq!(args.table_mappings_json.len(), 1);
        assert_eq!(args.replication_mode, "cdc");
        assert_eq!(args.read_preference, "secondaryPreferred");
        assert_eq!(args.tls_host, None);
        assert_eq!(args.ca_certificate, None);
        assert!(!args.disable_tls);
        assert!(!args.skip_cert_verification);
        assert_eq!(args.sync_interval_seconds, None);
        assert_eq!(args.pull_batch_size, None);
        assert_eq!(args.snapshot_rows_per_partition, None);
        assert_eq!(args.snapshot_parallel_collections, None);
        assert_eq!(args.delete_on_merge, None);
        assert_eq!(args.use_json_native_format, None);
        assert_eq!(args.destination.destination_database, "default");

        for (flag, invalid) in [
            ("--sync-interval-seconds", "0"),
            ("--pull-batch-size", "0"),
            ("--snapshot-rows-per-partition", "999"),
            ("--snapshot-parallel-collections", "0"),
        ] {
            assert_rejected(&[
                "create",
                "mongodb",
                "svc-1",
                "--name",
                "pipe-1",
                "--uri",
                "mongodb://mongo.example/source",
                "--username",
                "user",
                "--password",
                "password",
                "--table-mapping",
                "source.events:events",
                flag,
                invalid,
            ]);
        }
    }

    #[test]
    fn mongodb_initial_load_parallelism_parses_validates_and_preserves_omission() {
        let base = [
            "create",
            "mongodb",
            "svc-1",
            "--name",
            "pipe",
            "--uri",
            "mongodb://host/db",
            "--username",
            "user",
            "--password",
            "pass",
            "--table-mapping",
            "db.events:events",
        ];
        for (value, expected) in [
            (None, None),
            (Some("1"), Some(1)),
            (Some("9223372036854775807"), Some(i64::MAX)),
        ] {
            let mut args = base.to_vec();
            if let Some(value) = value {
                args.extend(["--initial-load-parallelism", value]);
            }
            let ClickPipeCommands::Create {
                command: ClickPipeCreateCommands::MongoDB(args),
            } = parse_clickpipe(&args)
            else {
                panic!("mongodb");
            };
            assert_eq!(args.initial_load_parallelism, expected);
            let request = build_mongodb_request(&args).unwrap();
            assert_eq!(
                request
                    .source
                    .mongodb
                    .unwrap()
                    .settings
                    .initial_load_parallelism,
                expected
            );
        }
        for value in ["0", "-1", "1.5", "9223372036854775808"] {
            let mut args = base.to_vec();
            args.extend(["--initial-load-parallelism", value]);
            assert_rejected(&args);
        }
        let mut args = mongodb_builder_args();
        args.initial_load_parallelism = Some(0);
        let message = build_mongodb_request(&args).unwrap_err().message;
        assert_names(&message, &["--initial-load-parallelism"]);
        assert!(names_word(&message, "1"), "{message}");
    }

    fn mongodb_builder_args() -> MongoDbCreateArgs {
        MongoDbCreateArgs {
            service_id: "svc-1".into(),
            name: "pipe-1".into(),
            validation: ClickPipeCreateValidationArgs::default(),
            uri: "mongodb://mongo.example/source".into(),
            username: "user".into(),
            password: "password".into(),
            table_mappings: vec!["source.events:events".into()],
            table_mappings_json: vec![],
            replication_mode: "cdc".into(),
            read_preference: "secondaryPreferred".into(),
            tls_host: None,
            ca_certificate: None,
            disable_tls: false,
            skip_cert_verification: false,
            sync_interval_seconds: None,
            pull_batch_size: None,
            initial_load_parallelism: None,
            snapshot_rows_per_partition: None,
            snapshot_parallel_collections: None,
            delete_on_merge: None,
            use_json_native_format: None,
            destination: DatabaseDestinationArgs {
                destination_database: "default".into(),
            },
            destination_roles: DestinationRoleArgs::default(),
        }
    }

    #[test]
    fn build_mongodb_request_supports_minimal_fields() {
        let request = build_mongodb_request(&mongodb_builder_args()).unwrap();

        assert_eq!(request.destination.database, "default");
        assert_eq!(request.destination.table, None);
        assert_eq!(request.destination.roles, None);
        let source = request.source.mongodb.as_ref().expect("mongodb source");
        assert_eq!(source.uri, "mongodb://mongo.example/source");
        assert_eq!(source.table_mappings.len(), 1);
        assert_eq!(source.table_mappings[0].source_database_name, "source");
        assert_eq!(source.table_mappings[0].source_collection, "events");
        assert_eq!(source.table_mappings[0].target_table, "events");
        assert_eq!(source.skip_cert_verification, None);
        assert_eq!(source.settings.sync_interval_seconds, None);
        assert_eq!(source.settings.pull_batch_size, None);
        assert_eq!(source.settings.initial_load_parallelism, None);
        assert_eq!(source.settings.snapshot_num_rows_per_partition, None);
        assert_eq!(source.settings.snapshot_number_of_parallel_tables, None);
        assert_eq!(source.settings.delete_on_merge, None);
        assert_eq!(source.settings.use_json_native_format, None);
    }

    #[test]
    fn build_mongodb_request_supports_maximal_fields() {
        let directory = tempfile::tempdir().unwrap();
        let ca_certificate = directory.path().join("mongodb-ca.pem");
        std::fs::write(&ca_certificate, "MONGODB_CA").unwrap();
        let mut args = mongodb_builder_args();
        args.table_mappings = vec!["sales.orders:orders_raw".into()];
        args.table_mappings_json = vec![MAXIMAL_MONGODB_TABLE_MAPPING_JSON.into()];
        args.replication_mode = "snapshot".into();
        args.read_preference = "nearest".into();
        args.tls_host = Some("mongodb.internal".into());
        args.ca_certificate = Some(ca_certificate.to_string_lossy().into_owned());
        args.disable_tls = false;
        args.skip_cert_verification = true;
        args.sync_interval_seconds = Some(1);
        args.pull_batch_size = Some(2);
        args.initial_load_parallelism = Some(4);
        args.snapshot_rows_per_partition = Some(1000);
        args.snapshot_parallel_collections = Some(3);
        args.delete_on_merge = Some(false);
        args.use_json_native_format = Some(true);
        args.destination.destination_database = "analytics".into();
        args.destination_roles.roles = vec!["analytics_reader".into()];

        let request = build_mongodb_request(&args).unwrap();

        assert_eq!(request.destination.database, "analytics");
        assert_eq!(
            request.destination.roles,
            Some(vec!["analytics_reader".into()])
        );
        let source = request.source.mongodb.as_ref().expect("mongodb source");
        assert_eq!(source.read_preference.to_string(), "nearest");
        assert_eq!(source.tls_host.as_deref(), Some("mongodb.internal"));
        assert_eq!(source.ca_certificate.as_deref(), Some("MONGODB_CA"));
        assert_eq!(source.disable_tls, None);
        assert_eq!(source.skip_cert_verification, Some(true));
        assert_eq!(source.settings.replication_mode.to_string(), "snapshot");
        assert_eq!(source.settings.sync_interval_seconds, Some(1));
        assert_eq!(source.settings.pull_batch_size, Some(2));
        assert_eq!(source.settings.initial_load_parallelism, Some(4));
        assert_eq!(source.settings.snapshot_num_rows_per_partition, Some(1000));
        assert_eq!(source.settings.snapshot_number_of_parallel_tables, Some(3));
        assert_eq!(source.settings.delete_on_merge, Some(false));
        assert_eq!(source.settings.use_json_native_format, Some(true));
        assert_eq!(source.table_mappings.len(), 2);
        assert_eq!(source.table_mappings[1].source_database_name, "sales");
        assert_eq!(source.table_mappings[1].source_collection, "orders");
        assert_eq!(source.table_mappings[1].target_table, "orders_raw");
        assert_eq!(
            source.table_mappings[1].table_engine,
            Some(ClickPipeMongoDBPipeTableMappingTableengine::Null)
        );
    }

    #[test]
    fn build_mongodb_request_defensively_validates_ranges_and_tls_flags() {
        let mut args = mongodb_builder_args();
        args.snapshot_rows_per_partition = Some(999);
        let error = build_mongodb_request(&args).unwrap_err();
        assert_names(&error.message, &["--snapshot-rows-per-partition"]);
        assert!(names_word(&error.message, "1000"), "{}", error.message);

        args.snapshot_rows_per_partition = None;
        args.disable_tls = true;
        args.tls_host = Some("mongo.internal".into());
        let error = build_mongodb_request(&args).unwrap_err();
        assert_names(&error.message, &["--disable-tls", "--tls-host"]);
    }
}
