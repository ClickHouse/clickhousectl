use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::config::{deserialize_strict_config, read_config_value, read_typed_config};
use crate::cloud::output::{or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use clap::Subcommand;
use clickhouse_cloud_api::models::{
    ClickStackCreateDashboardRequest, ClickStackCreateRoleRequest, ClickStackDashboardResponse,
    ClickStackLogSource, ClickStackLogSourceUsetextindexforimplicitcolumn,
    ClickStackMaterializedView, ClickStackMaterializedViewMingranularity, ClickStackMetricSource,
    ClickStackPromqlSource, ClickStackRole, ClickStackSavedSearch, ClickStackSavedSearchFilterType,
    ClickStackSavedSearchInput, ClickStackSavedSearchInputWherelanguage, ClickStackSessionSource,
    ClickStackSource, ClickStackSourceResponse, ClickStackTraceSource,
    ClickStackTraceSourceUsetextindexforimplicitcolumn, ClickStackUpdateDashboardRequest,
    ClickStackUpdateRoleRequest, ClickStackValidateDashboardResponse,
};
use serde_json::Value;
use tabled::{Table, Tabled, settings::Style};

#[derive(Subcommand)]
pub enum ClickStackCommands {
    /// Manage ClickStack data sources
    Source {
        #[command(subcommand)]
        command: SourceCommands,
    },

    /// Manage ClickStack roles
    Role {
        #[command(subcommand)]
        command: RoleCommands,
    },

    /// Manage ClickStack dashboards
    Dashboard {
        #[command(subcommand)]
        command: DashboardCommands,
    },
    /// Manage ClickStack saved searches
    SavedSearch {
        #[command(subcommand)]
        command: SavedSearchCommands,
    },
}

impl ClickStackCommands {
    pub fn is_write(&self) -> bool {
        match self {
            Self::Source { command } => command.is_write(),
            Self::Role { command } => command.is_write(),
            Self::Dashboard { command } => command.is_write(),
            Self::SavedSearch { command } => command.is_write(),
        }
    }
}

#[derive(Subcommand)]
pub enum DashboardCommands {
    /// List ClickStack dashboards
    List {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Get ClickStack dashboard details
    Get {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Dashboard ID (from `cloud clickstack dashboard list`)
        dashboard_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Create a ClickStack dashboard
    Create {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// JSON request body path, or `-` for stdin
        #[arg(long, value_name = "PATH|-", required = true)]
        config_file: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Replace a ClickStack dashboard
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  This is a full PUT replacement; include every required and desired field.
  Serialize edits to one dashboard; concurrent updates can overwrite each other.")]
    Update {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Dashboard ID (from `cloud clickstack dashboard list`)
        dashboard_id: String,
        /// Complete JSON request body path, or `-` for stdin
        #[arg(long, value_name = "PATH|-", required = true)]
        config_file: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Delete a ClickStack dashboard
    Delete {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Dashboard ID (from `cloud clickstack dashboard list`)
        dashboard_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Validate a ClickStack dashboard without saving it
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Validation never persists the dashboard.
  Uses API key authentication under the CLI's write-command policy.")]
    Validate {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// JSON request body path, or `-` for stdin
        #[arg(long, value_name = "PATH|-", required = true)]
        config_file: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl DashboardCommands {
    pub fn is_write(&self) -> bool {
        match self {
            Self::List { .. } | Self::Get { .. } => false,
            // Validation is side-effect-free, but the Cloud API exposes it as POST;
            // classify it consistently with API-key-only mutation-style endpoints.
            Self::Create { .. }
            | Self::Update { .. }
            | Self::Delete { .. }
            | Self::Validate { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum SavedSearchCommands {
    /// List ClickStack saved searches
    List {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Get ClickStack saved search details
    Get {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Saved search ID (from `cloud clickstack saved-search list`)
        saved_search_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Create a ClickStack saved search
    Create {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// JSON request body path, or `-` for stdin
        #[arg(long, value_name = "PATH|-", required = true)]
        config_file: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Replace a ClickStack saved search
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  This is a full PUT replacement; include every required and desired field.")]
    Update {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Saved search ID (from `cloud clickstack saved-search list`)
        saved_search_id: String,
        /// Complete JSON request body path, or `-` for stdin
        #[arg(long, value_name = "PATH|-", required = true)]
        config_file: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Delete a ClickStack saved search
    Delete {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Saved search ID (from `cloud clickstack saved-search list`)
        saved_search_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl SavedSearchCommands {
    pub fn is_write(&self) -> bool {
        match self {
            Self::List { .. } | Self::Get { .. } => false,
            Self::Create { .. } | Self::Update { .. } | Self::Delete { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum SourceCommands {
    /// List ClickStack sources
    List {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Get ClickStack source details
    Get {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Source ID (from `cloud clickstack source list`)
        source_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Create a ClickStack source
    Create {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// JSON request body path, or `-` for stdin
        #[arg(long, value_name = "PATH|-", required = true)]
        config_file: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Replace a ClickStack source
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  This is a full PUT replacement; include every required and desired field.")]
    Update {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Source ID (from `cloud clickstack source list`)
        source_id: String,
        /// Complete JSON request body path, or `-` for stdin
        #[arg(long, value_name = "PATH|-", required = true)]
        config_file: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Delete a ClickStack source
    Delete {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Source ID (from `cloud clickstack source list`)
        source_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl SourceCommands {
    pub fn is_write(&self) -> bool {
        match self {
            Self::List { .. } | Self::Get { .. } => false,
            Self::Create { .. } | Self::Update { .. } | Self::Delete { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum RoleCommands {
    /// List ClickStack roles
    List {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Get ClickStack role details
    Get {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Role ID (from `cloud clickstack role list`)
        role_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Create a ClickStack role
    Create {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// JSON request body path, or `-` for stdin
        #[arg(long, value_name = "PATH|-", required = true)]
        config_file: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Replace a ClickStack role
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  This is a full PUT replacement; permissions is the complete role permission set.")]
    Update {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Role ID (from `cloud clickstack role list`)
        role_id: String,
        /// Complete JSON request body path, or `-` for stdin
        #[arg(long, value_name = "PATH|-", required = true)]
        config_file: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
    /// Delete a ClickStack role
    Delete {
        /// Service ID (from `cloud service list`)
        service_id: String,
        /// Role ID (from `cloud clickstack role list`)
        role_id: String,
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl RoleCommands {
    pub fn is_write(&self) -> bool {
        match self {
            Self::List { .. } | Self::Get { .. } => false,
            Self::Create { .. } | Self::Update { .. } | Self::Delete { .. } => true,
        }
    }
}

fn field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value.as_object()?.get(name)
}

fn build_source_request(config_file: &str) -> CloudResult<ClickStackSource> {
    parse_source_request(read_config_value(config_file)?, config_file)
}

fn build_create_role_request(config_file: &str) -> CloudResult<ClickStackCreateRoleRequest> {
    read_typed_config(config_file)
}

fn build_update_role_request(config_file: &str) -> CloudResult<ClickStackUpdateRoleRequest> {
    read_typed_config(config_file)
}

fn build_create_dashboard_request(
    config_file: &str,
) -> CloudResult<ClickStackCreateDashboardRequest> {
    let value = read_config_value(config_file)?;
    validate_dashboard_unions(&value, config_file)?;
    let request = deserialize_strict_config(value, config_file)?;
    validate_create_dashboard_enums(&request, config_file)?;
    Ok(request)
}

fn build_update_dashboard_request(
    config_file: &str,
) -> CloudResult<ClickStackUpdateDashboardRequest> {
    let value = read_config_value(config_file)?;
    validate_dashboard_unions(&value, config_file)?;
    let request = deserialize_strict_config(value, config_file)?;
    validate_update_dashboard_enums(&request, config_file)?;
    Ok(request)
}

fn invalid_dashboard(source: &str, message: impl std::fmt::Display) -> CloudError {
    CloudError::new(format!(
        "invalid dashboard request body in config {source}: {message}"
    ))
}

fn require_discriminator<'a>(
    value: &'a Value,
    key: &str,
    path: &str,
    source: &str,
) -> CloudResult<&'a str> {
    field(value, key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_dashboard(source, format!("{path}.{key} must be a string")))
}

fn strict_dashboard_variant<T>(value: &Value, path: &str, source: &str) -> CloudResult<T>
where
    T: serde::de::DeserializeOwned,
{
    deserialize_strict_config::<T>(value.clone(), source)
        .map_err(|error| invalid_dashboard(source, format!("invalid {path}: {}", error.message)))
}

macro_rules! reject_unknown_enum {
    ($value:expr, $variant:path, $binding:ident, $path:expr, $source:expr) => {
        #[allow(clippy::collapsible_match)]
        match $value {
            $variant($binding) => {
                return Err(invalid_dashboard(
                    $source,
                    format!("unknown {} value `{}`", $path, $binding),
                ))
            }
            _ => {}
        }
    };
}

fn validate_number_format(
    format: &clickhouse_cloud_api::models::ClickStackNumberFormat,
    path: &str,
    source: &str,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickStackNumberFormatNumericunit, ClickStackNumberFormatOutput,
    };
    reject_unknown_enum!(
        &format.numeric_unit,
        ClickStackNumberFormatNumericunit::Unknown,
        value,
        format!("{path}.numericUnit"),
        source
    );
    reject_unknown_enum!(
        &format.output,
        ClickStackNumberFormatOutput::Unknown,
        value,
        format!("{path}.output"),
        source
    );
    Ok(())
}

fn validate_formulas(
    formulas: Option<&Vec<clickhouse_cloud_api::models::ClickStackFormula>>,
    path: &str,
    source: &str,
) -> CloudResult<()> {
    for (index, formula) in formulas.into_iter().flatten().enumerate() {
        if let Some(format) = &formula.number_format {
            validate_number_format(format, &format!("{path}[{index}].numberFormat"), source)?;
        }
    }
    Ok(())
}

fn validate_select_items(
    items: &[clickhouse_cloud_api::models::ClickStackSelectItem],
    path: &str,
    source: &str,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickStackSelectItemAggfn, ClickStackSelectItemLevel, ClickStackSelectItemMetrictype,
        ClickStackSelectItemPeriodaggfn, ClickStackSelectItemWherelanguage,
    };
    for (index, item) in items.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        reject_unknown_enum!(
            &item.agg_fn,
            ClickStackSelectItemAggfn::Unknown,
            value,
            format!("{item_path}.aggFn"),
            source
        );
        if let Some(level) = &item.level {
            reject_unknown_enum!(
                level,
                ClickStackSelectItemLevel::Unknown,
                value,
                format!("{item_path}.level"),
                source
            );
        }
        if let Some(metric_type) = &item.metric_type {
            reject_unknown_enum!(
                metric_type,
                ClickStackSelectItemMetrictype::Unknown,
                value,
                format!("{item_path}.metricType"),
                source
            );
        }
        if let Some(period_agg_fn) = &item.period_agg_fn {
            reject_unknown_enum!(
                period_agg_fn,
                ClickStackSelectItemPeriodaggfn::Unknown,
                value,
                format!("{item_path}.periodAggFn"),
                source
            );
        }
        if let Some(where_language) = &item.where_language {
            reject_unknown_enum!(
                where_language,
                ClickStackSelectItemWherelanguage::Unknown,
                value,
                format!("{item_path}.whereLanguage"),
                source
            );
        }
        if let Some(format) = &item.number_format {
            validate_number_format(format, &format!("{item_path}.numberFormat"), source)?;
        }
    }
    Ok(())
}

fn validate_on_click_filters(
    filters: Option<&Vec<clickhouse_cloud_api::models::ClickStackOnClickFilterTemplate>>,
    path: &str,
    source: &str,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::ClickStackOnClickFilterTemplateKind;
    for (index, filter) in filters.into_iter().flatten().enumerate() {
        reject_unknown_enum!(
            &filter.kind,
            ClickStackOnClickFilterTemplateKind::Unknown,
            value,
            format!("{path}[{index}].kind"),
            source
        );
    }
    Ok(())
}

fn validate_on_click(value: &Value, path: &str, source: &str) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickStackOnClickDashboard, ClickStackOnClickDashboardType,
        ClickStackOnClickDashboardWherelanguage, ClickStackOnClickExternal,
        ClickStackOnClickExternalType, ClickStackOnClickSearch, ClickStackOnClickSearchType,
        ClickStackOnClickSearchWherelanguage, ClickStackOnClickTargetIdVariant,
        ClickStackOnClickTargetIdVariantMode, ClickStackOnClickTargetTemplateVariant,
        ClickStackOnClickTargetTemplateVariantMode,
    };

    match require_discriminator(value, "type", path, source)? {
        "search" => {
            let on_click =
                strict_dashboard_variant::<ClickStackOnClickSearch>(value, path, source)?;
            reject_unknown_enum!(
                &on_click.r#type,
                ClickStackOnClickSearchType::Unknown,
                value,
                format!("{path}.type"),
                source
            );
            if let Some(language) = &on_click.where_language {
                reject_unknown_enum!(
                    language,
                    ClickStackOnClickSearchWherelanguage::Unknown,
                    value,
                    format!("{path}.whereLanguage"),
                    source
                );
            }
            validate_on_click_filters(
                on_click.filters.as_ref(),
                &format!("{path}.filters"),
                source,
            )?;
        }
        "dashboard" => {
            let on_click =
                strict_dashboard_variant::<ClickStackOnClickDashboard>(value, path, source)?;
            reject_unknown_enum!(
                &on_click.r#type,
                ClickStackOnClickDashboardType::Unknown,
                value,
                format!("{path}.type"),
                source
            );
            if let Some(language) = &on_click.where_language {
                reject_unknown_enum!(
                    language,
                    ClickStackOnClickDashboardWherelanguage::Unknown,
                    value,
                    format!("{path}.whereLanguage"),
                    source
                );
            }
            validate_on_click_filters(
                on_click.filters.as_ref(),
                &format!("{path}.filters"),
                source,
            )?;
        }
        "external" => {
            let on_click =
                strict_dashboard_variant::<ClickStackOnClickExternal>(value, path, source)?;
            reject_unknown_enum!(
                &on_click.r#type,
                ClickStackOnClickExternalType::Unknown,
                value,
                format!("{path}.type"),
                source
            );
        }
        other => {
            return Err(invalid_dashboard(
                source,
                format!("unknown {path}.type `{other}`; expected search, dashboard, or external"),
            ));
        }
    }

    if let Some(target) = field(value, "target") {
        let target_path = format!("{path}.target");
        match require_discriminator(target, "mode", &target_path, source)? {
            "id" => {
                let target = strict_dashboard_variant::<ClickStackOnClickTargetIdVariant>(
                    target,
                    &target_path,
                    source,
                )?;
                reject_unknown_enum!(
                    &target.mode,
                    ClickStackOnClickTargetIdVariantMode::Unknown,
                    value,
                    format!("{target_path}.mode"),
                    source
                );
            }
            "template" => {
                let target = strict_dashboard_variant::<ClickStackOnClickTargetTemplateVariant>(
                    target,
                    &target_path,
                    source,
                )?;
                reject_unknown_enum!(
                    &target.mode,
                    ClickStackOnClickTargetTemplateVariantMode::Unknown,
                    value,
                    format!("{target_path}.mode"),
                    source
                );
            }
            other => {
                return Err(invalid_dashboard(
                    source,
                    format!("unknown {target_path}.mode `{other}`; expected id or template"),
                ));
            }
        }
    }
    Ok(())
}

fn validate_number_color_rules(value: &Value, path: &str, source: &str) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickStackBetweenColorCondition, ClickStackBetweenColorConditionOperator,
        ClickStackChartColor, ClickStackEqualityColorCondition,
        ClickStackEqualityColorConditionOperator, ClickStackNumericColorCondition,
        ClickStackNumericColorConditionOperator,
    };
    let Some(rules) = field(value, "colorRules").and_then(Value::as_array) else {
        return Ok(());
    };
    for (index, rule) in rules.iter().enumerate() {
        let rule_path = format!("{path}.colorRules[{index}]");
        match require_discriminator(rule, "operator", &rule_path, source)? {
            "gt" | "gte" | "lt" | "lte" => {
                let condition = strict_dashboard_variant::<ClickStackNumericColorCondition>(
                    rule, &rule_path, source,
                )?;
                reject_unknown_enum!(
                    &condition.color,
                    ClickStackChartColor::Unknown,
                    value,
                    format!("{rule_path}.color"),
                    source
                );
                reject_unknown_enum!(
                    &condition.operator,
                    ClickStackNumericColorConditionOperator::Unknown,
                    value,
                    format!("{rule_path}.operator"),
                    source
                );
            }
            "between" => {
                let condition = strict_dashboard_variant::<ClickStackBetweenColorCondition>(
                    rule, &rule_path, source,
                )?;
                reject_unknown_enum!(
                    &condition.color,
                    ClickStackChartColor::Unknown,
                    value,
                    format!("{rule_path}.color"),
                    source
                );
                reject_unknown_enum!(
                    &condition.operator,
                    ClickStackBetweenColorConditionOperator::Unknown,
                    value,
                    format!("{rule_path}.operator"),
                    source
                );
            }
            "eq" | "neq" => {
                let condition = strict_dashboard_variant::<ClickStackEqualityColorCondition>(
                    rule, &rule_path, source,
                )?;
                reject_unknown_enum!(
                    &condition.color,
                    ClickStackChartColor::Unknown,
                    value,
                    format!("{rule_path}.color"),
                    source
                );
                reject_unknown_enum!(
                    &condition.operator,
                    ClickStackEqualityColorConditionOperator::Unknown,
                    value,
                    format!("{rule_path}.operator"),
                    source
                );
                let valid_value = condition.value.as_str().is_some()
                    || condition.value.as_f64().is_some_and(f64::is_finite);
                if !valid_value {
                    return Err(invalid_dashboard(
                        source,
                        format!("{rule_path}.value must be a finite number or string"),
                    ));
                }
            }
            other => {
                return Err(invalid_dashboard(
                    source,
                    format!("unknown {rule_path}.operator `{other}`"),
                ));
            }
        }
    }
    Ok(())
}

fn validate_optional_number_format(
    format: Option<&clickhouse_cloud_api::models::ClickStackNumberFormat>,
    path: &str,
    source: &str,
) -> CloudResult<()> {
    if let Some(format) = format {
        validate_number_format(format, path, source)?;
    }
    Ok(())
}

fn validate_common_builder_fields(
    formulas: Option<&Vec<clickhouse_cloud_api::models::ClickStackFormula>>,
    select: &[clickhouse_cloud_api::models::ClickStackSelectItem],
    number_format: Option<&clickhouse_cloud_api::models::ClickStackNumberFormat>,
    path: &str,
    source: &str,
) -> CloudResult<()> {
    validate_formulas(formulas, &format!("{path}.formulas"), source)?;
    validate_select_items(select, &format!("{path}.select"), source)?;
    validate_optional_number_format(number_format, &format!("{path}.numberFormat"), source)
}

fn validate_chart_config(value: &Value, path: &str, source: &str) -> CloudResult<()> {
    use clickhouse_cloud_api::models::*;

    let display_type = require_discriminator(value, "displayType", path, source)?;
    let config_type = field(value, "configType");
    if let Some(config_type) = config_type
        && config_type.as_str() != Some("sql")
    {
        let rendered = config_type
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| config_type.to_string());
        return Err(invalid_dashboard(
            source,
            format!("unknown {path}.configType `{rendered}`; expected sql or omission"),
        ));
    }

    match display_type {
        "line" => {
            if config_type.is_some() {
                let config = strict_dashboard_variant::<ClickStackLineRawSqlChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.config_type,
                    ClickStackLineRawSqlChartConfigConfigtype::Unknown,
                    unknown,
                    format!("{path}.configType"),
                    source
                );
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackLineRawSqlChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                validate_optional_number_format(
                    config.number_format.as_ref(),
                    &format!("{path}.numberFormat"),
                    source,
                )?;
            } else {
                let config = strict_dashboard_variant::<ClickStackLineBuilderChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackLineBuilderChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                validate_common_builder_fields(
                    config.formulas.as_ref(),
                    &config.select,
                    config.number_format.as_ref(),
                    path,
                    source,
                )?;
            }
        }
        "stacked_bar" => {
            if config_type.is_some() {
                let config = strict_dashboard_variant::<ClickStackBarRawSqlChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.config_type,
                    ClickStackBarRawSqlChartConfigConfigtype::Unknown,
                    unknown,
                    format!("{path}.configType"),
                    source
                );
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackBarRawSqlChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                validate_optional_number_format(
                    config.number_format.as_ref(),
                    &format!("{path}.numberFormat"),
                    source,
                )?;
            } else {
                let config = strict_dashboard_variant::<ClickStackBarBuilderChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackBarBuilderChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                validate_common_builder_fields(
                    config.formulas.as_ref(),
                    &config.select,
                    config.number_format.as_ref(),
                    path,
                    source,
                )?;
            }
        }
        "bar" => {
            if config_type.is_some() {
                let config = strict_dashboard_variant::<ClickStackCategoricalBarRawSqlChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.config_type,
                    ClickStackCategoricalBarRawSqlChartConfigConfigtype::Unknown,
                    unknown,
                    format!("{path}.configType"),
                    source
                );
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackCategoricalBarRawSqlChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                validate_optional_number_format(
                    config.number_format.as_ref(),
                    &format!("{path}.numberFormat"),
                    source,
                )?;
            } else {
                let config = strict_dashboard_variant::<ClickStackCategoricalBarBuilderChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackCategoricalBarBuilderChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                validate_common_builder_fields(
                    None,
                    &config.select,
                    config.number_format.as_ref(),
                    path,
                    source,
                )?;
            }
        }
        "table" => {
            if config_type.is_some() {
                let config = strict_dashboard_variant::<ClickStackTableRawSqlChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.config_type,
                    ClickStackTableRawSqlChartConfigConfigtype::Unknown,
                    unknown,
                    format!("{path}.configType"),
                    source
                );
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackTableRawSqlChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                validate_optional_number_format(
                    config.number_format.as_ref(),
                    &format!("{path}.numberFormat"),
                    source,
                )?;
            } else {
                let config = strict_dashboard_variant::<ClickStackTableBuilderChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackTableBuilderChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                validate_common_builder_fields(
                    config.formulas.as_ref(),
                    &config.select,
                    config.number_format.as_ref(),
                    path,
                    source,
                )?;
            }
            if let Some(on_click) = field(value, "onClick") {
                validate_on_click(on_click, &format!("{path}.onClick"), source)?;
            }
        }
        "number" => {
            if config_type.is_some() {
                let config = strict_dashboard_variant::<ClickStackNumberRawSqlChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.config_type,
                    ClickStackNumberRawSqlChartConfigConfigtype::Unknown,
                    unknown,
                    format!("{path}.configType"),
                    source
                );
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackNumberRawSqlChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                if let Some(color) = &config.color {
                    reject_unknown_enum!(
                        color,
                        ClickStackChartColor::Unknown,
                        unknown,
                        format!("{path}.color"),
                        source
                    );
                }
                validate_optional_number_format(
                    config.number_format.as_ref(),
                    &format!("{path}.numberFormat"),
                    source,
                )?;
            } else {
                let config = strict_dashboard_variant::<ClickStackNumberBuilderChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackNumberBuilderChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                if let Some(color) = &config.color {
                    reject_unknown_enum!(
                        color,
                        ClickStackChartColor::Unknown,
                        unknown,
                        format!("{path}.color"),
                        source
                    );
                }
                if let Some(background) = &config.background_chart {
                    reject_unknown_enum!(
                        &background.r#type,
                        ClickStackBackgroundChartType::Unknown,
                        unknown,
                        format!("{path}.backgroundChart.type"),
                        source
                    );
                    if let Some(color) = &background.color {
                        reject_unknown_enum!(
                            color,
                            ClickStackChartColor::Unknown,
                            unknown,
                            format!("{path}.backgroundChart.color"),
                            source
                        );
                    }
                }
                validate_common_builder_fields(
                    config.formulas.as_ref(),
                    &config.select,
                    config.number_format.as_ref(),
                    path,
                    source,
                )?;
            }
            validate_number_color_rules(value, path, source)?;
        }
        "pie" => {
            if config_type.is_some() {
                let config = strict_dashboard_variant::<ClickStackPieRawSqlChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.config_type,
                    ClickStackPieRawSqlChartConfigConfigtype::Unknown,
                    unknown,
                    format!("{path}.configType"),
                    source
                );
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackPieRawSqlChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                validate_optional_number_format(
                    config.number_format.as_ref(),
                    &format!("{path}.numberFormat"),
                    source,
                )?;
            } else {
                let config = strict_dashboard_variant::<ClickStackPieBuilderChartConfig>(
                    value, path, source,
                )?;
                reject_unknown_enum!(
                    &config.display_type,
                    ClickStackPieBuilderChartConfigDisplaytype::Unknown,
                    unknown,
                    format!("{path}.displayType"),
                    source
                );
                validate_common_builder_fields(
                    None,
                    &config.select,
                    config.number_format.as_ref(),
                    path,
                    source,
                )?;
            }
        }
        "heatmap" => {
            let config =
                strict_dashboard_variant::<ClickStackHeatmapChartConfig>(value, path, source)?;
            reject_unknown_enum!(
                &config.display_type,
                ClickStackHeatmapChartConfigDisplaytype::Unknown,
                unknown,
                format!("{path}.displayType"),
                source
            );
            if let Some(language) = &config.where_language {
                reject_unknown_enum!(
                    language,
                    ClickStackHeatmapChartConfigWherelanguage::Unknown,
                    unknown,
                    format!("{path}.whereLanguage"),
                    source
                );
            }
            for (index, item) in config.select.iter().enumerate() {
                if let Some(scale) = &item.heatmap_scale_type {
                    reject_unknown_enum!(
                        scale,
                        ClickStackHeatmapSelectItemHeatmapscaletype::Unknown,
                        unknown,
                        format!("{path}.select[{index}].heatmapScaleType"),
                        source
                    );
                }
            }
            validate_optional_number_format(
                config.number_format.as_ref(),
                &format!("{path}.numberFormat"),
                source,
            )?;
        }
        "search" => {
            let config =
                strict_dashboard_variant::<ClickStackSearchChartConfig>(value, path, source)?;
            reject_unknown_enum!(
                &config.display_type,
                ClickStackSearchChartConfigDisplaytype::Unknown,
                unknown,
                format!("{path}.displayType"),
                source
            );
            reject_unknown_enum!(
                &config.where_language,
                ClickStackSearchChartConfigWherelanguage::Unknown,
                unknown,
                format!("{path}.whereLanguage"),
                source
            );
        }
        "event_patterns" => {
            let config = strict_dashboard_variant::<ClickStackEventPatternsChartConfig>(
                value, path, source,
            )?;
            reject_unknown_enum!(
                &config.display_type,
                ClickStackEventPatternsChartConfigDisplaytype::Unknown,
                unknown,
                format!("{path}.displayType"),
                source
            );
            if let Some(language) = &config.where_language {
                reject_unknown_enum!(
                    language,
                    ClickStackEventPatternsChartConfigWherelanguage::Unknown,
                    unknown,
                    format!("{path}.whereLanguage"),
                    source
                );
            }
        }
        "markdown" => {
            let config =
                strict_dashboard_variant::<ClickStackMarkdownChartConfig>(value, path, source)?;
            reject_unknown_enum!(
                &config.display_type,
                ClickStackMarkdownChartConfigDisplaytype::Unknown,
                unknown,
                format!("{path}.displayType"),
                source
            );
        }
        other => {
            return Err(invalid_dashboard(
                source,
                format!(
                    "unknown {path}.displayType `{other}`; expected line, stacked_bar, bar, table, number, pie, heatmap, search, event_patterns, or markdown"
                ),
            ));
        }
    }
    Ok(())
}

fn validate_saved_filter_value(value: &Value, path: &str, source: &str) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickStackSavedFilterValueType, ClickStackSqlSavedFilterValue,
        ClickStackVariableSavedFilterValue, ClickStackVariableSavedFilterValueType,
    };
    match field(value, "type") {
        None => {
            let saved =
                strict_dashboard_variant::<ClickStackSqlSavedFilterValue>(value, path, source)?;
            if let Some(kind) = &saved.r#type {
                reject_unknown_enum!(
                    kind,
                    ClickStackSavedFilterValueType::Unknown,
                    unknown,
                    format!("{path}.type"),
                    source
                );
            }
            Ok(())
        }
        Some(Value::String(kind)) if kind == "sql" => {
            let saved =
                strict_dashboard_variant::<ClickStackSqlSavedFilterValue>(value, path, source)?;
            if let Some(kind) = &saved.r#type {
                reject_unknown_enum!(
                    kind,
                    ClickStackSavedFilterValueType::Unknown,
                    unknown,
                    format!("{path}.type"),
                    source
                );
            }
            Ok(())
        }
        Some(Value::String(kind)) if kind == "variable" => {
            let saved = strict_dashboard_variant::<ClickStackVariableSavedFilterValue>(
                value, path, source,
            )?;
            reject_unknown_enum!(
                &saved.r#type,
                ClickStackVariableSavedFilterValueType::Unknown,
                unknown,
                format!("{path}.type"),
                source
            );
            Ok(())
        }
        Some(other) => Err(invalid_dashboard(
            source,
            format!("unknown {path}.type `{other}`; expected sql, variable, or omission"),
        )),
    }
}

fn validate_dashboard_unions(value: &Value, source: &str) -> CloudResult<()> {
    let Some(tiles) = field(value, "tiles").and_then(Value::as_array) else {
        return Ok(());
    };
    for (index, tile) in tiles.iter().enumerate() {
        if let Some(config) = field(tile, "config") {
            validate_chart_config(config, &format!("tiles[{index}].config"), source)?;
        }
    }
    if let Some(values) = field(value, "savedFilterValues").and_then(Value::as_array) {
        for (index, saved) in values.iter().enumerate() {
            validate_saved_filter_value(saved, &format!("savedFilterValues[{index}]"), source)?;
        }
    }
    Ok(())
}

fn build_saved_search_request(config_file: &str) -> CloudResult<ClickStackSavedSearchInput> {
    let request: ClickStackSavedSearchInput = read_typed_config(config_file)?;
    validate_saved_search_closed_enums(&request, config_file)?;
    Ok(request)
}

fn validate_saved_search_closed_enums(
    request: &ClickStackSavedSearchInput,
    source: &str,
) -> CloudResult<()> {
    if let Some(ClickStackSavedSearchInputWherelanguage::Unknown(value)) = &request.where_language {
        return Err(CloudError::new(format!(
            "invalid request body in config {source}: unknown whereLanguage value `{value}`"
        )));
    }
    if let Some((index, value)) = request.filters.as_ref().and_then(|filters| {
        filters.iter().enumerate().find_map(|(index, filter)| {
            if let Some(ClickStackSavedSearchFilterType::Unknown(value)) = &filter.r#type {
                Some((index, value))
            } else {
                None
            }
        })
    }) {
        return Err(CloudError::new(format!(
            "invalid request body in config {source}: unknown filters[{index}].type value `{value}`"
        )));
    }
    Ok(())
}

fn validate_create_dashboard_enums(
    request: &ClickStackCreateDashboardRequest,
    source: &str,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickStackCreateDashboardRequestSavedquerylanguage, ClickStackFilterInputSourcemetrictype,
        ClickStackFilterInputType, ClickStackFilterInputWherelanguage,
    };
    if let Some(language) = &request.saved_query_language {
        reject_unknown_enum!(
            language,
            ClickStackCreateDashboardRequestSavedquerylanguage::Unknown,
            unknown,
            "savedQueryLanguage",
            source
        );
    }
    for (index, filter) in request.filters.iter().flatten().enumerate() {
        let path = format!("filters[{index}]");
        if let Some(metric_type) = &filter.source_metric_type {
            reject_unknown_enum!(
                metric_type,
                ClickStackFilterInputSourcemetrictype::Unknown,
                unknown,
                format!("{path}.sourceMetricType"),
                source
            );
        }
        reject_unknown_enum!(
            &filter.r#type,
            ClickStackFilterInputType::Unknown,
            unknown,
            format!("{path}.type"),
            source
        );
        if let Some(language) = &filter.where_language {
            reject_unknown_enum!(
                language,
                ClickStackFilterInputWherelanguage::Unknown,
                unknown,
                format!("{path}.whereLanguage"),
                source
            );
        }
    }
    Ok(())
}

fn validate_update_dashboard_enums(
    request: &ClickStackUpdateDashboardRequest,
    source: &str,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickStackFilterSourcemetrictype, ClickStackFilterType, ClickStackFilterWherelanguage,
        ClickStackUpdateDashboardRequestSavedquerylanguage,
    };
    if let Some(language) = &request.saved_query_language {
        reject_unknown_enum!(
            language,
            ClickStackUpdateDashboardRequestSavedquerylanguage::Unknown,
            unknown,
            "savedQueryLanguage",
            source
        );
    }
    for (index, filter) in request.filters.iter().flatten().enumerate() {
        let path = format!("filters[{index}]");
        if let Some(metric_type) = &filter.source_metric_type {
            reject_unknown_enum!(
                metric_type,
                ClickStackFilterSourcemetrictype::Unknown,
                unknown,
                format!("{path}.sourceMetricType"),
                source
            );
        }
        reject_unknown_enum!(
            &filter.r#type,
            ClickStackFilterType::Unknown,
            unknown,
            format!("{path}.type"),
            source
        );
        if let Some(language) = &filter.where_language {
            reject_unknown_enum!(
                language,
                ClickStackFilterWherelanguage::Unknown,
                unknown,
                format!("{path}.whereLanguage"),
                source
            );
        }
    }
    Ok(())
}

fn parse_source_request(value: Value, source: &str) -> CloudResult<ClickStackSource> {
    let kind = field(&value, "kind").and_then(Value::as_str);
    let request = match kind {
        Some("log") => ClickStackSource::ClickStackLogSource(deserialize_strict_config::<
            ClickStackLogSource,
        >(value, source)?),
        Some("trace") => ClickStackSource::ClickStackTraceSource(deserialize_strict_config::<
            ClickStackTraceSource,
        >(value, source)?),
        Some("metric") => ClickStackSource::ClickStackMetricSource(deserialize_strict_config::<
            ClickStackMetricSource,
        >(value, source)?),
        Some("session") => ClickStackSource::ClickStackSessionSource(deserialize_strict_config::<
            ClickStackSessionSource,
        >(value, source)?),
        Some("promql") => ClickStackSource::ClickStackPromqlSource(deserialize_strict_config::<
            ClickStackPromqlSource,
        >(value, source)?),
        Some(other) => {
            return Err(CloudError::new(format!(
                "invalid request body in config {source}: unknown source discriminator `{other}`; expected log, trace, metric, session, or promql"
            )));
        }
        None => {
            return Err(CloudError::new(format!(
                "invalid request body in config {source}: source `kind` must be one of log, trace, metric, session, or promql"
            )));
        }
    };
    validate_source_closed_enums(&request, source)?;
    Ok(request)
}

fn validate_materialized_views(
    views: Option<&Vec<ClickStackMaterializedView>>,
    source: &str,
) -> CloudResult<()> {
    if let Some((index, value)) = views.and_then(|views| {
        views.iter().enumerate().find_map(|(index, view)| {
            if let ClickStackMaterializedViewMingranularity::Unknown(value) = &view.min_granularity
            {
                Some((index, value))
            } else {
                None
            }
        })
    }) {
        return Err(CloudError::new(format!(
            "invalid request body in config {source}: unknown materializedViews[{index}].minGranularity value `{value}`"
        )));
    }
    Ok(())
}

fn validate_source_closed_enums(request: &ClickStackSource, source: &str) -> CloudResult<()> {
    match request {
        ClickStackSource::ClickStackLogSource(log) => {
            if let Some(ClickStackLogSourceUsetextindexforimplicitcolumn::Unknown(value)) =
                &log.use_text_index_for_implicit_column
            {
                return Err(CloudError::new(format!(
                    "invalid request body in config {source}: unknown useTextIndexForImplicitColumn value `{value}`"
                )));
            }
            validate_materialized_views(log.materialized_views.as_ref(), source)
        }
        ClickStackSource::ClickStackTraceSource(trace) => {
            if let Some(ClickStackTraceSourceUsetextindexforimplicitcolumn::Unknown(value)) =
                &trace.use_text_index_for_implicit_column
            {
                return Err(CloudError::new(format!(
                    "invalid request body in config {source}: unknown useTextIndexForImplicitColumn value `{value}`"
                )));
            }
            validate_materialized_views(trace.materialized_views.as_ref(), source)
        }
        ClickStackSource::ClickStackMetricSource(_)
        | ClickStackSource::ClickStackSessionSource(_)
        | ClickStackSource::ClickStackPromqlSource(_) => Ok(()),
        ClickStackSource::Unknown(_) => unreachable!("source is built from a concrete variant"),
    }
}

pub async fn run(client: &CloudClient, command: ClickStackCommands, json: bool) -> CloudResult<()> {
    match command {
        ClickStackCommands::Source { command } => run_source(client, command, json).await,
        ClickStackCommands::Role { command } => run_role(client, command, json).await,
        ClickStackCommands::Dashboard { command } => run_dashboard(client, command, json).await,
        ClickStackCommands::SavedSearch { command } => {
            run_saved_search(client, command, json).await
        }
    }
}

async fn run_dashboard(
    client: &CloudClient,
    command: DashboardCommands,
    json: bool,
) -> CloudResult<()> {
    match command {
        DashboardCommands::List { service_id, org_id } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let dashboards = client
                .click_stack_list_dashboards(&org_id, &service_id)
                .await?;
            print_dashboard_list(&dashboards, json)
        }
        DashboardCommands::Get {
            service_id,
            dashboard_id,
            org_id,
        } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let dashboard = client
                .click_stack_get_dashboard(&org_id, &service_id, &dashboard_id)
                .await?;
            print_detail(&dashboard, json)
        }
        DashboardCommands::Create {
            service_id,
            config_file,
            org_id,
        } => {
            let request = build_create_dashboard_request(&config_file)?;
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let dashboard = client
                .click_stack_create_dashboard(&org_id, &service_id, &request)
                .await?;
            print_detail(&dashboard, json)
        }
        DashboardCommands::Update {
            service_id,
            dashboard_id,
            config_file,
            org_id,
        } => {
            let request = build_update_dashboard_request(&config_file)?;
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let dashboard = client
                .click_stack_update_dashboard(&org_id, &service_id, &dashboard_id, &request)
                .await?;
            print_detail(&dashboard, json)
        }
        DashboardCommands::Delete {
            service_id,
            dashboard_id,
            org_id,
        } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            client
                .click_stack_delete_dashboard(&org_id, &service_id, &dashboard_id)
                .await?;
            print_deleted("ClickStack dashboard", &dashboard_id, json);
            Ok(())
        }
        DashboardCommands::Validate {
            service_id,
            config_file,
            org_id,
        } => {
            let request = build_create_dashboard_request(&config_file)?;
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let validation = client
                .click_stack_validate_dashboard(&org_id, &service_id, &request)
                .await?;
            print_detail(&validation, json)
        }
    }
}

async fn run_saved_search(
    client: &CloudClient,
    command: SavedSearchCommands,
    json: bool,
) -> CloudResult<()> {
    match command {
        SavedSearchCommands::List { service_id, org_id } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let searches = client
                .click_stack_list_saved_searches(&org_id, &service_id)
                .await?;
            print_saved_search_list(&searches, json)
        }
        SavedSearchCommands::Get {
            service_id,
            saved_search_id,
            org_id,
        } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let search = client
                .click_stack_get_saved_search(&org_id, &service_id, &saved_search_id)
                .await?;
            print_detail(&search, json)
        }
        SavedSearchCommands::Create {
            service_id,
            config_file,
            org_id,
        } => {
            let request = build_saved_search_request(&config_file)?;
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let search = client
                .click_stack_create_saved_search(&org_id, &service_id, &request)
                .await?;
            print_detail(&search, json)
        }
        SavedSearchCommands::Update {
            service_id,
            saved_search_id,
            config_file,
            org_id,
        } => {
            let request = build_saved_search_request(&config_file)?;
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let search = client
                .click_stack_update_saved_search(&org_id, &service_id, &saved_search_id, &request)
                .await?;
            print_detail(&search, json)
        }
        SavedSearchCommands::Delete {
            service_id,
            saved_search_id,
            org_id,
        } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            client
                .click_stack_delete_saved_search(&org_id, &service_id, &saved_search_id)
                .await?;
            print_deleted("ClickStack saved search", &saved_search_id, json);
            Ok(())
        }
    }
}

async fn run_source(client: &CloudClient, command: SourceCommands, json: bool) -> CloudResult<()> {
    match command {
        SourceCommands::List { service_id, org_id } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let sources = client
                .click_stack_list_sources(&org_id, &service_id)
                .await?;
            print_source_list(&sources, json)
        }
        SourceCommands::Get {
            service_id,
            source_id,
            org_id,
        } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let source = client
                .click_stack_get_source(&org_id, &service_id, &source_id)
                .await?;
            print_detail(&source, json)
        }
        SourceCommands::Create {
            service_id,
            config_file,
            org_id,
        } => {
            let request = build_source_request(&config_file)?;
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let source = client
                .click_stack_create_source(&org_id, &service_id, &request)
                .await?;
            print_detail(&source, json)
        }
        SourceCommands::Update {
            service_id,
            source_id,
            config_file,
            org_id,
        } => {
            let request = build_source_request(&config_file)?;
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let source = client
                .click_stack_update_source(&org_id, &service_id, &source_id, &request)
                .await?;
            print_detail(&source, json)
        }
        SourceCommands::Delete {
            service_id,
            source_id,
            org_id,
        } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            client
                .click_stack_delete_source(&org_id, &service_id, &source_id)
                .await?;
            print_deleted("ClickStack source", &source_id, json);
            Ok(())
        }
    }
}

async fn run_role(client: &CloudClient, command: RoleCommands, json: bool) -> CloudResult<()> {
    match command {
        RoleCommands::List { service_id, org_id } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let roles = client.click_stack_list_roles(&org_id, &service_id).await?;
            print_role_list(&roles, json)
        }
        RoleCommands::Get {
            service_id,
            role_id,
            org_id,
        } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let role = client
                .click_stack_get_role(&org_id, &service_id, &role_id)
                .await?;
            print_detail(&role, json)
        }
        RoleCommands::Create {
            service_id,
            config_file,
            org_id,
        } => {
            let request = build_create_role_request(&config_file)?;
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let role = client
                .click_stack_create_role(&org_id, &service_id, &request)
                .await?;
            print_detail(&role, json)
        }
        RoleCommands::Update {
            service_id,
            role_id,
            config_file,
            org_id,
        } => {
            let request = build_update_role_request(&config_file)?;
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            let role = client
                .click_stack_update_role(&org_id, &service_id, &role_id, &request)
                .await?;
            print_detail(&role, json)
        }
        RoleCommands::Delete {
            service_id,
            role_id,
            org_id,
        } => {
            let org_id = resolve_org_id(client, org_id.as_deref()).await?;
            client
                .click_stack_delete_role(&org_id, &service_id, &role_id)
                .await?;
            print_deleted("ClickStack role", &role_id, json);
            Ok(())
        }
    }
}

fn print_detail<T: serde::Serialize>(value: &T, json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        print_human(value)?;
    }
    Ok(())
}

fn print_deleted(noun: &str, id: &str, json: bool) {
    if json {
        println!("{}", serde_json::json!({ "deleted": id }));
    } else {
        println!("{noun} {id} deleted");
    }
}

fn source_summary(source: &ClickStackSourceResponse) -> (Option<&str>, Option<&str>, String) {
    macro_rules! known {
        ($source:expr) => {{
            (
                $source.id.as_deref(),
                $source.name.as_deref(),
                or_absent($source.kind.as_ref()),
            )
        }};
    }
    match source {
        ClickStackSourceResponse::ClickStackLogSource(source) => known!(source),
        ClickStackSourceResponse::ClickStackTraceSource(source) => known!(source),
        ClickStackSourceResponse::ClickStackMetricSource(source) => known!(source),
        ClickStackSourceResponse::ClickStackSessionSource(source) => known!(source),
        ClickStackSourceResponse::ClickStackPromqlSource(source) => known!(source),
        ClickStackSourceResponse::Unknown(value) => (
            field(value, "id").and_then(Value::as_str),
            field(value, "name").and_then(Value::as_str),
            or_absent(field(value, "kind").and_then(Value::as_str)),
        ),
    }
}

fn print_source_list(sources: &[ClickStackSourceResponse], json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(sources)?);
        return Ok(());
    }
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Name")]
        name: String,
        #[tabled(rename = "ID")]
        id: String,
        #[tabled(rename = "Kind")]
        kind: String,
    }
    let rows = sources
        .iter()
        .map(|source| {
            let (id, name, kind) = source_summary(source);
            Row {
                name: or_absent(name),
                id: or_absent(id),
                kind,
            }
        })
        .collect::<Vec<_>>();
    println!("{}", Table::new(rows).with(Style::rounded()));
    Ok(())
}

fn print_role_list(roles: &[ClickStackRole], json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(roles)?);
        return Ok(());
    }
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Name")]
        name: String,
        #[tabled(rename = "ID")]
        id: String,
        #[tabled(rename = "Predefined")]
        predefined: String,
        #[tabled(rename = "Description")]
        description: String,
    }
    let rows = roles
        .iter()
        .map(|role| Row {
            name: or_absent(role.name.as_deref()),
            id: or_absent(role.id.as_deref()),
            predefined: or_absent(role.is_predefined),
            description: or_absent(role.description.as_deref()),
        })
        .collect::<Vec<_>>();
    println!("{}", Table::new(rows).with(Style::rounded()));
    Ok(())
}

fn print_dashboard_list(dashboards: &[ClickStackDashboardResponse], json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(dashboards)?);
        return Ok(());
    }
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Name")]
        name: String,
        #[tabled(rename = "ID")]
        id: String,
        #[tabled(rename = "Tiles")]
        tiles: String,
        #[tabled(rename = "Tags")]
        tags: String,
    }
    let rows = dashboards
        .iter()
        .map(|dashboard| Row {
            name: or_absent(dashboard.name.as_deref()),
            id: or_absent(dashboard.id.as_deref()),
            tiles: dashboard
                .tiles
                .as_ref()
                .map(|tiles| tiles.len().to_string())
                .unwrap_or_else(|| "-".into()),
            tags: dashboard
                .tags
                .as_ref()
                .map(|tags| tags.join(", "))
                .filter(|tags| !tags.is_empty())
                .unwrap_or_else(|| "-".into()),
        })
        .collect::<Vec<_>>();
    println!("{}", Table::new(rows).with(Style::rounded()));
    Ok(())
}

fn print_saved_search_list(searches: &[ClickStackSavedSearch], json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(searches)?);
        return Ok(());
    }
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Name")]
        name: String,
        #[tabled(rename = "ID")]
        id: String,
        #[tabled(rename = "Source ID")]
        source_id: String,
        #[tabled(rename = "Language")]
        language: String,
    }
    let rows = searches
        .iter()
        .map(|search| Row {
            name: or_absent(search.name.as_deref()),
            id: or_absent(search.id.as_deref()),
            source_id: or_absent(search.source_id.as_deref()),
            language: or_absent(search.where_language.as_ref()),
        })
        .collect::<Vec<_>>();
    println!("{}", Table::new(rows).with(Style::rounded()));
    Ok(())
}

impl CloudClient {
    async fn click_stack_list_dashboards(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> CloudResult<Vec<ClickStackDashboardResponse>> {
        let response = self
            .api()
            .click_stack_list_dashboards(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_create_dashboard(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ClickStackCreateDashboardRequest,
    ) -> CloudResult<ClickStackDashboardResponse> {
        let response = self
            .api()
            .click_stack_create_dashboard(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_get_dashboard(
        &self,
        org_id: &str,
        service_id: &str,
        dashboard_id: &str,
    ) -> CloudResult<ClickStackDashboardResponse> {
        let response = self
            .api()
            .click_stack_get_dashboard(org_id, service_id, dashboard_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_update_dashboard(
        &self,
        org_id: &str,
        service_id: &str,
        dashboard_id: &str,
        request: &ClickStackUpdateDashboardRequest,
    ) -> CloudResult<ClickStackDashboardResponse> {
        let response = self
            .api()
            .click_stack_update_dashboard(org_id, service_id, dashboard_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_delete_dashboard(
        &self,
        org_id: &str,
        service_id: &str,
        dashboard_id: &str,
    ) -> CloudResult<crate::cloud::types::DeleteResponse> {
        let response = self
            .api()
            .click_stack_delete_dashboard(org_id, service_id, dashboard_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(crate::cloud::types::DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    async fn click_stack_validate_dashboard(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ClickStackCreateDashboardRequest,
    ) -> CloudResult<ClickStackValidateDashboardResponse> {
        let response = self
            .api()
            .click_stack_validate_dashboard(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_list_saved_searches(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> CloudResult<Vec<ClickStackSavedSearch>> {
        let response = self
            .api()
            .click_stack_list_saved_searches(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_create_saved_search(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ClickStackSavedSearchInput,
    ) -> CloudResult<ClickStackSavedSearch> {
        let response = self
            .api()
            .click_stack_create_saved_search(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_get_saved_search(
        &self,
        org_id: &str,
        service_id: &str,
        saved_search_id: &str,
    ) -> CloudResult<ClickStackSavedSearch> {
        let response = self
            .api()
            .click_stack_get_saved_search(org_id, service_id, saved_search_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_update_saved_search(
        &self,
        org_id: &str,
        service_id: &str,
        saved_search_id: &str,
        request: &ClickStackSavedSearchInput,
    ) -> CloudResult<ClickStackSavedSearch> {
        let response = self
            .api()
            .click_stack_update_saved_search(org_id, service_id, saved_search_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_delete_saved_search(
        &self,
        org_id: &str,
        service_id: &str,
        saved_search_id: &str,
    ) -> CloudResult<crate::cloud::types::DeleteResponse> {
        let response = self
            .api()
            .click_stack_delete_saved_search(org_id, service_id, saved_search_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(crate::cloud::types::DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    async fn click_stack_list_sources(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> CloudResult<Vec<ClickStackSourceResponse>> {
        let response = self
            .api()
            .click_stack_list_sources(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_create_source(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ClickStackSource,
    ) -> CloudResult<ClickStackSourceResponse> {
        let response = self
            .api()
            .click_stack_create_source(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_get_source(
        &self,
        org_id: &str,
        service_id: &str,
        source_id: &str,
    ) -> CloudResult<ClickStackSourceResponse> {
        let response = self
            .api()
            .click_stack_get_source(org_id, service_id, source_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_update_source(
        &self,
        org_id: &str,
        service_id: &str,
        source_id: &str,
        request: &ClickStackSource,
    ) -> CloudResult<ClickStackSourceResponse> {
        let response = self
            .api()
            .click_stack_update_source(org_id, service_id, source_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_delete_source(
        &self,
        org_id: &str,
        service_id: &str,
        source_id: &str,
    ) -> CloudResult<crate::cloud::types::DeleteResponse> {
        let response = self
            .api()
            .click_stack_delete_source(org_id, service_id, source_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(crate::cloud::types::DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    async fn click_stack_list_roles(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> CloudResult<Vec<ClickStackRole>> {
        let response = self
            .api()
            .click_stack_list_roles(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_create_role(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ClickStackCreateRoleRequest,
    ) -> CloudResult<ClickStackRole> {
        let response = self
            .api()
            .click_stack_create_role(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_get_role(
        &self,
        org_id: &str,
        service_id: &str,
        role_id: &str,
    ) -> CloudResult<ClickStackRole> {
        let response = self
            .api()
            .click_stack_get_role(org_id, service_id, role_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_update_role(
        &self,
        org_id: &str,
        service_id: &str,
        role_id: &str,
        request: &ClickStackUpdateRoleRequest,
    ) -> CloudResult<ClickStackRole> {
        let response = self
            .api()
            .click_stack_update_role(org_id, service_id, role_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    async fn click_stack_delete_role(
        &self,
        org_id: &str,
        service_id: &str,
        role_id: &str,
    ) -> CloudResult<crate::cloud::types::DeleteResponse> {
        let response = self
            .api()
            .click_stack_delete_role(org_id, service_id, role_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(crate::cloud::types::DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    fn parse_clickstack(args: &[&str]) -> ClickStackCommands {
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::Cloud(cloud) = cli.command else {
            panic!("expected cloud command")
        };
        let crate::cloud::cli::CloudCommands::ClickStack { command } = cloud.command else {
            panic!("expected clickstack command")
        };
        command
    }

    #[test]
    fn parses_clickstack_config_commands() {
        let command = parse_clickstack(&[
            "clickhousectl",
            "cloud",
            "clickstack",
            "source",
            "create",
            "svc-1",
            "--config-file",
            "-",
            "--org-id",
            "org-1",
        ]);
        let ClickStackCommands::Source {
            command:
                SourceCommands::Create {
                    config_file,
                    org_id,
                    ..
                },
        } = command
        else {
            panic!("expected source create")
        };
        assert_eq!(config_file, "-");
        assert_eq!(org_id.as_deref(), Some("org-1"));

        let command = parse_clickstack(&[
            "clickhousectl",
            "cloud",
            "clickstack",
            "role",
            "update",
            "svc-1",
            "role-1",
            "--config-file",
            "role.json",
            "--org-id",
            "org-1",
        ]);
        let ClickStackCommands::Role {
            command:
                RoleCommands::Update {
                    config_file,
                    role_id,
                    ..
                },
        } = command
        else {
            panic!("expected role update")
        };
        assert_eq!(config_file, "role.json");
        assert_eq!(role_id, "role-1");

        let command = parse_clickstack(&[
            "clickhousectl",
            "cloud",
            "clickstack",
            "saved-search",
            "update",
            "svc-1",
            "search-1",
            "--config-file",
            "-",
            "--org-id",
            "org-1",
        ]);
        let ClickStackCommands::SavedSearch {
            command:
                SavedSearchCommands::Update {
                    saved_search_id,
                    config_file,
                    org_id,
                    ..
                },
        } = command
        else {
            panic!("expected saved search update")
        };
        assert_eq!(saved_search_id, "search-1");
        assert_eq!(config_file, "-");
        assert_eq!(org_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn parses_dashboard_config_commands() {
        let command = parse_clickstack(&[
            "clickhousectl",
            "cloud",
            "clickstack",
            "dashboard",
            "validate",
            "svc-1",
            "--config-file",
            "-",
            "--org-id",
            "org-1",
        ]);
        let ClickStackCommands::Dashboard {
            command:
                DashboardCommands::Validate {
                    config_file,
                    org_id,
                    ..
                },
        } = command
        else {
            panic!("expected dashboard validate")
        };
        assert_eq!(config_file, "-");
        assert_eq!(org_id.as_deref(), Some("org-1"));

        let command = parse_clickstack(&[
            "clickhousectl",
            "cloud",
            "clickstack",
            "dashboard",
            "update",
            "svc-1",
            "dash-1",
            "--config-file",
            "dashboard.json",
        ]);
        let ClickStackCommands::Dashboard {
            command:
                DashboardCommands::Update {
                    dashboard_id,
                    config_file,
                    ..
                },
        } = command
        else {
            panic!("expected dashboard update")
        };
        assert_eq!(dashboard_id, "dash-1");
        assert_eq!(config_file, "dashboard.json");
    }

    #[test]
    fn classifies_every_clickstack_operation() {
        for (resource, operation, expected) in [
            ("source", "list", false),
            ("source", "get", false),
            ("source", "create", true),
            ("source", "update", true),
            ("source", "delete", true),
            ("role", "list", false),
            ("role", "get", false),
            ("role", "create", true),
            ("role", "update", true),
            ("role", "delete", true),
            ("saved-search", "list", false),
            ("saved-search", "get", false),
            ("saved-search", "create", true),
            ("saved-search", "update", true),
            ("saved-search", "delete", true),
        ] {
            let mut args = vec![
                "clickhousectl",
                "cloud",
                "clickstack",
                resource,
                operation,
                "svc-1",
            ];
            if matches!(operation, "get" | "update" | "delete") {
                args.push("resource-1");
            }
            if matches!(operation, "create" | "update") {
                args.extend(["--config-file", "body.json"]);
            }
            let command = parse_clickstack(&args);
            assert_eq!(command.is_write(), expected, "{resource} {operation}");
        }
    }

    #[test]
    fn classifies_every_dashboard_operation() {
        for (operation, expected) in [
            ("list", false),
            ("get", false),
            ("create", true),
            ("update", true),
            ("delete", true),
            ("validate", true),
        ] {
            let mut args = vec![
                "clickhousectl",
                "cloud",
                "clickstack",
                "dashboard",
                operation,
                "svc-1",
            ];
            if matches!(operation, "get" | "update" | "delete") {
                args.push("dash-1");
            }
            if matches!(operation, "create" | "update" | "validate") {
                args.extend(["--config-file", "dashboard.json"]);
            }
            let command = parse_clickstack(&args);
            assert_eq!(command.is_write(), expected, "dashboard {operation}");
        }
    }

    fn number_format() -> Value {
        serde_json::json!({
            "output":"number", "mantissa":2, "thousandSeparated":true,
            "average":false, "decimalBytes":false, "factor":1.0,
            "currencySymbol":"$", "numericUnit":"bytes_iec", "unit":"requests"
        })
    }

    fn dashboard_tile(name: &str, x: i64, config: Value) -> Value {
        serde_json::json!({"name":name,"x":x,"y":0,"w":4,"h":3,"config":config})
    }

    fn complete_dashboard(update: bool) -> Value {
        let select = serde_json::json!([{
            "aggFn":"quantile", "valueExpression":"Duration", "alias":"duration", "level":"0.95",
            "where":"service = 'api'", "whereLanguage":"sql", "metricName":"latency",
            "metricType":"gauge", "periodAggFn":"delta", "numberFormat":number_format()
        }]);
        let raw = |display: &str| {
            serde_json::json!({
                "configType":"sql", "displayType":display, "connectionId":"conn-1",
                "sourceId":"src-1", "sqlTemplate":"SELECT 1", "numberFormat":number_format()
            })
        };
        let mut tiles = vec![
            dashboard_tile(
                "line-builder",
                0,
                serde_json::json!({
                    "displayType":"line", "sourceId":"src-1", "select":select,
                    "groupBy":"service", "asRatio":false, "alignDateRangeToGranularity":true,
                    "fillNulls":true, "fitYAxisToData":true, "compareToPreviousPeriod":true,
                    "seriesLimit":5, "showOperandSeries":false,
                    "formulas":[{"expression":"A * 100", "alias":"percent", "numberFormat":number_format()}]
                }),
            ),
            dashboard_tile("line-sql", 1, raw("line")),
            dashboard_tile(
                "stacked-builder",
                2,
                serde_json::json!({
                    "displayType":"stacked_bar", "sourceId":"src-1", "select":select,
                    "seriesLimit":0, "showOperandSeries":true,
                    "formulas":[{"expression":"A", "numberFormat":number_format()}]
                }),
            ),
            dashboard_tile("stacked-sql", 3, raw("stacked_bar")),
            dashboard_tile(
                "bar-builder",
                4,
                serde_json::json!({
                    "displayType":"bar", "sourceId":"src-1", "select":select,
                    "groupBy":"service", "limit":10, "orderBy":"duration", "numberFormat":number_format()
                }),
            ),
            dashboard_tile("bar-sql", 5, raw("bar")),
            dashboard_tile(
                "table-builder",
                6,
                serde_json::json!({
                    "displayType":"table", "sourceId":"src-1", "select":select,
                    "showOperandSeries":false, "formulas":[{"expression":"A"}],
                    "onClick":{"type":"search","target":{"mode":"id","id":"src-2"},
                        "whereTemplate":"service = '{{service}}'","whereLanguage":"sql",
                        "filters":[{"kind":"expressionTemplate","expression":"service","template":"{{service}}"}]}
                }),
            ),
            dashboard_tile("table-sql", 7, {
                let mut config = raw("table");
                config["onClick"] = serde_json::json!({"type":"dashboard","target":{"mode":"template","template":"{{dashboard}}"},"whereLanguage":"lucene"});
                config
            }),
            dashboard_tile("table-external", 8, {
                let mut config = raw("table");
                config["onClick"] = serde_json::json!({"type":"external","urlTemplate":"https://example.com/{{id}}"});
                config
            }),
            dashboard_tile(
                "number-builder",
                9,
                serde_json::json!({
                    "displayType":"number", "sourceId":"src-1", "select":select,
                    "formulas":[{"expression":"A","alias":"total"}], "color":"chart-blue",
                    "backgroundChart":{"type":"area","color":"chart-green"},
                    "colorRules":[
                    {"operator":"gt","value":100.0,"color":"chart-error","label":"high"},
                    {"operator":"between","value":[50.0,100.0],"color":"chart-warning"},
                        {"operator":"eq","value":"ok","color":"chart-success"}
                    ], "numberFormat":number_format()
                }),
            ),
            dashboard_tile("number-sql", 10, raw("number")),
            dashboard_tile(
                "pie-builder",
                11,
                serde_json::json!({
                    "displayType":"pie", "sourceId":"src-1", "select":select,
                    "groupBy":"service", "limit":5, "orderBy":"duration", "numberFormat":number_format()
                }),
            ),
            dashboard_tile("pie-sql", 12, raw("pie")),
            dashboard_tile(
                "heatmap",
                13,
                serde_json::json!({
                    "displayType":"heatmap", "sourceId":"src-1", "where":"true",
                    "whereLanguage":"sql", "numberFormat":number_format(),
                    "select":[{"valueExpression":"Duration","countExpression":"count()","heatmapScaleType":"log"}]
                }),
            ),
            dashboard_tile(
                "search",
                14,
                serde_json::json!({
                    "displayType":"search", "sourceId":"src-1", "select":"*",
                    "where":"level:error", "whereLanguage":"lucene"
                }),
            ),
            dashboard_tile(
                "event-patterns",
                15,
                serde_json::json!({
                    "displayType":"event_patterns", "sourceId":"src-1", "select":"Body",
                    "where":"level:error", "whereLanguage":"lucene"
                }),
            ),
            dashboard_tile(
                "markdown",
                16,
                serde_json::json!({
                    "displayType":"markdown", "markdown":"# Service health"
                }),
            ),
        ];
        tiles[0]["id"] = Value::String("tile-1".into());
        tiles[0]["containerId"] = Value::String("health".into());
        tiles[0]["tabId"] = Value::String("errors".into());
        let mut filter = serde_json::json!({
            "name":"Service", "expression":"ServiceName", "sourceId":"src-1",
            "type":"QUERY_EXPRESSION", "where":"true", "whereLanguage":"sql",
            "sourceMetricType":"gauge", "appliesToSourceIds":["src-1"],
            "isBroadcastEnabled":true, "isVariableEnabled":true, "variableName":"service"
        });
        if update {
            filter["id"] = Value::String("filter-1".into());
        }
        serde_json::json!({
            "name":"Service overview", "tiles":tiles, "tags":["production","api"],
            "filters":[filter], "savedQuery":"level:error", "savedQueryLanguage":"lucene",
            "savedFilterValues":[
                {"type":"sql","condition":"ServiceName = 'api'"},
                {"type":"variable","name":"service","values":["api","worker"]}
            ],
            "containers":[{"id":"health","title":"Health","collapsed":false,"collapsible":true,
                "bordered":true,"tabs":[{"id":"errors","title":"Errors"}]}]
        })
    }

    #[test]
    fn dashboard_builders_cover_minimal_and_every_configuration_variant() {
        let directory = tempfile::tempdir().unwrap();
        let config_file = directory.path().join("dashboard.json");
        std::fs::write(&config_file, r#"{"name":"empty","tiles":[]}"#).unwrap();
        let minimal = build_create_dashboard_request(config_file.to_str().unwrap()).unwrap();
        assert_eq!(minimal.name, "empty");
        assert!(minimal.tiles.is_empty());

        let create_body = complete_dashboard(false);
        std::fs::write(&config_file, create_body.to_string()).unwrap();
        let create = build_create_dashboard_request(config_file.to_str().unwrap()).unwrap();
        assert_eq!(create.tiles.len(), 17);
        assert_eq!(
            create.filters.as_ref().unwrap()[0].variable_name.as_deref(),
            Some("service")
        );
        assert_eq!(create.saved_filter_values.as_ref().unwrap().len(), 2);
        assert_eq!(serde_json::to_value(&create).unwrap(), create_body);

        let update_body = complete_dashboard(true);
        std::fs::write(&config_file, update_body.to_string()).unwrap();
        let update = build_update_dashboard_request(config_file.to_str().unwrap()).unwrap();
        assert_eq!(update.filters.as_ref().unwrap()[0].id, "filter-1");
        assert_eq!(
            update.containers.as_ref().unwrap()[0]
                .tabs
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(serde_json::to_value(&update).unwrap(), update_body);
    }

    #[test]
    fn dashboard_builder_rejects_unknown_nested_union_enum_and_field() {
        let directory = tempfile::tempdir().unwrap();
        let config_file = directory.path().join("dashboard.json");
        for (body, expected) in [
            (
                serde_json::json!({"name":"bad","tiles":[{"name":"x","x":0,"y":0,"w":1,"h":1,"config":{"displayType":"map"}}]}),
                "displayType",
            ),
            (
                serde_json::json!({"name":"bad","tiles":[{"name":"x","x":0,"y":0,"w":1,"h":1,"config":{"displayType":"table","sourceId":"s","select":[],"onClick":{"type":"popup","target":{"mode":"id","id":"x"}}}}]}),
                "onClick.type",
            ),
            (
                serde_json::json!({"name":"bad","tiles":[{"name":"x","x":0,"y":0,"w":1,"h":1,"config":{"displayType":"heatmap","sourceId":"s","select":[{"valueExpression":"v","heatmapScaleType":"logg"}]}}]}),
                "heatmapScaleType",
            ),
            (
                serde_json::json!({"name":"bad","tiles":[{"name":"x","x":0,"y":0,"w":1,"h":1,"config":{"displayType":"line","sourceId":"s","select":[],"numberFormat":{"output":"custom","mantissa":0,"thousandSeparated":false,"average":false,"decimalBytes":false,"factor":1.0,"currencySymbol":"","numericUnit":"bytes_iec","unit":""}}}]}),
                "numberFormat.output",
            ),
            (
                serde_json::json!({"name":"bad","tiles":[{"name":"x","x":0,"y":0,"w":1,"h":1,"config":{"displayType":"number","sourceId":"s","select":[],"colorRules":[{"operator":"eq","value":{"nested":true},"color":"chart-blue"}]}}]}),
                "value must be a finite number or string",
            ),
            (
                serde_json::json!({"name":"bad","tiles":[{"name":"x","x":0,"y":0,"w":1,"h":1,"config":{"displayType":"number","sourceId":"s","select":[],"colorRules":[{"operator":"neq","value":true,"color":"chart-blue"}]}}]}),
                "value must be a finite number or string",
            ),
            (
                serde_json::json!({"name":"bad","tiles":[{"name":"x","x":0,"y":0,"w":1,"h":1,"config":{"displayType":"line","sourceId":"s","select":[{"aggFn":"count","aliass":"x"}]}}]}),
                "aliass",
            ),
            (
                serde_json::json!({"name":"bad","tiles":[],"savedFilterValues":[{"type":"variable","name":"v","values":[],"valuess":[]}]}),
                "valuess",
            ),
        ] {
            std::fs::write(&config_file, body.to_string()).unwrap();
            let error = build_create_dashboard_request(config_file.to_str().unwrap()).unwrap_err();
            assert!(error.message.contains(expected), "{error}");
        }
    }

    #[test]
    fn source_builders_accept_minimal_and_maximal_variants() {
        let directory = tempfile::tempdir().unwrap();
        let config_file = directory.path().join("source.json");
        std::fs::write(
            &config_file,
            r#"{"kind":"promql","name":"p","connection":"c","from":{"databaseName":"d","tableName":"t"},"timestampValueExpression":"ts"}"#,
        )
        .unwrap();
        let minimal = build_source_request(config_file.to_str().unwrap()).unwrap();
        assert!(matches!(
            minimal,
            ClickStackSource::ClickStackPromqlSource(_)
        ));

        let maximal = serde_json::json!({
            "kind": "log", "id": "src", "name": "logs", "section": "prod", "disabled": false,
            "connection": "conn", "from": {"databaseName": "db", "tableName": "logs"},
            "querySettings": [{"setting": "max_threads", "value": "4"}],
            "filterSettings": {"databaseName": "db", "tableName": "filters", "columns": [
                {"name": "service", "label": "Service", "valueExpression": "ServiceName", "allowAll": true}
            ]},
            "defaultTableSelectExpression": "*", "timestampValueExpression": "Timestamp",
            "serviceNameExpression": "ServiceName", "serviceVersionExpression": "ServiceVersion",
            "severityTextExpression": "SeverityText", "bodyExpression": "Body",
            "eventAttributesExpression": "Events", "resourceAttributesExpression": "ResourceAttributes",
            "displayedTimestampValueExpression": "Timestamp", "metricSourceId": "metrics",
            "traceSourceId": "traces", "traceIdExpression": "TraceId", "spanIdExpression": "SpanId",
            "implicitColumnExpression": "Attributes", "knownColumnsListExpression": "known",
            "useTextIndexForImplicitColumn": "enabled",
            "highlightedTraceAttributeExpressions": [{"sqlExpression": "TraceId", "luceneExpression": "trace.id", "alias": "trace"}],
            "highlightedRowAttributeExpressions": [{"sqlExpression": "Body", "luceneExpression": "body", "alias": "body"}],
            "materializedViews": [{"databaseName": "db", "tableName": "mv", "dimensionColumns": "ServiceName",
                "minGranularity": "1m", "minDate": "2026-01-01T00:00:00Z", "timestampColumn": "Timestamp",
                "aggregatedColumns": [{"sourceColumn": "Body", "aggFn": "count", "mvColumn": "count"}]}],
            "metadataMaterializedViews": {"keyRollupTable": "keys", "kvRollupTable": "kv", "granularity": "1m"}
        });
        std::fs::write(&config_file, maximal.to_string()).unwrap();
        let source = build_source_request(config_file.to_str().unwrap()).unwrap();
        let ClickStackSource::ClickStackLogSource(source) = source else {
            panic!("expected log")
        };
        assert_eq!(
            source.service_version_expression.as_deref(),
            Some("ServiceVersion")
        );
        let column = &source.filter_settings.unwrap().columns[0];
        assert_eq!(column.allow_all, Some(true));
        assert_eq!(column.value_expression.as_deref(), Some("ServiceName"));
    }

    #[test]
    fn every_source_variant_deserializes_as_its_request_type() {
        let bodies = [
            serde_json::json!({"kind":"log","name":"l","connection":"c","from":{"databaseName":"d","tableName":"t"},"defaultTableSelectExpression":"*","timestampValueExpression":"ts"}),
            serde_json::json!({"kind":"trace","name":"t","connection":"c","from":{"databaseName":"d","tableName":"t"},"defaultTableSelectExpression":"*","timestampValueExpression":"ts","durationExpression":"dur","durationPrecision":9,"traceIdExpression":"trace","spanIdExpression":"span","parentSpanIdExpression":"parent","spanNameExpression":"name","spanKindExpression":"kind"}),
            serde_json::json!({"kind":"metric","name":"m","connection":"c","from":{"databaseName":"d"},"metricTables":{"gauge":"g","histogram":"h","sum":"s","summary":"summary","exponential histogram":"eh"},"timestampValueExpression":"ts","resourceAttributesExpression":"r"}),
            serde_json::json!({"kind":"session","name":"s","connection":"c","from":{"databaseName":"d","tableName":"t"},"traceSourceId":"trace"}),
            serde_json::json!({"kind":"promql","name":"p","connection":"c","from":{"databaseName":"d","tableName":"t"},"timestampValueExpression":"ts"}),
        ];
        for body in bodies {
            let source = parse_source_request(body, "test").unwrap();
            assert!(!matches!(source, ClickStackSource::Unknown(_)));
        }
    }

    #[test]
    fn role_builders_cover_minimal_and_complete_permissions() {
        let directory = tempfile::tempdir().unwrap();
        let config_file = directory.path().join("role.json");
        std::fs::write(&config_file, r#"{"name":"reader","permissions":[]}"#).unwrap();
        let minimal = build_create_role_request(config_file.to_str().unwrap()).unwrap();
        assert_eq!(minimal.name, "reader");

        let maximal = r#"{"name":"ops","description":"operators","permissions":[{"action":"manage","subject":"Dashboard","inverted":true,"integration":"slack","conditions":{"teamId":"team-1"}}]}"#;
        std::fs::write(&config_file, maximal).unwrap();
        let create = build_create_role_request(config_file.to_str().unwrap()).unwrap();
        assert_eq!(create.permissions[0].integration.as_deref(), Some("slack"));
        assert_eq!(
            create.permissions[0].conditions.as_ref().unwrap()["teamId"],
            "team-1"
        );
        let update = build_update_role_request(config_file.to_str().unwrap()).unwrap();
        assert_eq!(update.permissions[0].inverted, Some(true));
    }

    #[test]
    fn saved_search_builder_preserves_minimal_maximal_and_empty_fields() {
        let directory = tempfile::tempdir().unwrap();
        let config_file = directory.path().join("saved-search.json");
        std::fs::write(&config_file, r#"{"name":"errors","sourceId":"source-1"}"#).unwrap();
        let minimal = build_saved_search_request(config_file.to_str().unwrap()).unwrap();
        assert_eq!(minimal.name, "errors");
        assert_eq!(minimal.source_id, "source-1");
        assert_eq!(minimal.select, None);
        assert_eq!(minimal.r#where, None);
        assert_eq!(minimal.where_language, None);
        assert_eq!(minimal.order_by, None);
        assert_eq!(minimal.tags, None);
        assert_eq!(minimal.filters, None);

        std::fs::write(
            &config_file,
            serde_json::json!({
                "name": "production errors",
                "sourceId": "source-2",
                "select": "Timestamp, Body",
                "where": "SeverityText = 'ERROR'",
                "whereLanguage": "sql",
                "orderBy": "Timestamp DESC",
                "tags": ["production", "errors"],
                "filters": [{"type": "sql", "condition": "ServiceName = 'api'"}]
            })
            .to_string(),
        )
        .unwrap();
        let maximal = build_saved_search_request(config_file.to_str().unwrap()).unwrap();
        assert_eq!(maximal.select.as_deref(), Some("Timestamp, Body"));
        assert_eq!(maximal.r#where.as_deref(), Some("SeverityText = 'ERROR'"));
        assert_eq!(
            maximal.where_language,
            Some(ClickStackSavedSearchInputWherelanguage::Sql)
        );
        assert_eq!(maximal.order_by.as_deref(), Some("Timestamp DESC"));
        assert_eq!(
            maximal.tags.as_deref(),
            Some(&["production".into(), "errors".into()][..])
        );
        let filter = &maximal.filters.as_ref().unwrap()[0];
        assert_eq!(filter.condition, "ServiceName = 'api'");
        assert_eq!(filter.r#type, Some(ClickStackSavedSearchFilterType::Sql));

        std::fs::write(
            &config_file,
            r#"{"name":"","sourceId":"source-3","select":"","where":"","whereLanguage":"lucene","orderBy":"","tags":[],"filters":[]}"#,
        )
        .unwrap();
        let empty = build_saved_search_request(config_file.to_str().unwrap()).unwrap();
        assert_eq!(empty.name, "");
        assert_eq!(empty.select.as_deref(), Some(""));
        assert_eq!(empty.r#where.as_deref(), Some(""));
        assert_eq!(
            empty.where_language,
            Some(ClickStackSavedSearchInputWherelanguage::Lucene)
        );
        assert_eq!(empty.order_by.as_deref(), Some(""));
        assert_eq!(empty.tags, Some(vec![]));
        assert_eq!(empty.filters, Some(vec![]));
    }

    #[test]
    fn input_validation_rejects_unknown_discriminator_and_nested_fields() {
        let error = parse_source_request(serde_json::json!({"kind":"logs","naem":"bad"}), "test")
            .unwrap_err();
        assert!(
            error
                .message
                .contains("unknown source discriminator `logs`")
        );

        let error = deserialize_strict_config::<ClickStackCreateRoleRequest>(
            serde_json::json!({"name":"r","permissions":[{"action":"read","subject":"x","conditons":{}}]}),
            "test",
        )
        .unwrap_err();
        assert!(error.message.contains("conditons"), "{error}");

        for (body, expected) in [
            (
                serde_json::json!({"name":"bad","sourceId":"s","whereLanguage":"sqlish"}),
                "unknown whereLanguage",
            ),
            (
                serde_json::json!({"name":"bad","sourceId":"s","filters":[{"type":"lucene","condition":"x"}]}),
                "unknown filters[0].type",
            ),
        ] {
            let request: ClickStackSavedSearchInput =
                deserialize_strict_config(body, "test").unwrap();
            let error = validate_saved_search_closed_enums(&request, "test").unwrap_err();
            assert!(error.message.contains(expected), "{error}");
        }

        let error = deserialize_strict_config::<ClickStackSavedSearchInput>(
            serde_json::json!({"name":"bad","sourceId":"s","orderByy":null}),
            "test",
        )
        .unwrap_err();
        assert!(error.message.contains("orderByy"), "{error}");
    }
}
