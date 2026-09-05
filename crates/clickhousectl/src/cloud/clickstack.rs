use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::config::{deserialize_strict_config, read_config_value, read_typed_config};
use crate::cloud::output::{or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use clap::Subcommand;
use clickhouse_cloud_api::models::{
    ClickStackCreateRoleRequest, ClickStackLogSource,
    ClickStackLogSourceUsetextindexforimplicitcolumn, ClickStackMaterializedView,
    ClickStackMaterializedViewMingranularity, ClickStackMetricSource, ClickStackPromqlSource,
    ClickStackRole, ClickStackSessionSource, ClickStackSource, ClickStackSourceResponse,
    ClickStackTraceSource, ClickStackTraceSourceUsetextindexforimplicitcolumn,
    ClickStackUpdateRoleRequest,
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
}

impl ClickStackCommands {
    pub fn is_write(&self) -> bool {
        match self {
            Self::Source { command } => command.is_write(),
            Self::Role { command } => command.is_write(),
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

impl CloudClient {
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
    fn parses_source_and_role_config_commands() {
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
    }

    #[test]
    fn classifies_every_source_and_role_operation() {
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
    }
}
