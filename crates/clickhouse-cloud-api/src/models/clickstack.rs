use super::clickstack_enums::*;
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

/// `ClickStackCASLPermission` - one of multiple variants.
///
/// Dispatched on the `integration` field, which each variant requires with its
/// own single wire value; see the `discriminated_union!` invocation below.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackCASLPermission {
    ClickStackMongoDBCASLPermission(ClickStackMongoDBCASLPermission),
    ClickStackClickHouseCASLPermission(ClickStackClickHouseCASLPermission),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackCASLPermission, "integration" {
        "mongodb" => ClickStackMongoDBCASLPermission,
        "clickhouse" => ClickStackClickHouseCASLPermission,
    }
}

impl std::fmt::Display for ClickStackCASLPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackMongoDBCASLPermission(_) => {
                write!(f, "ClickStackMongoDBCASLPermission")
            }
            Self::ClickStackClickHouseCASLPermission(_) => {
                write!(f, "ClickStackClickHouseCASLPermission")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackCASLPermission` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackCASLPermission`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `integration` field, exactly as the request union is:
/// dispatch reads the raw JSON rather than trying each variant's shape, so the
/// all-`Option` arms — which would match any object under `untagged` matching —
/// cannot misroute a payload. An `integration` this crate does not know, a
/// payload without one, or a payload that does not fit the variant its
/// `integration` selects, lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `integration` and so would not deserialize back to the
/// same variant. Build a [`ClickStackCASLPermission`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackCASLPermissionResponse {
    ClickStackMongoDBCASLPermission(ClickStackMongoDBCASLPermissionResponse),
    ClickStackClickHouseCASLPermission(ClickStackClickHouseCASLPermissionResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackCASLPermissionResponse, "integration" {
        "mongodb" => ClickStackMongoDBCASLPermission,
        "clickhouse" => ClickStackClickHouseCASLPermission,
    }
}

impl std::fmt::Display for ClickStackCASLPermissionResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackMongoDBCASLPermission(_) => {
                write!(f, "ClickStackMongoDBCASLPermission")
            }
            Self::ClickStackClickHouseCASLPermission(_) => {
                write!(f, "ClickStackClickHouseCASLPermission")
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

/// `ClickStackWebhook` - one of multiple variants.
///
/// Dispatched on the `service` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackWebhook {
    ClickStackSlackWebhook(ClickStackSlackWebhook),
    ClickStackIncidentIOWebhook(ClickStackIncidentIOWebhook),
    ClickStackGenericWebhook(ClickStackGenericWebhook),
    ClickStackSlackAPIWebhook(ClickStackSlackAPIWebhook),
    ClickStackPagerDutyAPIWebhook(ClickStackPagerDutyAPIWebhook),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackWebhook, "service" {
        "slack" => ClickStackSlackWebhook,
        "incidentio" => ClickStackIncidentIOWebhook,
        "generic" => ClickStackGenericWebhook,
        "slack_api" => ClickStackSlackAPIWebhook,
        "pagerduty_api" => ClickStackPagerDutyAPIWebhook,
    }
}

impl std::fmt::Display for ClickStackWebhook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackSlackWebhook(_) => write!(f, "ClickStackSlackWebhook"),
            Self::ClickStackIncidentIOWebhook(_) => write!(f, "ClickStackIncidentIOWebhook"),
            Self::ClickStackGenericWebhook(_) => write!(f, "ClickStackGenericWebhook"),
            Self::ClickStackSlackAPIWebhook(_) => write!(f, "ClickStackSlackAPIWebhook"),
            Self::ClickStackPagerDutyAPIWebhook(_) => write!(f, "ClickStackPagerDutyAPIWebhook"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackMongoDBCASLPermissionConditions` from the ClickHouse Cloud API.
///
/// A MongoDB-style query keyed by document field name. Each value is either a
/// string to match exactly or a `$regex` pattern.
pub type ClickStackMongoDBCASLPermissionConditions =
    std::collections::BTreeMap<String, ClickStackMongoDBCASLPermissionConditionValue>;

/// `ClickStackMongoDBCASLPermissionConditions` from the ClickHouse Cloud API, in response position.
///
/// Tolerant response map: keeps every field name, and each value as raw JSON
/// so a value shape the API adds later is preserved without coercion.
pub type ClickStackMongoDBCASLPermissionConditionsResponse =
    std::collections::BTreeMap<String, serde_json::Value>;

/// One value of [`ClickStackMongoDBCASLPermissionConditions`]: the spec's
/// `oneOf` of a string and a `$regex` object.
///
/// The two shapes are disjoint JSON types (a string and an object), so
/// `untagged` matching cannot route a value to the wrong variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackMongoDBCASLPermissionConditionValue {
    String(String),
    Regex(ClickStackMongoDBCASLPermissionRegexCondition),
}

/// Type alias for `ClickStackValidateDashboardResponseNormalized`.
pub type ClickStackValidateDashboardResponseNormalized = serde_json::Value;

/// Type alias for `ClickStackValidateWebhookResponseNormalized`.
pub type ClickStackValidateWebhookResponseNormalized = serde_json::Value;

/// Type alias for `ClickStackWebhookInputHeaders`.
pub type ClickStackWebhookInputHeaders = std::collections::BTreeMap<String, String>;

/// Type alias for `ClickStackWebhookInputQueryParams`.
pub type ClickStackWebhookInputQueryParams = std::collections::BTreeMap<String, String>;

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

/// `ClickStackClickHouseCASLPermission` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermission {
    pub action: ClickStackCASLPermissionAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ClickStackClickHouseCASLPermissionConditions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    pub integration: ClickStackClickHouseCASLPermissionIntegration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    pub subject: ClickStackCASLPermissionSubject,
}

/// `ClickStackClickHouseCASLPermission` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackClickHouseCASLPermission`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermissionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ClickStackCASLPermissionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ClickStackClickHouseCASLPermissionConditionsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<ClickStackClickHouseCASLPermissionIntegration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<ClickStackCASLPermissionSubject>,
}

/// `ClickStackClickHouseCASLPermissionConditions` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermissionConditions {
    pub connection: String,
    pub from: ClickStackClickHouseCASLPermissionConditionsFrom,
    #[serde(rename = "where")]
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackClickHouseCASLPermissionConditionsWherelanguage,
}

/// `ClickStackClickHouseCASLPermissionConditions` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackClickHouseCASLPermissionConditions`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermissionConditionsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackClickHouseCASLPermissionConditionsFromResponse>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackClickHouseCASLPermissionConditionsWherelanguage>,
}

/// `ClickStackClickHouseCASLPermissionConditionsFrom` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermissionConditionsFrom {
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
}

/// `ClickStackClickHouseCASLPermissionConditionsFrom` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackClickHouseCASLPermissionConditionsFrom`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermissionConditionsFromResponse {
    #[serde(rename = "databaseName", skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
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

/// `ClickStackCreateRoleRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateRoleRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub name: String,
    pub permissions: Vec<ClickStackCASLPermission>,
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

/// `ClickStackGenericWebhook` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackGenericWebhook {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ClickStackGenericWebhookService>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
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

/// `ClickStackIncidentIOWebhook` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackIncidentIOWebhook {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ClickStackIncidentIOWebhookService>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
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

/// `ClickStackMongoDBCASLPermission` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMongoDBCASLPermission {
    pub action: ClickStackCASLPermissionAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ClickStackMongoDBCASLPermissionConditions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    pub integration: ClickStackMongoDBCASLPermissionIntegration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    pub subject: ClickStackCASLPermissionSubject,
}

/// `ClickStackMongoDBCASLPermission` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMongoDBCASLPermission`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMongoDBCASLPermissionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ClickStackCASLPermissionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ClickStackMongoDBCASLPermissionConditionsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<ClickStackMongoDBCASLPermissionIntegration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<ClickStackCASLPermissionSubject>,
}

/// The `$regex` object branch of a [`ClickStackMongoDBCASLPermissionConditionValue`].
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMongoDBCASLPermissionRegexCondition {
    #[serde(rename = "$regex")]
    pub regex: String,
    #[serde(rename = "$options", skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
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

/// `ClickStackPagerDutyAPIWebhook` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPagerDutyAPIWebhook {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ClickStackPagerDutyAPIWebhookService>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
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

/// `ClickStackRole` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackRole {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "isPredefined", skip_serializing_if = "Option::is_none")]
    pub is_predefined: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<ClickStackCASLPermissionResponse>>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
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

/// `ClickStackSavedSearch` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedSearch {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackSavedSearchFilterResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "teamId", skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackSavedSearchWherelanguage>,
}

/// `ClickStackSavedSearchFilter` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedSearchFilter {
    pub condition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackSavedSearchFilterType>,
}

/// `ClickStackSavedSearchFilter` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSavedSearchFilter`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedSearchFilterResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackSavedSearchFilterType>,
}

/// `ClickStackSavedSearchInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedSearchInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackSavedSearchFilter>>,
    pub name: String,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackSavedSearchInputWherelanguage>,
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

/// `ClickStackSlackAPIWebhook` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSlackAPIWebhook {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ClickStackSlackAPIWebhookService>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// `ClickStackSlackWebhook` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSlackWebhook {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ClickStackSlackWebhookService>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
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

/// `ClickStackUpdateRoleRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateRoleRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub permissions: Vec<ClickStackCASLPermission>,
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

/// `ClickStackValidateWebhookResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackValidateWebhookResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ClickStackValidateWebhookResponseErrorsItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized: Option<ClickStackValidateWebhookResponseNormalized>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid: Option<bool>,
}

/// `ClickStackValidateWebhookResponseErrorsItem` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackValidateWebhookResponseErrorsItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// `ClickStackWebhookInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackWebhookInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<ClickStackWebhookInputHeaders>,
    pub name: String,
    #[serde(rename = "queryParams", skip_serializing_if = "Option::is_none")]
    pub query_params: Option<ClickStackWebhookInputQueryParams>,
    pub service: ClickStackWebhookInputService,
    pub url: String,
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

impl Default for ClickStackSource {
    fn default() -> Self {
        Self::ClickStackLogSource(ClickStackLogSource::default())
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

impl Default for ClickStackWebhook {
    fn default() -> Self {
        // Every field of this response-only union's variants is `Option<T>`,
        // so the derived `ClickStackSlackWebhook::default()` leaves `service`
        // absent and serializes to `{}` — which deserializes back through the
        // discriminator dispatch as `Unknown`, not as this variant. Naming the
        // variant's own wire value keeps the default round-tripping.
        Self::ClickStackSlackWebhook(ClickStackSlackWebhook {
            service: Some(ClickStackSlackWebhookService::default()),
            ..ClickStackSlackWebhook::default()
        })
    }
}

/// Notification channels for an alert.
pub type ClickStackAlertChannels = Vec<ClickStackAlertChannel>;
/// Tolerant notification channels returned by the API.
pub type ClickStackAlertChannelsResponse = Vec<ClickStackAlertChannelResponse>;

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
