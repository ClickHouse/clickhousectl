use serde::{Deserialize, Serialize};

/// Inline enum for `ClickStackAlertChannelEmail.type`.
///
/// The spec gives both alert-channel variants the same `enum: ["webhook",
/// "email"]`, so `#[default]` sits on `Email` rather than on the first value:
/// this field discriminates the `ClickStackAlertChannel` union, and defaulting
/// it to `webhook` would make `ClickStackAlertChannelEmail::default()`
/// deserialize back as the webhook variant.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelEmailType {
    #[serde(rename = "webhook")]
    Webhook,
    #[serde(rename = "email")]
    #[default]
    Email,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertChannelEmailType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Webhook => write!(f, "webhook"),
            Self::Email => write!(f, "email"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertChannelWebhook.severity`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelWebhookSeverity {
    #[serde(rename = "critical")]
    #[default]
    Critical,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "info")]
    Info,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertChannelWebhookSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertChannelWebhook.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelWebhookType {
    #[serde(rename = "webhook")]
    #[default]
    Webhook,
    #[serde(rename = "email")]
    Email,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertChannelWebhookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Webhook => write!(f, "webhook"),
            Self::Email => write!(f, "email"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertExecutionError.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertExecutionErrorType {
    #[default]
    QUERY_ERROR,
    WEBHOOK_ERROR,
    INVALID_ALERT,
    UNKNOWN,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertExecutionErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QUERY_ERROR => write!(f, "QUERY_ERROR"),
            Self::WEBHOOK_ERROR => write!(f, "WEBHOOK_ERROR"),
            Self::INVALID_ALERT => write!(f, "INVALID_ALERT"),
            Self::UNKNOWN => write!(f, "UNKNOWN"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saved_search => write!(f, "saved_search"),
            Self::Tile => write!(f, "tile"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseState {
    #[default]
    ALERT,
    OK,
    INSUFFICIENT_DATA,
    DISABLED,
    PENDING,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ALERT => write!(f, "ALERT"),
            Self::OK => write!(f, "OK"),
            Self::INSUFFICIENT_DATA => write!(f, "INSUFFICIENT_DATA"),
            Self::DISABLED => write!(f, "DISABLED"),
            Self::PENDING => write!(f, "PENDING"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
    #[serde(rename = "above_exclusive")]
    Above_exclusive,
    #[serde(rename = "below_or_equal")]
    Below_or_equal,
    #[serde(rename = "equal")]
    Equal,
    #[serde(rename = "not_equal")]
    Not_equal,
    #[serde(rename = "between")]
    Between,
    #[serde(rename = "not_between")]
    Not_between,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseThresholdtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Above => write!(f, "above"),
            Self::Below => write!(f, "below"),
            Self::Above_exclusive => write!(f, "above_exclusive"),
            Self::Below_or_equal => write!(f, "below_or_equal"),
            Self::Equal => write!(f, "equal"),
            Self::Not_equal => write!(f, "not_equal"),
            Self::Between => write!(f, "between"),
            Self::Not_between => write!(f, "not_between"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBackgroundChart.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBackgroundChartType {
    #[serde(rename = "line")]
    #[default]
    Line,
    #[serde(rename = "area")]
    Area,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBackgroundChartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line => write!(f, "line"),
            Self::Area => write!(f, "area"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBarBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarBuilderChartConfigDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBarBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stacked_bar => write!(f, "stacked_bar"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBarRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBarRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBarRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarRawSqlChartConfigDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBarRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stacked_bar => write!(f, "stacked_bar"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBetweenColorCondition.operator`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBetweenColorConditionOperator {
    #[serde(rename = "between")]
    #[default]
    Between,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBetweenColorConditionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Between => write!(f, "between"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCategoricalBarBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCategoricalBarBuilderChartConfigDisplaytype {
    #[serde(rename = "bar")]
    #[default]
    Bar,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCategoricalBarBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bar => write!(f, "bar"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCategoricalBarRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCategoricalBarRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCategoricalBarRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCategoricalBarRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCategoricalBarRawSqlChartConfigDisplaytype {
    #[serde(rename = "bar")]
    #[default]
    Bar,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCategoricalBarRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bar => write!(f, "bar"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Palette-token colors shared by ClickStack chart tiles.
///
/// Used by `ClickStackBackgroundChart`, `ClickStackNumericColorCondition`,
/// `ClickStackBetweenColorCondition`, `ClickStackEqualityColorCondition`,
/// `ClickStackNumberBuilderChartConfig`, and `ClickStackNumberRawSqlChartConfig`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackChartColor {
    #[serde(rename = "chart-blue")]
    #[default]
    Chart_blue,
    #[serde(rename = "chart-orange")]
    Chart_orange,
    #[serde(rename = "chart-red")]
    Chart_red,
    #[serde(rename = "chart-cyan")]
    Chart_cyan,
    #[serde(rename = "chart-green")]
    Chart_green,
    #[serde(rename = "chart-pink")]
    Chart_pink,
    #[serde(rename = "chart-purple")]
    Chart_purple,
    #[serde(rename = "chart-light-blue")]
    Chart_light_blue,
    #[serde(rename = "chart-brown")]
    Chart_brown,
    #[serde(rename = "chart-gray")]
    Chart_gray,
    #[serde(rename = "chart-success")]
    Chart_success,
    #[serde(rename = "chart-warning")]
    Chart_warning,
    #[serde(rename = "chart-error")]
    Chart_error,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackChartColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Chart_blue => write!(f, "chart-blue"),
            Self::Chart_orange => write!(f, "chart-orange"),
            Self::Chart_red => write!(f, "chart-red"),
            Self::Chart_cyan => write!(f, "chart-cyan"),
            Self::Chart_green => write!(f, "chart-green"),
            Self::Chart_pink => write!(f, "chart-pink"),
            Self::Chart_purple => write!(f, "chart-purple"),
            Self::Chart_light_blue => write!(f, "chart-light-blue"),
            Self::Chart_brown => write!(f, "chart-brown"),
            Self::Chart_gray => write!(f, "chart-gray"),
            Self::Chart_success => write!(f, "chart-success"),
            Self::Chart_warning => write!(f, "chart-warning"),
            Self::Chart_error => write!(f, "chart-error"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateAlertRequest.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateAlertRequestInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateAlertRequest.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateAlertRequestSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saved_search => write!(f, "saved_search"),
            Self::Tile => write!(f, "tile"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateAlertRequest.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
    #[serde(rename = "above_exclusive")]
    Above_exclusive,
    #[serde(rename = "below_or_equal")]
    Below_or_equal,
    #[serde(rename = "equal")]
    Equal,
    #[serde(rename = "not_equal")]
    Not_equal,
    #[serde(rename = "between")]
    Between,
    #[serde(rename = "not_between")]
    Not_between,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateAlertRequestThresholdtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Above => write!(f, "above"),
            Self::Below => write!(f, "below"),
            Self::Above_exclusive => write!(f, "above_exclusive"),
            Self::Below_or_equal => write!(f, "below_or_equal"),
            Self::Equal => write!(f, "equal"),
            Self::Not_equal => write!(f, "not_equal"),
            Self::Between => write!(f, "between"),
            Self::Not_between => write!(f, "not_between"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateDashboardRequest.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateDashboardRequestSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateDashboardRequestSavedquerylanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackDashboardResponse.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackDashboardResponseSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackDashboardResponseSavedquerylanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackEqualityColorCondition.operator`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackEqualityColorConditionOperator {
    #[serde(rename = "eq")]
    #[default]
    Eq,
    #[serde(rename = "neq")]
    Neq,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackEqualityColorConditionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eq => write!(f, "eq"),
            Self::Neq => write!(f, "neq"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackEventPatternsChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackEventPatternsChartConfigDisplaytype {
    #[serde(rename = "event_patterns")]
    #[default]
    Event_patterns,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackEventPatternsChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Event_patterns => write!(f, "event_patterns"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackEventPatternsChartConfig.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackEventPatternsChartConfigWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackEventPatternsChartConfigWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilter.sourceMetricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterSourcemetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterSourcemetrictype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilter.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterType {
    #[default]
    QUERY_EXPRESSION,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QUERY_EXPRESSION => write!(f, "QUERY_EXPRESSION"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilter.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilterInput.sourceMetricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterInputSourcemetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterInputSourcemetrictype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilterInput.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterInputType {
    #[default]
    QUERY_EXPRESSION,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterInputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QUERY_EXPRESSION => write!(f, "QUERY_EXPRESSION"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilterInput.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterInputWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterInputWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackGenericWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackGenericWebhookService {
    #[serde(rename = "generic")]
    #[default]
    Generic,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackGenericWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generic => write!(f, "generic"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackHeatmapChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackHeatmapChartConfigDisplaytype {
    #[serde(rename = "heatmap")]
    #[default]
    Heatmap,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackHeatmapChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Heatmap => write!(f, "heatmap"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackHeatmapChartConfig.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackHeatmapChartConfigWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackHeatmapChartConfigWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackHeatmapSelectItem.heatmapScaleType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackHeatmapSelectItemHeatmapscaletype {
    #[serde(rename = "log")]
    #[default]
    Log,
    #[serde(rename = "linear")]
    Linear,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackHeatmapSelectItemHeatmapscaletype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Log => write!(f, "log"),
            Self::Linear => write!(f, "linear"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackIncidentIOWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackIncidentIOWebhookService {
    #[serde(rename = "incidentio")]
    #[default]
    Incidentio,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackIncidentIOWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incidentio => write!(f, "incidentio"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLineBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineBuilderChartConfigDisplaytype {
    #[serde(rename = "line")]
    #[default]
    Line,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLineBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line => write!(f, "line"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLineRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLineRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLineRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineRawSqlChartConfigDisplaytype {
    #[serde(rename = "line")]
    #[default]
    Line,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLineRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line => write!(f, "line"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLogSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLogSourceKind {
    #[serde(rename = "log")]
    #[default]
    Log,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLogSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Log => write!(f, "log"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLogSource.useTextIndexForImplicitColumn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLogSourceUsetextindexforimplicitcolumn {
    #[serde(rename = "auto")]
    #[default]
    Auto,
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLogSourceUsetextindexforimplicitcolumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMarkdownChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMarkdownChartConfigDisplaytype {
    #[serde(rename = "markdown")]
    #[default]
    Markdown,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMarkdownChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Markdown => write!(f, "markdown"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMarkdownChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMarkdownChartSeriesType {
    #[serde(rename = "markdown")]
    #[default]
    Markdown,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMarkdownChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Markdown => write!(f, "markdown"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMaterializedView.minGranularity`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMaterializedViewMingranularity {
    #[serde(rename = "1s")]
    #[default]
    _1s,
    #[serde(rename = "15s")]
    _15s,
    #[serde(rename = "30s")]
    _30s,
    #[serde(rename = "1m")]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "2h")]
    _2h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    #[serde(rename = "2d")]
    _2d,
    #[serde(rename = "7d")]
    _7d,
    #[serde(rename = "30d")]
    _30d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMaterializedViewMingranularity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1s => write!(f, "1s"),
            Self::_15s => write!(f, "15s"),
            Self::_30s => write!(f, "30s"),
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_2h => write!(f, "2h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::_2d => write!(f, "2d"),
            Self::_7d => write!(f, "7d"),
            Self::_30d => write!(f, "30d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMetricSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMetricSourceKind {
    #[serde(rename = "metric")]
    #[default]
    Metric,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMetricSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Metric => write!(f, "metric"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberBuilderChartConfigDisplaytype {
    #[serde(rename = "number")]
    #[default]
    Number,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number => write!(f, "number"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesMetricdatatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesType {
    #[serde(rename = "number")]
    #[default]
    Number,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number => write!(f, "number"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberFormat.numericUnit`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberFormatNumericunit {
    #[serde(rename = "bytes_iec")]
    #[default]
    Bytes_iec,
    #[serde(rename = "bytes_si")]
    Bytes_si,
    #[serde(rename = "bits_iec")]
    Bits_iec,
    #[serde(rename = "bits_si")]
    Bits_si,
    #[serde(rename = "kibibytes")]
    Kibibytes,
    #[serde(rename = "kilobytes")]
    Kilobytes,
    #[serde(rename = "mebibytes")]
    Mebibytes,
    #[serde(rename = "megabytes")]
    Megabytes,
    #[serde(rename = "gibibytes")]
    Gibibytes,
    #[serde(rename = "gigabytes")]
    Gigabytes,
    #[serde(rename = "tebibytes")]
    Tebibytes,
    #[serde(rename = "terabytes")]
    Terabytes,
    #[serde(rename = "pebibytes")]
    Pebibytes,
    #[serde(rename = "petabytes")]
    Petabytes,
    #[serde(rename = "packets_sec")]
    Packets_sec,
    #[serde(rename = "bytes_sec_iec")]
    Bytes_sec_iec,
    #[serde(rename = "bytes_sec_si")]
    Bytes_sec_si,
    #[serde(rename = "bits_sec_iec")]
    Bits_sec_iec,
    #[serde(rename = "bits_sec_si")]
    Bits_sec_si,
    #[serde(rename = "kibibytes_sec")]
    Kibibytes_sec,
    #[serde(rename = "kibibits_sec")]
    Kibibits_sec,
    #[serde(rename = "kilobytes_sec")]
    Kilobytes_sec,
    #[serde(rename = "kilobits_sec")]
    Kilobits_sec,
    #[serde(rename = "mebibytes_sec")]
    Mebibytes_sec,
    #[serde(rename = "mebibits_sec")]
    Mebibits_sec,
    #[serde(rename = "megabytes_sec")]
    Megabytes_sec,
    #[serde(rename = "megabits_sec")]
    Megabits_sec,
    #[serde(rename = "gibibytes_sec")]
    Gibibytes_sec,
    #[serde(rename = "gibibits_sec")]
    Gibibits_sec,
    #[serde(rename = "gigabytes_sec")]
    Gigabytes_sec,
    #[serde(rename = "gigabits_sec")]
    Gigabits_sec,
    #[serde(rename = "tebibytes_sec")]
    Tebibytes_sec,
    #[serde(rename = "tebibits_sec")]
    Tebibits_sec,
    #[serde(rename = "terabytes_sec")]
    Terabytes_sec,
    #[serde(rename = "terabits_sec")]
    Terabits_sec,
    #[serde(rename = "pebibytes_sec")]
    Pebibytes_sec,
    #[serde(rename = "pebibits_sec")]
    Pebibits_sec,
    #[serde(rename = "petabytes_sec")]
    Petabytes_sec,
    #[serde(rename = "petabits_sec")]
    Petabits_sec,
    #[serde(rename = "cps")]
    Cps,
    #[serde(rename = "ops")]
    Ops,
    #[serde(rename = "rps")]
    Rps,
    #[serde(rename = "reads_sec")]
    Reads_sec,
    #[serde(rename = "wps")]
    Wps,
    #[serde(rename = "iops")]
    Iops,
    #[serde(rename = "cpm")]
    Cpm,
    #[serde(rename = "opm")]
    Opm,
    #[serde(rename = "rpm_reads")]
    Rpm_reads,
    #[serde(rename = "wpm")]
    Wpm,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberFormatNumericunit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bytes_iec => write!(f, "bytes_iec"),
            Self::Bytes_si => write!(f, "bytes_si"),
            Self::Bits_iec => write!(f, "bits_iec"),
            Self::Bits_si => write!(f, "bits_si"),
            Self::Kibibytes => write!(f, "kibibytes"),
            Self::Kilobytes => write!(f, "kilobytes"),
            Self::Mebibytes => write!(f, "mebibytes"),
            Self::Megabytes => write!(f, "megabytes"),
            Self::Gibibytes => write!(f, "gibibytes"),
            Self::Gigabytes => write!(f, "gigabytes"),
            Self::Tebibytes => write!(f, "tebibytes"),
            Self::Terabytes => write!(f, "terabytes"),
            Self::Pebibytes => write!(f, "pebibytes"),
            Self::Petabytes => write!(f, "petabytes"),
            Self::Packets_sec => write!(f, "packets_sec"),
            Self::Bytes_sec_iec => write!(f, "bytes_sec_iec"),
            Self::Bytes_sec_si => write!(f, "bytes_sec_si"),
            Self::Bits_sec_iec => write!(f, "bits_sec_iec"),
            Self::Bits_sec_si => write!(f, "bits_sec_si"),
            Self::Kibibytes_sec => write!(f, "kibibytes_sec"),
            Self::Kibibits_sec => write!(f, "kibibits_sec"),
            Self::Kilobytes_sec => write!(f, "kilobytes_sec"),
            Self::Kilobits_sec => write!(f, "kilobits_sec"),
            Self::Mebibytes_sec => write!(f, "mebibytes_sec"),
            Self::Mebibits_sec => write!(f, "mebibits_sec"),
            Self::Megabytes_sec => write!(f, "megabytes_sec"),
            Self::Megabits_sec => write!(f, "megabits_sec"),
            Self::Gibibytes_sec => write!(f, "gibibytes_sec"),
            Self::Gibibits_sec => write!(f, "gibibits_sec"),
            Self::Gigabytes_sec => write!(f, "gigabytes_sec"),
            Self::Gigabits_sec => write!(f, "gigabits_sec"),
            Self::Tebibytes_sec => write!(f, "tebibytes_sec"),
            Self::Tebibits_sec => write!(f, "tebibits_sec"),
            Self::Terabytes_sec => write!(f, "terabytes_sec"),
            Self::Terabits_sec => write!(f, "terabits_sec"),
            Self::Pebibytes_sec => write!(f, "pebibytes_sec"),
            Self::Pebibits_sec => write!(f, "pebibits_sec"),
            Self::Petabytes_sec => write!(f, "petabytes_sec"),
            Self::Petabits_sec => write!(f, "petabits_sec"),
            Self::Cps => write!(f, "cps"),
            Self::Ops => write!(f, "ops"),
            Self::Rps => write!(f, "rps"),
            Self::Reads_sec => write!(f, "reads_sec"),
            Self::Wps => write!(f, "wps"),
            Self::Iops => write!(f, "iops"),
            Self::Cpm => write!(f, "cpm"),
            Self::Opm => write!(f, "opm"),
            Self::Rpm_reads => write!(f, "rpm_reads"),
            Self::Wpm => write!(f, "wpm"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberFormat.output`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberFormatOutput {
    #[serde(rename = "currency")]
    #[default]
    Currency,
    #[serde(rename = "percent")]
    Percent,
    #[serde(rename = "byte")]
    Byte,
    #[serde(rename = "time")]
    Time,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "data_rate")]
    Data_rate,
    #[serde(rename = "throughput")]
    Throughput,
    #[serde(rename = "duration")]
    Duration,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberFormatOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Currency => write!(f, "currency"),
            Self::Percent => write!(f, "percent"),
            Self::Byte => write!(f, "byte"),
            Self::Time => write!(f, "time"),
            Self::Number => write!(f, "number"),
            Self::Data_rate => write!(f, "data_rate"),
            Self::Throughput => write!(f, "throughput"),
            Self::Duration => write!(f, "duration"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberRawSqlChartConfigDisplaytype {
    #[serde(rename = "number")]
    #[default]
    Number,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number => write!(f, "number"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumericColorCondition.operator`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumericColorConditionOperator {
    #[serde(rename = "gt")]
    #[default]
    Gt,
    #[serde(rename = "gte")]
    Gte,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "lte")]
    Lte,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumericColorConditionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gt => write!(f, "gt"),
            Self::Gte => write!(f, "gte"),
            Self::Lt => write!(f, "lt"),
            Self::Lte => write!(f, "lte"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickDashboard.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickDashboardType {
    #[serde(rename = "dashboard")]
    #[default]
    Dashboard,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickDashboardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dashboard => write!(f, "dashboard"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickDashboard.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickDashboardWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickDashboardWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickExternal.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickExternalType {
    #[serde(rename = "external")]
    #[default]
    External,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickExternalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::External => write!(f, "external"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickFilterTemplate.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickFilterTemplateKind {
    #[serde(rename = "expressionTemplate")]
    #[default]
    ExpressionTemplate,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickFilterTemplateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExpressionTemplate => write!(f, "expressionTemplate"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickSearch.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickSearchType {
    #[serde(rename = "search")]
    #[default]
    Search,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickSearchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Search => write!(f, "search"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickSearch.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickSearchWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickSearchWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickTargetIdVariant.mode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickTargetIdVariantMode {
    #[serde(rename = "id")]
    #[default]
    Id,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickTargetIdVariantMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id => write!(f, "id"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickTargetTemplateVariant.mode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickTargetTemplateVariantMode {
    #[serde(rename = "template")]
    #[default]
    Template,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickTargetTemplateVariantMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Template => write!(f, "template"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPagerDutyAPIWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPagerDutyAPIWebhookService {
    #[serde(rename = "pagerduty_api")]
    #[default]
    Pagerduty_api,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPagerDutyAPIWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pagerduty_api => write!(f, "pagerduty_api"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPieBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieBuilderChartConfigDisplaytype {
    #[serde(rename = "pie")]
    #[default]
    Pie,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPieBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pie => write!(f, "pie"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPieRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPieRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPieRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieRawSqlChartConfigDisplaytype {
    #[serde(rename = "pie")]
    #[default]
    Pie,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPieRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pie => write!(f, "pie"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPromqlSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPromqlSourceKind {
    #[serde(rename = "promql")]
    #[default]
    Promql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPromqlSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Promql => write!(f, "promql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSavedFilterValue.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSavedFilterValueType {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSavedFilterValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSavedSearchFilter.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSavedSearchFilterType {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSavedSearchFilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSavedSearchInput.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSavedSearchInputWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSavedSearchInputWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSavedSearch.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSavedSearchWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSavedSearchWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartConfigDisplaytype {
    #[serde(rename = "search")]
    #[default]
    Search,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Search => write!(f, "search"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartConfig.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartConfigWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartConfigWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartSeriesType {
    #[serde(rename = "search")]
    #[default]
    Search,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Search => write!(f, "search"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.level`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemLevel {
    #[serde(rename = "0.5")]
    #[default]
    _0_5,
    #[serde(rename = "0.9")]
    _0_9,
    #[serde(rename = "0.95")]
    _0_95,
    #[serde(rename = "0.99")]
    _0_99,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_0_5 => write!(f, "0.5"),
            Self::_0_9 => write!(f, "0.9"),
            Self::_0_95 => write!(f, "0.95"),
            Self::_0_99 => write!(f, "0.99"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.metricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemMetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemMetrictype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.periodAggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemPeriodaggfn {
    #[serde(rename = "delta")]
    #[default]
    Delta,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemPeriodaggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Delta => write!(f, "delta"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSessionSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSessionSourceKind {
    #[serde(rename = "session")]
    #[default]
    Session,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSessionSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session => write!(f, "session"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSlackAPIWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSlackAPIWebhookService {
    #[serde(rename = "slack_api")]
    #[default]
    Slack_api,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSlackAPIWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack_api => write!(f, "slack_api"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSlackWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSlackWebhookService {
    #[serde(rename = "slack")]
    #[default]
    Slack,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSlackWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack => write!(f, "slack"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableBuilderChartConfigDisplaytype {
    #[serde(rename = "table")]
    #[default]
    Table,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesMetricdatatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.sortOrder`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesSortorder {
    #[serde(rename = "desc")]
    #[default]
    Desc,
    #[serde(rename = "asc")]
    Asc,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesSortorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Desc => write!(f, "desc"),
            Self::Asc => write!(f, "asc"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesType {
    #[serde(rename = "table")]
    #[default]
    Table,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableRawSqlChartConfigDisplaytype {
    #[serde(rename = "table")]
    #[default]
    Table,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
    #[serde(rename = "line")]
    Line,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stacked_bar => write!(f, "stacked_bar"),
            Self::Line => write!(f, "line"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesMetricdatatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesType {
    #[serde(rename = "time")]
    #[default]
    Time,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Time => write!(f, "time"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTraceSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTraceSourceKind {
    #[serde(rename = "trace")]
    #[default]
    Trace,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTraceSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => write!(f, "trace"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTraceSource.useTextIndexForImplicitColumn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTraceSourceUsetextindexforimplicitcolumn {
    #[serde(rename = "auto")]
    #[default]
    Auto,
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTraceSourceUsetextindexforimplicitcolumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateAlertRequest.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateAlertRequestInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateAlertRequest.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateAlertRequestSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saved_search => write!(f, "saved_search"),
            Self::Tile => write!(f, "tile"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateAlertRequest.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
    #[serde(rename = "above_exclusive")]
    Above_exclusive,
    #[serde(rename = "below_or_equal")]
    Below_or_equal,
    #[serde(rename = "equal")]
    Equal,
    #[serde(rename = "not_equal")]
    Not_equal,
    #[serde(rename = "between")]
    Between,
    #[serde(rename = "not_between")]
    Not_between,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateAlertRequestThresholdtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Above => write!(f, "above"),
            Self::Below => write!(f, "below"),
            Self::Above_exclusive => write!(f, "above_exclusive"),
            Self::Below_or_equal => write!(f, "below_or_equal"),
            Self::Equal => write!(f, "equal"),
            Self::Not_equal => write!(f, "not_equal"),
            Self::Between => write!(f, "between"),
            Self::Not_between => write!(f, "not_between"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateDashboardRequest.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateDashboardRequestSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateDashboardRequestSavedquerylanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackWebhookInput.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackWebhookInputService {
    #[serde(rename = "slack")]
    #[default]
    Slack,
    #[serde(rename = "incidentio")]
    Incidentio,
    #[serde(rename = "generic")]
    Generic,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackWebhookInputService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack => write!(f, "slack"),
            Self::Incidentio => write!(f, "incidentio"),
            Self::Generic => write!(f, "generic"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}
