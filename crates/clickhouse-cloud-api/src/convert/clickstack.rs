use super::MissingRequiredFields;
use crate::models::*;

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
            (Some(label), Some(name)) => Ok(Self {
                label,
                name,
                allow_all: value.allow_all,
                value_expression: value.value_expression,
            }),
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
                service_version_expression: value.service_version_expression,
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
                service_version_expression: value.service_version_expression,
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

impl TryFrom<ClickStackNumberFormatResponse> for ClickStackNumberFormat {
    type Error = MissingRequiredFields;
    fn try_from(value: ClickStackNumberFormatResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.average.is_none() {
            missing.push("average");
        }
        if value.currency_symbol.is_none() {
            missing.push("currencySymbol");
        }
        if value.decimal_bytes.is_none() {
            missing.push("decimalBytes");
        }
        if value.factor.is_none() {
            missing.push("factor");
        }
        if value.mantissa.is_none() {
            missing.push("mantissa");
        }
        if value.numeric_unit.is_none() {
            missing.push("numericUnit");
        }
        if value.output.is_none() {
            missing.push("output");
        }
        if value.thousand_separated.is_none() {
            missing.push("thousandSeparated");
        }
        if value.unit.is_none() {
            missing.push("unit");
        }
        match (
            value.average,
            value.currency_symbol,
            value.decimal_bytes,
            value.factor,
            value.mantissa,
            value.numeric_unit,
            value.output,
            value.thousand_separated,
            value.unit,
        ) {
            (
                Some(average),
                Some(currency_symbol),
                Some(decimal_bytes),
                Some(factor),
                Some(mantissa),
                Some(numeric_unit),
                Some(output),
                Some(thousand_separated),
                Some(unit),
            ) => Ok(Self {
                average,
                currency_symbol,
                decimal_bytes,
                factor,
                mantissa,
                numeric_unit,
                output,
                thousand_separated,
                unit,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackFormulaResponse> for ClickStackFormula {
    type Error = MissingRequiredFields;
    fn try_from(value: ClickStackFormulaResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.expression.is_none() {
            missing.push("expression");
        }
        match (value.expression,) {
            (Some(expression),) => Ok(Self {
                expression,
                alias: value.alias,
                number_format: value.number_format.map(TryInto::try_into).transpose()?,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackSqlSavedFilterValueResponse> for ClickStackSqlSavedFilterValue {
    type Error = MissingRequiredFields;
    fn try_from(value: ClickStackSqlSavedFilterValueResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.condition.is_none() {
            missing.push("condition");
        }
        match (value.condition,) {
            (Some(condition),) => Ok(Self {
                condition,
                r#type: value.r#type,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackVariableSavedFilterValueResponse> for ClickStackVariableSavedFilterValue {
    type Error = MissingRequiredFields;
    fn try_from(value: ClickStackVariableSavedFilterValueResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.r#type.is_none() {
            missing.push("type");
        }
        if value.name.is_none() {
            missing.push("name");
        }
        if value.values.is_none() {
            missing.push("values");
        }
        match (value.r#type, value.name, value.values) {
            (Some(r#type), Some(name), Some(values)) => Ok(Self {
                r#type,
                name,
                values,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<ClickStackSavedFilterValueResponse> for ClickStackSavedFilterValue {
    type Error = MissingRequiredFields;
    fn try_from(value: ClickStackSavedFilterValueResponse) -> Result<Self, Self::Error> {
        Ok(match value {
            ClickStackSavedFilterValueResponse::ClickStackSqlSavedFilterValue(value) => {
                Self::ClickStackSqlSavedFilterValue(value.try_into()?)
            }
            ClickStackSavedFilterValueResponse::ClickStackVariableSavedFilterValue(value) => {
                Self::ClickStackVariableSavedFilterValue(value.try_into()?)
            }
            ClickStackSavedFilterValueResponse::Unknown(value) => Self::Unknown(value),
        })
    }
}
