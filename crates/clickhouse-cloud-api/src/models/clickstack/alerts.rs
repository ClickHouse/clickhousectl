use super::enums::*;
use super::{
    ClickStackBackgroundChart, ClickStackBackgroundChartResponse, ClickStackFormula,
    ClickStackFormulaResponse, ClickStackNumberFormat, ClickStackNumberFormatResponse,
    ClickStackNumberTileColorCondition, ClickStackNumberTileColorConditionResponse,
    ClickStackSelectItem, ClickStackSelectItemResponse,
};
#[cfg(doc)]
use super::{
    ClickStackBarBuilderChartConfig, ClickStackBarChartConfig, ClickStackBarRawSqlChartConfig,
    ClickStackLineBuilderChartConfig, ClickStackLineChartConfig, ClickStackLineRawSqlChartConfig,
    ClickStackNumberBuilderChartConfig, ClickStackNumberChartConfig,
    ClickStackNumberRawSqlChartConfig, ClickStackStackedLineBuilderChartConfig,
    ClickStackStackedLineChartConfig, ClickStackStackedLineRawSqlChartConfig,
};
use serde::{Deserialize, Serialize};

/// `ClickStackAlertChannel` - one of multiple variants.
///
/// Dispatched on the `type` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertChannel {
    ClickStackAlertChannelEmail(ClickStackAlertChannelEmail),
    ClickStackAlertChannelWebhook(ClickStackAlertChannelWebhook),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertChannel, "type" {
        "email" => ClickStackAlertChannelEmail,
        "webhook" => ClickStackAlertChannelWebhook,
    }
}

impl std::fmt::Display for ClickStackAlertChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertChannelEmail(_) => write!(f, "ClickStackAlertChannelEmail"),
            Self::ClickStackAlertChannelWebhook(_) => write!(f, "ClickStackAlertChannelWebhook"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertChannel` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackAlertChannel`]: each arm is the all-`Option`
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
/// variant. Build a [`ClickStackAlertChannel`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertChannelResponse {
    ClickStackAlertChannelEmail(ClickStackAlertChannelEmailResponse),
    ClickStackAlertChannelWebhook(ClickStackAlertChannelWebhookResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertChannelResponse, "type" {
        "email" => ClickStackAlertChannelEmail,
        "webhook" => ClickStackAlertChannelWebhook,
    }
}

impl std::fmt::Display for ClickStackAlertChannelResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertChannelEmail(_) => write!(f, "ClickStackAlertChannelEmail"),
            Self::ClickStackAlertChannelWebhook(_) => write!(f, "ClickStackAlertChannelWebhook"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertBarChartConfig` - one of multiple variants.
///
/// The alert counterpart of [`ClickStackBarChartConfig`]: each variant adds
/// the alert overlay's fields. Dispatched on the `configType` field (absent or
/// non-string dispatches to the builder variant, unless the payload carries a
/// raw-SQL-only key); see the `discriminated_union!` invocation below for the
/// wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertBarChartConfig {
    ClickStackAlertBarBuilderChartConfig(ClickStackAlertBarBuilderChartConfig),
    ClickStackAlertBarRawSqlChartConfig(ClickStackAlertBarRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertBarChartConfig, "configType" {
        "sql" => ClickStackAlertBarRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackAlertBarBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackAlertBarChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertBarBuilderChartConfig(_) => {
                write!(f, "ClickStackAlertBarBuilderChartConfig")
            }
            Self::ClickStackAlertBarRawSqlChartConfig(_) => {
                write!(f, "ClickStackAlertBarRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertBarChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackAlertBarChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
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
/// deserialization, never constructed; build a [`ClickStackAlertBarChartConfig`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertBarChartConfigResponse {
    ClickStackAlertBarBuilderChartConfig(ClickStackAlertBarBuilderChartConfigResponse),
    ClickStackAlertBarRawSqlChartConfig(ClickStackAlertBarRawSqlChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertBarChartConfigResponse, "configType" {
        "sql" => ClickStackAlertBarRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackAlertBarBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackAlertBarChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertBarBuilderChartConfig(_) => {
                write!(f, "ClickStackAlertBarBuilderChartConfig")
            }
            Self::ClickStackAlertBarRawSqlChartConfig(_) => {
                write!(f, "ClickStackAlertBarRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertChartConfig` - one of multiple variants.
///
/// The chart an `inline` alert queries: a line, stacked-bar, stacked-line or
/// number chart config plus the alert overlay's fields. Dispatched on the
/// `displayType` field, which every variant requires with its chart's own
/// single wire value; see the `discriminated_union!` invocation below. Each
/// arm is itself a builder/raw-SQL union dispatched on `configType`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertChartConfig {
    ClickStackAlertLineChartConfig(ClickStackAlertLineChartConfig),
    ClickStackAlertBarChartConfig(ClickStackAlertBarChartConfig),
    ClickStackAlertStackedLineChartConfig(ClickStackAlertStackedLineChartConfig),
    ClickStackAlertNumberChartConfig(ClickStackAlertNumberChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertChartConfig, "displayType" {
        "line" => ClickStackAlertLineChartConfig,
        "stacked_bar" => ClickStackAlertBarChartConfig,
        "stacked_line" => ClickStackAlertStackedLineChartConfig,
        "number" => ClickStackAlertNumberChartConfig,
    }
}

impl std::fmt::Display for ClickStackAlertChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertLineChartConfig(_) => write!(f, "ClickStackAlertLineChartConfig"),
            Self::ClickStackAlertBarChartConfig(_) => write!(f, "ClickStackAlertBarChartConfig"),
            Self::ClickStackAlertStackedLineChartConfig(_) => {
                write!(f, "ClickStackAlertStackedLineChartConfig")
            }
            Self::ClickStackAlertNumberChartConfig(_) => {
                write!(f, "ClickStackAlertNumberChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackAlertChartConfig`]: each arm is the
/// response variant of its request union, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `displayType` field, exactly as the request union is:
/// dispatch reads the raw JSON rather than trying each variant's shape, so
/// all-`Option` arms cannot misroute a payload. A `displayType` this crate does
/// not know, or none at all, lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackAlertChartConfig`]
/// instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertChartConfigResponse {
    ClickStackAlertLineChartConfig(ClickStackAlertLineChartConfigResponse),
    ClickStackAlertBarChartConfig(ClickStackAlertBarChartConfigResponse),
    ClickStackAlertStackedLineChartConfig(ClickStackAlertStackedLineChartConfigResponse),
    ClickStackAlertNumberChartConfig(ClickStackAlertNumberChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertChartConfigResponse, "displayType" {
        "line" => ClickStackAlertLineChartConfig,
        "stacked_bar" => ClickStackAlertBarChartConfig,
        "stacked_line" => ClickStackAlertStackedLineChartConfig,
        "number" => ClickStackAlertNumberChartConfig,
    }
}

impl std::fmt::Display for ClickStackAlertChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertLineChartConfig(_) => write!(f, "ClickStackAlertLineChartConfig"),
            Self::ClickStackAlertBarChartConfig(_) => write!(f, "ClickStackAlertBarChartConfig"),
            Self::ClickStackAlertStackedLineChartConfig(_) => {
                write!(f, "ClickStackAlertStackedLineChartConfig")
            }
            Self::ClickStackAlertNumberChartConfig(_) => {
                write!(f, "ClickStackAlertNumberChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertLineChartConfig` - one of multiple variants.
///
/// The alert counterpart of [`ClickStackLineChartConfig`]: each variant adds
/// the alert overlay's fields. Dispatched on the `configType` field (absent or
/// non-string dispatches to the builder variant, unless the payload carries a
/// raw-SQL-only key); see the `discriminated_union!` invocation below for the
/// wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertLineChartConfig {
    ClickStackAlertLineBuilderChartConfig(ClickStackAlertLineBuilderChartConfig),
    ClickStackAlertLineRawSqlChartConfig(ClickStackAlertLineRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertLineChartConfig, "configType" {
        "sql" => ClickStackAlertLineRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackAlertLineBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackAlertLineChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertLineBuilderChartConfig(_) => {
                write!(f, "ClickStackAlertLineBuilderChartConfig")
            }
            Self::ClickStackAlertLineRawSqlChartConfig(_) => {
                write!(f, "ClickStackAlertLineRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertLineChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackAlertLineChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
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
/// deserialization, never constructed; build a [`ClickStackAlertLineChartConfig`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertLineChartConfigResponse {
    ClickStackAlertLineBuilderChartConfig(ClickStackAlertLineBuilderChartConfigResponse),
    ClickStackAlertLineRawSqlChartConfig(ClickStackAlertLineRawSqlChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertLineChartConfigResponse, "configType" {
        "sql" => ClickStackAlertLineRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackAlertLineBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackAlertLineChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertLineBuilderChartConfig(_) => {
                write!(f, "ClickStackAlertLineBuilderChartConfig")
            }
            Self::ClickStackAlertLineRawSqlChartConfig(_) => {
                write!(f, "ClickStackAlertLineRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertNumberChartConfig` - one of multiple variants.
///
/// The alert counterpart of [`ClickStackNumberChartConfig`]: each variant adds
/// the alert overlay's fields. Dispatched on the `configType` field (absent or
/// non-string dispatches to the builder variant, unless the payload carries a
/// raw-SQL-only key); see the `discriminated_union!` invocation below for the
/// wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertNumberChartConfig {
    ClickStackAlertNumberBuilderChartConfig(ClickStackAlertNumberBuilderChartConfig),
    ClickStackAlertNumberRawSqlChartConfig(ClickStackAlertNumberRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertNumberChartConfig, "configType" {
        "sql" => ClickStackAlertNumberRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackAlertNumberBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackAlertNumberChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertNumberBuilderChartConfig(_) => {
                write!(f, "ClickStackAlertNumberBuilderChartConfig")
            }
            Self::ClickStackAlertNumberRawSqlChartConfig(_) => {
                write!(f, "ClickStackAlertNumberRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertNumberChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackAlertNumberChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
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
/// deserialization, never constructed; build a [`ClickStackAlertNumberChartConfig`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertNumberChartConfigResponse {
    ClickStackAlertNumberBuilderChartConfig(ClickStackAlertNumberBuilderChartConfigResponse),
    ClickStackAlertNumberRawSqlChartConfig(ClickStackAlertNumberRawSqlChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertNumberChartConfigResponse, "configType" {
        "sql" => ClickStackAlertNumberRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackAlertNumberBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackAlertNumberChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertNumberBuilderChartConfig(_) => {
                write!(f, "ClickStackAlertNumberBuilderChartConfig")
            }
            Self::ClickStackAlertNumberRawSqlChartConfig(_) => {
                write!(f, "ClickStackAlertNumberRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertStackedLineChartConfig` - one of multiple variants.
///
/// The alert counterpart of [`ClickStackStackedLineChartConfig`]: each variant adds
/// the alert overlay's fields. Dispatched on the `configType` field (absent or
/// non-string dispatches to the builder variant, unless the payload carries a
/// raw-SQL-only key); see the `discriminated_union!` invocation below for the
/// wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertStackedLineChartConfig {
    ClickStackAlertStackedLineBuilderChartConfig(ClickStackAlertStackedLineBuilderChartConfig),
    ClickStackAlertStackedLineRawSqlChartConfig(ClickStackAlertStackedLineRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertStackedLineChartConfig, "configType" {
        "sql" => ClickStackAlertStackedLineRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackAlertStackedLineBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackAlertStackedLineChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertStackedLineBuilderChartConfig(_) => {
                write!(f, "ClickStackAlertStackedLineBuilderChartConfig")
            }
            Self::ClickStackAlertStackedLineRawSqlChartConfig(_) => {
                write!(f, "ClickStackAlertStackedLineRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertStackedLineChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackAlertStackedLineChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
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
/// deserialization, never constructed; build a [`ClickStackAlertStackedLineChartConfig`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertStackedLineChartConfigResponse {
    ClickStackAlertStackedLineBuilderChartConfig(
        ClickStackAlertStackedLineBuilderChartConfigResponse,
    ),
    ClickStackAlertStackedLineRawSqlChartConfig(
        ClickStackAlertStackedLineRawSqlChartConfigResponse,
    ),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertStackedLineChartConfigResponse, "configType" {
        "sql" => ClickStackAlertStackedLineRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackAlertStackedLineBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackAlertStackedLineChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertStackedLineBuilderChartConfig(_) => {
                write!(f, "ClickStackAlertStackedLineBuilderChartConfig")
            }
            Self::ClickStackAlertStackedLineRawSqlChartConfig(_) => {
                write!(f, "ClickStackAlertStackedLineRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertBarBuilderChartConfig` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackBarBuilderChartConfig`] and
/// [`ClickStackAlertChartConfigOverlay`], so it carries every field of the
/// chart config plus the overlay's `name`, `where` and `whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertBarBuilderChartConfig {
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
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormula>>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "seriesLimit", skip_serializing_if = "Option::is_none")]
    pub series_limit: Option<i64>,
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertBarBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertBarBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertBarBuilderChartConfigResponse {
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
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormulaResponse>>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertBarRawSqlChartConfig` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackBarRawSqlChartConfig`] and
/// [`ClickStackAlertChartConfigOverlay`], so it carries every field of the
/// chart config plus the overlay's `name`, `where` and `whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertBarRawSqlChartConfig {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertBarRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertBarRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertBarRawSqlChartConfigResponse {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertChartConfigOverlay` from the ClickHouse Cloud API.
///
/// The alert-specific fields every [`ClickStackAlertChartConfig`] variant adds
/// to its chart config through `allOf`. The spec never uses it on its own, so
/// each variant struct carries these fields directly.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChartConfigOverlay {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertLineBuilderChartConfig` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackLineBuilderChartConfig`] and
/// [`ClickStackAlertChartConfigOverlay`], so it carries every field of the
/// chart config plus the overlay's `name`, `where` and `whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertLineBuilderChartConfig {
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
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormula>>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "lineInterpolation", skip_serializing_if = "Option::is_none")]
    pub line_interpolation: Option<ClickStackLineBuilderChartConfigLineinterpolation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "seriesLimit", skip_serializing_if = "Option::is_none")]
    pub series_limit: Option<i64>,
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertLineBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertLineBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertLineBuilderChartConfigResponse {
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
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormulaResponse>>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "lineInterpolation", skip_serializing_if = "Option::is_none")]
    pub line_interpolation: Option<ClickStackLineBuilderChartConfigLineinterpolation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertLineRawSqlChartConfig` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackLineRawSqlChartConfig`] and
/// [`ClickStackAlertChartConfigOverlay`], so it carries every field of the
/// chart config plus the overlay's `name`, `where` and `whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertLineRawSqlChartConfig {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertLineRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertLineRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertLineRawSqlChartConfigResponse {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertNumberBuilderChartConfig` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackNumberBuilderChartConfig`] and
/// [`ClickStackAlertChartConfigOverlay`], so it carries every field of the
/// chart config plus the overlay's `name`, `where` and `whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertNumberBuilderChartConfig {
    #[serde(rename = "backgroundChart", skip_serializing_if = "Option::is_none")]
    pub background_chart: Option<ClickStackBackgroundChart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(rename = "colorRules", skip_serializing_if = "Option::is_none")]
    pub color_rules: Option<Vec<ClickStackNumberTileColorCondition>>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackNumberBuilderChartConfigDisplaytype,
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormula>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertNumberBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertNumberBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertNumberBuilderChartConfigResponse {
    #[serde(rename = "backgroundChart", skip_serializing_if = "Option::is_none")]
    pub background_chart: Option<ClickStackBackgroundChartResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(rename = "colorRules", skip_serializing_if = "Option::is_none")]
    pub color_rules: Option<Vec<ClickStackNumberTileColorConditionResponse>>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackNumberBuilderChartConfigDisplaytype>,
    #[serde(rename = "formulas", skip_serializing_if = "Option::is_none")]
    pub formulas: Option<Vec<ClickStackFormulaResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertNumberRawSqlChartConfig` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackNumberRawSqlChartConfig`] and
/// [`ClickStackAlertChartConfigOverlay`], so it carries every field of the
/// chart config plus the overlay's `name`, `where` and `whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertNumberRawSqlChartConfig {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertNumberRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertNumberRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertNumberRawSqlChartConfigResponse {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertStackedLineBuilderChartConfig` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackStackedLineBuilderChartConfig`] and
/// [`ClickStackAlertChartConfigOverlay`], so it carries every field of the
/// chart config plus the overlay's `name`, `where` and `whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertStackedLineBuilderChartConfig {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "seriesLimit", skip_serializing_if = "Option::is_none")]
    pub series_limit: Option<i64>,
    #[serde(rename = "showOperandSeries", skip_serializing_if = "Option::is_none")]
    pub show_operand_series: Option<bool>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertStackedLineBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertStackedLineBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertStackedLineBuilderChartConfigResponse {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertStackedLineRawSqlChartConfig` from the ClickHouse Cloud API.
///
/// The spec defines it as `allOf` [`ClickStackStackedLineRawSqlChartConfig`] and
/// [`ClickStackAlertChartConfigOverlay`], so it carries every field of the
/// chart config plus the overlay's `name`, `where` and `whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertStackedLineRawSqlChartConfig {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertStackedLineRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertStackedLineRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertStackedLineRawSqlChartConfigResponse {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackAlertChartConfigOverlayWherelanguage>,
}

/// `ClickStackAlertChannelEmail` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelEmail {
    #[serde(rename = "emailRecipients")]
    pub email_recipients: Vec<String>,
    pub r#type: ClickStackAlertChannelEmailType,
}

/// `ClickStackAlertChannelEmail` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertChannelEmail`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelEmailResponse {
    #[serde(rename = "emailRecipients", skip_serializing_if = "Option::is_none")]
    pub email_recipients: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackAlertChannelEmailType>,
}

/// `ClickStackAlertChannelWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelWebhook {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<ClickStackAlertChannelWebhookSeverity>,
    #[serde(rename = "slackChannelId", skip_serializing_if = "Option::is_none")]
    pub slack_channel_id: Option<String>,
    pub r#type: ClickStackAlertChannelWebhookType,
    #[serde(rename = "webhookId")]
    pub webhook_id: String,
    #[serde(rename = "webhookService", skip_serializing_if = "Option::is_none")]
    pub webhook_service: Option<String>,
}

/// `ClickStackAlertChannelWebhook` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertChannelWebhook`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelWebhookResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<ClickStackAlertChannelWebhookSeverity>,
    #[serde(rename = "slackChannelId", skip_serializing_if = "Option::is_none")]
    pub slack_channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackAlertChannelWebhookType>,
    #[serde(rename = "webhookId", skip_serializing_if = "Option::is_none")]
    pub webhook_id: Option<String>,
    #[serde(rename = "webhookService", skip_serializing_if = "Option::is_none")]
    pub webhook_service: Option<String>,
}

/// `ClickStackAlertExecutionError` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertExecutionError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackAlertExecutionErrorType>,
}

/// `ClickStackAlertResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertResponse {
    #[serde(rename = "channels", skip_serializing_if = "Option::is_none")]
    pub channels: Option<ClickStackAlertChannelsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<ClickStackAlertChannelResponse>,
    #[serde(rename = "chartConfig", skip_serializing_if = "Option::is_none")]
    pub chart_config: Option<ClickStackAlertChartConfigResponse>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none")]
    pub dashboard_id: Option<String>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "executionErrors", skip_serializing_if = "Option::is_none")]
    pub execution_errors: Option<Vec<ClickStackAlertExecutionError>>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<ClickStackAlertResponseInterval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(
        rename = "numConsecutiveWindows",
        skip_serializing_if = "Option::is_none"
    )]
    pub num_consecutive_windows: Option<i64>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none")]
    pub saved_search_id: Option<String>,
    #[serde(
        rename = "scheduleOffsetMinutes",
        skip_serializing_if = "Option::is_none"
    )]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none")]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silenced: Option<ClickStackAlertSilenced>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ClickStackAlertResponseSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ClickStackAlertResponseState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "teamId", skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    #[serde(rename = "thresholdMax", skip_serializing_if = "Option::is_none")]
    pub threshold_max: Option<f64>,
    #[serde(rename = "thresholdType", skip_serializing_if = "Option::is_none")]
    pub threshold_type: Option<ClickStackAlertResponseThresholdtype>,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none")]
    pub tile_id: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickStackAlertSilenced` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertSilenced {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickStackCreateAlertRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateAlertRequest {
    #[serde(rename = "channels", skip_serializing_if = "Option::is_none")]
    pub channels: Option<ClickStackAlertChannels>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<ClickStackAlertChannel>,
    #[serde(rename = "chartConfig", skip_serializing_if = "Option::is_none")]
    pub chart_config: Option<ClickStackAlertChartConfig>,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none")]
    pub dashboard_id: Option<String>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    pub interval: ClickStackCreateAlertRequestInterval,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(
        rename = "numConsecutiveWindows",
        skip_serializing_if = "Option::is_none"
    )]
    pub num_consecutive_windows: Option<i64>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none")]
    pub saved_search_id: Option<String>,
    #[serde(
        rename = "scheduleOffsetMinutes",
        skip_serializing_if = "Option::is_none"
    )]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none")]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ClickStackCreateAlertRequestSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub threshold: f64,
    #[serde(rename = "thresholdMax", skip_serializing_if = "Option::is_none")]
    pub threshold_max: Option<f64>,
    #[serde(rename = "thresholdType")]
    pub threshold_type: ClickStackCreateAlertRequestThresholdtype,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none")]
    pub tile_id: Option<String>,
}

/// `ClickStackUpdateAlertRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateAlertRequest {
    #[serde(rename = "channels", skip_serializing_if = "Option::is_none")]
    pub channels: Option<ClickStackAlertChannels>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<ClickStackAlertChannel>,
    #[serde(rename = "chartConfig", skip_serializing_if = "Option::is_none")]
    pub chart_config: Option<ClickStackAlertChartConfig>,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none")]
    pub dashboard_id: Option<String>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    pub interval: ClickStackUpdateAlertRequestInterval,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(
        rename = "numConsecutiveWindows",
        skip_serializing_if = "Option::is_none"
    )]
    pub num_consecutive_windows: Option<i64>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none")]
    pub saved_search_id: Option<String>,
    #[serde(
        rename = "scheduleOffsetMinutes",
        skip_serializing_if = "Option::is_none"
    )]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none")]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ClickStackUpdateAlertRequestSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub threshold: f64,
    #[serde(rename = "thresholdMax", skip_serializing_if = "Option::is_none")]
    pub threshold_max: Option<f64>,
    #[serde(rename = "thresholdType")]
    pub threshold_type: ClickStackUpdateAlertRequestThresholdtype,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none")]
    pub tile_id: Option<String>,
}

impl Default for ClickStackAlertChannel {
    fn default() -> Self {
        Self::ClickStackAlertChannelEmail(ClickStackAlertChannelEmail::default())
    }
}

impl Default for ClickStackAlertBarChartConfig {
    fn default() -> Self {
        Self::ClickStackAlertBarBuilderChartConfig(ClickStackAlertBarBuilderChartConfig::default())
    }
}

impl Default for ClickStackAlertChartConfig {
    fn default() -> Self {
        Self::ClickStackAlertLineChartConfig(ClickStackAlertLineChartConfig::default())
    }
}

impl Default for ClickStackAlertLineChartConfig {
    fn default() -> Self {
        Self::ClickStackAlertLineBuilderChartConfig(ClickStackAlertLineBuilderChartConfig::default())
    }
}

impl Default for ClickStackAlertNumberChartConfig {
    fn default() -> Self {
        Self::ClickStackAlertNumberBuilderChartConfig(
            ClickStackAlertNumberBuilderChartConfig::default(),
        )
    }
}

impl Default for ClickStackAlertStackedLineChartConfig {
    fn default() -> Self {
        Self::ClickStackAlertStackedLineBuilderChartConfig(
            ClickStackAlertStackedLineBuilderChartConfig::default(),
        )
    }
}

/// Notification channels for an alert.
pub type ClickStackAlertChannels = Vec<ClickStackAlertChannel>;
/// Tolerant notification channels returned by the API.
pub type ClickStackAlertChannelsResponse = Vec<ClickStackAlertChannelResponse>;
