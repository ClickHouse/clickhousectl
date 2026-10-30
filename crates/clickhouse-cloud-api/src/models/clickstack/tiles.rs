use super::enums::*;
use super::{
    ClickStackBackgroundChart, ClickStackBackgroundChartResponse, ClickStackFormula,
    ClickStackFormulaResponse, ClickStackNumberFormat, ClickStackNumberFormatResponse,
    ClickStackNumberTileColorCondition, ClickStackNumberTileColorConditionResponse,
    ClickStackSelectItem, ClickStackSelectItemResponse,
};
use serde::{Deserialize, Serialize};

/// `ClickStackBarChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackBarChartConfig {
    ClickStackBarBuilderChartConfig(ClickStackBarBuilderChartConfig),
    ClickStackBarRawSqlChartConfig(ClickStackBarRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackBarChartConfig, "configType" {
        "sql" => ClickStackBarRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackBarBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackBarChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackBarBuilderChartConfig(_) => {
                write!(f, "ClickStackBarBuilderChartConfig")
            }
            Self::ClickStackBarRawSqlChartConfig(_) => write!(f, "ClickStackBarRawSqlChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackBarChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackBarChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackBarChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackBarChartConfigResponse {
    ClickStackBarRawSqlChartConfig(ClickStackBarRawSqlChartConfigResponse),
    ClickStackBarBuilderChartConfig(ClickStackBarBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackBarChartConfigResponse, "configType" {
        "sql" => ClickStackBarRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackBarBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackBarChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackBarRawSqlChartConfig(_) => write!(f, "ClickStackBarRawSqlChartConfig"),
            Self::ClickStackBarBuilderChartConfig(_) => {
                write!(f, "ClickStackBarBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackCategoricalBarChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackCategoricalBarChartConfig {
    ClickStackCategoricalBarBuilderChartConfig(ClickStackCategoricalBarBuilderChartConfig),
    ClickStackCategoricalBarRawSqlChartConfig(ClickStackCategoricalBarRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackCategoricalBarChartConfig, "configType" {
        "sql" => ClickStackCategoricalBarRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackCategoricalBarBuilderChartConfig,
    }
}

impl Default for ClickStackCategoricalBarChartConfig {
    fn default() -> Self {
        Self::ClickStackCategoricalBarBuilderChartConfig(
            ClickStackCategoricalBarBuilderChartConfig::default(),
        )
    }
}

impl std::fmt::Display for ClickStackCategoricalBarChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackCategoricalBarBuilderChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarBuilderChartConfig")
            }
            Self::ClickStackCategoricalBarRawSqlChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackCategoricalBarChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackCategoricalBarChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackCategoricalBarChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackCategoricalBarChartConfigResponse {
    ClickStackCategoricalBarRawSqlChartConfig(ClickStackCategoricalBarRawSqlChartConfigResponse),
    ClickStackCategoricalBarBuilderChartConfig(ClickStackCategoricalBarBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackCategoricalBarChartConfigResponse, "configType" {
        "sql" => ClickStackCategoricalBarRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackCategoricalBarBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackCategoricalBarChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackCategoricalBarRawSqlChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarRawSqlChartConfig")
            }
            Self::ClickStackCategoricalBarBuilderChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackDashboardChartSeries` - one of multiple variants.
///
/// Dispatched on the `type` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackDashboardChartSeries {
    ClickStackTimeChartSeries(ClickStackTimeChartSeries),
    ClickStackTableChartSeries(ClickStackTableChartSeries),
    ClickStackNumberChartSeries(ClickStackNumberChartSeries),
    ClickStackSearchChartSeries(ClickStackSearchChartSeries),
    ClickStackMarkdownChartSeries(ClickStackMarkdownChartSeries),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackDashboardChartSeries, "type" {
        "time" => ClickStackTimeChartSeries,
        "table" => ClickStackTableChartSeries,
        "number" => ClickStackNumberChartSeries,
        "search" => ClickStackSearchChartSeries,
        "markdown" => ClickStackMarkdownChartSeries,
    }
}

impl std::fmt::Display for ClickStackDashboardChartSeries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackTimeChartSeries(_) => write!(f, "ClickStackTimeChartSeries"),
            Self::ClickStackTableChartSeries(_) => write!(f, "ClickStackTableChartSeries"),
            Self::ClickStackNumberChartSeries(_) => write!(f, "ClickStackNumberChartSeries"),
            Self::ClickStackSearchChartSeries(_) => write!(f, "ClickStackSearchChartSeries"),
            Self::ClickStackMarkdownChartSeries(_) => write!(f, "ClickStackMarkdownChartSeries"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackHeatmapChartConfig` - one of multiple variants.
///
/// Dispatched on the `heatmapMode` field. The spec makes it optional on the
/// distribution variant (its default) and required on the series variant, so an
/// absent or non-string `heatmapMode` dispatches to the distribution variant,
/// unless the payload carries the series-only `groupBy` key; see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackHeatmapChartConfig {
    ClickStackHeatmapDistributionChartConfig(ClickStackHeatmapDistributionChartConfig),
    ClickStackHeatmapSeriesChartConfig(ClickStackHeatmapSeriesChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackHeatmapChartConfig, "heatmapMode" {
        "distribution" => ClickStackHeatmapDistributionChartConfig,
        "series" => ClickStackHeatmapSeriesChartConfig,
        none unless "groupBy" => ClickStackHeatmapDistributionChartConfig,
    }
}

impl std::fmt::Display for ClickStackHeatmapChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackHeatmapDistributionChartConfig(_) => {
                write!(f, "ClickStackHeatmapDistributionChartConfig")
            }
            Self::ClickStackHeatmapSeriesChartConfig(_) => {
                write!(f, "ClickStackHeatmapSeriesChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackHeatmapChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackHeatmapChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `heatmapMode` field exactly as the request union is (absent
/// or non-string dispatches to the distribution variant, unless the payload
/// carries the series-only `groupBy` key): dispatch reads the raw JSON rather
/// than trying each variant's shape, so the all-`Option` arms cannot misroute a
/// payload, and the `unless` guard keeps a series payload with a dropped
/// `heatmapMode` out of the distribution arm. A `heatmapMode` this crate does
/// not know, or a payload that does not fit the variant its `heatmapMode`
/// selects, lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackHeatmapChartConfig`]
/// instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackHeatmapChartConfigResponse {
    ClickStackHeatmapDistributionChartConfig(ClickStackHeatmapDistributionChartConfigResponse),
    ClickStackHeatmapSeriesChartConfig(ClickStackHeatmapSeriesChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackHeatmapChartConfigResponse, "heatmapMode" {
        "distribution" => ClickStackHeatmapDistributionChartConfig,
        "series" => ClickStackHeatmapSeriesChartConfig,
        none unless "groupBy" => ClickStackHeatmapDistributionChartConfig,
    }
}

impl std::fmt::Display for ClickStackHeatmapChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackHeatmapDistributionChartConfig(_) => {
                write!(f, "ClickStackHeatmapDistributionChartConfig")
            }
            Self::ClickStackHeatmapSeriesChartConfig(_) => {
                write!(f, "ClickStackHeatmapSeriesChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackLineChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackLineChartConfig {
    ClickStackLineBuilderChartConfig(ClickStackLineBuilderChartConfig),
    ClickStackLineRawSqlChartConfig(ClickStackLineRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackLineChartConfig, "configType" {
        "sql" => ClickStackLineRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackLineBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackLineChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLineBuilderChartConfig(_) => {
                write!(f, "ClickStackLineBuilderChartConfig")
            }
            Self::ClickStackLineRawSqlChartConfig(_) => {
                write!(f, "ClickStackLineRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackLineChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackLineChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackLineChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackLineChartConfigResponse {
    ClickStackLineRawSqlChartConfig(ClickStackLineRawSqlChartConfigResponse),
    ClickStackLineBuilderChartConfig(ClickStackLineBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackLineChartConfigResponse, "configType" {
        "sql" => ClickStackLineRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackLineBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackLineChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLineRawSqlChartConfig(_) => {
                write!(f, "ClickStackLineRawSqlChartConfig")
            }
            Self::ClickStackLineBuilderChartConfig(_) => {
                write!(f, "ClickStackLineBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackNumberChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackNumberChartConfig {
    ClickStackNumberBuilderChartConfig(ClickStackNumberBuilderChartConfig),
    ClickStackNumberRawSqlChartConfig(ClickStackNumberRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackNumberChartConfig, "configType" {
        "sql" => ClickStackNumberRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackNumberBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackNumberChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackNumberBuilderChartConfig(_) => {
                write!(f, "ClickStackNumberBuilderChartConfig")
            }
            Self::ClickStackNumberRawSqlChartConfig(_) => {
                write!(f, "ClickStackNumberRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackNumberChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackNumberChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackNumberChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackNumberChartConfigResponse {
    ClickStackNumberRawSqlChartConfig(ClickStackNumberRawSqlChartConfigResponse),
    ClickStackNumberBuilderChartConfig(ClickStackNumberBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackNumberChartConfigResponse, "configType" {
        "sql" => ClickStackNumberRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackNumberBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackNumberChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackNumberRawSqlChartConfig(_) => {
                write!(f, "ClickStackNumberRawSqlChartConfig")
            }
            Self::ClickStackNumberBuilderChartConfig(_) => {
                write!(f, "ClickStackNumberBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackOnClick` - one of multiple variants.
///
/// Dispatched on the `type` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackOnClick {
    ClickStackOnClickSearch(ClickStackOnClickSearch),
    ClickStackOnClickDashboard(ClickStackOnClickDashboard),
    ClickStackOnClickExternal(ClickStackOnClickExternal),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackOnClick, "type" {
        "search" => ClickStackOnClickSearch,
        "dashboard" => ClickStackOnClickDashboard,
        "external" => ClickStackOnClickExternal,
    }
}

impl Default for ClickStackOnClick {
    fn default() -> Self {
        Self::Unknown(serde_json::Value::Null)
    }
}

impl std::fmt::Display for ClickStackOnClick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackOnClickSearch(_) => write!(f, "ClickStackOnClickSearch"),
            Self::ClickStackOnClickDashboard(_) => write!(f, "ClickStackOnClickDashboard"),
            Self::ClickStackOnClickExternal(_) => write!(f, "ClickStackOnClickExternal"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackOnClick` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackOnClick`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `type` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `type` this crate does not know, or a payload that
/// does not fit the variant its `type` selects, lands in `Unknown` with the
/// raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `type` and so would not deserialize back to the same
/// variant. Build a [`ClickStackOnClick`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackOnClickResponse {
    ClickStackOnClickSearch(ClickStackOnClickSearchResponse),
    ClickStackOnClickDashboard(ClickStackOnClickDashboardResponse),
    ClickStackOnClickExternal(ClickStackOnClickExternalResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackOnClickResponse, "type" {
        "search" => ClickStackOnClickSearch,
        "dashboard" => ClickStackOnClickDashboard,
        "external" => ClickStackOnClickExternal,
    }
}

impl std::fmt::Display for ClickStackOnClickResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackOnClickSearch(_) => write!(f, "ClickStackOnClickSearch"),
            Self::ClickStackOnClickDashboard(_) => write!(f, "ClickStackOnClickDashboard"),
            Self::ClickStackOnClickExternal(_) => write!(f, "ClickStackOnClickExternal"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackOnClickTarget` - one of multiple variants.
///
/// Dispatched on the `mode` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackOnClickTarget {
    ClickStackOnClickTargetIdVariant(ClickStackOnClickTargetIdVariant),
    ClickStackOnClickTargetTemplateVariant(ClickStackOnClickTargetTemplateVariant),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackOnClickTarget, "mode" {
        "id" => ClickStackOnClickTargetIdVariant,
        "template" => ClickStackOnClickTargetTemplateVariant,
    }
}

impl Default for ClickStackOnClickTarget {
    fn default() -> Self {
        Self::Unknown(serde_json::Value::Null)
    }
}

impl std::fmt::Display for ClickStackOnClickTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackOnClickTargetIdVariant(_) => {
                write!(f, "ClickStackOnClickTargetIdVariant")
            }
            Self::ClickStackOnClickTargetTemplateVariant(_) => {
                write!(f, "ClickStackOnClickTargetTemplateVariant")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackOnClickTarget` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackOnClickTarget`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `mode` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `mode` this crate does not know, or a payload that
/// does not fit the variant its `mode` selects, lands in `Unknown` with the
/// raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `mode` and so would not deserialize back to the same
/// variant. Build a [`ClickStackOnClickTarget`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackOnClickTargetResponse {
    ClickStackOnClickTargetIdVariant(ClickStackOnClickTargetIdVariantResponse),
    ClickStackOnClickTargetTemplateVariant(ClickStackOnClickTargetTemplateVariantResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackOnClickTargetResponse, "mode" {
        "id" => ClickStackOnClickTargetIdVariant,
        "template" => ClickStackOnClickTargetTemplateVariant,
    }
}

impl std::fmt::Display for ClickStackOnClickTargetResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackOnClickTargetIdVariant(_) => {
                write!(f, "ClickStackOnClickTargetIdVariant")
            }
            Self::ClickStackOnClickTargetTemplateVariant(_) => {
                write!(f, "ClickStackOnClickTargetTemplateVariant")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackPieChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackPieChartConfig {
    ClickStackPieBuilderChartConfig(ClickStackPieBuilderChartConfig),
    ClickStackPieRawSqlChartConfig(ClickStackPieRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackPieChartConfig, "configType" {
        "sql" => ClickStackPieRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackPieBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackPieChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackPieBuilderChartConfig(_) => {
                write!(f, "ClickStackPieBuilderChartConfig")
            }
            Self::ClickStackPieRawSqlChartConfig(_) => write!(f, "ClickStackPieRawSqlChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackPieChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackPieChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackPieChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackPieChartConfigResponse {
    ClickStackPieRawSqlChartConfig(ClickStackPieRawSqlChartConfigResponse),
    ClickStackPieBuilderChartConfig(ClickStackPieBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackPieChartConfigResponse, "configType" {
        "sql" => ClickStackPieRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackPieBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackPieChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackPieRawSqlChartConfig(_) => write!(f, "ClickStackPieRawSqlChartConfig"),
            Self::ClickStackPieBuilderChartConfig(_) => {
                write!(f, "ClickStackPieBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackStackedLineChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackStackedLineChartConfig {
    ClickStackStackedLineBuilderChartConfig(ClickStackStackedLineBuilderChartConfig),
    ClickStackStackedLineRawSqlChartConfig(ClickStackStackedLineRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackStackedLineChartConfig, "configType" {
        "sql" => ClickStackStackedLineRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackStackedLineBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackStackedLineChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackStackedLineBuilderChartConfig(_) => {
                write!(f, "ClickStackStackedLineBuilderChartConfig")
            }
            Self::ClickStackStackedLineRawSqlChartConfig(_) => {
                write!(f, "ClickStackStackedLineRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackStackedLineChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackStackedLineChartConfig`]: each arm is the
/// all-`Option` response variant of its request type, so a field the API drops
/// or sends as `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms cannot misroute a payload, and the
/// `unless` guard keeps a raw-SQL payload with a dropped discriminator out of
/// the total builder arm. A payload that does not fit the variant its
/// discriminator selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a
/// [`ClickStackStackedLineChartConfig`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackStackedLineChartConfigResponse {
    ClickStackStackedLineBuilderChartConfig(ClickStackStackedLineBuilderChartConfigResponse),
    ClickStackStackedLineRawSqlChartConfig(ClickStackStackedLineRawSqlChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackStackedLineChartConfigResponse, "configType" {
        "sql" => ClickStackStackedLineRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackStackedLineBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackStackedLineChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackStackedLineBuilderChartConfig(_) => {
                write!(f, "ClickStackStackedLineBuilderChartConfig")
            }
            Self::ClickStackStackedLineRawSqlChartConfig(_) => {
                write!(f, "ClickStackStackedLineRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackTableChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackTableChartConfig {
    ClickStackTableBuilderChartConfig(ClickStackTableBuilderChartConfig),
    ClickStackTableRawSqlChartConfig(ClickStackTableRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackTableChartConfig, "configType" {
        "sql" => ClickStackTableRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackTableBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackTableChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackTableBuilderChartConfig(_) => {
                write!(f, "ClickStackTableBuilderChartConfig")
            }
            Self::ClickStackTableRawSqlChartConfig(_) => {
                write!(f, "ClickStackTableRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackTableChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackTableChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackTableChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackTableChartConfigResponse {
    ClickStackTableRawSqlChartConfig(ClickStackTableRawSqlChartConfigResponse),
    ClickStackTableBuilderChartConfig(ClickStackTableBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackTableChartConfigResponse, "configType" {
        "sql" => ClickStackTableRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackTableBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackTableChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackTableRawSqlChartConfig(_) => {
                write!(f, "ClickStackTableRawSqlChartConfig")
            }
            Self::ClickStackTableBuilderChartConfig(_) => {
                write!(f, "ClickStackTableBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackTileConfig` - one of multiple variants.
///
/// Dispatched on the `displayType` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackTileConfig {
    ClickStackCategoricalBarChartConfig(ClickStackCategoricalBarChartConfig),
    ClickStackLineChartConfig(ClickStackLineChartConfig),
    ClickStackBarChartConfig(ClickStackBarChartConfig),
    ClickStackStackedLineChartConfig(ClickStackStackedLineChartConfig),
    ClickStackTableChartConfig(ClickStackTableChartConfig),
    ClickStackNumberChartConfig(ClickStackNumberChartConfig),
    ClickStackPieChartConfig(ClickStackPieChartConfig),
    ClickStackHeatmapChartConfig(ClickStackHeatmapChartConfig),
    ClickStackSearchChartConfig(ClickStackSearchChartConfig),
    ClickStackEventPatternsChartConfig(ClickStackEventPatternsChartConfig),
    ClickStackMarkdownChartConfig(ClickStackMarkdownChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackTileConfig, "displayType" {
        "line" => ClickStackLineChartConfig,
        "stacked_bar" => ClickStackBarChartConfig,
        "stacked_line" => ClickStackStackedLineChartConfig,
        "bar" => ClickStackCategoricalBarChartConfig,
        "table" => ClickStackTableChartConfig,
        "number" => ClickStackNumberChartConfig,
        "pie" => ClickStackPieChartConfig,
        "heatmap" => ClickStackHeatmapChartConfig,
        "search" => ClickStackSearchChartConfig,
        "event_patterns" => ClickStackEventPatternsChartConfig,
        "markdown" => ClickStackMarkdownChartConfig,
    }
}

impl std::fmt::Display for ClickStackTileConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackCategoricalBarChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarChartConfig")
            }
            Self::ClickStackLineChartConfig(_) => write!(f, "ClickStackLineChartConfig"),
            Self::ClickStackBarChartConfig(_) => write!(f, "ClickStackBarChartConfig"),
            Self::ClickStackStackedLineChartConfig(_) => {
                write!(f, "ClickStackStackedLineChartConfig")
            }
            Self::ClickStackTableChartConfig(_) => write!(f, "ClickStackTableChartConfig"),
            Self::ClickStackNumberChartConfig(_) => write!(f, "ClickStackNumberChartConfig"),
            Self::ClickStackPieChartConfig(_) => write!(f, "ClickStackPieChartConfig"),
            Self::ClickStackHeatmapChartConfig(_) => write!(f, "ClickStackHeatmapChartConfig"),
            Self::ClickStackSearchChartConfig(_) => write!(f, "ClickStackSearchChartConfig"),
            Self::ClickStackEventPatternsChartConfig(_) => {
                write!(f, "ClickStackEventPatternsChartConfig")
            }
            Self::ClickStackMarkdownChartConfig(_) => write!(f, "ClickStackMarkdownChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackTileConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackTileConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `displayType` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `displayType` this crate does not know, or a payload that
/// does not fit the variant its `displayType` selects, lands in `Unknown` with the
/// raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `displayType` and so would not deserialize back to the same
/// variant. Build a [`ClickStackTileConfig`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackTileConfigResponse {
    ClickStackLineChartConfig(ClickStackLineChartConfigResponse),
    ClickStackBarChartConfig(ClickStackBarChartConfigResponse),
    ClickStackStackedLineChartConfig(ClickStackStackedLineChartConfigResponse),
    ClickStackCategoricalBarChartConfig(ClickStackCategoricalBarChartConfigResponse),
    ClickStackTableChartConfig(ClickStackTableChartConfigResponse),
    ClickStackNumberChartConfig(ClickStackNumberChartConfigResponse),
    ClickStackPieChartConfig(ClickStackPieChartConfigResponse),
    ClickStackHeatmapChartConfig(ClickStackHeatmapChartConfigResponse),
    ClickStackSearchChartConfig(ClickStackSearchChartConfigResponse),
    ClickStackEventPatternsChartConfig(ClickStackEventPatternsChartConfigResponse),
    ClickStackMarkdownChartConfig(ClickStackMarkdownChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackTileConfigResponse, "displayType" {
        "line" => ClickStackLineChartConfig,
        "stacked_bar" => ClickStackBarChartConfig,
        "stacked_line" => ClickStackStackedLineChartConfig,
        "bar" => ClickStackCategoricalBarChartConfig,
        "table" => ClickStackTableChartConfig,
        "number" => ClickStackNumberChartConfig,
        "pie" => ClickStackPieChartConfig,
        "heatmap" => ClickStackHeatmapChartConfig,
        "search" => ClickStackSearchChartConfig,
        "event_patterns" => ClickStackEventPatternsChartConfig,
        "markdown" => ClickStackMarkdownChartConfig,
    }
}

impl std::fmt::Display for ClickStackTileConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLineChartConfig(_) => write!(f, "ClickStackLineChartConfig"),
            Self::ClickStackBarChartConfig(_) => write!(f, "ClickStackBarChartConfig"),
            Self::ClickStackStackedLineChartConfig(_) => {
                write!(f, "ClickStackStackedLineChartConfig")
            }
            Self::ClickStackCategoricalBarChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarChartConfig")
            }
            Self::ClickStackTableChartConfig(_) => write!(f, "ClickStackTableChartConfig"),
            Self::ClickStackNumberChartConfig(_) => write!(f, "ClickStackNumberChartConfig"),
            Self::ClickStackPieChartConfig(_) => write!(f, "ClickStackPieChartConfig"),
            Self::ClickStackHeatmapChartConfig(_) => write!(f, "ClickStackHeatmapChartConfig"),
            Self::ClickStackSearchChartConfig(_) => write!(f, "ClickStackSearchChartConfig"),
            Self::ClickStackEventPatternsChartConfig(_) => {
                write!(f, "ClickStackEventPatternsChartConfig")
            }
            Self::ClickStackMarkdownChartConfig(_) => write!(f, "ClickStackMarkdownChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackBarBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarBuilderChartConfig {
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "seriesLimit", skip_serializing_if = "Option::is_none")]
    pub series_limit: Option<i64>,
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormula>>,
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackBarBuilderChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackBarBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackBarBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarBuilderChartConfigResponse {
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "seriesLimit", skip_serializing_if = "Option::is_none")]
    pub series_limit: Option<i64>,
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormulaResponse>>,
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackBarBuilderChartConfigDisplaytype>,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackBarRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarRawSqlChartConfig {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackBarRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackBarRawSqlChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackBarRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackBarRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarRawSqlChartConfigResponse {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackBarRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackBarRawSqlChartConfigDisplaytype>,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackCategoricalBarBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCategoricalBarBuilderChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackCategoricalBarBuilderChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackCategoricalBarBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackCategoricalBarBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCategoricalBarBuilderChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackCategoricalBarBuilderChartConfigDisplaytype>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackCategoricalBarRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCategoricalBarRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackCategoricalBarRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackCategoricalBarRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackCategoricalBarRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackCategoricalBarRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCategoricalBarRawSqlChartConfigResponse {
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackCategoricalBarRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackCategoricalBarRawSqlChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackEventPatternsChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackEventPatternsChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackEventPatternsChartConfigDisplaytype,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackEventPatternsChartConfigWherelanguage>,
}

/// `ClickStackEventPatternsChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackEventPatternsChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackEventPatternsChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackEventPatternsChartConfigDisplaytype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackEventPatternsChartConfigWherelanguage>,
}

/// `ClickStackHeatmapDistributionChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHeatmapDistributionChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackHeatmapDistributionChartConfigDisplaytype,
    #[serde(rename = "heatmapMode", skip_serializing_if = "Option::is_none")]
    pub heatmap_mode: Option<ClickStackHeatmapDistributionChartConfigHeatmapmode>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackHeatmapSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackHeatmapDistributionChartConfigWherelanguage>,
}

/// `ClickStackHeatmapDistributionChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackHeatmapDistributionChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHeatmapDistributionChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackHeatmapDistributionChartConfigDisplaytype>,
    #[serde(rename = "heatmapMode", skip_serializing_if = "Option::is_none")]
    pub heatmap_mode: Option<ClickStackHeatmapDistributionChartConfigHeatmapmode>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackHeatmapSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackHeatmapDistributionChartConfigWherelanguage>,
}

/// `ClickStackHeatmapSelectItem` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHeatmapSelectItem {
    #[serde(rename = "countExpression", skip_serializing_if = "Option::is_none")]
    pub count_expression: Option<String>,
    #[serde(rename = "heatmapScaleType", skip_serializing_if = "Option::is_none")]
    pub heatmap_scale_type: Option<ClickStackHeatmapSelectItemHeatmapscaletype>,
    #[serde(rename = "valueExpression")]
    pub value_expression: String,
}

/// `ClickStackHeatmapSelectItem` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackHeatmapSelectItem`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHeatmapSelectItemResponse {
    #[serde(rename = "countExpression", skip_serializing_if = "Option::is_none")]
    pub count_expression: Option<String>,
    #[serde(rename = "heatmapScaleType", skip_serializing_if = "Option::is_none")]
    pub heatmap_scale_type: Option<ClickStackHeatmapSelectItemHeatmapscaletype>,
    #[serde(rename = "valueExpression", skip_serializing_if = "Option::is_none")]
    pub value_expression: Option<String>,
}

/// `ClickStackHeatmapSeriesChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHeatmapSeriesChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackHeatmapSeriesChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "heatmapMode")]
    pub heatmap_mode: ClickStackHeatmapSeriesChartConfigHeatmapmode,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackHeatmapSeriesChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackHeatmapSeriesChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHeatmapSeriesChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackHeatmapSeriesChartConfigDisplaytype>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "heatmapMode", skip_serializing_if = "Option::is_none")]
    pub heatmap_mode: Option<ClickStackHeatmapSeriesChartConfigHeatmapmode>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackLineBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineBuilderChartConfig {
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "seriesLimit", skip_serializing_if = "Option::is_none")]
    pub series_limit: Option<i64>,
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormula>>,
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(
        rename = "compareToPreviousPeriod",
        skip_serializing_if = "Option::is_none"
    )]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackLineBuilderChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "fitYAxisToData", skip_serializing_if = "Option::is_none")]
    pub fit_y_axis_to_data: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "lineInterpolation", skip_serializing_if = "Option::is_none")]
    pub line_interpolation: Option<ClickStackLineBuilderChartConfigLineinterpolation>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackLineBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackLineBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineBuilderChartConfigResponse {
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "seriesLimit", skip_serializing_if = "Option::is_none")]
    pub series_limit: Option<i64>,
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormulaResponse>>,
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(
        rename = "compareToPreviousPeriod",
        skip_serializing_if = "Option::is_none"
    )]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackLineBuilderChartConfigDisplaytype>,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "fitYAxisToData", skip_serializing_if = "Option::is_none")]
    pub fit_y_axis_to_data: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "lineInterpolation", skip_serializing_if = "Option::is_none")]
    pub line_interpolation: Option<ClickStackLineBuilderChartConfigLineinterpolation>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackLineRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineRawSqlChartConfig {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(
        rename = "compareToPreviousPeriod",
        skip_serializing_if = "Option::is_none"
    )]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackLineRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackLineRawSqlChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "fitYAxisToData", skip_serializing_if = "Option::is_none")]
    pub fit_y_axis_to_data: Option<bool>,
    #[serde(rename = "lineInterpolation", skip_serializing_if = "Option::is_none")]
    pub line_interpolation: Option<ClickStackLineRawSqlChartConfigLineinterpolation>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackLineRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackLineRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineRawSqlChartConfigResponse {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(
        rename = "compareToPreviousPeriod",
        skip_serializing_if = "Option::is_none"
    )]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackLineRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackLineRawSqlChartConfigDisplaytype>,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "fitYAxisToData", skip_serializing_if = "Option::is_none")]
    pub fit_y_axis_to_data: Option<bool>,
    #[serde(rename = "lineInterpolation", skip_serializing_if = "Option::is_none")]
    pub line_interpolation: Option<ClickStackLineRawSqlChartConfigLineinterpolation>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackMarkdownChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMarkdownChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackMarkdownChartConfigDisplaytype,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

/// `ClickStackMarkdownChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMarkdownChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMarkdownChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackMarkdownChartConfigDisplaytype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

/// `ClickStackMarkdownChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMarkdownChartSeries {
    pub content: String,
    pub r#type: ClickStackMarkdownChartSeriesType,
}

/// `ClickStackNumberBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberBuilderChartConfig {
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormula>>,
    #[serde(rename = "backgroundChart", skip_serializing_if = "Option::is_none")]
    pub background_chart: Option<ClickStackBackgroundChart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(rename = "colorRules", skip_serializing_if = "Option::is_none")]
    pub color_rules: Option<Vec<ClickStackNumberTileColorCondition>>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackNumberBuilderChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackNumberBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackNumberBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberBuilderChartConfigResponse {
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormulaResponse>>,
    #[serde(rename = "backgroundChart", skip_serializing_if = "Option::is_none")]
    pub background_chart: Option<ClickStackBackgroundChartResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(rename = "colorRules", skip_serializing_if = "Option::is_none")]
    pub color_rules: Option<Vec<ClickStackNumberTileColorConditionResponse>>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackNumberBuilderChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackNumberChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackNumberChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none")]
    pub metric_data_type: Option<ClickStackNumberChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackNumberChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackNumberChartSeriesWherelanguage,
}

/// `ClickStackNumberRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberRawSqlChartConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(rename = "colorRules", skip_serializing_if = "Option::is_none")]
    pub color_rules: Option<Vec<ClickStackNumberTileColorCondition>>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackNumberRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackNumberRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackNumberRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackNumberRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberRawSqlChartConfigResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(rename = "colorRules", skip_serializing_if = "Option::is_none")]
    pub color_rules: Option<Vec<ClickStackNumberTileColorConditionResponse>>,
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackNumberRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackNumberRawSqlChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackOnClickDashboard` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickDashboard {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackOnClickFilterTemplate>>,
    pub target: ClickStackOnClickTarget,
    pub r#type: ClickStackOnClickDashboardType,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackOnClickDashboardWherelanguage>,
    #[serde(rename = "whereTemplate", skip_serializing_if = "Option::is_none")]
    pub where_template: Option<String>,
}

/// `ClickStackOnClickDashboard` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickDashboard`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickDashboardResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackOnClickFilterTemplateResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<ClickStackOnClickTargetResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackOnClickDashboardType>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackOnClickDashboardWherelanguage>,
    #[serde(rename = "whereTemplate", skip_serializing_if = "Option::is_none")]
    pub where_template: Option<String>,
}

/// `ClickStackOnClickExternal` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickExternal {
    pub r#type: ClickStackOnClickExternalType,
    #[serde(rename = "urlTemplate")]
    pub url_template: String,
}

/// `ClickStackOnClickExternal` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickExternal`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickExternalResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackOnClickExternalType>,
    #[serde(rename = "urlTemplate", skip_serializing_if = "Option::is_none")]
    pub url_template: Option<String>,
}

/// `ClickStackOnClickFilterTemplate` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickFilterTemplate {
    pub expression: String,
    pub kind: ClickStackOnClickFilterTemplateKind,
    pub template: String,
}

/// `ClickStackOnClickFilterTemplate` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickFilterTemplate`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickFilterTemplateResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackOnClickFilterTemplateKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

/// `ClickStackOnClickSearch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackOnClickFilterTemplate>>,
    pub target: ClickStackOnClickTarget,
    pub r#type: ClickStackOnClickSearchType,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackOnClickSearchWherelanguage>,
    #[serde(rename = "whereTemplate", skip_serializing_if = "Option::is_none")]
    pub where_template: Option<String>,
}

/// `ClickStackOnClickSearch` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickSearch`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickSearchResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackOnClickFilterTemplateResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<ClickStackOnClickTargetResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackOnClickSearchType>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackOnClickSearchWherelanguage>,
    #[serde(rename = "whereTemplate", skip_serializing_if = "Option::is_none")]
    pub where_template: Option<String>,
}

/// `ClickStackOnClickTargetIdVariant` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickTargetIdVariant {
    pub id: String,
    pub mode: ClickStackOnClickTargetIdVariantMode,
}

/// `ClickStackOnClickTargetIdVariant` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickTargetIdVariant`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickTargetIdVariantResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ClickStackOnClickTargetIdVariantMode>,
}

/// `ClickStackOnClickTargetTemplateVariant` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickTargetTemplateVariant {
    pub mode: ClickStackOnClickTargetTemplateVariantMode,
    pub template: String,
}

/// `ClickStackOnClickTargetTemplateVariant` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickTargetTemplateVariant`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickTargetTemplateVariantResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ClickStackOnClickTargetTemplateVariantMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

/// `ClickStackPieBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieBuilderChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackPieBuilderChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackPieBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackPieBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieBuilderChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackPieBuilderChartConfigDisplaytype>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackPieRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackPieRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackPieRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackPieRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackPieRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieRawSqlChartConfigResponse {
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackPieRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackPieRawSqlChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackSearchChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSearchChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackSearchChartConfigDisplaytype,
    pub select: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackSearchChartConfigWherelanguage,
}

/// `ClickStackSearchChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSearchChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSearchChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackSearchChartConfigDisplaytype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackSearchChartConfigWherelanguage>,
}

/// `ClickStackSearchChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSearchChartSeries {
    pub fields: Vec<String>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackSearchChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackSearchChartSeriesWherelanguage,
}

/// `ClickStackStackedLineBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackStackedLineBuilderChartConfig {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackStackedLineBuilderChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormula>>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "lineInterpolation", skip_serializing_if = "Option::is_none")]
    pub line_interpolation: Option<ClickStackStackedLineBuilderChartConfigLineinterpolation>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "seriesLimit", skip_serializing_if = "Option::is_none")]
    pub series_limit: Option<i64>,
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackStackedLineBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackStackedLineBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackStackedLineBuilderChartConfigResponse {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackStackedLineBuilderChartConfigDisplaytype>,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormulaResponse>>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "lineInterpolation", skip_serializing_if = "Option::is_none")]
    pub line_interpolation: Option<ClickStackStackedLineBuilderChartConfigLineinterpolation>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "seriesLimit", skip_serializing_if = "Option::is_none")]
    pub series_limit: Option<i64>,
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackStackedLineRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackStackedLineRawSqlChartConfig {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackStackedLineRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackStackedLineRawSqlChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "lineInterpolation", skip_serializing_if = "Option::is_none")]
    pub line_interpolation: Option<ClickStackStackedLineRawSqlChartConfigLineinterpolation>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackStackedLineRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackStackedLineRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackStackedLineRawSqlChartConfigResponse {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackStackedLineRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackStackedLineRawSqlChartConfigDisplaytype>,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "lineInterpolation", skip_serializing_if = "Option::is_none")]
    pub line_interpolation: Option<ClickStackStackedLineRawSqlChartConfigLineinterpolation>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackTableBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableBuilderChartConfig {
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormula>>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackTableBuilderChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(
        rename = "groupByColumnsOnLeft",
        skip_serializing_if = "Option::is_none"
    )]
    pub group_by_columns_on_left: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub having: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "onClick", skip_serializing_if = "Option::is_none")]
    pub on_click: Option<ClickStackOnClick>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackTableBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackTableBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableBuilderChartConfigResponse {
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormulaResponse>>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackTableBuilderChartConfigDisplaytype>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(
        rename = "groupByColumnsOnLeft",
        skip_serializing_if = "Option::is_none"
    )]
    pub group_by_columns_on_left: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub having: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "onClick", skip_serializing_if = "Option::is_none")]
    pub on_click: Option<ClickStackOnClickResponse>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackTableChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackTableChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(rename = "groupBy")]
    pub group_by: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none")]
    pub metric_data_type: Option<ClickStackTableChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sortOrder", skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<ClickStackTableChartSeriesSortorder>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackTableChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackTableChartSeriesWherelanguage,
}

/// `ClickStackTableRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackTableRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackTableRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "onClick", skip_serializing_if = "Option::is_none")]
    pub on_click: Option<ClickStackOnClick>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackTableRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackTableRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableRawSqlChartConfigResponse {
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackTableRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackTableRawSqlChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "onClick", skip_serializing_if = "Option::is_none")]
    pub on_click: Option<ClickStackOnClickResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackTileInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTileInput {
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ClickStackTileConfig>,
    #[serde(rename = "containerId", skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub h: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<Vec<ClickStackDashboardChartSeries>>,
    #[serde(rename = "tabId", skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    pub w: i64,
    pub x: i64,
    pub y: i64,
}

/// `ClickStackTileOutput` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTileOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ClickStackTileConfigResponse>,
    #[serde(rename = "containerId", skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "tabId", skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i64>,
}

/// `ClickStackTimeChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTimeChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackTimeChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackTimeChartSeriesDisplaytype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(rename = "groupBy")]
    pub group_by: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none")]
    pub metric_data_type: Option<ClickStackTimeChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackTimeChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackTimeChartSeriesWherelanguage,
}

impl Default for ClickStackBarChartConfig {
    fn default() -> Self {
        Self::ClickStackBarBuilderChartConfig(ClickStackBarBuilderChartConfig::default())
    }
}

impl Default for ClickStackDashboardChartSeries {
    fn default() -> Self {
        Self::ClickStackTimeChartSeries(ClickStackTimeChartSeries::default())
    }
}

impl Default for ClickStackHeatmapChartConfig {
    fn default() -> Self {
        Self::ClickStackHeatmapDistributionChartConfig(
            ClickStackHeatmapDistributionChartConfig::default(),
        )
    }
}

impl Default for ClickStackLineChartConfig {
    fn default() -> Self {
        Self::ClickStackLineBuilderChartConfig(ClickStackLineBuilderChartConfig::default())
    }
}

impl Default for ClickStackNumberChartConfig {
    fn default() -> Self {
        Self::ClickStackNumberBuilderChartConfig(ClickStackNumberBuilderChartConfig::default())
    }
}

impl Default for ClickStackPieChartConfig {
    fn default() -> Self {
        Self::ClickStackPieBuilderChartConfig(ClickStackPieBuilderChartConfig::default())
    }
}

impl Default for ClickStackStackedLineChartConfig {
    fn default() -> Self {
        Self::ClickStackStackedLineBuilderChartConfig(
            ClickStackStackedLineBuilderChartConfig::default(),
        )
    }
}

impl Default for ClickStackTableChartConfig {
    fn default() -> Self {
        Self::ClickStackTableBuilderChartConfig(ClickStackTableBuilderChartConfig::default())
    }
}

impl Default for ClickStackTileConfig {
    fn default() -> Self {
        Self::ClickStackLineChartConfig(ClickStackLineChartConfig::default())
    }
}
