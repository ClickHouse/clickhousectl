mod alerts;
mod dashboards;
mod enums;
mod roles;
mod saved_searches;
mod sources;
mod tiles;
mod webhooks;

pub use alerts::*;
pub use dashboards::*;
pub use enums::*;
pub use roles::*;
pub use saved_searches::*;
pub use sources::*;
pub use tiles::*;
pub use webhooks::*;

use serde::{Deserialize, Serialize};

/// `ClickStackNumberTileColorCondition` - one of multiple variants.
///
/// Dispatched on the `operator` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackNumberTileColorCondition {
    ClickStackNumericColorCondition(ClickStackNumericColorCondition),
    ClickStackBetweenColorCondition(ClickStackBetweenColorCondition),
    ClickStackEqualityColorCondition(ClickStackEqualityColorCondition),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackNumberTileColorCondition, "operator" {
        "gt" | "gte" | "lt" | "lte" => ClickStackNumericColorCondition,
        "between" => ClickStackBetweenColorCondition,
        "eq" | "neq" => ClickStackEqualityColorCondition,
    }
}

impl std::fmt::Display for ClickStackNumberTileColorCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackNumericColorCondition(_) => {
                write!(f, "ClickStackNumericColorCondition")
            }
            Self::ClickStackBetweenColorCondition(_) => {
                write!(f, "ClickStackBetweenColorCondition")
            }
            Self::ClickStackEqualityColorCondition(_) => {
                write!(f, "ClickStackEqualityColorCondition")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackNumberTileColorCondition` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackNumberTileColorCondition`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `operator` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `operator` this crate does not know, or a payload that
/// does not fit the variant its `operator` selects, lands in `Unknown` with the
/// raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `operator` and so would not deserialize back to the same
/// variant. Build a [`ClickStackNumberTileColorCondition`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackNumberTileColorConditionResponse {
    ClickStackNumericColorCondition(ClickStackNumericColorConditionResponse),
    ClickStackBetweenColorCondition(ClickStackBetweenColorConditionResponse),
    ClickStackEqualityColorCondition(ClickStackEqualityColorConditionResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackNumberTileColorConditionResponse, "operator" {
        "gt" | "gte" | "lt" | "lte" => ClickStackNumericColorCondition,
        "between" => ClickStackBetweenColorCondition,
        "eq" | "neq" => ClickStackEqualityColorCondition,
    }
}

impl std::fmt::Display for ClickStackNumberTileColorConditionResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackNumericColorCondition(_) => {
                write!(f, "ClickStackNumericColorCondition")
            }
            Self::ClickStackBetweenColorCondition(_) => {
                write!(f, "ClickStackBetweenColorCondition")
            }
            Self::ClickStackEqualityColorCondition(_) => {
                write!(f, "ClickStackEqualityColorCondition")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackBackgroundChart` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBackgroundChart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    pub r#type: ClickStackBackgroundChartType,
}

/// `ClickStackBackgroundChart` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackBackgroundChart`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBackgroundChartResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackBackgroundChartType>,
}

/// `ClickStackBetweenColorCondition` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBetweenColorCondition {
    pub color: ClickStackChartColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub operator: ClickStackBetweenColorConditionOperator,
    pub value: Vec<f64>,
}

/// `ClickStackBetweenColorCondition` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackBetweenColorCondition`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBetweenColorConditionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<ClickStackBetweenColorConditionOperator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Vec<f64>>,
}

/// `ClickStackEqualityColorCondition` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackEqualityColorCondition {
    pub color: ClickStackChartColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub operator: ClickStackEqualityColorConditionOperator,
    /// A finite number or a string; the spec models this as `oneOf number|string`.
    pub value: serde_json::Value,
}

/// `ClickStackEqualityColorCondition` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackEqualityColorCondition`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackEqualityColorConditionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<ClickStackEqualityColorConditionOperator>,
    /// A finite number or a string; the spec models this as `oneOf number|string`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

/// `ClickStackNumberFormat` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberFormat {
    pub average: bool,
    #[serde(rename = "currencySymbol")]
    pub currency_symbol: String,
    #[serde(rename = "decimalBytes")]
    pub decimal_bytes: bool,
    pub factor: f64,
    pub mantissa: i64,
    #[serde(rename = "numericUnit")]
    pub numeric_unit: ClickStackNumberFormatNumericunit,
    pub output: ClickStackNumberFormatOutput,
    #[serde(rename = "thousandSeparated")]
    pub thousand_separated: bool,
    pub unit: String,
}

/// `ClickStackNumberFormat` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackNumberFormat`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberFormatResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average: Option<bool>,
    #[serde(rename = "currencySymbol", skip_serializing_if = "Option::is_none")]
    pub currency_symbol: Option<String>,
    #[serde(rename = "decimalBytes", skip_serializing_if = "Option::is_none")]
    pub decimal_bytes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mantissa: Option<i64>,
    #[serde(rename = "numericUnit", skip_serializing_if = "Option::is_none")]
    pub numeric_unit: Option<ClickStackNumberFormatNumericunit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<ClickStackNumberFormatOutput>,
    #[serde(rename = "thousandSeparated", skip_serializing_if = "Option::is_none")]
    pub thousand_separated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// `ClickStackNumericColorCondition` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumericColorCondition {
    pub color: ClickStackChartColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub operator: ClickStackNumericColorConditionOperator,
    pub value: f64,
}

/// `ClickStackNumericColorCondition` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackNumericColorCondition`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumericColorConditionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<ClickStackNumericColorConditionOperator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

/// `ClickStackSelectItem` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSelectItem {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackSelectItemAggfn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<ClickStackSelectItemLevel>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(rename = "metricType", skip_serializing_if = "Option::is_none")]
    pub metric_type: Option<ClickStackSelectItemMetrictype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "periodAggFn", skip_serializing_if = "Option::is_none")]
    pub period_agg_fn: Option<ClickStackSelectItemPeriodaggfn>,
    #[serde(rename = "valueExpression", skip_serializing_if = "Option::is_none")]
    pub value_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackSelectItemWherelanguage>,
}

/// `ClickStackSelectItem` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSelectItem`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSelectItemResponse {
    #[serde(rename = "aggFn", skip_serializing_if = "Option::is_none")]
    pub agg_fn: Option<ClickStackSelectItemAggfn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<ClickStackSelectItemLevel>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(rename = "metricType", skip_serializing_if = "Option::is_none")]
    pub metric_type: Option<ClickStackSelectItemMetrictype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "periodAggFn", skip_serializing_if = "Option::is_none")]
    pub period_agg_fn: Option<ClickStackSelectItemPeriodaggfn>,
    #[serde(rename = "valueExpression", skip_serializing_if = "Option::is_none")]
    pub value_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackSelectItemWherelanguage>,
}

/// `ClickStackFormula` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFormula {
    #[serde(rename = "expression")]
    pub expression: String,
    #[serde(rename = "alias", skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
}

/// `ClickStackFormulaResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFormulaResponse {
    #[serde(rename = "expression", skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(rename = "alias", skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
}
