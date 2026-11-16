use super::cli::{
    ClickPipeCommands, ClickPipeCreateCommands, ClickPipeCreateRequestArgs,
    ClickPipeSchemaDiscoverCommands, DestinationRoleArgs, StreamingDestinationArgs,
    StreamingDestinationTableArgs,
};
use super::sources::kafka::{KafkaCreateArgs, KafkaSourceFields};
use super::sources::object_storage::ObjectStorageSourceFields;
use super::sources::pubsub::{PubSubCreateArgs, PubSubSourceFields};
use crate::cli::{Cli, Commands};
use crate::cloud::cli::CloudCommands;
use clap::Parser;

/// Asserts `message` names every identifier in `names` (a flag, field,
/// value or suggested command) without pinning the prose around them.
#[track_caller]
pub(super) fn assert_names(message: &str, names: &[&str]) {
    for name in names {
        assert!(message.contains(name), "expected `{name}` in: {message}");
    }
}

/// Whether `message` carries `value` as a whole word, so a number such as
/// `8` is not matched inside `2000`.
pub(super) fn names_word(message: &str, value: &str) -> bool {
    message
        .split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))
        .any(|word| word.trim_end_matches('.') == value)
}

fn parse_cloud_command(args: &[&str]) -> CloudCommands {
    let cli = Cli::try_parse_from(
        ["clickhousectl", "cloud", "clickpipe"]
            .into_iter()
            .chain(args.iter().copied()),
    )
    .expect("ClickPipe command should parse");
    let Commands::Cloud(cloud) = cli.command else {
        panic!("expected cloud command");
    };
    crate::cloud::cli::tests::assert_org_selector(&cloud, args);
    cloud.command
}

pub(super) fn parse_clickpipe(args: &[&str]) -> ClickPipeCommands {
    let CloudCommands::ClickPipe { command } = parse_cloud_command(args) else {
        panic!("expected clickpipe command");
    };
    *command
}

/// The `clickpipe create` validation message for a parsed command,
/// dropping the source subcommand the usage error is reported against.
pub(super) fn clickpipe_validation_message(command: &ClickPipeCommands) -> Option<String> {
    command
        .clickpipe_create_validation_error()
        .map(|(_, message)| message)
}

pub(super) fn assert_rejected(args: &[&str]) {
    assert!(
        Cli::try_parse_from(
            ["clickhousectl", "cloud", "clickpipe"]
                .into_iter()
                .chain(args.iter().copied())
        )
        .is_err(),
        "expected parse failure for: {}",
        args.join(" ")
    );
}

pub(super) fn clickpipe_parse_error(args: &[&str]) -> clap::Error {
    Cli::try_parse_from(
        ["clickhousectl", "cloud", "clickpipe"]
            .into_iter()
            .chain(args.iter().copied()),
    )
    .err()
    .unwrap_or_else(|| panic!("expected parse failure for: {}", args.join(" ")))
}

/// Broker credentials that satisfy the Kafka authentication requirement.
const KAFKA_TEST_AUTH: [&str; 6] = [
    "--auth",
    "PLAIN",
    "--username",
    "user",
    "--password",
    "password",
];

/// Minimal `clickpipe create kafka` invocation, with PLAIN credentials.
pub(super) fn kafka_create_cli_args() -> Vec<&'static str> {
    let mut args = kafka_create_cli_args_without_auth();
    args.extend(KAFKA_TEST_AUTH);
    args
}

/// Minimal `clickpipe create kafka` invocation, before any auth flags.
pub(super) fn kafka_create_cli_args_without_auth() -> Vec<&'static str> {
    vec![
        "create",
        "kafka",
        "svc-1",
        "--name",
        "pipe-1",
        "--brokers",
        "broker:9092",
        "--topics",
        "topic",
        "--format",
        "JSONEachRow",
        "--database",
        "db",
        "--table",
        "events",
        "--column",
        "id:UInt64",
    ]
}

pub(super) fn object_storage_create_cli_args() -> Vec<&'static str> {
    vec![
        "create",
        "object-storage",
        "svc-1",
        "--name",
        "pipe-1",
        "--source-url",
        "https://bucket.example/data/*.json",
        "--format",
        "JSONEachRow",
        "--database",
        "db",
        "--table",
        "events",
    ]
}

/// Minimal `clickpipe schema-discover kafka <SERVICE_ID>` invocation, with
/// PLAIN credentials.
pub(super) fn kafka_discover_cli_args() -> Vec<&'static str> {
    let mut args = kafka_discover_cli_args_without_auth();
    args.extend(KAFKA_TEST_AUTH);
    args
}

/// Minimal `clickpipe schema-discover kafka <SERVICE_ID>` invocation, before
/// any auth flags. `KafkaSourceFields` is flattened into both commands, so
/// credential-pairing and authentication rules must hold for each.
pub(super) fn kafka_discover_cli_args_without_auth() -> Vec<&'static str> {
    vec![
        "schema-discover",
        "kafka",
        "svc-1",
        "--brokers",
        "broker:9092",
        "--topics",
        "topic",
        "--format",
        "JSONEachRow",
    ]
}

pub(super) fn postgres_cli_args(mapping: Option<&str>) -> Vec<&str> {
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
        "--username",
        "user",
        "--password",
        "password",
    ];
    if let Some(mapping) = mapping {
        args.extend(["--table-mapping", mapping]);
    }
    args
}

pub(super) fn assert_write(args: &[&str], expected: bool) {
    assert_eq!(
        parse_cloud_command(args).is_write_command(),
        expected,
        "wrong classification for: {}",
        args.join(" ")
    );
}

/// Write a service-account key file and return its directory (which the
/// caller keeps alive) plus the path to pass to --service-account-file.
pub(super) fn service_account_key_file() -> (tempfile::TempDir, String) {
    use std::io::Write as _;

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("service-account.json");
    std::fs::File::create(&path)
        .expect("create service account file")
        .write_all(br#"{"type":"service_account"}"#)
        .expect("write service account file");
    let path = path.to_str().expect("utf-8 temp path").to_string();
    (dir, path)
}

/// base64 of the `service_account_key_file` contents, which is what the
/// request carries — the path is never sent.
pub(super) const SERVICE_ACCOUNT_KEY_BASE64: &str = "eyJ0eXBlIjoic2VydmljZV9hY2NvdW50In0=";

/// The required Pub/Sub source flags, with a key path that is only read
/// when a request is built.
pub(super) fn pubsub_source_flags(service_account_file: &str) -> Vec<&str> {
    vec![
        "--topic",
        "events",
        "--project-id",
        "my-gcp-project",
        "--format",
        "JSONEachRow",
        "--seek-type",
        "earliest",
        "--service-account-file",
        service_account_file,
    ]
}

/// Parse `create pubsub` args, so the builder tests exercise the real
/// clap defaults rather than hand-built structs.
pub(super) fn parse_pubsub_create(flags: &[&str]) -> PubSubCreateArgs {
    let mut args = vec![
        "create",
        "pubsub",
        "svc-1",
        "--name",
        "pipe-1",
        "--database",
        "db",
        "--table",
        "events",
    ];
    args.extend(flags.iter().copied());
    if !flags.contains(&"--column") {
        args.extend(["--column", "id:UInt64"]);
    }
    let ClickPipeCommands::Create {
        command: ClickPipeCreateCommands::PubSub(args),
    } = parse_clickpipe(&args)
    else {
        panic!("expected pubsub create");
    };
    args
}

/// Parse `schema-discover pubsub` args and return the source fields.
pub(super) fn parse_pubsub_discovery(flags: &[&str]) -> Box<PubSubSourceFields> {
    let mut args = vec!["schema-discover", "pubsub", "svc-1"];
    args.extend(flags.iter().copied());
    let ClickPipeCommands::SchemaDiscover {
        command: ClickPipeSchemaDiscoverCommands::PubSub(source),
        ..
    } = parse_clickpipe(&args)
    else {
        panic!("expected pubsub schema discovery");
    };
    Box::new(source.source)
}

pub(super) fn assert_pubsub_value(flag: &str, value: &str) {
    let mut flags = pubsub_source_flags("./sa-key.json");
    // Replace the baseline value for a flag the minimal invocation already
    // carries, so no flag is passed twice.
    match flags.iter().position(|arg| *arg == flag) {
        Some(index) => flags[index + 1] = value,
        None => flags.extend([flag, value]),
    }
    parse_pubsub_create(&flags);
}

pub(super) fn assert_object_storage_value(flag: &str, value: &str) {
    if flag == "--format" {
        parse_clickpipe(&[
            "create",
            "object-storage",
            "--column",
            "id:Int64",
            "svc-1",
            "--name",
            "pipe-1",
            "--source-url",
            "https://bucket.example/data",
            "--format",
            value,
            "--database",
            "db",
            "--table",
            "events",
        ]);
        return;
    }
    parse_clickpipe(&[
        "create",
        "object-storage",
        "--column",
        "id:Int64",
        "svc-1",
        "--name",
        "pipe-1",
        "--source-url",
        "https://bucket.example/data",
        "--format",
        "JSONEachRow",
        "--database",
        "db",
        "--table",
        "events",
        flag,
        value,
    ]);
}

pub(super) fn assert_kafka_value(flag: &str, value: &str) {
    if flag == "--format" {
        parse_clickpipe(&[
            "create",
            "kafka",
            "svc-1",
            "--name",
            "pipe-1",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            value,
            "--database",
            "db",
            "--table",
            "events",
            "--column",
            "id:UInt64",
            "--iam-role",
            "arn:role",
        ]);
        return;
    }
    parse_clickpipe(&[
        "create",
        "kafka",
        "svc-1",
        "--name",
        "pipe-1",
        "--brokers",
        "broker:9092",
        "--topics",
        "topic",
        "--format",
        "JSONEachRow",
        "--database",
        "db",
        "--table",
        "events",
        "--column",
        "id:UInt64",
        "--iam-role",
        "arn:role",
        flag,
        value,
    ]);
}

pub(super) fn assert_kinesis_value(flag: &str, value: &str) {
    if flag == "--format" {
        let mut args = vec![
            "create",
            "kinesis",
            "svc-1",
            "--name",
            "pipe-1",
            "--stream-name",
            "stream-1",
            "--region",
            "us-east-1",
            "--format",
            value,
            "--database",
            "db",
            "--table",
            "events",
            "--column",
            "id:UInt64",
        ];
        if value == "Protobuf" {
            args.extend(["--protobuf-schema-file", "schema.proto"]);
        }
        parse_clickpipe(&args);
        return;
    }
    parse_clickpipe(&[
        "create",
        "kinesis",
        "svc-1",
        "--name",
        "pipe-1",
        "--stream-name",
        "stream-1",
        "--region",
        "us-east-1",
        "--format",
        "JSONEachRow",
        "--database",
        "db",
        "--table",
        "events",
        "--column",
        "id:UInt64",
        flag,
        value,
    ]);
}

pub(super) fn assert_postgres_value(flag: &str, value: &str) {
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
        flag,
        value,
    ];
    // Each auth mode gets only its own credential flags: the CLI rejects
    // --username/--password with IAM_ROLE, so pairing them here would
    // assert an invocation the CLI refuses to run.
    if flag == "--auth" && value == "IAM_ROLE" {
        args.extend(["--iam-role", "arn:aws:iam::123456789012:role/clickpipe"]);
    } else {
        args.extend(["--username", "user", "--password", "password"]);
    }
    parse_clickpipe(&args);
}

pub(super) fn assert_mysql_value(flag: &str, value: &str) {
    let mut args = vec![
        "create",
        "mysql",
        "svc-1",
        "--name",
        "pipe-1",
        "--host",
        "mysql.example",
        "--table-mapping",
        "source.events:events",
        flag,
        value,
    ];
    // Each auth mode gets only its own credential flags: the CLI rejects
    // --username/--password with IAM_ROLE, so pairing them here would
    // assert an invocation the CLI refuses to run.
    if flag == "--auth" && value == "IAM_ROLE" {
        args.extend(["--iam-role", "arn:aws:iam::123456789012:role/clickpipe"]);
    } else {
        args.extend(["--username", "user", "--password", "password"]);
    }
    parse_clickpipe(&args);
}

pub(super) fn assert_mongodb_value(flag: &str, value: &str) {
    parse_clickpipe(&[
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
        value,
    ]);
}

pub(super) fn assert_bigquery_value(flag: &str, value: &str) {
    parse_clickpipe(&[
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
}

/// One `--table-mapping-json` object with every field set, reused by the
/// clap, validation and builder tests.
pub(super) const MAXIMAL_TABLE_MAPPING_JSON: &str = r#"{
    "sourceSchemaName": "public",
    "sourceTable": "users",
    "targetTable": "users_raw",
    "excludedColumns": ["ssn", "dob"],
    "sortingKeys": ["created_at", "id"],
    "useCustomSortingKey": true,
    "partitionByExpr": "toYYYYMM(created_at)",
    "partitionKey": "id",
    "tableEngine": "ReplacingMergeTree"
}"#;

/// Minimal `clickpipe create mysql` invocation, before any auth flags.
pub(super) fn mysql_create_cli_args() -> Vec<&'static str> {
    vec![
        "create",
        "mysql",
        "svc-1",
        "--name",
        "pipe-1",
        "--host",
        "mysql.example",
        "--table-mapping",
        "source.events:events",
    ]
}

/// Parse `schema-discover object-storage` args and return the source
/// fields, so the builder tests exercise the real clap defaults.
pub(super) fn parse_object_storage_discovery(flags: &[&str]) -> Box<ObjectStorageSourceFields> {
    let mut args = vec!["schema-discover", "object-storage", "svc-1"];
    args.extend(flags.iter().copied());
    let ClickPipeCommands::SchemaDiscover {
        command: ClickPipeSchemaDiscoverCommands::ObjectStorage(source),
        ..
    } = parse_clickpipe(&args)
    else {
        panic!("expected object-storage schema discovery");
    };
    Box::new(source.source)
}

pub(super) const MAXIMAL_MYSQL_TABLE_MAPPING_JSON: &str = r#"{
    "sourceSchemaName":"sales",
    "sourceTable":"orders",
    "targetTable":"orders_raw",
    "excludedColumns":["internal_id","temporary"],
    "sortingKeys":["created_at","id"],
    "useCustomSortingKey":true,
    "partitionKey":"id",
    "partitionByExpr":"toYYYYMM(created_at)",
    "tableEngine":"ReplacingMergeTree"
}"#;

pub(super) const MAXIMAL_MONGODB_TABLE_MAPPING_JSON: &str = r#"{
    "sourceDatabaseName":"sales",
    "sourceCollection":"orders",
    "targetTable":"orders_raw",
    "tableEngine":"Null"
}"#;

pub(super) const MAXIMAL_BIGQUERY_TABLE_MAPPING_JSON: &str = r#"{
    "sourceDatasetName":"sales",
    "sourceTable":"orders",
    "targetTable":"orders_raw",
    "excludedColumns":["internal_id","temporary"],
    "sortingKeys":["created_at","id"],
    "useCustomSortingKey":true,
    "tableEngine":"MergeTree"
}"#;

pub(super) fn kafka_args() -> KafkaCreateArgs {
    KafkaCreateArgs {
        service_id: "svc".into(),
        name: "pipe".into(),
        request: ClickPipeCreateRequestArgs::default(),
        source: KafkaSourceFields {
            brokers: "b:9092".into(),
            topics: "t".into(),
            format: "JSONEachRow".into(),
            kafka_type: "kafka".into(),
            consumer_group: None,
            auth: None,
            event_hubs_connection_string: None,
            username: None,
            password: None,
            iam_role: None,
            access_key_id: None,
            secret_key: None,
            offset: "from_beginning".into(),
            offset_timestamp: None,
            schema_registry_type: None,
            schema_registry_url: None,
            schema_registry_username: None,
            schema_registry_password: None,
            glue_region: None,
            glue_registry_name: None,
            glue_role_arn: None,
            protobuf_schema_file: None,
            ca_certificate: None,
            client_certificate: None,
            client_key: None,
            schema_registry_ca_certificate: None,
            reverse_private_endpoint_ids: vec![],
        },
        exactly_once: None,
        destination: StreamingDestinationArgs {
            database: "d".into(),
            table: "t".into(),
            columns: vec![],
            columns_from: None,
            destination_table: StreamingDestinationTableArgs::default(),
            destination_roles: DestinationRoleArgs::default(),
            resolved_columns: None,
        },
    }
}

/// Kafka source args with complete PLAIN credentials, for tests about
/// other fields.
pub(super) fn kafka_source_with_plain_auth() -> KafkaSourceFields {
    let mut source = kafka_args().source;
    source.auth = Some("PLAIN".into());
    source.username = Some("user".into());
    source.password = Some("password".into());
    source
}
