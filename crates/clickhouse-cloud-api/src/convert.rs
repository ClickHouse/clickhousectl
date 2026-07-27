//! Explicit conversions from response models back into request models.
//!
//! Response models are tolerant by construction: every field is `Option<T>`, so
//! a field the API drops or sends as `null` deserializes to `None` instead of
//! failing. Request models are strict: a field the API requires is `T`.
//!
//! A caller that fetches a resource, edits it, and writes it back therefore has
//! to resolve absence explicitly, and that is the point of this module — the old
//! `#[serde(default)]` policy silently fabricated `""`/`0`/`false` for a dropped
//! field and persisted it on the next write. A conversion that can lose that
//! information is a [`TryFrom`] reporting the missing wire field names; a
//! conversion that cannot is a [`From`].
//!
//! The ClickStack source tree converts as a group: [`TryFrom<ClickStackSourceResponse>`]
//! for [`ClickStackSource`] is the entry point, and every nested object in that
//! tree carries its own conversion so a missing field is named at the level it is
//! missing from.

use std::fmt;

use crate::models::{
    ClickStackAggregatedColumn, ClickStackAggregatedColumnResponse, ClickStackFilterSettingsColumn,
    ClickStackFilterSettingsColumnResponse, ClickStackHighlightedAttributeExpression,
    ClickStackHighlightedAttributeExpressionResponse, ClickStackLogSource,
    ClickStackLogSourceMetadataMaterializedViews,
    ClickStackLogSourceMetadataMaterializedViewsResponse, ClickStackLogSourceResponse,
    ClickStackMaterializedView, ClickStackMaterializedViewResponse, ClickStackMetricSource,
    ClickStackMetricSourceFrom, ClickStackMetricSourceFromResponse, ClickStackMetricSourceResponse,
    ClickStackMetricTables, ClickStackMetricTablesResponse, ClickStackPromqlSource,
    ClickStackPromqlSourceResponse, ClickStackQuerySetting, ClickStackQuerySettingResponse,
    ClickStackSessionSource, ClickStackSessionSourceResponse, ClickStackSource,
    ClickStackSourceFilterSettings, ClickStackSourceFilterSettingsResponse, ClickStackSourceFrom,
    ClickStackSourceFromResponse, ClickStackSourceResponse, ClickStackTraceSource,
    ClickStackTraceSourceMetadataMaterializedViews,
    ClickStackTraceSourceMetadataMaterializedViewsResponse, ClickStackTraceSourceResponse,
    PgBouncerConfig, PgBouncerConfigResponse, PgConfig, PgConfigResponse, PostgresInstanceConfig,
    PostgresInstanceConfigResponse, ResourceTagsV1, ResourceTagsV1Response, ScalingScheduleEntry,
    ScalingScheduleEntryRequest, UpgradeWindow, UpgradeWindowPutRequest,
};

/// The response omitted fields that the matching request model requires.
///
/// Field names are the wire (spec) names, so an error message points at the
/// JSON the API returned rather than at Rust identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingRequiredFields {
    fields: Vec<&'static str>,
}

impl MissingRequiredFields {
    pub(crate) fn new(fields: Vec<&'static str>) -> Self {
        Self { fields }
    }

    /// The missing wire field names, in declaration order.
    pub fn fields(&self) -> &[&'static str] {
        &self.fields
    }
}

impl fmt::Display for MissingRequiredFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the API response is missing required field(s): {}",
            self.fields.join(", ")
        )
    }
}

impl std::error::Error for MissingRequiredFields {}

impl TryFrom<ClickStackAggregatedColumnResponse> for ClickStackAggregatedColumn {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackAggregatedColumnResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.agg_fn.is_none() {
            missing.push("aggFn");
        }
        if value.mv_column.is_none() {
            missing.push("mvColumn");
        }
        match (value.agg_fn, value.mv_column) {
            (Some(agg_fn), Some(mv_column)) => Ok(Self {
                agg_fn,
                mv_column,
                source_column: value.source_column,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackFilterSettingsColumnResponse> for ClickStackFilterSettingsColumn {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackFilterSettingsColumnResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.label.is_none() {
            missing.push("label");
        }
        if value.name.is_none() {
            missing.push("name");
        }
        match (value.label, value.name) {
            (Some(label), Some(name)) => Ok(Self { label, name }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackHighlightedAttributeExpressionResponse>
    for ClickStackHighlightedAttributeExpression
{
    type Error = MissingRequiredFields;

    fn try_from(
        value: ClickStackHighlightedAttributeExpressionResponse,
    ) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.sql_expression.is_none() {
            missing.push("sqlExpression");
        }
        match value.sql_expression {
            Some(sql_expression) => Ok(Self {
                alias: value.alias,
                lucene_expression: value.lucene_expression,
                sql_expression,
            }),
            None => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackLogSourceMetadataMaterializedViewsResponse>
    for ClickStackLogSourceMetadataMaterializedViews
{
    type Error = MissingRequiredFields;

    fn try_from(
        value: ClickStackLogSourceMetadataMaterializedViewsResponse,
    ) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.granularity.is_none() {
            missing.push("granularity");
        }
        if value.key_rollup_table.is_none() {
            missing.push("keyRollupTable");
        }
        if value.kv_rollup_table.is_none() {
            missing.push("kvRollupTable");
        }
        match (
            value.granularity,
            value.key_rollup_table,
            value.kv_rollup_table,
        ) {
            (Some(granularity), Some(key_rollup_table), Some(kv_rollup_table)) => Ok(Self {
                granularity,
                key_rollup_table,
                kv_rollup_table,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackLogSourceResponse> for ClickStackLogSource {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackLogSourceResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.connection.is_none() {
            missing.push("connection");
        }
        if value.default_table_select_expression.is_none() {
            missing.push("defaultTableSelectExpression");
        }
        if value.from.is_none() {
            missing.push("from");
        }
        if value.kind.is_none() {
            missing.push("kind");
        }
        if value.name.is_none() {
            missing.push("name");
        }
        if value.timestamp_value_expression.is_none() {
            missing.push("timestampValueExpression");
        }
        match (
            value.connection,
            value.default_table_select_expression,
            value.from,
            value.kind,
            value.name,
            value.timestamp_value_expression,
        ) {
            (
                Some(connection),
                Some(default_table_select_expression),
                Some(from),
                Some(kind),
                Some(name),
                Some(timestamp_value_expression),
            ) => Ok(Self {
                body_expression: value.body_expression,
                connection,
                default_table_select_expression,
                disabled: value.disabled,
                displayed_timestamp_value_expression: value.displayed_timestamp_value_expression,
                event_attributes_expression: value.event_attributes_expression,
                filter_settings: value.filter_settings.map(TryInto::try_into).transpose()?,
                from: from.try_into()?,
                highlighted_row_attribute_expressions: value
                    .highlighted_row_attribute_expressions
                    .map(|items| {
                        items
                            .into_iter()
                            .map(TryInto::try_into)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
                highlighted_trace_attribute_expressions: value
                    .highlighted_trace_attribute_expressions
                    .map(|items| {
                        items
                            .into_iter()
                            .map(TryInto::try_into)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
                id: value.id,
                implicit_column_expression: value.implicit_column_expression,
                kind,
                known_columns_list_expression: value.known_columns_list_expression,
                materialized_views: value
                    .materialized_views
                    .map(|items| {
                        items
                            .into_iter()
                            .map(TryInto::try_into)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
                metadata_materialized_views: value
                    .metadata_materialized_views
                    .map(TryInto::try_into)
                    .transpose()?,
                metric_source_id: value.metric_source_id,
                name,
                query_settings: value
                    .query_settings
                    .map(|items| {
                        items
                            .into_iter()
                            .map(TryInto::try_into)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
                resource_attributes_expression: value.resource_attributes_expression,
                section: value.section,
                service_name_expression: value.service_name_expression,
                severity_text_expression: value.severity_text_expression,
                span_id_expression: value.span_id_expression,
                timestamp_value_expression,
                trace_id_expression: value.trace_id_expression,
                trace_source_id: value.trace_source_id,
                use_text_index_for_implicit_column: value.use_text_index_for_implicit_column,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackMaterializedViewResponse> for ClickStackMaterializedView {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackMaterializedViewResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.aggregated_columns.is_none() {
            missing.push("aggregatedColumns");
        }
        if value.database_name.is_none() {
            missing.push("databaseName");
        }
        if value.dimension_columns.is_none() {
            missing.push("dimensionColumns");
        }
        if value.min_granularity.is_none() {
            missing.push("minGranularity");
        }
        if value.table_name.is_none() {
            missing.push("tableName");
        }
        if value.timestamp_column.is_none() {
            missing.push("timestampColumn");
        }
        match (
            value.aggregated_columns,
            value.database_name,
            value.dimension_columns,
            value.min_granularity,
            value.table_name,
            value.timestamp_column,
        ) {
            (
                Some(aggregated_columns),
                Some(database_name),
                Some(dimension_columns),
                Some(min_granularity),
                Some(table_name),
                Some(timestamp_column),
            ) => Ok(Self {
                aggregated_columns: aggregated_columns
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
                database_name,
                dimension_columns,
                min_date: value.min_date,
                min_granularity,
                table_name,
                timestamp_column,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackMetricSourceFromResponse> for ClickStackMetricSourceFrom {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackMetricSourceFromResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.database_name.is_none() {
            missing.push("databaseName");
        }
        match value.database_name {
            Some(database_name) => Ok(Self {
                database_name,
                table_name: value.table_name,
            }),
            None => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackMetricSourceResponse> for ClickStackMetricSource {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackMetricSourceResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.connection.is_none() {
            missing.push("connection");
        }
        if value.from.is_none() {
            missing.push("from");
        }
        if value.kind.is_none() {
            missing.push("kind");
        }
        if value.metric_tables.is_none() {
            missing.push("metricTables");
        }
        if value.name.is_none() {
            missing.push("name");
        }
        if value.resource_attributes_expression.is_none() {
            missing.push("resourceAttributesExpression");
        }
        if value.timestamp_value_expression.is_none() {
            missing.push("timestampValueExpression");
        }
        match (
            value.connection,
            value.from,
            value.kind,
            value.metric_tables,
            value.name,
            value.resource_attributes_expression,
            value.timestamp_value_expression,
        ) {
            (
                Some(connection),
                Some(from),
                Some(kind),
                Some(metric_tables),
                Some(name),
                Some(resource_attributes_expression),
                Some(timestamp_value_expression),
            ) => Ok(Self {
                connection,
                disabled: value.disabled,
                from: from.try_into()?,
                id: value.id,
                kind,
                log_source_id: value.log_source_id,
                metric_tables: metric_tables.try_into()?,
                name,
                query_settings: value
                    .query_settings
                    .map(|items| {
                        items
                            .into_iter()
                            .map(TryInto::try_into)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
                resource_attributes_expression,
                section: value.section,
                timestamp_value_expression,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackMetricTablesResponse> for ClickStackMetricTables {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackMetricTablesResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.exponential_histogram.is_none() {
            missing.push("exponential histogram");
        }
        if value.gauge.is_none() {
            missing.push("gauge");
        }
        if value.histogram.is_none() {
            missing.push("histogram");
        }
        if value.sum.is_none() {
            missing.push("sum");
        }
        if value.summary.is_none() {
            missing.push("summary");
        }
        match (
            value.exponential_histogram,
            value.gauge,
            value.histogram,
            value.sum,
            value.summary,
        ) {
            (
                Some(exponential_histogram),
                Some(gauge),
                Some(histogram),
                Some(sum),
                Some(summary),
            ) => Ok(Self {
                exponential_histogram,
                gauge,
                histogram,
                sum,
                summary,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackPromqlSourceResponse> for ClickStackPromqlSource {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackPromqlSourceResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.connection.is_none() {
            missing.push("connection");
        }
        if value.from.is_none() {
            missing.push("from");
        }
        if value.kind.is_none() {
            missing.push("kind");
        }
        if value.name.is_none() {
            missing.push("name");
        }
        if value.timestamp_value_expression.is_none() {
            missing.push("timestampValueExpression");
        }
        match (
            value.connection,
            value.from,
            value.kind,
            value.name,
            value.timestamp_value_expression,
        ) {
            (
                Some(connection),
                Some(from),
                Some(kind),
                Some(name),
                Some(timestamp_value_expression),
            ) => Ok(Self {
                connection,
                disabled: value.disabled,
                from: from.try_into()?,
                id: value.id,
                kind,
                name,
                query_settings: value
                    .query_settings
                    .map(|items| {
                        items
                            .into_iter()
                            .map(TryInto::try_into)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
                section: value.section,
                timestamp_value_expression,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackQuerySettingResponse> for ClickStackQuerySetting {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackQuerySettingResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.setting.is_none() {
            missing.push("setting");
        }
        if value.value.is_none() {
            missing.push("value");
        }
        match (value.setting, value.value) {
            (Some(setting), Some(value)) => Ok(Self { setting, value }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackSessionSourceResponse> for ClickStackSessionSource {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackSessionSourceResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.connection.is_none() {
            missing.push("connection");
        }
        if value.from.is_none() {
            missing.push("from");
        }
        if value.kind.is_none() {
            missing.push("kind");
        }
        if value.name.is_none() {
            missing.push("name");
        }
        if value.trace_source_id.is_none() {
            missing.push("traceSourceId");
        }
        match (
            value.connection,
            value.from,
            value.kind,
            value.name,
            value.trace_source_id,
        ) {
            (Some(connection), Some(from), Some(kind), Some(name), Some(trace_source_id)) => {
                Ok(Self {
                    connection,
                    disabled: value.disabled,
                    from: from.try_into()?,
                    id: value.id,
                    kind,
                    name,
                    query_settings: value
                        .query_settings
                        .map(|items| {
                            items
                                .into_iter()
                                .map(TryInto::try_into)
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .transpose()?,
                    section: value.section,
                    timestamp_value_expression: value.timestamp_value_expression,
                    trace_source_id,
                })
            }
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackSourceFilterSettingsResponse> for ClickStackSourceFilterSettings {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackSourceFilterSettingsResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.columns.is_none() {
            missing.push("columns");
        }
        if value.database_name.is_none() {
            missing.push("databaseName");
        }
        if value.table_name.is_none() {
            missing.push("tableName");
        }
        match (value.columns, value.database_name, value.table_name) {
            (Some(columns), Some(database_name), Some(table_name)) => Ok(Self {
                columns: columns
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
                database_name,
                table_name,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackSourceFromResponse> for ClickStackSourceFrom {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackSourceFromResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.database_name.is_none() {
            missing.push("databaseName");
        }
        if value.table_name.is_none() {
            missing.push("tableName");
        }
        match (value.database_name, value.table_name) {
            (Some(database_name), Some(table_name)) => Ok(Self {
                database_name,
                table_name,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackSourceResponse> for ClickStackSource {
    type Error = MissingRequiredFields;

    /// Turns a fetched source into a create/update body.
    ///
    /// A write replaces the whole source, so every field the schema requires for
    /// the source's `kind` has to be present in the response; a missing one is
    /// named rather than invented. Nested objects report their own wire names,
    /// unprefixed — `databaseName` for a source's `from`, say.
    ///
    /// An `Unknown` payload — a `kind` this crate does not model, or a body that
    /// did not fit the variant its `kind` selected — converts losslessly: the
    /// request union's own `Unknown` arm holds the raw JSON and serializes it
    /// verbatim, so such a source can still be written back.
    fn try_from(value: ClickStackSourceResponse) -> Result<Self, Self::Error> {
        Ok(match value {
            ClickStackSourceResponse::ClickStackLogSource(source) => {
                Self::ClickStackLogSource(source.try_into()?)
            }
            ClickStackSourceResponse::ClickStackTraceSource(source) => {
                Self::ClickStackTraceSource(source.try_into()?)
            }
            ClickStackSourceResponse::ClickStackMetricSource(source) => {
                Self::ClickStackMetricSource(source.try_into()?)
            }
            ClickStackSourceResponse::ClickStackSessionSource(source) => {
                Self::ClickStackSessionSource(source.try_into()?)
            }
            ClickStackSourceResponse::ClickStackPromqlSource(source) => {
                Self::ClickStackPromqlSource(source.try_into()?)
            }
            ClickStackSourceResponse::Unknown(raw) => Self::Unknown(raw),
        })
    }
}

impl TryFrom<ClickStackTraceSourceMetadataMaterializedViewsResponse>
    for ClickStackTraceSourceMetadataMaterializedViews
{
    type Error = MissingRequiredFields;

    fn try_from(
        value: ClickStackTraceSourceMetadataMaterializedViewsResponse,
    ) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.granularity.is_none() {
            missing.push("granularity");
        }
        if value.key_rollup_table.is_none() {
            missing.push("keyRollupTable");
        }
        if value.kv_rollup_table.is_none() {
            missing.push("kvRollupTable");
        }
        match (
            value.granularity,
            value.key_rollup_table,
            value.kv_rollup_table,
        ) {
            (Some(granularity), Some(key_rollup_table), Some(kv_rollup_table)) => Ok(Self {
                granularity,
                key_rollup_table,
                kv_rollup_table,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackTraceSourceResponse> for ClickStackTraceSource {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickStackTraceSourceResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.connection.is_none() {
            missing.push("connection");
        }
        if value.default_table_select_expression.is_none() {
            missing.push("defaultTableSelectExpression");
        }
        if value.duration_expression.is_none() {
            missing.push("durationExpression");
        }
        if value.duration_precision.is_none() {
            missing.push("durationPrecision");
        }
        if value.from.is_none() {
            missing.push("from");
        }
        if value.kind.is_none() {
            missing.push("kind");
        }
        if value.name.is_none() {
            missing.push("name");
        }
        if value.parent_span_id_expression.is_none() {
            missing.push("parentSpanIdExpression");
        }
        if value.span_id_expression.is_none() {
            missing.push("spanIdExpression");
        }
        if value.span_kind_expression.is_none() {
            missing.push("spanKindExpression");
        }
        if value.span_name_expression.is_none() {
            missing.push("spanNameExpression");
        }
        if value.timestamp_value_expression.is_none() {
            missing.push("timestampValueExpression");
        }
        if value.trace_id_expression.is_none() {
            missing.push("traceIdExpression");
        }
        match (
            value.connection,
            value.default_table_select_expression,
            value.duration_expression,
            value.duration_precision,
            value.from,
            value.kind,
            value.name,
            value.parent_span_id_expression,
            value.span_id_expression,
            value.span_kind_expression,
            value.span_name_expression,
            value.timestamp_value_expression,
            value.trace_id_expression,
        ) {
            (
                Some(connection),
                Some(default_table_select_expression),
                Some(duration_expression),
                Some(duration_precision),
                Some(from),
                Some(kind),
                Some(name),
                Some(parent_span_id_expression),
                Some(span_id_expression),
                Some(span_kind_expression),
                Some(span_name_expression),
                Some(timestamp_value_expression),
                Some(trace_id_expression),
            ) => Ok(Self {
                connection,
                default_table_select_expression,
                disabled: value.disabled,
                duration_expression,
                duration_precision,
                event_attributes_expression: value.event_attributes_expression,
                filter_settings: value.filter_settings.map(TryInto::try_into).transpose()?,
                from: from.try_into()?,
                highlighted_row_attribute_expressions: value
                    .highlighted_row_attribute_expressions
                    .map(|items| {
                        items
                            .into_iter()
                            .map(TryInto::try_into)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
                highlighted_trace_attribute_expressions: value
                    .highlighted_trace_attribute_expressions
                    .map(|items| {
                        items
                            .into_iter()
                            .map(TryInto::try_into)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
                id: value.id,
                implicit_column_expression: value.implicit_column_expression,
                kind,
                known_columns_list_expression: value.known_columns_list_expression,
                log_source_id: value.log_source_id,
                materialized_views: value
                    .materialized_views
                    .map(|items| {
                        items
                            .into_iter()
                            .map(TryInto::try_into)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
                metadata_materialized_views: value
                    .metadata_materialized_views
                    .map(TryInto::try_into)
                    .transpose()?,
                metric_source_id: value.metric_source_id,
                name,
                parent_span_id_expression,
                query_settings: value
                    .query_settings
                    .map(|items| {
                        items
                            .into_iter()
                            .map(TryInto::try_into)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?,
                resource_attributes_expression: value.resource_attributes_expression,
                section: value.section,
                service_name_expression: value.service_name_expression,
                session_source_id: value.session_source_id,
                span_events_value_expression: value.span_events_value_expression,
                span_id_expression,
                span_kind_expression,
                span_name_expression,
                status_code_expression: value.status_code_expression,
                status_message_expression: value.status_message_expression,
                timestamp_value_expression,
                trace_id_expression,
                use_text_index_for_implicit_column: value.use_text_index_for_implicit_column,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl From<PgBouncerConfigResponse> for PgBouncerConfig {
    fn from(_value: PgBouncerConfigResponse) -> Self {
        // The schema declares no properties, so the conversion is total.
        Self {}
    }
}

impl From<PgConfigResponse> for PgConfig {
    fn from(value: PgConfigResponse) -> Self {
        // Every `pgConfig` GUC is optional in both directions (omitting one
        // selects the server default), so the conversion is total.
        Self {
            autovacuum_analyze_scale_factor: value.autovacuum_analyze_scale_factor,
            autovacuum_max_workers: value.autovacuum_max_workers,
            autovacuum_naptime: value.autovacuum_naptime,
            autovacuum_vacuum_cost_delay: value.autovacuum_vacuum_cost_delay,
            autovacuum_vacuum_cost_limit: value.autovacuum_vacuum_cost_limit,
            autovacuum_vacuum_insert_scale_factor: value.autovacuum_vacuum_insert_scale_factor,
            autovacuum_vacuum_scale_factor: value.autovacuum_vacuum_scale_factor,
            autovacuum_work_mem: value.autovacuum_work_mem,
            default_transaction_isolation: value.default_transaction_isolation,
            effective_cache_size: value.effective_cache_size,
            effective_io_concurrency: value.effective_io_concurrency,
            idle_in_transaction_session_timeout: value.idle_in_transaction_session_timeout,
            idle_session_timeout: value.idle_session_timeout,
            lock_timeout: value.lock_timeout,
            maintenance_work_mem: value.maintenance_work_mem,
            max_connections: value.max_connections,
            max_parallel_maintenance_workers: value.max_parallel_maintenance_workers,
            max_parallel_workers: value.max_parallel_workers,
            max_parallel_workers_per_gather: value.max_parallel_workers_per_gather,
            max_slot_wal_keep_size: value.max_slot_wal_keep_size,
            max_wal_size: value.max_wal_size,
            max_worker_processes: value.max_worker_processes,
            min_wal_size: value.min_wal_size,
            random_page_cost: value.random_page_cost,
            ssl_min_protocol_version: value.ssl_min_protocol_version,
            statement_timeout: value.statement_timeout,
            transaction_timeout: value.transaction_timeout,
            wal_compression: value.wal_compression,
            wal_keep_size: value.wal_keep_size,
            wal_sender_timeout: value.wal_sender_timeout,
            work_mem: value.work_mem,
        }
    }
}

impl TryFrom<PostgresInstanceConfigResponse> for PostgresInstanceConfig {
    type Error = MissingRequiredFields;

    /// Turns a fetched configuration into a POST/PATCH body.
    ///
    /// The API requires both `pgConfig` and `pgBouncerConfig` in a write body
    /// (it rejects a body omitting either), so a response missing one cannot be
    /// written back verbatim and the caller has to supply it.
    fn try_from(value: PostgresInstanceConfigResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.pg_bouncer_config.is_none() {
            missing.push("pgBouncerConfig");
        }
        if value.pg_config.is_none() {
            missing.push("pgConfig");
        }
        match (value.pg_bouncer_config, value.pg_config) {
            (Some(pg_bouncer_config), Some(pg_config)) => Ok(Self {
                pg_bouncer_config: pg_bouncer_config.into(),
                pg_config: pg_config.into(),
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ResourceTagsV1Response> for ResourceTagsV1 {
    type Error = MissingRequiredFields;

    /// Turns a fetched tag into one that can be sent back.
    ///
    /// A tag is identified by its key, so a response tag without one cannot be
    /// written back — dropping it silently would delete the tag on the next
    /// write, and inventing an empty key would create a bogus one.
    fn try_from(value: ResourceTagsV1Response) -> Result<Self, Self::Error> {
        match value.key {
            Some(key) => Ok(Self {
                key,
                value: value.value,
            }),
            None => Err(MissingRequiredFields::new(vec!["key"])),
        }
    }
}

impl TryFrom<ScalingScheduleEntry> for ScalingScheduleEntryRequest {
    type Error = MissingRequiredFields;

    /// Turns a fetched schedule entry into one that can be re-sent.
    ///
    /// An upsert replaces the whole schedule, so a caller that reads a schedule
    /// and writes it back has to send every entry in full: the window bounds,
    /// weekdays and name the API requires cannot be defaulted away.
    fn try_from(value: ScalingScheduleEntry) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.end_hour_utc.is_none() {
            missing.push("endHourUtc");
        }
        if value.name.is_none() {
            missing.push("name");
        }
        if value.start_hour_utc.is_none() {
            missing.push("startHourUtc");
        }
        if value.weekdays.is_none() {
            missing.push("weekdays");
        }
        match (
            value.end_hour_utc,
            value.name,
            value.start_hour_utc,
            value.weekdays,
        ) {
            (Some(end_hour_utc), Some(name), Some(start_hour_utc), Some(weekdays)) => Ok(Self {
                autoscaling_mode: value.autoscaling_mode,
                end_hour_utc,
                idle_scaling: value.idle_scaling,
                idle_timeout_minutes: value.idle_timeout_minutes,
                max_replica_memory_gb: value.max_replica_memory_gb,
                max_replicas: value.max_replicas,
                min_replica_memory_gb: value.min_replica_memory_gb,
                min_replicas: value.min_replicas,
                name,
                // Not part of the response shape; a horizontal entry carries a
                // min/max band instead of a fixed replica count.
                num_replicas: None,
                start_hour_utc,
                weekdays,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<UpgradeWindow> for UpgradeWindowPutRequest {
    type Error = MissingRequiredFields;

    /// Turns a fetched upgrade window into one that can be re-sent.
    ///
    /// `duration` is response-only (the API derives it), so only the window's
    /// start hour and weekday cross over — and both are required.
    fn try_from(value: UpgradeWindow) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.start_hour_utc.is_none() {
            missing.push("startHourUtc");
        }
        if value.weekday.is_none() {
            missing.push("weekday");
        }
        match (value.start_hour_utc, value.weekday) {
            (Some(start_hour_utc), Some(weekday)) => Ok(Self {
                start_hour_utc,
                weekday,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}
