use super::enums::*;
use serde::{Deserialize, Serialize};

/// `ClickStackSource` - one of multiple variants.
///
/// Dispatched on the `kind` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackSource {
    ClickStackLogSource(ClickStackLogSource),
    ClickStackTraceSource(ClickStackTraceSource),
    ClickStackMetricSource(ClickStackMetricSource),
    ClickStackSessionSource(ClickStackSessionSource),
    ClickStackPromqlSource(ClickStackPromqlSource),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackSource, "kind" {
        "log" => ClickStackLogSource,
        "trace" => ClickStackTraceSource,
        "metric" => ClickStackMetricSource,
        "session" => ClickStackSessionSource,
        "promql" => ClickStackPromqlSource,
    }
}

impl std::fmt::Display for ClickStackSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLogSource(_) => write!(f, "ClickStackLogSource"),
            Self::ClickStackTraceSource(_) => write!(f, "ClickStackTraceSource"),
            Self::ClickStackMetricSource(_) => write!(f, "ClickStackMetricSource"),
            Self::ClickStackSessionSource(_) => write!(f, "ClickStackSessionSource"),
            Self::ClickStackPromqlSource(_) => write!(f, "ClickStackPromqlSource"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackSource` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackSource`]: each arm is the all-`Option`
/// response variant of its request struct, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `kind` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `kind` this crate does not know, or a payload that does
/// not fit the variant its `kind` selects, lands in `Unknown` with the raw JSON
/// intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `kind` and so would not deserialize back to the same
/// variant. Build a [`ClickStackSource`] instead when writing.
// The trace arm is the largest because the trace schema has the most
// expressions. Boxing it would change the public variant type that callers
// match on, for a size saving that does not matter in a decoded response.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackSourceResponse {
    ClickStackLogSource(ClickStackLogSourceResponse),
    ClickStackTraceSource(ClickStackTraceSourceResponse),
    ClickStackMetricSource(ClickStackMetricSourceResponse),
    ClickStackSessionSource(ClickStackSessionSourceResponse),
    ClickStackPromqlSource(ClickStackPromqlSourceResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackSourceResponse, "kind" {
        "log" => ClickStackLogSource,
        "trace" => ClickStackTraceSource,
        "metric" => ClickStackMetricSource,
        "session" => ClickStackSessionSource,
        "promql" => ClickStackPromqlSource,
    }
}

impl std::fmt::Display for ClickStackSourceResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLogSource(_) => write!(f, "ClickStackLogSource"),
            Self::ClickStackTraceSource(_) => write!(f, "ClickStackTraceSource"),
            Self::ClickStackMetricSource(_) => write!(f, "ClickStackMetricSource"),
            Self::ClickStackSessionSource(_) => write!(f, "ClickStackSessionSource"),
            Self::ClickStackPromqlSource(_) => write!(f, "ClickStackPromqlSource"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAggregatedColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAggregatedColumn {
    #[serde(rename = "aggFn")]
    pub agg_fn: String,
    #[serde(rename = "mvColumn")]
    pub mv_column: String,
    #[serde(rename = "sourceColumn", skip_serializing_if = "Option::is_none")]
    pub source_column: Option<String>,
}

/// `ClickStackAggregatedColumn` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAggregatedColumn`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAggregatedColumnResponse {
    #[serde(rename = "aggFn", skip_serializing_if = "Option::is_none")]
    pub agg_fn: Option<String>,
    #[serde(rename = "mvColumn", skip_serializing_if = "Option::is_none")]
    pub mv_column: Option<String>,
    #[serde(rename = "sourceColumn", skip_serializing_if = "Option::is_none")]
    pub source_column: Option<String>,
}

/// `ClickStackConnection` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackConnection {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(
        rename = "hyperdxSettingPrefix",
        skip_serializing_if = "Option::is_none"
    )]
    pub hyperdx_setting_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "isPrometheusEndpoint",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_prometheus_endpoint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// `ClickStackCreateConnectionRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateConnectionRequest {
    pub host: String,
    #[serde(
        rename = "hyperdxSettingPrefix",
        skip_serializing_if = "Option::is_none"
    )]
    pub hyperdx_setting_prefix: Option<String>,
    #[serde(
        rename = "isPrometheusEndpoint",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_prometheus_endpoint: Option<bool>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub username: String,
}

/// `ClickStackFilterSettingsColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilterSettingsColumn {
    #[serde(rename = "valueExpression", skip_serializing_if = "Option::is_none")]
    pub value_expression: Option<String>,
    #[serde(rename = "allowAll", skip_serializing_if = "Option::is_none")]
    pub allow_all: Option<bool>,
    pub label: String,
    pub name: String,
}

/// `ClickStackFilterSettingsColumn` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackFilterSettingsColumn`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilterSettingsColumnResponse {
    #[serde(rename = "valueExpression", skip_serializing_if = "Option::is_none")]
    pub value_expression: Option<String>,
    #[serde(rename = "allowAll", skip_serializing_if = "Option::is_none")]
    pub allow_all: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `ClickStackHighlightedAttributeExpression` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHighlightedAttributeExpression {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(rename = "luceneExpression", skip_serializing_if = "Option::is_none")]
    pub lucene_expression: Option<String>,
    #[serde(rename = "sqlExpression")]
    pub sql_expression: String,
}

/// `ClickStackHighlightedAttributeExpression` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackHighlightedAttributeExpression`]: every
/// field is `Option<T>`, so a field the API drops or sends as `null`
/// deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHighlightedAttributeExpressionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(rename = "luceneExpression", skip_serializing_if = "Option::is_none")]
    pub lucene_expression: Option<String>,
    #[serde(rename = "sqlExpression", skip_serializing_if = "Option::is_none")]
    pub sql_expression: Option<String>,
}

/// `ClickStackLogSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLogSource {
    #[serde(
        rename = "serviceVersionExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_version_expression: Option<String>,
    #[serde(rename = "bodyExpression", skip_serializing_if = "Option::is_none")]
    pub body_expression: Option<String>,
    pub connection: String,
    #[serde(rename = "defaultTableSelectExpression")]
    pub default_table_select_expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(
        rename = "displayedTimestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub displayed_timestamp_value_expression: Option<String>,
    #[serde(
        rename = "eventAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub event_attributes_expression: Option<String>,
    #[serde(
        rename = "eventNameExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub event_name_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none")]
    pub filter_settings: Option<ClickStackSourceFilterSettings>,
    pub from: ClickStackSourceFrom,
    #[serde(
        rename = "highlightedRowAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_row_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(
        rename = "highlightedTraceAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_trace_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "implicitColumnExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub implicit_column_expression: Option<String>,
    pub kind: ClickStackLogSourceKind,
    #[serde(
        rename = "knownColumnsListExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub known_columns_list_expression: Option<String>,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none")]
    pub materialized_views: Option<Vec<ClickStackMaterializedView>>,
    #[serde(
        rename = "metadataMaterializedViews",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata_materialized_views: Option<ClickStackLogSourceMetadataMaterializedViews>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none")]
    pub metric_source_id: Option<String>,
    pub name: String,
    #[serde(rename = "orderByExpression", skip_serializing_if = "Option::is_none")]
    pub order_by_expression: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "serviceNameExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_name_expression: Option<String>,
    #[serde(
        rename = "severityNumberExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub severity_number_expression: Option<String>,
    #[serde(
        rename = "severityTextExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub severity_text_expression: Option<String>,
    #[serde(rename = "spanIdExpression", skip_serializing_if = "Option::is_none")]
    pub span_id_expression: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
    #[serde(rename = "traceIdExpression", skip_serializing_if = "Option::is_none")]
    pub trace_id_expression: Option<String>,
    #[serde(rename = "traceSourceId", skip_serializing_if = "Option::is_none")]
    pub trace_source_id: Option<String>,
    #[serde(
        rename = "useTextIndexForImplicitColumn",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_text_index_for_implicit_column:
        Option<ClickStackLogSourceUsetextindexforimplicitcolumn>,
}

/// `ClickStackLogSource` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackLogSource`]: every field is `Option<T>`, so
/// a field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLogSourceResponse {
    #[serde(
        rename = "serviceVersionExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_version_expression: Option<String>,
    #[serde(rename = "bodyExpression", skip_serializing_if = "Option::is_none")]
    pub body_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(
        rename = "defaultTableSelectExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub default_table_select_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(
        rename = "displayedTimestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub displayed_timestamp_value_expression: Option<String>,
    #[serde(
        rename = "eventAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub event_attributes_expression: Option<String>,
    #[serde(
        rename = "eventNameExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub event_name_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none")]
    pub filter_settings: Option<ClickStackSourceFilterSettingsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackSourceFromResponse>,
    #[serde(
        rename = "highlightedRowAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_row_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpressionResponse>>,
    #[serde(
        rename = "highlightedTraceAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_trace_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpressionResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "implicitColumnExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub implicit_column_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackLogSourceKind>,
    #[serde(
        rename = "knownColumnsListExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub known_columns_list_expression: Option<String>,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none")]
    pub materialized_views: Option<Vec<ClickStackMaterializedViewResponse>>,
    #[serde(
        rename = "metadataMaterializedViews",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata_materialized_views: Option<ClickStackLogSourceMetadataMaterializedViewsResponse>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none")]
    pub metric_source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "orderByExpression", skip_serializing_if = "Option::is_none")]
    pub order_by_expression: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySettingResponse>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "serviceNameExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_name_expression: Option<String>,
    #[serde(
        rename = "severityNumberExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub severity_number_expression: Option<String>,
    #[serde(
        rename = "severityTextExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub severity_text_expression: Option<String>,
    #[serde(rename = "spanIdExpression", skip_serializing_if = "Option::is_none")]
    pub span_id_expression: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
    #[serde(rename = "traceIdExpression", skip_serializing_if = "Option::is_none")]
    pub trace_id_expression: Option<String>,
    #[serde(rename = "traceSourceId", skip_serializing_if = "Option::is_none")]
    pub trace_source_id: Option<String>,
    #[serde(
        rename = "useTextIndexForImplicitColumn",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_text_index_for_implicit_column:
        Option<ClickStackLogSourceUsetextindexforimplicitcolumn>,
}

/// `ClickStackLogSourceMetadataMaterializedViews` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLogSourceMetadataMaterializedViews {
    pub granularity: String,
    #[serde(rename = "keyRollupTable")]
    pub key_rollup_table: String,
    #[serde(rename = "kvRollupTable")]
    pub kv_rollup_table: String,
}

/// `ClickStackLogSourceMetadataMaterializedViews` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackLogSourceMetadataMaterializedViews`]: every
/// field is `Option<T>`, so a field the API drops or sends as `null`
/// deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLogSourceMetadataMaterializedViewsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<String>,
    #[serde(rename = "keyRollupTable", skip_serializing_if = "Option::is_none")]
    pub key_rollup_table: Option<String>,
    #[serde(rename = "kvRollupTable", skip_serializing_if = "Option::is_none")]
    pub kv_rollup_table: Option<String>,
}

/// `ClickStackMaterializedView` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMaterializedView {
    #[serde(rename = "aggregatedColumns")]
    pub aggregated_columns: Vec<ClickStackAggregatedColumn>,
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "dimensionColumns")]
    pub dimension_columns: String,
    #[serde(rename = "minDate", skip_serializing_if = "Option::is_none")]
    pub min_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "minGranularity")]
    pub min_granularity: ClickStackMaterializedViewMingranularity,
    #[serde(rename = "tableName")]
    pub table_name: String,
    #[serde(rename = "timestampColumn")]
    pub timestamp_column: String,
}

/// `ClickStackMaterializedView` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMaterializedView`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMaterializedViewResponse {
    #[serde(rename = "aggregatedColumns", skip_serializing_if = "Option::is_none")]
    pub aggregated_columns: Option<Vec<ClickStackAggregatedColumnResponse>>,
    #[serde(rename = "databaseName", skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "dimensionColumns", skip_serializing_if = "Option::is_none")]
    pub dimension_columns: Option<String>,
    #[serde(rename = "minDate", skip_serializing_if = "Option::is_none")]
    pub min_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "minGranularity", skip_serializing_if = "Option::is_none")]
    pub min_granularity: Option<ClickStackMaterializedViewMingranularity>,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
    #[serde(rename = "timestampColumn", skip_serializing_if = "Option::is_none")]
    pub timestamp_column: Option<String>,
}

/// `ClickStackMetricSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSource {
    pub connection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    pub from: ClickStackMetricSourceFrom,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub kind: ClickStackMetricSourceKind,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none")]
    pub log_source_id: Option<String>,
    #[serde(rename = "metricTables")]
    pub metric_tables: ClickStackMetricTables,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(rename = "resourceAttributesExpression")]
    pub resource_attributes_expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
}

/// `ClickStackMetricSource` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMetricSource`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSourceResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackMetricSourceFromResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackMetricSourceKind>,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none")]
    pub log_source_id: Option<String>,
    #[serde(rename = "metricTables", skip_serializing_if = "Option::is_none")]
    pub metric_tables: Option<ClickStackMetricTablesResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySettingResponse>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
}

/// `ClickStackMetricSourceFrom` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSourceFrom {
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
}

/// `ClickStackMetricSourceFrom` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMetricSourceFrom`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSourceFromResponse {
    #[serde(rename = "databaseName", skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
}

/// `ClickStackMetricTables` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricTables {
    #[serde(rename = "exponential histogram")]
    pub exponential_histogram: String,
    pub gauge: String,
    pub histogram: String,
    pub sum: String,
    pub summary: String,
}

/// `ClickStackMetricTables` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMetricTables`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricTablesResponse {
    #[serde(
        rename = "exponential histogram",
        skip_serializing_if = "Option::is_none"
    )]
    pub exponential_histogram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gauge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub histogram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// `ClickStackPromqlSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPromqlSource {
    pub connection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    pub from: ClickStackSourceFrom,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub kind: ClickStackPromqlSourceKind,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
}

/// `ClickStackPromqlSource` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackPromqlSource`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPromqlSourceResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackSourceFromResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackPromqlSourceKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySettingResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
}

/// `ClickStackQuerySetting` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackQuerySetting {
    pub setting: String,
    pub value: String,
}

/// `ClickStackQuerySetting` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackQuerySetting`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackQuerySettingResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// `ClickStackSessionSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSessionSource {
    pub connection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    pub from: ClickStackSourceFrom,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub kind: ClickStackSessionSourceKind,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
    #[serde(rename = "traceSourceId")]
    pub trace_source_id: String,
}

/// `ClickStackSessionSource` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSessionSource`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSessionSourceResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackSourceFromResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackSessionSourceKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySettingResponse>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
    #[serde(rename = "traceSourceId", skip_serializing_if = "Option::is_none")]
    pub trace_source_id: Option<String>,
}

/// `ClickStackSourceFilterSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFilterSettings {
    pub columns: Vec<ClickStackFilterSettingsColumn>,
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
}

/// `ClickStackSourceFilterSettings` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSourceFilterSettings`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFilterSettingsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ClickStackFilterSettingsColumnResponse>>,
    #[serde(rename = "databaseName", skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
}

/// `ClickStackSourceFrom` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFrom {
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
}

/// `ClickStackSourceFrom` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSourceFrom`]: every field is `Option<T>`, so
/// a field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFromResponse {
    #[serde(rename = "databaseName", skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
}

/// `ClickStackTraceSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTraceSource {
    #[serde(
        rename = "serviceVersionExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_version_expression: Option<String>,
    pub connection: String,
    #[serde(rename = "defaultTableSelectExpression")]
    pub default_table_select_expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(
        rename = "displayedTimestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub displayed_timestamp_value_expression: Option<String>,
    #[serde(rename = "durationExpression")]
    pub duration_expression: String,
    #[serde(rename = "durationPrecision")]
    pub duration_precision: i64,
    #[serde(
        rename = "eventAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub event_attributes_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none")]
    pub filter_settings: Option<ClickStackSourceFilterSettings>,
    pub from: ClickStackSourceFrom,
    #[serde(
        rename = "highlightedRowAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_row_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(
        rename = "highlightedTraceAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_trace_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "implicitColumnExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub implicit_column_expression: Option<String>,
    pub kind: ClickStackTraceSourceKind,
    #[serde(
        rename = "knownColumnsListExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub known_columns_list_expression: Option<String>,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none")]
    pub log_source_id: Option<String>,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none")]
    pub materialized_views: Option<Vec<ClickStackMaterializedView>>,
    #[serde(
        rename = "metadataMaterializedViews",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata_materialized_views: Option<ClickStackTraceSourceMetadataMaterializedViews>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none")]
    pub metric_source_id: Option<String>,
    pub name: String,
    #[serde(rename = "orderByExpression", skip_serializing_if = "Option::is_none")]
    pub order_by_expression: Option<String>,
    #[serde(rename = "parentSpanIdExpression")]
    pub parent_span_id_expression: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(
        rename = "sampleRateExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub sample_rate_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "serviceNameExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_name_expression: Option<String>,
    #[serde(rename = "sessionSourceId", skip_serializing_if = "Option::is_none")]
    pub session_source_id: Option<String>,
    #[serde(
        rename = "spanEventsValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub span_events_value_expression: Option<String>,
    #[serde(rename = "spanIdExpression")]
    pub span_id_expression: String,
    #[serde(rename = "spanKindExpression")]
    pub span_kind_expression: String,
    #[serde(
        rename = "spanLinksValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub span_links_value_expression: Option<String>,
    #[serde(rename = "spanNameExpression")]
    pub span_name_expression: String,
    #[serde(
        rename = "statusCodeExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_code_expression: Option<String>,
    #[serde(
        rename = "statusMessageExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_message_expression: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
    #[serde(rename = "traceIdExpression")]
    pub trace_id_expression: String,
    #[serde(
        rename = "useTextIndexForImplicitColumn",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_text_index_for_implicit_column:
        Option<ClickStackTraceSourceUsetextindexforimplicitcolumn>,
}

/// `ClickStackTraceSource` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackTraceSource`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTraceSourceResponse {
    #[serde(
        rename = "serviceVersionExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_version_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(
        rename = "defaultTableSelectExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub default_table_select_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(
        rename = "displayedTimestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub displayed_timestamp_value_expression: Option<String>,
    #[serde(rename = "durationExpression", skip_serializing_if = "Option::is_none")]
    pub duration_expression: Option<String>,
    #[serde(rename = "durationPrecision", skip_serializing_if = "Option::is_none")]
    pub duration_precision: Option<i64>,
    #[serde(
        rename = "eventAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub event_attributes_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none")]
    pub filter_settings: Option<ClickStackSourceFilterSettingsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackSourceFromResponse>,
    #[serde(
        rename = "highlightedRowAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_row_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpressionResponse>>,
    #[serde(
        rename = "highlightedTraceAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_trace_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpressionResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "implicitColumnExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub implicit_column_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackTraceSourceKind>,
    #[serde(
        rename = "knownColumnsListExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub known_columns_list_expression: Option<String>,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none")]
    pub log_source_id: Option<String>,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none")]
    pub materialized_views: Option<Vec<ClickStackMaterializedViewResponse>>,
    #[serde(
        rename = "metadataMaterializedViews",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata_materialized_views: Option<ClickStackTraceSourceMetadataMaterializedViewsResponse>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none")]
    pub metric_source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "orderByExpression", skip_serializing_if = "Option::is_none")]
    pub order_by_expression: Option<String>,
    #[serde(
        rename = "parentSpanIdExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub parent_span_id_expression: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySettingResponse>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(
        rename = "sampleRateExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub sample_rate_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "serviceNameExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_name_expression: Option<String>,
    #[serde(rename = "sessionSourceId", skip_serializing_if = "Option::is_none")]
    pub session_source_id: Option<String>,
    #[serde(
        rename = "spanEventsValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub span_events_value_expression: Option<String>,
    #[serde(rename = "spanIdExpression", skip_serializing_if = "Option::is_none")]
    pub span_id_expression: Option<String>,
    #[serde(rename = "spanKindExpression", skip_serializing_if = "Option::is_none")]
    pub span_kind_expression: Option<String>,
    #[serde(
        rename = "spanLinksValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub span_links_value_expression: Option<String>,
    #[serde(rename = "spanNameExpression", skip_serializing_if = "Option::is_none")]
    pub span_name_expression: Option<String>,
    #[serde(
        rename = "statusCodeExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_code_expression: Option<String>,
    #[serde(
        rename = "statusMessageExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_message_expression: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
    #[serde(rename = "traceIdExpression", skip_serializing_if = "Option::is_none")]
    pub trace_id_expression: Option<String>,
    #[serde(
        rename = "useTextIndexForImplicitColumn",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_text_index_for_implicit_column:
        Option<ClickStackTraceSourceUsetextindexforimplicitcolumn>,
}

/// `ClickStackTraceSourceMetadataMaterializedViews` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTraceSourceMetadataMaterializedViews {
    pub granularity: String,
    #[serde(rename = "keyRollupTable")]
    pub key_rollup_table: String,
    #[serde(rename = "kvRollupTable")]
    pub kv_rollup_table: String,
}

/// `ClickStackTraceSourceMetadataMaterializedViews` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackTraceSourceMetadataMaterializedViews`]:
/// every field is `Option<T>`, so a field the API drops or sends as `null`
/// deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTraceSourceMetadataMaterializedViewsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<String>,
    #[serde(rename = "keyRollupTable", skip_serializing_if = "Option::is_none")]
    pub key_rollup_table: Option<String>,
    #[serde(rename = "kvRollupTable", skip_serializing_if = "Option::is_none")]
    pub kv_rollup_table: Option<String>,
}

/// `ClickStackUpdateConnectionRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateConnectionRequest {
    pub host: String,
    #[serde(
        rename = "hyperdxSettingPrefix",
        skip_serializing_if = "Option::is_none"
    )]
    pub hyperdx_setting_prefix: Option<String>,
    #[serde(
        rename = "isPrometheusEndpoint",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_prometheus_endpoint: Option<bool>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub username: String,
}

impl Default for ClickStackSource {
    fn default() -> Self {
        Self::ClickStackLogSource(ClickStackLogSource::default())
    }
}
