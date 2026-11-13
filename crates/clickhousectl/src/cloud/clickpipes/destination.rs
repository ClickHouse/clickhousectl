use super::cli::{StreamingDestinationArgs, StreamingDestinationTableArgs};
use super::non_empty;
use crate::cloud::client::{CloudError, Result as CloudResult};
use crate::cloud::config::{
    config_source_label, deserialize_strict_config, read_flag_config_value, read_flag_text,
};

/// Parse `name:type` column specifications into library destination columns.
/// A name given twice is a usage error.
pub(super) fn parse_columns(
    columns: &[String],
) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>> {
    let mut parsed: Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn> =
        Vec::with_capacity(columns.len());
    for column in columns {
        let column = parse_column(column)?;
        if parsed.iter().any(|existing| existing.name == column.name) {
            return Err(CloudError::usage(format!(
                "--column {} is given more than once",
                column.name
            )));
        }
        parsed.push(column);
    }
    Ok(parsed)
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

/// Convert a `clickpipe schema-discover --json` response into destination
/// columns, in discovery order. Source fields and default-enabled metadata
/// (`isVirtual`) are kept; metadata the API marks `isVirtualDisabledDefault`
/// is left out, as the console does. `type` is used as given and `optional`
/// is ignored.
pub(super) fn columns_from_discovery(
    response: clickhouse_cloud_api::models::ClickPipeSchemaDiscoveryResponse,
) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>> {
    let mut columns: Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn> = Vec::new();
    for (index, field) in response.fields.unwrap_or_default().into_iter().enumerate() {
        if field.is_virtual == Some(true) && field.is_virtual_disabled_default == Some(true) {
            continue;
        }
        let name = field.name.filter(|name| !name.is_empty()).ok_or_else(|| {
            CloudError::usage(format!("--columns-from: fields[{index}] has no name"))
        })?;
        let column_type = field
            .r#type
            .filter(|column_type| !column_type.is_empty())
            .ok_or_else(|| {
                CloudError::usage(format!("--columns-from: field `{name}` has no type"))
            })?;
        if columns.iter().any(|existing| existing.name == name) {
            return Err(CloudError::usage(format!(
                "--columns-from: field `{name}` is listed more than once"
            )));
        }
        columns.push(clickhouse_cloud_api::models::ClickPipeDestinationColumn {
            name,
            r#type: column_type,
        });
    }
    if columns.is_empty() {
        return Err(CloudError::usage(
            "--columns-from has no fields to use as destination columns",
        ));
    }
    Ok(columns)
}

/// Read `--columns-from` from a file or stdin and convert it with
/// [`columns_from_discovery`]. An unreadable file is an I/O failure; empty or
/// malformed input is a usage error.
fn read_columns_from(
    source: &str,
) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>> {
    let contents = read_flag_text("--columns-from", std::path::Path::new(source))?;
    let label = config_source_label(source);
    if contents.trim().is_empty() {
        return Err(CloudError::usage(format!(
            "--columns-from {label} is empty"
        )));
    }
    let response = serde_json::from_str(&contents).map_err(|error| {
        CloudError::usage(format!(
            "--columns-from {label} is not `clickpipe schema-discover --json` output: {error}"
        ))
    })?;
    columns_from_discovery(response)
}

/// Refuse more than one flag reading stdin (`-`) in one invocation: the first
/// reader would consume the stream and leave the others empty.
fn refuse_more_than_one_stdin_reader(readers: &[(&str, Option<&str>)]) -> CloudResult<()> {
    let flags: Vec<&str> = readers
        .iter()
        .filter(|(_, value)| *value == Some("-"))
        .map(|(flag, _)| *flag)
        .collect();
    if flags.len() > 1 {
        return Err(CloudError::usage(format!(
            "{} all read stdin; give `-` to only one of them",
            flags.join(", ")
        )));
    }
    Ok(())
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
    /// The destination columns: those read from `--columns-from` first, then
    /// each `--column`. A name given by both is a usage error. Columns already
    /// resolved by [`Self::resolve_columns_from_file`] are reused.
    pub(super) fn parsed_columns(
        &self,
    ) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>> {
        if let Some(columns) = &self.resolved_columns {
            return Ok(columns.clone());
        }
        self.read_columns()
    }

    fn read_columns(
        &self,
    ) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>> {
        let explicit = parse_columns(&self.columns)?;
        let Some(source) = self.columns_from.as_deref() else {
            return Ok(explicit);
        };
        let mut columns = read_columns_from(source)?;
        if let Some(duplicate) = explicit
            .iter()
            .find(|column| columns.iter().any(|imported| imported.name == column.name))
        {
            return Err(CloudError::usage(format!(
                "--column {} is also in --columns-from {}",
                duplicate.name,
                config_source_label(source)
            )));
        }
        columns.extend(explicit);
        Ok(columns)
    }

    /// Validate the explicit `--column` values without any I/O.
    pub(super) fn validate_explicit_columns(&self) -> CloudResult<()> {
        parse_columns(&self.columns).map(|_| ())
    }

    /// Read and validate a `--columns-from` file before credentials are
    /// resolved, keeping the result for [`Self::parsed_columns`]. Stdin (`-`)
    /// is left for the handler, so it is read once and never blocks a run
    /// that is about to fail on credentials.
    pub(super) fn resolve_columns_from_file(&mut self) -> CloudResult<()> {
        if matches!(self.columns_from.as_deref(), Some(source) if source != "-") {
            self.resolved_columns = Some(self.read_columns()?);
        }
        Ok(())
    }

    /// Refuse an invocation where this destination's file flags and the
    /// source's `source_readers` name stdin more than once.
    pub(super) fn refuse_shared_stdin(
        &self,
        source_readers: &[(&'static str, Option<&str>)],
    ) -> CloudResult<()> {
        let mut readers = vec![
            ("--columns-from", self.columns_from.as_deref()),
            (
                "--table-definition-file",
                self.destination_table.table_definition_file.as_deref(),
            ),
        ];
        readers.extend_from_slice(source_readers);
        refuse_more_than_one_stdin_reader(&readers)
    }

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
        assert_names, clickpipe_parse_error, clickpipe_validation_message, kafka_create_cli_args,
        object_storage_create_cli_args, parse_clickpipe, postgres_cli_args, pubsub_source_flags,
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
            assert_names(&error.to_string(), &["--column", "--columns-from"]);

            // Either flag alone satisfies the requirement.
            let mut with_file = args.clone();
            with_file.extend(["--columns-from", "discovery.json"]);
            let ClickPipeCommands::Create { command } = parse_clickpipe(&with_file) else {
                panic!("expected create");
            };
            let destination = match &command {
                ClickPipeCreateCommands::ObjectStorage(args) => &args.destination,
                ClickPipeCreateCommands::Kafka(args) => &args.destination,
                ClickPipeCreateCommands::Kinesis(args) => &args.destination,
                ClickPipeCreateCommands::PubSub(args) => &args.destination,
                _ => panic!("expected a streaming create"),
            };
            assert_eq!(destination.columns_from.as_deref(), Some("discovery.json"));
            assert!(destination.columns.is_empty());
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
    fn parse_columns_rejects_a_repeated_name() {
        let columns = vec!["id:UInt64".to_string(), "id:String".to_string()];
        let error = parse_columns(&columns).unwrap_err();
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Usage);
        assert_names(&error.message, &["--column", "id"]);
    }

    /// Parse `json` the way `--columns-from` does: as the response type that
    /// `clickpipe schema-discover --json` prints.
    fn discovery(
        json: serde_json::Value,
    ) -> clickhouse_cloud_api::models::ClickPipeSchemaDiscoveryResponse {
        serde_json::from_value(json).expect("schema-discover output")
    }

    fn name_type_pairs(
        columns: &[clickhouse_cloud_api::models::ClickPipeDestinationColumn],
    ) -> Vec<(&str, &str)> {
        columns
            .iter()
            .map(|column| (column.name.as_str(), column.r#type.as_str()))
            .collect()
    }

    #[test]
    fn columns_from_discovery_keeps_source_and_default_enabled_metadata_fields() {
        let columns = columns_from_discovery(discovery(serde_json::json!({
            "fields": [
                {"name": "id", "type": "Int64", "optional": false,
                 "isVirtual": false, "isVirtualDisabledDefault": false},
                {"name": "note", "type": "Nullable(String)", "optional": true},
                {"name": "_topic", "type": "LowCardinality(String)", "optional": false,
                 "isVirtual": true, "isVirtualDisabledDefault": false},
                {"name": "_raw_message", "type": "String", "optional": false,
                 "isVirtual": true, "isVirtualDisabledDefault": true},
                {"name": "_key", "type": "String", "isVirtual": true},
            ],
            "meta": {"format": "JSONEachRow"}
        })))
        .unwrap();
        assert_eq!(
            name_type_pairs(&columns),
            [
                ("id", "Int64"),
                ("note", "Nullable(String)"),
                ("_topic", "LowCardinality(String)"),
                ("_key", "String"),
            ]
        );
    }

    #[test]
    fn columns_from_discovery_rejects_unusable_fields_and_empty_results() {
        for json in [
            serde_json::json!({}),
            serde_json::json!({"fields": []}),
            serde_json::json!({"fields": [{"name": "_raw_message", "type": "String",
                "isVirtual": true, "isVirtualDisabledDefault": true}]}),
            serde_json::json!({"fields": [{"type": "String"}]}),
            serde_json::json!({"fields": [{"name": "", "type": "String"}]}),
            serde_json::json!({"fields": [{"name": "id"}]}),
            serde_json::json!({"fields": [{"name": "id", "type": "Int64"},
                                          {"name": "id", "type": "String"}]}),
        ] {
            let error = columns_from_discovery(discovery(json.clone())).unwrap_err();
            assert_eq!(
                error.kind,
                crate::cloud::client::CloudErrorKind::Usage,
                "{json}"
            );
            assert_names(&error.message, &["--columns-from"]);
        }
    }

    #[test]
    fn columns_from_file_reports_empty_and_malformed_input_as_usage_errors() {
        use std::io::Write as _;

        for contents in ["", "  \n", "not json", "[1, 2]", r#"{"fields": "id"}"#] {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            file.write_all(contents.as_bytes()).unwrap();
            let path = file.path().to_str().unwrap();
            let error = read_columns_from(path).unwrap_err();
            assert_eq!(
                error.kind,
                crate::cloud::client::CloudErrorKind::Usage,
                "{contents:?}"
            );
            assert_names(&error.message, &["--columns-from", path]);
        }

        let missing = tempfile::tempdir().unwrap().path().join("missing.json");
        let error = read_columns_from(missing.to_str().unwrap()).unwrap_err();
        assert_ne!(error.kind, crate::cloud::client::CloudErrorKind::Usage);
        assert_names(&error.message, &["--columns-from"]);
    }

    #[test]
    fn parsed_columns_merges_imported_and_explicit_columns_and_rejects_duplicates() {
        use std::io::Write as _;

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(
            br#"{"fields": [{"name": "id", "type": "Int64", "optional": false},
                            {"name": "_path", "type": "LowCardinality(String)",
                             "isVirtual": true, "isVirtualDisabledDefault": false}]}"#,
        )
        .unwrap();
        let path = file.path().to_str().unwrap().to_string();
        let destination = |columns: &[&str]| StreamingDestinationArgs {
            database: "db".into(),
            table: "events".into(),
            columns: columns.iter().map(|column| column.to_string()).collect(),
            columns_from: Some(path.clone()),
            destination_table: StreamingDestinationTableArgs::default(),
            destination_roles: Default::default(),
            resolved_columns: None,
        };

        let columns = destination(&["ingested_at:DateTime"])
            .parsed_columns()
            .unwrap();
        assert_eq!(
            name_type_pairs(&columns),
            [
                ("id", "Int64"),
                ("_path", "LowCardinality(String)"),
                ("ingested_at", "DateTime"),
            ]
        );

        let error = destination(&["id:UInt64"]).parsed_columns().unwrap_err();
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Usage);
        assert_names(&error.message, &["--column", "id", "--columns-from", &path]);
    }

    fn destination_reading(columns_from: &str) -> StreamingDestinationArgs {
        StreamingDestinationArgs {
            database: "db".into(),
            table: "events".into(),
            columns: vec!["ingested_at:DateTime".into()],
            columns_from: Some(columns_from.into()),
            destination_table: StreamingDestinationTableArgs::default(),
            destination_roles: Default::default(),
            resolved_columns: None,
        }
    }

    #[test]
    fn resolve_columns_from_file_reads_the_file_once_before_credentials() {
        use std::io::Write as _;

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(br#"{"fields": [{"name": "id", "type": "Int64"}]}"#)
            .unwrap();
        let mut destination = destination_reading(file.path().to_str().unwrap());
        destination.resolve_columns_from_file().unwrap();
        // The handler reuses the resolved columns rather than reading again.
        drop(file);
        assert_eq!(
            name_type_pairs(&destination.parsed_columns().unwrap()),
            [("id", "Int64"), ("ingested_at", "DateTime")]
        );
    }

    #[test]
    fn resolve_columns_from_file_keeps_usage_and_io_failures_apart() {
        use crate::cloud::client::CloudErrorKind;
        use std::io::Write as _;

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"not json").unwrap();
        let path = file.path().to_str().unwrap();
        let error = destination_reading(path)
            .resolve_columns_from_file()
            .unwrap_err();
        assert_eq!(error.kind, CloudErrorKind::Usage);
        assert_names(&error.message, &["--columns-from", path]);

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(br#"{"fields": [{"name": "ingested_at", "type": "DateTime"}]}"#)
            .unwrap();
        let error = destination_reading(file.path().to_str().unwrap())
            .resolve_columns_from_file()
            .unwrap_err();
        assert_eq!(error.kind, CloudErrorKind::Usage);
        assert_names(
            &error.message,
            &["--column", "ingested_at", "--columns-from"],
        );

        let missing = tempfile::tempdir().unwrap().path().join("missing.json");
        let error = destination_reading(missing.to_str().unwrap())
            .resolve_columns_from_file()
            .unwrap_err();
        assert_eq!(error.kind, CloudErrorKind::Generic);
        assert_eq!(
            error.failure.map(|failure| failure.kind),
            Some(crate::failure::FailureKind::Io)
        );
    }

    #[test]
    fn resolve_columns_from_file_leaves_stdin_for_the_handler() {
        let mut destination = destination_reading("-");
        destination.resolve_columns_from_file().unwrap();
        assert!(destination.resolved_columns.is_none());

        let mut destination = destination_reading("-");
        destination.columns_from = None;
        destination.resolve_columns_from_file().unwrap();
        assert!(destination.resolved_columns.is_none());
    }

    #[test]
    fn streaming_creates_reject_a_repeated_column_before_any_io() {
        let mut kinesis = vec![
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
        kinesis.extend(["--column", "id:UInt64"]);
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
            "--column",
            "id:UInt64",
        ];
        pubsub.extend(pubsub_source_flags("./sa-key.json"));
        let mut object_storage = object_storage_create_cli_args();
        object_storage.extend(["--column", "id:UInt64"]);

        for args in [object_storage, kafka_create_cli_args(), kinesis, pubsub] {
            for columns_from in [None, Some("-"), Some("/missing/discovery.json")] {
                let mut args = args.clone();
                args.extend(["--column", "id:String"]);
                if let Some(columns_from) = columns_from {
                    args.extend(["--columns-from", columns_from]);
                }
                let message = clickpipe_validation_message(&parse_clickpipe(&args))
                    .unwrap_or_else(|| panic!("expected a duplicate refusal for {args:?}"));
                assert_names(&message, &["--column", "id"]);
            }
        }
    }

    #[test]
    fn every_streaming_create_reads_columns_from_files_before_credentials() {
        use std::io::Write as _;

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"not json").unwrap();
        let path = file.path().to_str().unwrap();
        let mut kinesis = vec![
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
        let mut object_storage = object_storage_create_cli_args();
        let mut kafka = kafka_create_cli_args();
        for args in [&mut object_storage, &mut kafka, &mut kinesis, &mut pubsub] {
            args.extend(["--columns-from", path]);
            let mut command = parse_clickpipe(args);
            let error = command.prepare_create_before_credentials().unwrap_err();
            assert_eq!(
                error.kind,
                crate::cloud::client::CloudErrorKind::Usage,
                "{}",
                args[1]
            );
            assert_names(&error.message, &["--columns-from", path]);
        }

        // Other creates have no destination columns to read.
        let mut postgres = parse_clickpipe(&postgres_cli_args(Some("public.events:events")));
        postgres.prepare_create_before_credentials().unwrap();
    }

    #[test]
    fn streaming_creates_refuse_two_stdin_readers_before_any_io() {
        let mut kinesis = vec![
            "create",
            "kinesis",
            "svc-1",
            "--name",
            "pipe-1",
            "--stream-name",
            "events",
            "--region",
            "eu-west-1",
            "--database",
            "db",
            "--table",
            "events",
        ];
        let mut object_storage = object_storage_create_cli_args();
        object_storage.extend(["--storage-type", "gcs", "--service-account-file", "-"]);
        let mut kafka: Vec<&str> = kafka_create_cli_args()
            .into_iter()
            .map(|arg| {
                if arg == "JSONEachRow" {
                    "Protobuf"
                } else {
                    arg
                }
            })
            .collect();
        kafka.extend(["--protobuf-schema-file", "-"]);
        let mut kinesis_protobuf = kinesis.clone();
        kinesis_protobuf.extend(["--format", "Protobuf", "--protobuf-schema-file", "-"]);
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
        pubsub.extend(pubsub_source_flags("-"));

        // Each source's own stdin flag collides with --columns-from -.
        for (mut args, source_flag) in [
            (object_storage, "--service-account-file"),
            (kafka, "--protobuf-schema-file"),
            (kinesis_protobuf, "--protobuf-schema-file"),
            (pubsub, "--service-account-file"),
        ] {
            args.extend(["--columns-from", "-"]);
            let message = clickpipe_validation_message(&parse_clickpipe(&args))
                .unwrap_or_else(|| panic!("expected a stdin refusal for {}", args[1]));
            assert_names(&message, &["--columns-from", source_flag]);
        }

        // So does the destination's own --table-definition-file -; one
        // reader alone is accepted.
        kinesis.extend(["--format", "JSONEachRow", "--columns-from", "-"]);
        assert_eq!(
            clickpipe_validation_message(&parse_clickpipe(&kinesis)),
            None
        );
        kinesis.extend(["--table-definition-file", "-"]);
        let message = clickpipe_validation_message(&parse_clickpipe(&kinesis))
            .expect("expected a stdin refusal");
        assert_names(&message, &["--columns-from", "--table-definition-file"]);
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
