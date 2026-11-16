use super::cli::{StreamingDestinationArgs, StreamingDestinationTableArgs};
use super::non_empty;
use crate::cloud::client::{CloudError, Result as CloudResult};
use crate::cloud::config::{deserialize_strict_config, read_flag_config_value};

/// Parse `name:type` column specifications into library destination columns.
pub(super) fn parse_columns(
    columns: &[String],
) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>> {
    columns.iter().map(|column| parse_column(column)).collect()
}

fn parse_column(
    column: &str,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipeDestinationColumn> {
    let (name, column_type) = column.split_once(':').ok_or_else(|| {
        CloudError::usage(format!(
            "Invalid column format '{}': expected name:type",
            column
        ))
    })?;
    Ok(clickhouse_cloud_api::models::ClickPipeDestinationColumn {
        name: name.to_string(),
        r#type: column_type.to_string(),
    })
}

/// Validate the same grammar as the request builder before credential lookup.
pub(super) fn parse_destination_column(column: &str) -> Result<String, String> {
    parse_column(column)
        .map(|_| column.to_string())
        .map_err(|error| error.message)
}

/// Role names the API reserves for ClickPipes itself and rejects in
/// `destination.roles`. Caught client-side so the failure names the flag
/// instead of relaying a server-side validation error.
const RESERVED_DESTINATION_ROLES: &[&str] = &["clickpipes", "clickpipes_system"];

/// Validate one `--role` value, returning the trimmed role name.
fn parse_destination_role_name(role: &str) -> CloudResult<String> {
    let trimmed = role.trim();
    if trimmed.is_empty() {
        return Err(CloudError::usage("a role name must not be empty"));
    }
    if RESERVED_DESTINATION_ROLES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(trimmed))
    {
        return Err(CloudError::usage(format!(
            "role names reserved by ClickPipes cannot be granted: {}",
            RESERVED_DESTINATION_ROLES.join(", ")
        )));
    }
    Ok(trimmed.to_string())
}

/// clap `value_parser` wrapper for `--role`, so a bad name is a usage error
/// (exit 2) formatted against the create subcommand that was invoked, before
/// any credential lookup or network call.
pub(super) fn parse_destination_role(role: &str) -> Result<String, String> {
    parse_destination_role_name(role).map_err(|error| error.message)
}

/// Resolve the repeatable `--role` values into `destination.roles`. Every value
/// is already validated and trimmed by `parse_destination_role`; duplicates are
/// dropped with declaration order preserved, and no flags at all leave the
/// field absent from the request body.
pub(super) fn build_destination_roles(roles: &[String]) -> Option<Vec<String>> {
    if roles.is_empty() {
        return None;
    }
    let mut deduped: Vec<String> = Vec::with_capacity(roles.len());
    for role in roles {
        if !deduped.iter().any(|existing| existing == role) {
            deduped.push(role.clone());
        }
    }
    Some(deduped)
}

fn read_destination_table_definition(
    config_file: &str,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipeDestinationTableDefinition> {
    use clickhouse_cloud_api::models::ClickPipeDestinationTableEngineType;

    let mut value = read_flag_config_value("--table-definition-file", config_file)?;
    // TTL was added after table-definition files shipped. Preserve omission in
    // existing files; the strict library request omits this empty sentinel when
    // serialized. Explicit nulls or wrong types still fail strict parsing.
    if let Some(object) = value.as_object_mut() {
        object
            .entry("ttl")
            .or_insert_with(|| serde_json::Value::String(String::new()));
    }
    let definition: clickhouse_cloud_api::models::ClickPipeDestinationTableDefinition =
        deserialize_strict_config(value, config_file)?;
    if let ClickPipeDestinationTableEngineType::Unknown(value) = &definition.engine.r#type {
        return Err(CloudError::usage(format!(
            "invalid destination table definition in config {config_file}: unknown engine.type value `{value}`"
        )));
    }
    Ok(definition)
}

/// The table definition sent for a managed destination table when no
/// `--table-definition-file` is given: a MergeTree engine with no keys. The
/// empty `ttl` is omitted when serialized.
fn managed_merge_tree_table_definition()
-> clickhouse_cloud_api::models::ClickPipeDestinationTableDefinition {
    use clickhouse_cloud_api::models::{
        ClickPipeDestinationTableDefinition, ClickPipeDestinationTableEngine,
        ClickPipeDestinationTableEngineType,
    };

    ClickPipeDestinationTableDefinition {
        engine: ClickPipeDestinationTableEngine {
            column_ids: Vec::new(),
            r#type: ClickPipeDestinationTableEngineType::MergeTree,
            version_column_id: None,
        },
        partition_by: String::new(),
        primary_key: String::new(),
        sorting_key: Vec::new(),
        ttl: String::new(),
    }
}

/// Build the destination shared by Kafka, Kinesis, object-storage, and Pub/Sub.
/// Omitting both new flags retains the historical managed MergeTree request.
pub(super) fn build_streaming_destination(
    database: &str,
    table: &str,
    columns: Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>,
    roles: Option<Vec<String>>,
    options: &StreamingDestinationTableArgs,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipeMutateDestination> {
    let table_definition = options
        .table_definition_file
        .as_deref()
        .map(read_destination_table_definition)
        .transpose()?
        .unwrap_or_else(managed_merge_tree_table_definition);

    Ok(clickhouse_cloud_api::models::ClickPipeMutateDestination {
        database: database.to_string(),
        table: Some(table.to_string()),
        columns: non_empty(columns),
        managed_table: Some(options.managed_table),
        roles,
        table_definition: Some(table_definition),
    })
}

impl StreamingDestinationArgs {
    /// Build this create's destination from columns the caller already parsed,
    /// so each handler keeps its own validation order.
    pub(super) fn build(
        &self,
        columns: Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>,
    ) -> CloudResult<clickhouse_cloud_api::models::ClickPipeMutateDestination> {
        build_streaming_destination(
            &self.database,
            &self.table,
            columns,
            build_destination_roles(&self.destination_roles.roles),
            &self.destination_table,
        )
    }
}

/// Build a managed-table destination with the default MergeTree engine.
pub(super) fn build_destination(
    database: &str,
    table: &str,
    columns: Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>,
    roles: Option<Vec<String>>,
) -> clickhouse_cloud_api::models::ClickPipeMutateDestination {
    // Database pipes (Postgres/MySQL/BigQuery) carry the destination table on
    // the per-mapping `targetTable` and reject any of {table, managedTable,
    // tableDefinition, columns} at the top level. Detect that case via empty
    // `table` and emit a destination with only `database` populated.
    if table.is_empty() {
        return clickhouse_cloud_api::models::ClickPipeMutateDestination {
            database: database.to_string(),
            // `roles` is not one of the four fields database pipes reject, so
            // it is wired here too.
            roles,
            columns: None,
            managed_table: None,
            table: None,
            table_definition: None,
        };
    }
    clickhouse_cloud_api::models::ClickPipeMutateDestination {
        database: database.to_string(),
        table: Some(table.to_string()),
        columns: non_empty(columns),
        managed_table: Some(true),
        roles,
        table_definition: Some(managed_merge_tree_table_definition()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::cloud::clickpipes::cli::{ClickPipeCommands, ClickPipeCreateCommands};
    use crate::cloud::clickpipes::test_support::{
        assert_names, clickpipe_parse_error, kafka_create_cli_args, object_storage_create_cli_args,
        parse_clickpipe, postgres_cli_args, pubsub_source_flags,
    };
    use clap::Parser;

    #[test]
    fn streaming_creates_require_columns_during_parsing() {
        let mut kafka = kafka_create_cli_args();
        let column = kafka
            .iter()
            .position(|arg| *arg == "--column")
            .expect("baseline --column");
        kafka.drain(column..=column + 1);
        let kinesis = vec![
            "create",
            "kinesis",
            "svc-1",
            "--name",
            "pipe-1",
            "--stream-name",
            "events",
            "--region",
            "eu-west-1",
            "--format",
            "JSONEachRow",
            "--database",
            "db",
            "--table",
            "events",
        ];
        let mut pubsub = vec![
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
        pubsub.extend(pubsub_source_flags("./sa-key.json"));

        for args in [object_storage_create_cli_args(), kafka, kinesis, pubsub] {
            let error = clickpipe_parse_error(&args);
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "{}",
                args[1]
            );
            assert_eq!(error.exit_code(), 2);
            assert!(
                error.to_string().contains("--column <NAME:TYPE>"),
                "{}",
                args[1]
            );
        }

        let mut args = object_storage_create_cli_args();
        args.extend([
            "--column",
            "_path:LowCardinality(String)",
            "--column",
            "payload:Nullable(Tuple(id UInt64, label String))",
        ]);
        let command = parse_clickpipe(&args);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::ObjectStorage(args),
        } = command
        else {
            panic!("expected object-storage create");
        };
        assert_eq!(
            args.destination.columns,
            [
                "_path:LowCardinality(String)",
                "payload:Nullable(Tuple(id UInt64, label String))"
            ]
        );
    }

    #[test]
    fn column_grammar_is_validated_on_every_streaming_create() {
        let mut pubsub = vec!["create", "pubsub", "svc-1", "--name", "pipe-1"];
        pubsub.extend(pubsub_source_flags("./sa-key.json"));
        pubsub.extend(["--database", "db", "--table", "events"]);
        for source_args in [
            vec![
                "create",
                "object-storage",
                "svc-1",
                "--name",
                "pipe-1",
                "--source-url",
                "https://bucket.example/events",
                "--format",
                "JSONEachRow",
                "--database",
                "db",
                "--table",
                "events",
            ],
            kafka_create_cli_args(),
            vec![
                "create",
                "kinesis",
                "svc-1",
                "--name",
                "pipe-1",
                "--stream-name",
                "events",
                "--region",
                "eu-west-1",
                "--format",
                "JSONEachRow",
                "--database",
                "db",
                "--table",
                "events",
            ],
            pubsub,
        ] {
            for (column, valid) in [
                ("bad_no_colon", false),
                ("id:Int64", true),
                ("metadata:Tuple(key String, value String)", true),
            ] {
                let mut args = vec!["clickhousectl", "cloud", "clickpipe"];
                args.extend(source_args.iter().copied());
                args.extend(["--column", column]);
                let parsed = Cli::try_parse_from(args);
                if valid {
                    assert!(parsed.is_ok(), "{}: {column}", source_args[1]);
                } else {
                    let error = parsed.err().expect("invalid column");
                    assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
                    assert_eq!(error.exit_code(), 2);
                }
            }
        }
    }

    #[test]
    fn parse_columns_valid() {
        let columns = vec!["id:Int64".to_string(), "name:String".to_string()];
        let parsed = parse_columns(&columns).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "id");
        assert_eq!(parsed[0].r#type, "Int64");
        assert_eq!(parsed[1].name, "name");
        assert_eq!(parsed[1].r#type, "String");
    }

    #[test]
    fn parse_columns_missing_colon_errors() {
        let columns = vec!["id_without_type".to_string()];
        let error = parse_columns(&columns).unwrap_err();
        assert_names(&error.message, &["id_without_type", "name:type"]);
    }

    #[test]
    fn build_streaming_destination_uses_compatible_defaults() {
        let destination = build_streaming_destination(
            "mydb",
            "events",
            vec![],
            None,
            &StreamingDestinationTableArgs::default(),
        )
        .unwrap();
        assert_eq!(destination.database, "mydb");
        assert_eq!(destination.table.as_deref(), Some("events"));
        assert_eq!(destination.managed_table, Some(true));
        // Default table engine is MergeTree, not something else.
        assert_eq!(
            destination
                .table_definition
                .as_ref()
                .expect("non-database pipe gets a tableDefinition")
                .engine
                .r#type,
            clickhouse_cloud_api::models::ClickPipeDestinationTableEngineType::MergeTree
        );
    }

    #[test]
    fn build_streaming_destination_reads_the_complete_typed_definition() {
        use clickhouse_cloud_api::models::ClickPipeDestinationTableEngineType;
        use std::io::Write as _;

        let mut config = tempfile::NamedTempFile::new().unwrap();
        write!(
            config,
            "{}",
            serde_json::json!({
                "engine": {
                    "columnIds": ["amount", "tax"],
                    "type": "SummingMergeTree",
                    "versionColumnId": null
                },
                "partitionBy": "toYYYYMM(created_at)",
                "primaryKey": "event_id",
                "sortingKey": ["event_id", "created_at"]
            })
        )
        .unwrap();
        let options = StreamingDestinationTableArgs {
            managed_table: false,
            table_definition_file: Some(config.path().to_string_lossy().into_owned()),
        };

        let destination =
            build_streaming_destination("analytics", "events", vec![], None, &options).unwrap();
        assert_eq!(destination.managed_table, Some(false));
        let definition = destination.table_definition.unwrap();
        assert_eq!(
            definition.engine.r#type,
            ClickPipeDestinationTableEngineType::SummingMergeTree
        );
        assert_eq!(definition.engine.column_ids, ["amount", "tax"]);
        assert_eq!(definition.engine.version_column_id, None);
        assert_eq!(definition.partition_by, "toYYYYMM(created_at)");
        assert_eq!(definition.primary_key, "event_id");
        assert_eq!(definition.sorting_key, ["event_id", "created_at"]);
        assert!(definition.ttl.is_empty());
        assert!(
            serde_json::to_value(&definition)
                .unwrap()
                .get("ttl")
                .is_none()
        );
    }

    #[test]
    fn destination_table_definition_preserves_explicit_ttl_and_rejects_invalid_types() {
        use std::io::Write as _;

        for ttl in [
            serde_json::json!("created_at + INTERVAL 30 DAY"),
            serde_json::Value::Null,
            serde_json::json!(false),
        ] {
            let mut config = tempfile::NamedTempFile::new().unwrap();
            let value = serde_json::json!({
                "engine": {"columnIds": [], "type": "MergeTree", "versionColumnId": null},
                "partitionBy": "tuple()", "primaryKey": "event_id", "sortingKey": ["event_id"],
                "ttl": ttl
            });
            write!(config, "{value}").unwrap();
            let result = read_destination_table_definition(config.path().to_str().unwrap());
            if let Some(expected) = ttl.as_str() {
                assert_eq!(result.unwrap().ttl, expected);
            } else {
                assert!(
                    result.is_err(),
                    "invalid TTL must not become an omitted field"
                );
            }
        }
    }

    #[test]
    fn destination_table_definition_accepts_every_engine_and_nullable_version() {
        use clickhouse_cloud_api::models::ClickPipeDestinationTableEngineType;
        use std::io::Write as _;

        for (wire_value, expected) in [
            ("MergeTree", ClickPipeDestinationTableEngineType::MergeTree),
            (
                "ReplacingMergeTree",
                ClickPipeDestinationTableEngineType::ReplacingMergeTree,
            ),
            (
                "SummingMergeTree",
                ClickPipeDestinationTableEngineType::SummingMergeTree,
            ),
            ("Null", ClickPipeDestinationTableEngineType::Null),
        ] {
            let mut config = tempfile::NamedTempFile::new().unwrap();
            write!(
                config,
                "{}",
                serde_json::json!({
                    "engine": {
                        "columnIds": [],
                        "type": wire_value,
                        "versionColumnId": null
                    },
                    "partitionBy": "tuple()",
                    "primaryKey": "event_id",
                    "sortingKey": ["event_id"]
                })
            )
            .unwrap();
            let definition =
                read_destination_table_definition(config.path().to_str().unwrap()).unwrap();
            assert_eq!(definition.engine.r#type, expected);
            assert_eq!(definition.engine.version_column_id, None);
        }
    }

    #[test]
    fn destination_table_definition_rejects_unknown_nested_fields_and_enums() {
        use std::io::Write as _;

        for (value, rejected_value) in [
            (
                serde_json::json!({
                    "engine": {
                        "columnIds": [],
                        "type": "MergeTree",
                        "versionColumnId": null
                    },
                    "partitionBy": "tuple()",
                    "primaryKey": "event_id",
                    "sortingKey": ["event_id"],
                    "sortingKeys": ["typo"]
                }),
                "sortingKeys",
            ),
            (
                serde_json::json!({
                "engine": {
                    "columnIds": [],
                    "type": "MergeTree",
                    "versionColumnId": null,
                    "versionColumn": "typo"
                },
                "partitionBy": "tuple()",
                "primaryKey": "event_id",
                "sortingKey": ["event_id"]
                }),
                "versionColumn",
            ),
            (
                serde_json::json!({
                "engine": {
                    "columnIds": [],
                    "type": "MergeTreeWithTypo",
                    "versionColumnId": null
                },
                "partitionBy": "tuple()",
                "primaryKey": "event_id",
                "sortingKey": ["event_id"]
                }),
                "MergeTreeWithTypo",
            ),
        ] {
            let mut config = tempfile::NamedTempFile::new().unwrap();
            write!(config, "{value}").unwrap();
            let error = read_destination_table_definition(config.path().to_str().unwrap())
                .expect_err("invalid definitions fail before an API request");
            assert!(error.message.contains(rejected_value), "{error:?}");
        }
    }

    #[test]
    fn destination_table_definition_does_not_default_missing_required_fields() {
        use std::io::Write as _;

        let mut config = tempfile::NamedTempFile::new().unwrap();
        write!(
            config,
            "{}",
            serde_json::json!({
                "engine": {
                    "type": "MergeTree",
                    "versionColumnId": null
                },
                "partitionBy": "tuple()",
                "sortingKey": ["event_id"]
            })
        )
        .unwrap();
        let error = read_destination_table_definition(config.path().to_str().unwrap())
            .expect_err("required fields are not fabricated");
        assert!(
            error.message.contains("columnIds") || error.message.contains("primaryKey"),
            "{error:?}"
        );
    }

    #[test]
    fn build_destination_omits_table_fields_for_database_pipes() {
        let destination = build_destination("default", "", vec![], None);
        assert_eq!(destination.database, "default");
        assert_eq!(destination.table, None);
        assert_eq!(destination.columns, None);
        assert_eq!(destination.managed_table, None);
        assert_eq!(destination.roles, None);
        assert_eq!(destination.table_definition, None);
    }

    // `--role` → `destination.roles` (issue #568). Omitted means the field is
    // absent from the body, so ClickPipes applies the default role on its own.

    #[test]
    fn build_destination_carries_roles_on_both_destination_shapes() {
        let roles = Some(vec!["reader".to_string(), "writer".to_string()]);
        let streaming = build_destination("mydb", "events", vec![], roles.clone());
        assert_eq!(streaming.roles, roles);
        // Database pipes send only `database`, but `roles` is not one of the
        // four fields they reject, so it must survive that branch too.
        let database_pipe = build_destination("default", "", vec![], roles.clone());
        assert_eq!(database_pipe.roles, roles);
        assert_eq!(database_pipe.table, None);
    }

    #[test]
    fn build_destination_roles_omits_field_when_no_flags_passed() {
        assert_eq!(build_destination_roles(&[]), None);
    }

    #[test]
    fn build_destination_roles_dedupes_preserving_declaration_order() {
        let roles = [
            "writer".to_string(),
            "reader".to_string(),
            "writer".to_string(),
        ];
        assert_eq!(
            build_destination_roles(&roles),
            Some(vec!["writer".to_string(), "reader".to_string()])
        );
    }

    #[test]
    fn parse_destination_role_name_trims_and_accepts_ordinary_names() {
        assert_eq!(
            parse_destination_role_name("  analytics_reader  ").unwrap(),
            "analytics_reader"
        );
    }

    #[test]
    fn parse_destination_role_name_rejects_blank_names() {
        for blank in ["", "   ", "\t"] {
            let error = parse_destination_role_name(blank)
                .expect_err("a blank role name must be rejected client-side");
            // Not the reserved-name rejection, which lists the reserved names.
            for name in RESERVED_DESTINATION_ROLES {
                assert!(!error.message.contains(name), "{error:?}");
            }
        }
    }

    #[test]
    fn parse_destination_role_name_rejects_api_reserved_names() {
        for reserved in [
            "clickpipes",
            "clickpipes_system",
            "ClickPipes",
            "  clickpipes_system  ",
        ] {
            let error = parse_destination_role_name(reserved)
                .expect_err("reserved role names must be rejected client-side");
            assert_names(&error.message, RESERVED_DESTINATION_ROLES);
        }
    }

    #[test]
    fn parses_repeatable_role_on_kafka_create() {
        let mut args = kafka_create_cli_args();
        args.extend(["--role", "reader", "--role", "writer"]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kafka(parsed),
        } = parse_clickpipe(&args)
        else {
            panic!("expected clickpipe create kafka");
        };
        assert_eq!(
            parsed.destination.destination_roles.roles,
            vec!["reader", "writer"]
        );

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kafka(parsed),
        } = parse_clickpipe(&kafka_create_cli_args())
        else {
            panic!("expected clickpipe create kafka");
        };
        assert!(parsed.destination.destination_roles.roles.is_empty());
    }

    #[test]
    fn streaming_destination_options_parse_on_every_streaming_create() {
        let mut pubsub = vec!["create", "pubsub", "svc-1", "--name", "pipe-1"];
        pubsub.extend(pubsub_source_flags("./sa-key.json"));
        pubsub.extend([
            "--database",
            "db",
            "--table",
            "events",
            "--column",
            "id:UInt64",
        ]);

        for mut args in [
            vec![
                "create",
                "object-storage",
                "--column",
                "id:Int64",
                "svc-1",
                "--name",
                "pipe-1",
                "--source-url",
                "https://bucket.example/events",
                "--format",
                "JSONEachRow",
                "--database",
                "db",
                "--table",
                "events",
            ],
            kafka_create_cli_args(),
            vec![
                "create",
                "kinesis",
                "svc-1",
                "--name",
                "pipe-1",
                "--stream-name",
                "events",
                "--region",
                "eu-west-1",
                "--format",
                "JSONEachRow",
                "--database",
                "db",
                "--table",
                "events",
                "--column",
                "id:UInt64",
            ],
            pubsub,
        ] {
            args.extend([
                "--managed-table",
                "false",
                "--table-definition-file",
                "definition.json",
            ]);
            let options = match parse_clickpipe(&args) {
                ClickPipeCommands::Create {
                    command: ClickPipeCreateCommands::ObjectStorage(args),
                } => args.destination.destination_table,
                ClickPipeCommands::Create {
                    command: ClickPipeCreateCommands::Kafka(args),
                } => args.destination.destination_table,
                ClickPipeCommands::Create {
                    command: ClickPipeCreateCommands::Kinesis(args),
                } => args.destination.destination_table,
                ClickPipeCommands::Create {
                    command: ClickPipeCreateCommands::PubSub(args),
                } => args.destination.destination_table,
                _ => panic!("expected streaming create"),
            };
            assert!(!options.managed_table);
            assert_eq!(
                options.table_definition_file.as_deref(),
                Some("definition.json")
            );
        }

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kafka(args),
        } = parse_clickpipe(&kafka_create_cli_args())
        else {
            panic!("expected Kafka create");
        };
        assert!(args.destination.destination_table.managed_table);
        assert_eq!(
            args.destination.destination_table.table_definition_file,
            None
        );
    }

    #[test]
    fn parses_repeatable_role_on_postgres_create() {
        let mut args = postgres_cli_args(Some("public.events:events"));
        args.extend(["--role", "reader", "--role", "writer"]);
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(parsed),
        } = parse_clickpipe(&args)
        else {
            panic!("expected clickpipe create postgres");
        };
        assert_eq!(parsed.destination_roles.roles, vec!["reader", "writer"]);

        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Postgres(parsed),
        } = parse_clickpipe(&postgres_cli_args(Some("public.events:events")))
        else {
            panic!("expected clickpipe create postgres");
        };
        assert!(parsed.destination_roles.roles.is_empty());
    }

    #[test]
    fn role_is_available_on_every_create_subcommand() {
        for (subcommand, extra) in [
            (
                "object-storage",
                vec![
                    "--column",
                    "id:Int64",
                    "--source-url",
                    "https://bucket.example/data",
                    "--format",
                    "JSONEachRow",
                    "--database",
                    "db",
                    "--table",
                    "events",
                ],
            ),
            (
                "kafka",
                vec![
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
                    "id:Int64",
                    "--iam-role",
                    "arn:role",
                ],
            ),
            (
                "kinesis",
                vec![
                    "--stream-name",
                    "stream",
                    "--region",
                    "us-east-1",
                    "--format",
                    "JSONEachRow",
                    "--database",
                    "db",
                    "--table",
                    "events",
                    "--column",
                    "id:Int64",
                ],
            ),
            (
                "postgres",
                vec![
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
                ],
            ),
            (
                "mysql",
                vec![
                    "--host",
                    "mysql.example",
                    "--username",
                    "user",
                    "--password",
                    "password",
                    "--table-mapping",
                    "mydb.events:events",
                ],
            ),
            (
                "mongodb",
                vec![
                    "--uri",
                    "mongodb://mongo.example:27017",
                    "--username",
                    "user",
                    "--password",
                    "password",
                    "--table-mapping",
                    "mydb.events:events",
                ],
            ),
            (
                "bigquery",
                vec![
                    "--service-account-file",
                    "./sa-key.json",
                    "--staging-path",
                    "gs://bucket/staging",
                    "--table-mapping",
                    "dataset.events:events",
                ],
            ),
            ("pubsub", {
                let mut flags = pubsub_source_flags("./sa-key.json");
                flags.extend([
                    "--database",
                    "db",
                    "--table",
                    "events",
                    "--column",
                    "id:Int64",
                ]);
                flags
            }),
        ] {
            let mut args = vec!["create", subcommand, "svc-1", "--name", "pipe-1"];
            args.extend(extra);
            args.extend(["--role", "reader"]);
            parse_clickpipe(&args);

            let mut reserved = args.clone();
            reserved.pop();
            reserved.push("clickpipes");
            let error = clickpipe_parse_error(&reserved);
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            assert_eq!(error.exit_code(), 2);
            assert_names(&error.to_string(), RESERVED_DESTINATION_ROLES);
        }
    }
}
