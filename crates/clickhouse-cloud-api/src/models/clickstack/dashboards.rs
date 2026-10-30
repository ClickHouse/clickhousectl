use super::enums::*;
use super::{ClickStackTileInput, ClickStackTileOutput};
use serde::{Deserialize, Serialize};

/// `ClickStackFilter` - one of multiple variants.
///
/// A dashboard filter as sent in an update request, which carries its `id`.
/// Dispatched on the `type` field, which each variant requires with its own
/// single wire value; see the `discriminated_union!` invocation below.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackFilter {
    ClickStackQueryExpressionFilter(ClickStackQueryExpressionFilter),
    ClickStackStaticListFilter(ClickStackStaticListFilter),
    ClickStackPrometheusLabelFilter(ClickStackPrometheusLabelFilter),
    ClickStackSourceFilter(ClickStackSourceFilter),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackFilter, "type" {
        "QUERY_EXPRESSION" => ClickStackQueryExpressionFilter,
        "STATIC_LIST" => ClickStackStaticListFilter,
        "PROMETHEUS_LABEL" => ClickStackPrometheusLabelFilter,
        "SOURCE_FILTER" => ClickStackSourceFilter,
    }
}

impl std::fmt::Display for ClickStackFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackQueryExpressionFilter(_) => {
                write!(f, "ClickStackQueryExpressionFilter")
            }
            Self::ClickStackStaticListFilter(_) => write!(f, "ClickStackStaticListFilter"),
            Self::ClickStackPrometheusLabelFilter(_) => {
                write!(f, "ClickStackPrometheusLabelFilter")
            }
            Self::ClickStackSourceFilter(_) => write!(f, "ClickStackSourceFilter"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackFilter` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackFilter`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `type` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so the
/// all-`Option` arms — which would match any object under `untagged` matching —
/// cannot misroute a payload. A `type` this crate does not know, a payload
/// without one, or a payload that does not fit the variant its `type` selects,
/// lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `type` and so would not deserialize back to the same
/// variant. Build a [`ClickStackFilter`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackFilterResponse {
    ClickStackQueryExpressionFilter(ClickStackQueryExpressionFilterResponse),
    ClickStackStaticListFilter(ClickStackStaticListFilterResponse),
    ClickStackPrometheusLabelFilter(ClickStackPrometheusLabelFilterResponse),
    ClickStackSourceFilter(ClickStackSourceFilterResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackFilterResponse, "type" {
        "QUERY_EXPRESSION" => ClickStackQueryExpressionFilter,
        "STATIC_LIST" => ClickStackStaticListFilter,
        "PROMETHEUS_LABEL" => ClickStackPrometheusLabelFilter,
        "SOURCE_FILTER" => ClickStackSourceFilter,
    }
}

impl std::fmt::Display for ClickStackFilterResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackQueryExpressionFilter(_) => {
                write!(f, "ClickStackQueryExpressionFilter")
            }
            Self::ClickStackStaticListFilter(_) => write!(f, "ClickStackStaticListFilter"),
            Self::ClickStackPrometheusLabelFilter(_) => {
                write!(f, "ClickStackPrometheusLabelFilter")
            }
            Self::ClickStackSourceFilter(_) => write!(f, "ClickStackSourceFilter"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackFilterInput` - one of multiple variants.
///
/// A dashboard filter as sent in a create request, without an `id`.
/// Dispatched on the `type` field, which each variant requires with its own
/// single wire value; see the `discriminated_union!` invocation below.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackFilterInput {
    ClickStackQueryExpressionFilterInput(ClickStackQueryExpressionFilterInput),
    ClickStackStaticListFilterInput(ClickStackStaticListFilterInput),
    ClickStackPrometheusLabelFilterInput(ClickStackPrometheusLabelFilterInput),
    ClickStackSourceFilterInput(ClickStackSourceFilterInput),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackFilterInput, "type" {
        "QUERY_EXPRESSION" => ClickStackQueryExpressionFilterInput,
        "STATIC_LIST" => ClickStackStaticListFilterInput,
        "PROMETHEUS_LABEL" => ClickStackPrometheusLabelFilterInput,
        "SOURCE_FILTER" => ClickStackSourceFilterInput,
    }
}

impl std::fmt::Display for ClickStackFilterInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackQueryExpressionFilterInput(_) => {
                write!(f, "ClickStackQueryExpressionFilterInput")
            }
            Self::ClickStackStaticListFilterInput(_) => {
                write!(f, "ClickStackStaticListFilterInput")
            }
            Self::ClickStackPrometheusLabelFilterInput(_) => {
                write!(f, "ClickStackPrometheusLabelFilterInput")
            }
            Self::ClickStackSourceFilterInput(_) => write!(f, "ClickStackSourceFilterInput"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Type alias for `ClickStackValidateDashboardResponseNormalized`.
pub type ClickStackValidateDashboardResponseNormalized = serde_json::Value;

/// `ClickStackCreateDashboardRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateDashboardRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers: Option<Vec<ClickStackDashboardContainer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackFilterInput>>,
    pub name: String,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none")]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValue>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none")]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none")]
    pub saved_query_language: Option<ClickStackCreateDashboardRequestSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub tiles: Vec<ClickStackTileInput>,
}

/// `ClickStackDashboardContainer` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardContainer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bordered: Option<bool>,
    pub collapsed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsible: Option<bool>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabs: Option<Vec<ClickStackDashboardContainerTab>>,
    pub title: String,
}

/// `ClickStackDashboardContainer` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackDashboardContainer`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardContainerResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bordered: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabs: Option<Vec<ClickStackDashboardContainerTabResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// `ClickStackDashboardContainerTab` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardContainerTab {
    pub id: String,
    pub title: String,
}

/// `ClickStackDashboardContainerTab` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackDashboardContainerTab`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardContainerTabResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// `ClickStackDashboardResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers: Option<Vec<ClickStackDashboardContainerResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackFilterResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none")]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValueResponse>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none")]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none")]
    pub saved_query_language: Option<ClickStackDashboardResponseSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiles: Option<Vec<ClickStackTileOutput>>,
}

/// `ClickStackPrometheusLabelFilterInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPrometheusLabelFilterInput {
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    pub label: String,
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub r#match: Option<String>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "type")]
    pub r#type: ClickStackPrometheusLabelFilterInputType,
    #[serde(rename = "variableName", skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
}

/// `ClickStackPrometheusLabelFilter` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackPrometheusLabelFilterInput`]
/// and a required `id`, so it carries every field of the input type plus `id`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPrometheusLabelFilter {
    pub id: String,
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    pub label: String,
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub r#match: Option<String>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "type")]
    pub r#type: ClickStackPrometheusLabelFilterInputType,
    #[serde(rename = "variableName", skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
}

/// `ClickStackPrometheusLabelFilter` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackPrometheusLabelFilter`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPrometheusLabelFilterResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "match", skip_serializing_if = "Option::is_none")]
    pub r#match: Option<String>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackPrometheusLabelFilterInputType>,
    #[serde(rename = "variableName", skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
}

/// `ClickStackQueryExpressionFilterInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackQueryExpressionFilterInput {
    #[serde(rename = "appliesToSourceIds", skip_serializing_if = "Option::is_none")]
    pub applies_to_source_ids: Option<Vec<String>>,
    pub expression: String,
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "sourceMetricType", skip_serializing_if = "Option::is_none")]
    pub source_metric_type: Option<ClickStackQueryExpressionFilterInputSourcemetrictype>,
    #[serde(rename = "type")]
    pub r#type: ClickStackQueryExpressionFilterInputType,
    #[serde(rename = "variableName", skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackQueryExpressionFilterInputWherelanguage>,
}

/// `ClickStackQueryExpressionFilter` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackQueryExpressionFilterInput`]
/// and a required `id`, so it carries every field of the input type plus `id`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackQueryExpressionFilter {
    #[serde(rename = "appliesToSourceIds", skip_serializing_if = "Option::is_none")]
    pub applies_to_source_ids: Option<Vec<String>>,
    pub expression: String,
    pub id: String,
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "sourceMetricType", skip_serializing_if = "Option::is_none")]
    pub source_metric_type: Option<ClickStackQueryExpressionFilterInputSourcemetrictype>,
    #[serde(rename = "type")]
    pub r#type: ClickStackQueryExpressionFilterInputType,
    #[serde(rename = "variableName", skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackQueryExpressionFilterInputWherelanguage>,
}

/// `ClickStackQueryExpressionFilter` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackQueryExpressionFilter`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackQueryExpressionFilterResponse {
    #[serde(rename = "appliesToSourceIds", skip_serializing_if = "Option::is_none")]
    pub applies_to_source_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sourceMetricType", skip_serializing_if = "Option::is_none")]
    pub source_metric_type: Option<ClickStackQueryExpressionFilterInputSourcemetrictype>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackQueryExpressionFilterInputType>,
    #[serde(rename = "variableName", skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackQueryExpressionFilterInputWherelanguage>,
}

/// `ClickStackSavedFilterValue` from the ClickHouse Cloud API, selected by `type`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackSavedFilterValue {
    ClickStackSqlSavedFilterValue(ClickStackSqlSavedFilterValue),
    ClickStackVariableSavedFilterValue(ClickStackVariableSavedFilterValue),
    Unknown(serde_json::Value),
}
discriminated_union! {
    ClickStackSavedFilterValue, "type" {
        "sql" => ClickStackSqlSavedFilterValue,
        "variable" => ClickStackVariableSavedFilterValue,
        none unless "name" | "values" => ClickStackSqlSavedFilterValue,
    }
}
impl Default for ClickStackSavedFilterValue {
    fn default() -> Self {
        Self::ClickStackSqlSavedFilterValue(ClickStackSqlSavedFilterValue::default())
    }
}
impl std::fmt::Display for ClickStackSavedFilterValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackSqlSavedFilterValue(_) => write!(f, "ClickStackSqlSavedFilterValue"),
            Self::ClickStackVariableSavedFilterValue(_) => {
                write!(f, "ClickStackVariableSavedFilterValue")
            }
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickStackSavedFilterValueResponse` from the ClickHouse Cloud API, selected by `type`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackSavedFilterValueResponse {
    ClickStackSqlSavedFilterValue(ClickStackSqlSavedFilterValueResponse),
    ClickStackVariableSavedFilterValue(ClickStackVariableSavedFilterValueResponse),
    Unknown(serde_json::Value),
}
discriminated_union! {
    ClickStackSavedFilterValueResponse, "type" {
        "sql" => ClickStackSqlSavedFilterValue,
        "variable" => ClickStackVariableSavedFilterValue,
        none unless "name" | "values" => ClickStackSqlSavedFilterValue,
    }
}
impl Default for ClickStackSavedFilterValueResponse {
    fn default() -> Self {
        Self::ClickStackSqlSavedFilterValue(ClickStackSqlSavedFilterValueResponse::default())
    }
}
impl std::fmt::Display for ClickStackSavedFilterValueResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackSqlSavedFilterValue(_) => write!(f, "ClickStackSqlSavedFilterValue"),
            Self::ClickStackVariableSavedFilterValue(_) => {
                write!(f, "ClickStackVariableSavedFilterValue")
            }
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickStackSourceFilterInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFilterInput {
    #[serde(rename = "appliesToSourceIds", skip_serializing_if = "Option::is_none")]
    pub applies_to_source_ids: Option<Vec<String>>,
    pub expression: String,
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "type")]
    pub r#type: ClickStackSourceFilterInputType,
    #[serde(rename = "variableName")]
    pub variable_name: String,
}

/// `ClickStackSourceFilter` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackSourceFilterInput`]
/// and a required `id`, so it carries every field of the input type plus `id`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFilter {
    #[serde(rename = "appliesToSourceIds", skip_serializing_if = "Option::is_none")]
    pub applies_to_source_ids: Option<Vec<String>>,
    pub expression: String,
    pub id: String,
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "type")]
    pub r#type: ClickStackSourceFilterInputType,
    #[serde(rename = "variableName")]
    pub variable_name: String,
}

/// `ClickStackSourceFilter` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSourceFilter`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFilterResponse {
    #[serde(rename = "appliesToSourceIds", skip_serializing_if = "Option::is_none")]
    pub applies_to_source_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackSourceFilterInputType>,
    #[serde(rename = "variableName", skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
}

/// `ClickStackStaticListFilterInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackStaticListFilterInput {
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    pub name: String,
    pub options: Vec<String>,
    #[serde(rename = "type")]
    pub r#type: ClickStackStaticListFilterInputType,
    #[serde(rename = "variableName", skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
}

/// `ClickStackStaticListFilter` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackStaticListFilterInput`]
/// and a required `id`, so it carries every field of the input type plus `id`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackStaticListFilter {
    pub id: String,
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    pub name: String,
    pub options: Vec<String>,
    #[serde(rename = "type")]
    pub r#type: ClickStackStaticListFilterInputType,
    #[serde(rename = "variableName", skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
}

/// `ClickStackStaticListFilter` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackStaticListFilter`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackStaticListFilterResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "isBroadcastEnabled", skip_serializing_if = "Option::is_none")]
    pub is_broadcast_enabled: Option<bool>,
    #[serde(
        rename = "isGlobalRequirement",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_global_requirement: Option<bool>,
    #[serde(rename = "isVariableEnabled", skip_serializing_if = "Option::is_none")]
    pub is_variable_enabled: Option<bool>,
    #[serde(rename = "maxSelections", skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<ClickStackFilterMaxSelections>,
    #[serde(rename = "minSelections", skip_serializing_if = "Option::is_none")]
    pub min_selections: Option<ClickStackFilterMinSelections>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackStaticListFilterInputType>,
    #[serde(rename = "variableName", skip_serializing_if = "Option::is_none")]
    pub variable_name: Option<String>,
}

/// `ClickStackUpdateDashboardRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateDashboardRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers: Option<Vec<ClickStackDashboardContainer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackFilter>>,
    pub name: String,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none")]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValue>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none")]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none")]
    pub saved_query_language: Option<ClickStackUpdateDashboardRequestSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub tiles: Vec<ClickStackTileInput>,
}

/// `ClickStackValidateDashboardError` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackValidateDashboardError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// `ClickStackValidateDashboardResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackValidateDashboardResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ClickStackValidateDashboardError>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized: Option<ClickStackValidateDashboardResponseNormalized>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid: Option<bool>,
}

impl Default for ClickStackFilter {
    fn default() -> Self {
        Self::ClickStackQueryExpressionFilter(ClickStackQueryExpressionFilter::default())
    }
}

impl Default for ClickStackFilterInput {
    fn default() -> Self {
        Self::ClickStackQueryExpressionFilterInput(ClickStackQueryExpressionFilterInput::default())
    }
}

/// `ClickStackSqlSavedFilterValue` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSqlSavedFilterValue {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackSavedFilterValueType>,
    #[serde(rename = "condition")]
    pub condition: String,
}

/// `ClickStackSqlSavedFilterValueResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSqlSavedFilterValueResponse {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackSavedFilterValueType>,
    #[serde(rename = "condition", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// `ClickStackVariableSavedFilterValue` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackVariableSavedFilterValue {
    #[serde(rename = "type")]
    pub r#type: ClickStackVariableSavedFilterValueType,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "values")]
    pub values: Vec<String>,
    #[serde(rename = "all", skip_serializing_if = "Option::is_none")]
    pub all: Option<bool>,
}

/// `ClickStackVariableSavedFilterValueResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackVariableSavedFilterValueResponse {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackVariableSavedFilterValueType>,
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "values", skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(rename = "all", skip_serializing_if = "Option::is_none")]
    pub all: Option<bool>,
}
