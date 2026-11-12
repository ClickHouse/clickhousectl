use crate::cloud::client::{CloudError, Result as CloudResult};
use crate::cloud::shared::parse_serde_enum;
use clickhouse_cloud_api::models::{
    ClickPipeBigQueryPipeTableMappingTableengine, ClickPipeMongoDBPipeTableMappingTableengine,
    ClickPipeMySQLPipeTableMappingTableengine, ClickPipePostgresPipeTableMapping,
    ClickPipePostgresPipeTableMappingTableengine,
};

pub(super) fn parse_postgres_table_mapping_parts(
    mapping: &str,
) -> CloudResult<(String, String, String)> {
    let (source, target) = mapping.split_once(':').ok_or_else(|| {
        CloudError::usage(format!(
            "invalid table mapping '{}': expected schema.table:target_table",
            mapping
        ))
    })?;
    let (schema, table) = source.split_once('.').ok_or_else(|| {
        CloudError::usage(format!(
            "invalid table mapping '{}': expected schema.table:target_table",
            mapping
        ))
    })?;
    if schema.trim().is_empty() {
        return Err(CloudError::usage(format!(
            "invalid table mapping '{}': source schema must not be empty",
            mapping
        )));
    }
    if table.trim().is_empty() {
        return Err(CloudError::usage(format!(
            "invalid table mapping '{}': source table must not be empty",
            mapping
        )));
    }
    if target.trim().is_empty() {
        return Err(CloudError::usage(format!(
            "invalid table mapping '{}': target table must not be empty",
            mapping
        )));
    }

    Ok((
        schema.trim().to_string(),
        table.trim().to_string(),
        target.trim().to_string(),
    ))
}

pub(super) fn parse_postgres_table_mapping(mapping: &str) -> Result<String, String> {
    parse_postgres_table_mapping_parts(mapping)
        .map(|_| mapping.to_string())
        .map_err(|error| error.message)
}

pub(super) fn parse_mongodb_table_mapping_parts(
    mapping: &str,
) -> CloudResult<(String, String, String)> {
    let (source, target) = mapping.split_once(':').ok_or_else(|| {
        CloudError::usage(format!(
            "invalid table mapping '{}': expected database.collection:target_table",
            mapping
        ))
    })?;
    let (database, collection) = source.split_once('.').ok_or_else(|| {
        CloudError::usage(format!(
            "invalid table mapping '{}': expected database.collection:target_table",
            mapping
        ))
    })?;
    if database.trim().is_empty() {
        return Err(CloudError::usage(format!(
            "invalid table mapping '{}': source database must not be empty",
            mapping
        )));
    }
    if collection.trim().is_empty() {
        return Err(CloudError::usage(format!(
            "invalid table mapping '{}': source collection must not be empty",
            mapping
        )));
    }
    if target.trim().is_empty() {
        return Err(CloudError::usage(format!(
            "invalid table mapping '{}': target table must not be empty",
            mapping
        )));
    }

    Ok((
        database.trim().to_string(),
        collection.trim().to_string(),
        target.trim().to_string(),
    ))
}

pub(super) fn parse_mongodb_table_mapping(mapping: &str) -> Result<String, String> {
    parse_mongodb_table_mapping_parts(mapping)
        .map(|_| mapping.to_string())
        .map_err(|error| error.message)
}

fn parse_strict_table_mapping_json<T>(position: usize, raw: &str, fields: &[&str]) -> CloudResult<T>
where
    T: serde::de::DeserializeOwned,
{
    let flag = format!("--table-mapping-json #{}", position + 1);
    let invalid = |detail: String| CloudError::usage(format!("{flag}: {detail}"));
    let field_list = fields.join(", ");
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| invalid(format!("invalid JSON: {error}")))?;
    let object = value.as_object().ok_or_else(|| {
        invalid(format!(
            "expected a JSON object with the fields {field_list}"
        ))
    })?;
    let unknown: Vec<&str> = object
        .keys()
        .map(String::as_str)
        .filter(|key| !fields.contains(key))
        .collect();
    if !unknown.is_empty() {
        return Err(invalid(format!(
            "unknown field{} {}; valid fields are {field_list}",
            if unknown.len() == 1 { "" } else { "s" },
            unknown.join(", "),
        )));
    }

    crate::cloud::config::deserialize_strict_config(value, &flag).map_err(|error| {
        let prefix = format!("invalid request body in config {flag}: ");
        invalid(
            error
                .message
                .strip_prefix(&prefix)
                .unwrap_or(&error.message)
                .to_string(),
        )
    })
}

fn required_table_mapping_name(flag: &str, field: &str, value: String) -> CloudResult<String> {
    if value.trim().is_empty() {
        return Err(CloudError::usage(format!(
            "{flag}: {field} is required and must not be empty"
        )));
    }
    Ok(value.trim().to_string())
}

fn validate_table_mapping_entries(
    flag: &str,
    field: &str,
    values: Option<Vec<String>>,
    unique: bool,
) -> CloudResult<Option<Vec<String>>> {
    values
        .map(|values| {
            let mut seen = std::collections::HashSet::new();
            values
                .into_iter()
                .map(|entry| {
                    let entry = entry.trim().to_string();
                    if entry.is_empty() {
                        return Err(CloudError::usage(format!(
                            "{flag}: {field} must not contain an empty entry"
                        )));
                    }
                    if unique && !seen.insert(entry.clone()) {
                        return Err(CloudError::usage(format!(
                            "{flag}: {field} must not contain duplicate entry '{entry}'"
                        )));
                    }
                    Ok(entry)
                })
                .collect()
        })
        .transpose()
}

fn resolve_custom_sorting_key(
    flag: &str,
    sorting_keys: &Option<Vec<String>>,
    use_custom_sorting_key: Option<bool>,
) -> CloudResult<Option<bool>> {
    let has_sorting_keys = sorting_keys
        .as_ref()
        .is_some_and(|sorting_keys| !sorting_keys.is_empty());
    match use_custom_sorting_key {
        Some(true) if !has_sorting_keys => Err(CloudError::usage(format!(
            "{flag}: useCustomSortingKey is true but sortingKeys is empty; list the destination \
             ORDER BY columns in sortingKeys"
        ))),
        Some(false) if has_sorting_keys => Err(CloudError::usage(format!(
            "{flag}: sortingKeys is set but useCustomSortingKey is false, which would ignore the \
             keys; omit useCustomSortingKey or set it to true"
        ))),
        None if has_sorting_keys => Ok(Some(true)),
        explicit => Ok(explicit),
    }
}

fn validate_table_mapping_engine<T>(
    flag: &str,
    table_engine: &Option<T>,
    values: &[&str],
) -> CloudResult<()>
where
    T: std::fmt::Display,
{
    let Some(table_engine) = table_engine else {
        return Ok(());
    };
    let table_engine = table_engine.to_string();
    if values.contains(&table_engine.as_str()) {
        Ok(())
    } else {
        Err(CloudError::usage(format!(
            "{flag}: invalid tableEngine: unknown value '{table_engine}', expected one of: {}",
            values.join(", ")
        )))
    }
}

const MYSQL_TABLE_MAPPING_JSON_FIELDS: &[&str] = &[
    "sourceSchemaName",
    "sourceTable",
    "targetTable",
    "excludedColumns",
    "sortingKeys",
    "useCustomSortingKey",
    "partitionKey",
    "partitionByExpr",
    "tableEngine",
];

pub(super) fn parse_mysql_table_mapping_json(
    position: usize,
    raw: &str,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipeMySQLPipeTableMapping> {
    use clickhouse_cloud_api::models::ClickPipeMySQLPipeTableMapping;

    let flag = format!("--table-mapping-json #{}", position + 1);
    let mut mapping: ClickPipeMySQLPipeTableMapping =
        parse_strict_table_mapping_json(position, raw, MYSQL_TABLE_MAPPING_JSON_FIELDS)?;
    mapping.source_schema_name =
        required_table_mapping_name(&flag, "sourceSchemaName", mapping.source_schema_name)?;
    mapping.source_table = required_table_mapping_name(&flag, "sourceTable", mapping.source_table)?;
    mapping.target_table = required_table_mapping_name(&flag, "targetTable", mapping.target_table)?;
    mapping.excluded_columns =
        validate_table_mapping_entries(&flag, "excludedColumns", mapping.excluded_columns, true)?;
    mapping.sorting_keys =
        validate_table_mapping_entries(&flag, "sortingKeys", mapping.sorting_keys, true)?;
    mapping.use_custom_sorting_key =
        resolve_custom_sorting_key(&flag, &mapping.sorting_keys, mapping.use_custom_sorting_key)?;
    validate_table_mapping_engine(
        &flag,
        &mapping.table_engine,
        ClickPipeMySQLPipeTableMappingTableengine::VALUES,
    )?;
    Ok(mapping)
}

const MONGODB_TABLE_MAPPING_JSON_FIELDS: &[&str] = &[
    "sourceDatabaseName",
    "sourceCollection",
    "targetTable",
    "tableEngine",
    "structuredIngestion",
    "mongoConfig",
];

pub(super) fn parse_mongodb_table_mapping_json(
    position: usize,
    raw: &str,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipeMongoDBPipeTableMapping> {
    use clickhouse_cloud_api::models::ClickPipeMongoDBPipeTableMapping;

    let flag = format!("--table-mapping-json #{}", position + 1);
    let mut mapping: ClickPipeMongoDBPipeTableMapping =
        parse_strict_table_mapping_json(position, raw, MONGODB_TABLE_MAPPING_JSON_FIELDS)?;
    mapping.source_database_name =
        required_table_mapping_name(&flag, "sourceDatabaseName", mapping.source_database_name)?;
    mapping.source_collection =
        required_table_mapping_name(&flag, "sourceCollection", mapping.source_collection)?;
    mapping.target_table = required_table_mapping_name(&flag, "targetTable", mapping.target_table)?;
    validate_table_mapping_engine(
        &flag,
        &mapping.table_engine,
        ClickPipeMongoDBPipeTableMappingTableengine::VALUES,
    )?;
    mapping.structured_ingestion = mapping
        .structured_ingestion
        .map(|ingestion| validate_mongodb_structured_ingestion(&flag, ingestion))
        .transpose()?;
    Ok(mapping)
}

/// Trims the names in `structuredIngestion` and rejects empty ones; the
/// spec requires `excludedColumns` and `sortingKeys` entries to be unique.
fn validate_mongodb_structured_ingestion(
    flag: &str,
    mut ingestion: clickhouse_cloud_api::models::ClickPipeMongoDBStructuredIngestion,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipeMongoDBStructuredIngestion> {
    if let Some(columns) = ingestion.columns.as_mut() {
        for column in columns.iter_mut() {
            column.name = required_table_mapping_name(
                flag,
                "structuredIngestion.columns[].name",
                std::mem::take(&mut column.name),
            )?;
            column.r#type = required_table_mapping_name(
                flag,
                "structuredIngestion.columns[].type",
                std::mem::take(&mut column.r#type),
            )?;
        }
    }
    ingestion.excluded_columns = validate_table_mapping_entries(
        flag,
        "structuredIngestion.excludedColumns",
        ingestion.excluded_columns,
        true,
    )?;
    ingestion.sorting_keys = validate_table_mapping_entries(
        flag,
        "structuredIngestion.sortingKeys",
        ingestion.sorting_keys,
        true,
    )?;
    if let Some(expr) = &ingestion.partition_by_expr {
        ingestion.partition_by_expr = Some(required_table_mapping_name(
            flag,
            "structuredIngestion.partitionByExpr",
            expr.clone(),
        )?);
    }
    Ok(ingestion)
}

const BIGQUERY_TABLE_MAPPING_JSON_FIELDS: &[&str] = &[
    "sourceDatasetName",
    "sourceTable",
    "targetTable",
    "excludedColumns",
    "sortingKeys",
    "useCustomSortingKey",
    "tableEngine",
    "queryCdcWatermarkColumn",
    "eventsFunction",
];

pub(super) fn parse_bigquery_table_mapping_json(
    position: usize,
    raw: &str,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipeBigQueryPipeTableMapping> {
    use clickhouse_cloud_api::models::ClickPipeBigQueryPipeTableMapping;

    let flag = format!("--table-mapping-json #{}", position + 1);
    let mut mapping: ClickPipeBigQueryPipeTableMapping =
        parse_strict_table_mapping_json(position, raw, BIGQUERY_TABLE_MAPPING_JSON_FIELDS)?;
    mapping.source_dataset_name =
        required_table_mapping_name(&flag, "sourceDatasetName", mapping.source_dataset_name)?;
    mapping.source_table = required_table_mapping_name(&flag, "sourceTable", mapping.source_table)?;
    mapping.target_table = required_table_mapping_name(&flag, "targetTable", mapping.target_table)?;
    mapping.excluded_columns =
        validate_table_mapping_entries(&flag, "excludedColumns", mapping.excluded_columns, false)?;
    mapping.sorting_keys =
        validate_table_mapping_entries(&flag, "sortingKeys", mapping.sorting_keys, false)?;
    mapping.use_custom_sorting_key =
        resolve_custom_sorting_key(&flag, &mapping.sorting_keys, mapping.use_custom_sorting_key)?;
    validate_table_mapping_engine(
        &flag,
        &mapping.table_engine,
        ClickPipeBigQueryPipeTableMappingTableengine::VALUES,
    )?;
    if let Some(column) = mapping.query_cdc_watermark_column {
        mapping.query_cdc_watermark_column = Some(required_table_mapping_name(
            &flag,
            "queryCdcWatermarkColumn",
            column,
        )?);
    }
    if let Some(function) = mapping.events_function {
        let function = required_table_mapping_name(&flag, "eventsFunction", function)?;
        if !["appends", "changes"].contains(&function.as_str()) {
            return Err(CloudError::usage(format!(
                "{flag}: eventsFunction must be appends or changes"
            )));
        }
        mapping.events_function = Some(function);
    }
    Ok(mapping)
}

/// Wire field names `ClickPipePostgresPipeTableMapping` accepts, in the order
/// the help text and the error messages list them.
const POSTGRES_TABLE_MAPPING_JSON_FIELDS: &[&str] = &[
    "sourceSchemaName",
    "sourceTable",
    "targetTable",
    "excludedColumns",
    "sortingKeys",
    "useCustomSortingKey",
    "partitionByExpr",
    "partitionKey",
    "tableEngine",
];

/// One `--table-mapping-json` value, before validation.
///
/// The library's `ClickPipePostgresPipeTableMapping` is a request type and so
/// strict: every field is required, which a hand-written mapping object is not
/// expected to spell out. This mirror is all-`Option` so absence is
/// representable, and each field is then resolved explicitly. It carries no
/// `deny_unknown_fields`: unknown fields are reported against
/// `POSTGRES_TABLE_MAPPING_JSON_FIELDS` instead, so the diagnostic names the
/// wire fields rather than serde's view of this struct.
#[derive(serde::Deserialize)]
struct PostgresTableMappingJson {
    #[serde(rename = "sourceSchemaName")]
    source_schema_name: Option<String>,
    #[serde(rename = "sourceTable")]
    source_table: Option<String>,
    #[serde(rename = "targetTable")]
    target_table: Option<String>,
    #[serde(rename = "excludedColumns")]
    excluded_columns: Option<Vec<String>>,
    #[serde(rename = "sortingKeys")]
    sorting_keys: Option<Vec<String>>,
    #[serde(rename = "useCustomSortingKey")]
    use_custom_sorting_key: Option<bool>,
    #[serde(rename = "partitionByExpr")]
    partition_by_expr: Option<String>,
    #[serde(rename = "partitionKey")]
    partition_key: Option<String>,
    #[serde(rename = "tableEngine")]
    table_engine: Option<String>,
}

/// Parse and validate one `--table-mapping-json` value into the library's
/// table mapping. `position` is the flag's zero-based occurrence, so an error
/// names the offending value instead of the whole request.
pub(super) fn parse_postgres_table_mapping_json(
    position: usize,
    raw: &str,
) -> CloudResult<ClickPipePostgresPipeTableMapping> {
    let flag = format!("--table-mapping-json #{}", position + 1);
    let invalid = |detail: String| CloudError::usage(format!("{flag}: {detail}"));
    let fields = POSTGRES_TABLE_MAPPING_JSON_FIELDS.join(", ");

    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| invalid(format!("invalid JSON: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| invalid(format!("expected a JSON object with the fields {fields}")))?;
    let unknown: Vec<&str> = object
        .keys()
        .map(String::as_str)
        .filter(|key| !POSTGRES_TABLE_MAPPING_JSON_FIELDS.contains(key))
        .collect();
    if !unknown.is_empty() {
        return Err(invalid(format!(
            "unknown field{} {}; valid fields are {fields}",
            if unknown.len() == 1 { "" } else { "s" },
            unknown.join(", "),
        )));
    }
    let mapping: PostgresTableMappingJson =
        serde_json::from_value(value).map_err(|error| invalid(error.to_string()))?;

    let required = |field: &str, value: Option<String>| -> CloudResult<String> {
        let value = value.unwrap_or_default();
        if value.trim().is_empty() {
            return Err(invalid(format!(
                "{field} is required and must not be empty"
            )));
        }
        Ok(value.trim().to_string())
    };
    let entries = |field: &str, values: Option<Vec<String>>| -> CloudResult<Vec<String>> {
        values
            .unwrap_or_default()
            .into_iter()
            .map(|entry| {
                if entry.trim().is_empty() {
                    return Err(invalid(format!("{field} must not contain an empty entry")));
                }
                Ok(entry.trim().to_string())
            })
            .collect()
    };

    let source_schema_name = required("sourceSchemaName", mapping.source_schema_name)?;
    let source_table = required("sourceTable", mapping.source_table)?;
    let target_table = required("targetTable", mapping.target_table)?;
    let excluded_columns = entries("excludedColumns", mapping.excluded_columns)?;
    let sorting_keys = entries("sortingKeys", mapping.sorting_keys)?;
    // The API applies `sortingKeys` only when `useCustomSortingKey` is true,
    // so keys on their own would be silently ignored: turn the flag on for the
    // caller, and reject the two spellings that contradict each other.
    let use_custom_sorting_key = match mapping.use_custom_sorting_key {
        Some(true) if sorting_keys.is_empty() => {
            return Err(invalid(
                "useCustomSortingKey is true but sortingKeys is empty; list the destination \
                 ORDER BY columns in sortingKeys"
                    .to_string(),
            ));
        }
        Some(false) if !sorting_keys.is_empty() => {
            return Err(invalid(
                "sortingKeys is set but useCustomSortingKey is false, which would ignore the \
                 keys; omit useCustomSortingKey or set it to true"
                    .to_string(),
            ));
        }
        Some(explicit) => explicit,
        None => !sorting_keys.is_empty(),
    };
    let table_engine = match mapping.table_engine {
        Some(engine) => parse_serde_enum(
            &engine,
            "tableEngine",
            ClickPipePostgresPipeTableMappingTableengine::VALUES,
        )
        .map_err(|error| invalid(error.message))?,
        None => ClickPipePostgresPipeTableMappingTableengine::ReplacingMergeTree,
    };

    Ok(ClickPipePostgresPipeTableMapping {
        source_schema_name,
        source_table,
        target_table,
        excluded_columns,
        sorting_keys,
        use_custom_sorting_key,
        partition_by_expr: mapping.partition_by_expr.unwrap_or_default(),
        partition_key: mapping.partition_key.unwrap_or_default(),
        table_engine,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::clickpipes::test_support::{
        MAXIMAL_BIGQUERY_TABLE_MAPPING_JSON, MAXIMAL_MONGODB_TABLE_MAPPING_JSON,
        MAXIMAL_MYSQL_TABLE_MAPPING_JSON, MAXIMAL_TABLE_MAPPING_JSON, assert_names, names_word,
    };

    #[test]
    fn parse_postgres_table_mapping_json_accepts_a_minimal_object() {
        let mapping = parse_postgres_table_mapping_json(
            0,
            r#"{"sourceSchemaName":" public ","sourceTable":" users ","targetTable":" users_raw "}"#,
        )
        .unwrap();

        // Absent optional fields fall back to the same request shape the
        // simple `schema.table:target` form sends.
        assert_eq!(
            mapping,
            ClickPipePostgresPipeTableMapping {
                source_schema_name: "public".into(),
                source_table: "users".into(),
                target_table: "users_raw".into(),
                excluded_columns: vec![],
                sorting_keys: vec![],
                use_custom_sorting_key: false,
                partition_by_expr: String::new(),
                partition_key: String::new(),
                table_engine: ClickPipePostgresPipeTableMappingTableengine::ReplacingMergeTree,
            }
        );
    }

    #[test]
    fn parse_postgres_table_mapping_json_accepts_a_maximal_object() {
        let mapping = parse_postgres_table_mapping_json(0, MAXIMAL_TABLE_MAPPING_JSON).unwrap();

        assert_eq!(
            mapping,
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
    }

    #[test]
    fn parse_postgres_table_mapping_json_enables_the_custom_sorting_key() {
        // The API ignores `sortingKeys` unless `useCustomSortingKey` is true,
        // so omitting the flag must not silently drop the keys.
        let mapping = parse_postgres_table_mapping_json(
            0,
            r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users",
                "sortingKeys":["created_at"," id "]}"#,
        )
        .unwrap();
        assert!(mapping.use_custom_sorting_key);
        assert_eq!(mapping.sorting_keys, ["created_at", "id"]);

        // Without keys, the flag stays false and the pipe keeps the default
        // ordering key.
        let mapping = parse_postgres_table_mapping_json(
            0,
            r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users"}"#,
        )
        .unwrap();
        assert!(!mapping.use_custom_sorting_key);
    }

    #[test]
    fn parse_postgres_table_mapping_json_accepts_every_table_engine() {
        for engine in ClickPipePostgresPipeTableMappingTableengine::VALUES {
            let mapping = parse_postgres_table_mapping_json(
                0,
                &format!(
                    r#"{{"sourceSchemaName":"public","sourceTable":"users",
                        "targetTable":"users","tableEngine":"{engine}"}}"#
                ),
            )
            .unwrap();
            assert_eq!(mapping.table_engine.to_string(), *engine);
        }
    }

    #[test]
    fn parse_postgres_table_mapping_json_rejects_invalid_objects() {
        let fields = POSTGRES_TABLE_MAPPING_JSON_FIELDS;
        // serde's own diagnostic for a value, which the error passes through.
        let serde_detail = |raw: &str| {
            serde_json::from_value::<PostgresTableMappingJson>(serde_json::from_str(raw).unwrap())
                .err()
                .expect("the value has the wrong type")
                .to_string()
        };
        let wrong_type = r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","sortingKeys":"id"}"#;
        // Each input breaks one rule, so each fails on its own branch; the
        // error names what it rejected.
        let cases: Vec<(&str, Vec<String>)> = vec![
            // Not JSON, and not an object.
            (
                "{ nope",
                vec![
                    serde_json::from_str::<serde_json::Value>("{ nope")
                        .unwrap_err()
                        .to_string(),
                ],
            ),
            (
                r#"["public.users"]"#,
                fields.iter().map(|field| field.to_string()).collect(),
            ),
            // A typo in a field name is rejected instead of being ignored.
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","excludeColumns":["ssn"]}"#,
                std::iter::once("excludeColumns")
                    .chain(fields.iter().copied())
                    .map(str::to_string)
                    .collect(),
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","engine":"Null","order":"id"}"#,
                vec!["engine".into(), "order".into()],
            ),
            // Wrong JSON type for a known field.
            (wrong_type, vec![serde_detail(wrong_type)]),
            // Required fields, absent or empty.
            (
                r#"{"sourceTable":"users","targetTable":"users"}"#,
                vec!["sourceSchemaName".into()],
            ),
            (
                r#"{"sourceSchemaName":"public","targetTable":"users"}"#,
                vec!["sourceTable".into()],
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users"}"#,
                vec!["targetTable".into()],
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"  "}"#,
                vec!["targetTable".into()],
            ),
            // Empty list entries.
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","excludedColumns":["ssn",""]}"#,
                vec!["excludedColumns".into()],
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","sortingKeys":[" "]}"#,
                vec!["sortingKeys".into()],
            ),
            // The two contradictions between sortingKeys and its flag.
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","useCustomSortingKey":true}"#,
                vec!["useCustomSortingKey".into(), "sortingKeys".into()],
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","sortingKeys":["id"],"useCustomSortingKey":false}"#,
                vec!["useCustomSortingKey".into(), "sortingKeys".into()],
            ),
            // An unknown table engine is caught here rather than reaching the
            // API, because the engine cannot be changed after creation. The
            // error names the field, the value and every accepted engine.
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","tableEngine":"MergeTre"}"#,
                ["tableEngine", "'MergeTre'"]
                    .into_iter()
                    .chain(
                        ClickPipePostgresPipeTableMappingTableengine::VALUES
                            .iter()
                            .copied(),
                    )
                    .map(str::to_string)
                    .collect(),
            ),
        ];

        for (raw, names) in cases {
            let error = parse_postgres_table_mapping_json(0, raw).unwrap_err();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            assert_names(&error.message, &names);
            // Every diagnostic names the offending flag occurrence.
            assert!(
                error.message.starts_with("--table-mapping-json #1: "),
                "{}",
                error.message
            );
        }

        // A missing or blank field names only that field, unlike the
        // not-an-object error that lists them all.
        for (raw, field) in [
            (
                r#"{"sourceTable":"users","targetTable":"users"}"#,
                "sourceSchemaName",
            ),
            (
                r#"{"sourceSchemaName":"public","sourceTable":"users","targetTable":"  "}"#,
                "targetTable",
            ),
        ] {
            let message = parse_postgres_table_mapping_json(0, raw)
                .unwrap_err()
                .message;
            let named: Vec<_> = fields
                .iter()
                .filter(|name| names_word(&message, name))
                .collect();
            assert_eq!(named, [&field], "{message}");
        }

        // The two sortingKeys contradictions are told apart.
        let contradiction = |use_custom: &str, keys: &str| {
            parse_postgres_table_mapping_json(
                0,
                &format!(
                    r#"{{"sourceSchemaName":"public","sourceTable":"users","targetTable":"users","useCustomSortingKey":{use_custom},"sortingKeys":{keys}}}"#
                ),
            )
            .unwrap_err()
            .message
        };
        assert_ne!(
            contradiction("true", "[]"),
            contradiction("false", r#"["id"]"#)
        );

        // The occurrence number is one-based over the flag's own values.
        let error = parse_postgres_table_mapping_json(2, "{ nope").unwrap_err();
        assert!(
            error.message.starts_with("--table-mapping-json #3: "),
            "{}",
            error.message
        );
    }

    #[test]
    fn non_postgres_table_mapping_json_accepts_every_field() {
        let mysql = parse_mysql_table_mapping_json(0, MAXIMAL_MYSQL_TABLE_MAPPING_JSON).unwrap();
        assert_eq!(mysql.source_schema_name, "sales");
        assert_eq!(mysql.source_table, "orders");
        assert_eq!(mysql.target_table, "orders_raw");
        assert_eq!(
            mysql.excluded_columns.unwrap(),
            ["internal_id", "temporary"]
        );
        assert_eq!(mysql.sorting_keys.unwrap(), ["created_at", "id"]);
        assert_eq!(mysql.use_custom_sorting_key, Some(true));
        assert_eq!(mysql.partition_key.as_deref(), Some("id"));
        assert_eq!(
            mysql.partition_by_expr.as_deref(),
            Some("toYYYYMM(created_at)")
        );
        assert_eq!(
            mysql.table_engine,
            Some(ClickPipeMySQLPipeTableMappingTableengine::ReplacingMergeTree)
        );

        let mongodb =
            parse_mongodb_table_mapping_json(0, MAXIMAL_MONGODB_TABLE_MAPPING_JSON).unwrap();
        assert_eq!(mongodb.source_database_name, "sales");
        assert_eq!(mongodb.source_collection, "orders");
        assert_eq!(mongodb.target_table, "orders_raw");
        assert_eq!(
            mongodb.table_engine,
            Some(ClickPipeMongoDBPipeTableMappingTableengine::Null)
        );

        let bigquery =
            parse_bigquery_table_mapping_json(0, MAXIMAL_BIGQUERY_TABLE_MAPPING_JSON).unwrap();
        assert_eq!(bigquery.source_dataset_name, "sales");
        assert_eq!(bigquery.source_table, "orders");
        assert_eq!(bigquery.target_table, "orders_raw");
        assert_eq!(
            bigquery.excluded_columns.unwrap(),
            ["internal_id", "temporary"]
        );
        assert_eq!(bigquery.sorting_keys.unwrap(), ["created_at", "id"]);
        assert_eq!(bigquery.use_custom_sorting_key, Some(true));
        assert_eq!(
            bigquery.table_engine,
            Some(ClickPipeBigQueryPipeTableMappingTableengine::MergeTree)
        );
    }

    #[test]
    fn non_postgres_table_mapping_json_preserves_optional_omission_and_empty_lists() {
        let mysql = parse_mysql_table_mapping_json(
            0,
            r#"{"sourceSchemaName":"db","sourceTable":"t","targetTable":"t"}"#,
        )
        .unwrap();
        assert_eq!(mysql.excluded_columns, None);
        assert_eq!(mysql.sorting_keys, None);
        assert_eq!(mysql.use_custom_sorting_key, None);
        assert_eq!(mysql.partition_key, None);
        assert_eq!(mysql.partition_by_expr, None);
        assert_eq!(mysql.table_engine, None);

        let bigquery = parse_bigquery_table_mapping_json(
            0,
            r#"{"sourceDatasetName":"ds","sourceTable":"t","targetTable":"t","excludedColumns":[],"sortingKeys":[],"useCustomSortingKey":false}"#,
        )
        .unwrap();
        assert_eq!(bigquery.excluded_columns, Some(vec![]));
        assert_eq!(bigquery.sorting_keys, Some(vec![]));
        assert_eq!(bigquery.use_custom_sorting_key, Some(false));
        assert_eq!(bigquery.table_engine, None);
    }

    #[test]
    fn mongodb_table_mapping_json_accepts_structured_ingestion() {
        use clickhouse_cloud_api::models::{
            ClickPipeMongoDBPipeColumn, ClickPipeMongoDBStructuredIngestion,
        };

        let minimal = parse_mongodb_table_mapping_json(
            0,
            r#"{"sourceDatabaseName":"db","sourceCollection":"c","targetTable":"t","structuredIngestion":{"enabled":true}}"#,
        )
        .unwrap();
        assert_eq!(
            minimal.structured_ingestion,
            Some(ClickPipeMongoDBStructuredIngestion {
                enabled: true,
                columns: None,
                drop_unexpected_values: None,
                excluded_columns: None,
                partition_by_expr: None,
                sorting_keys: None,
            })
        );

        let maximal = parse_mongodb_table_mapping_json(
            0,
            r#"{"sourceDatabaseName":"db","sourceCollection":"c","targetTable":"t","structuredIngestion":{
                "enabled":true,
                "dropUnexpectedValues":true,
                "columns":[{"name":" amount ","type":"Float64"}],
                "excludedColumns":["internal_notes"],
                "sortingKeys":["_id","created_at"],
                "partitionByExpr":"toYYYYMM(created_at)"
            }}"#,
        )
        .unwrap();
        assert_eq!(
            maximal.structured_ingestion,
            Some(ClickPipeMongoDBStructuredIngestion {
                enabled: true,
                drop_unexpected_values: Some(true),
                columns: Some(vec![ClickPipeMongoDBPipeColumn {
                    name: "amount".to_string(),
                    r#type: "Float64".to_string(),
                }]),
                excluded_columns: Some(vec!["internal_notes".to_string()]),
                sorting_keys: Some(vec!["_id".to_string(), "created_at".to_string()]),
                partition_by_expr: Some("toYYYYMM(created_at)".to_string()),
            })
        );
    }

    #[test]
    fn mongodb_table_mapping_json_rejects_invalid_structured_ingestion() {
        let base = r#""sourceDatabaseName":"db","sourceCollection":"c","targetTable":"t""#;
        // Each input breaks one rule; the error names the offending field
        // (and the duplicated entry).
        let cases: [(&str, &[&str]); 7] = [
            (r#"{}"#, &["`enabled`"]),
            (
                r#"{"enabled":true,"sortKeys":[]}"#,
                &["`structuredIngestion.sortKeys`"],
            ),
            (r#"{"enabled":true,"columns":[{"name":"a"}]}"#, &["`type`"]),
            (
                r#"{"enabled":true,"columns":[{"name":"","type":"String"}]}"#,
                &["structuredIngestion.columns[].name"],
            ),
            (
                r#"{"enabled":true,"excludedColumns":["a","a"]}"#,
                &["structuredIngestion.excludedColumns", "'a'"],
            ),
            (
                r#"{"enabled":true,"sortingKeys":["_id",""]}"#,
                &["structuredIngestion.sortingKeys"],
            ),
            (
                r#"{"enabled":true,"partitionByExpr":" "}"#,
                &["structuredIngestion.partitionByExpr"],
            ),
        ];
        for (ingestion, names) in cases {
            let raw = format!(r#"{{{base},"structuredIngestion":{ingestion}}}"#);
            let error = parse_mongodb_table_mapping_json(0, &raw).unwrap_err();
            assert!(error.message.starts_with("--table-mapping-json #1: "));
            assert_names(&error.message, names);
        }
    }

    #[test]
    fn mongodb_table_mapping_json_accepts_mongo_config() {
        use clickhouse_cloud_api::models::ClickPipeMongoDBTableConfig;

        let base = r#""sourceDatabaseName":"db","sourceCollection":"c","targetTable":"t""#;
        let unset = parse_mongodb_table_mapping_json(0, &format!("{{{base}}}")).unwrap();
        assert_eq!(unset.mongo_config, None);

        for persist in [true, false] {
            let raw =
                format!(r#"{{{base},"mongoConfig":{{"persistDeletedDocuments":{persist}}}}}"#);
            let mapping = parse_mongodb_table_mapping_json(0, &raw).unwrap();
            assert_eq!(
                mapping.mongo_config,
                Some(ClickPipeMongoDBTableConfig {
                    persist_deleted_documents: persist,
                })
            );
        }

        // serde's diagnostics pass through; each names the field or value.
        for (config, name) in [
            (r#"{}"#, "`persistDeletedDocuments`"),
            (
                r#"{"persistDeletedDocuments":true,"preImages":true}"#,
                "`mongoConfig.preImages`",
            ),
            (r#"{"persistDeletedDocuments":"yes"}"#, "\"yes\""),
        ] {
            let raw = format!(r#"{{{base},"mongoConfig":{config}}}"#);
            let error = parse_mongodb_table_mapping_json(0, &raw).unwrap_err();
            assert!(error.message.starts_with("--table-mapping-json #1: "));
            assert_names(&error.message, &[name]);
        }
    }

    #[test]
    fn non_postgres_table_mapping_json_accepts_every_table_engine() {
        for engine in ClickPipeMySQLPipeTableMappingTableengine::VALUES {
            let mapping = parse_mysql_table_mapping_json(
                0,
                &format!(
                    r#"{{"sourceSchemaName":"db","sourceTable":"t","targetTable":"t","tableEngine":"{engine}"}}"#
                ),
            )
            .unwrap();
            assert_eq!(mapping.table_engine.unwrap().to_string(), *engine);
        }
        for engine in ClickPipeMongoDBPipeTableMappingTableengine::VALUES {
            let mapping = parse_mongodb_table_mapping_json(
                0,
                &format!(
                    r#"{{"sourceDatabaseName":"db","sourceCollection":"c","targetTable":"t","tableEngine":"{engine}"}}"#
                ),
            )
            .unwrap();
            assert_eq!(mapping.table_engine.unwrap().to_string(), *engine);
        }
        for engine in ClickPipeBigQueryPipeTableMappingTableengine::VALUES {
            let mapping = parse_bigquery_table_mapping_json(
                0,
                &format!(
                    r#"{{"sourceDatasetName":"ds","sourceTable":"t","targetTable":"t","tableEngine":"{engine}"}}"#
                ),
            )
            .unwrap();
            assert_eq!(mapping.table_engine.unwrap().to_string(), *engine);
        }
    }

    #[test]
    fn non_postgres_table_mapping_json_rejects_invalid_objects() {
        let not_json = serde_json::from_str::<serde_json::Value>("{ nope")
            .unwrap_err()
            .to_string();
        // Each input breaks one rule; the error names what it rejected.
        let mysql_cases: [(&str, &[&str]); 9] = [
            ("{ nope", &[&not_json]),
            (r#"["db.t"]"#, MYSQL_TABLE_MAPPING_JSON_FIELDS),
            (
                r#"{"sourceSchemaName":"db","sourceTable":"t","targetTable":"t","excludeColumns":[]}"#,
                &["excludeColumns"],
            ),
            (
                r#"{"sourceTable":"t","targetTable":"t"}"#,
                &["`sourceSchemaName`"],
            ),
            (
                r#"{"sourceSchemaName":"db","sourceTable":"t","targetTable":"t","excludedColumns":["id",""]}"#,
                &["excludedColumns"],
            ),
            (
                r#"{"sourceSchemaName":"db","sourceTable":"t","targetTable":"t","sortingKeys":["id","id"]}"#,
                &["sortingKeys", "'id'"],
            ),
            (
                r#"{"sourceSchemaName":"db","sourceTable":"t","targetTable":"t","useCustomSortingKey":true}"#,
                &["useCustomSortingKey", "sortingKeys"],
            ),
            (
                r#"{"sourceSchemaName":"db","sourceTable":"t","targetTable":"t","sortingKeys":["id"],"useCustomSortingKey":false}"#,
                &["useCustomSortingKey", "sortingKeys"],
            ),
            (
                r#"{"sourceSchemaName":"db","sourceTable":"t","targetTable":"t","tableEngine":"MergeTre"}"#,
                &["tableEngine", "'MergeTre'"],
            ),
        ];
        for (raw, names) in mysql_cases {
            let error = parse_mysql_table_mapping_json(1, raw).unwrap_err();
            assert!(error.message.starts_with("--table-mapping-json #2: "));
            assert_names(&error.message, names);
        }

        let mongodb = parse_mongodb_table_mapping_json(
            0,
            r#"{"sourceDatabaseName":"db","sourceCollection":"c","targetTable":"t","sortingKeys":[]}"#,
        )
        .unwrap_err();
        assert_names(&mongodb.message, &["sortingKeys"]);
        let bigquery = parse_bigquery_table_mapping_json(
            0,
            r#"{"sourceDatasetName":"ds","sourceTable":"","targetTable":"t"}"#,
        )
        .unwrap_err();
        assert_names(&bigquery.message, &["sourceTable"]);
    }
}
