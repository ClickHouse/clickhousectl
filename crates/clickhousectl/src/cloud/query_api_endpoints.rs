use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::config::{deserialize_strict_config, read_config_value};
use crate::cloud::output::{ABSENT, or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use crate::cloud::types::DeleteResponse;
use clap::{Args, Subcommand};
use clickhouse_cloud_api::models::{
    PublicQueryApiEndpoint, PublicQueryApiEndpointRequest, QueryApiEndpointListResponse,
};
use serde::Serialize;
use serde_json::Value;
use tabled::{Table, Tabled, settings::Style};

#[derive(Args)]
pub struct QueryApiEndpointArgs {
    /// Organization ID (auto-detected only if you have one org)
    #[arg(long, global = true)]
    org_id: Option<String>,
    #[command(subcommand)]
    command: QueryApiEndpointCommands,
}

#[derive(Subcommand)]
pub enum QueryApiEndpointCommands {
    /// List Query API endpoints
    List {
        /// Service ID
        service_id: String,
        /// Cursor from pagination.nextCursor
        #[arg(long)]
        cursor: Option<String>,
        /// Maximum records per page (1–100)
        #[arg(long, allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(1..=100))]
        limit: Option<i64>,
    },
    /// Get Query API endpoint details
    Get(QueryApiEndpointTarget),
    /// Create a Query API endpoint
    Create(QueryApiEndpointCreateArgs),
    /// Update a Query API endpoint
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Supply the complete definition; omitted parameters and allowedOrigins reset to empty."
    )]
    Update {
        #[command(flatten)]
        target: QueryApiEndpointTarget,
        #[command(flatten)]
        input: QueryApiEndpointConfigArgs,
    },
    /// Delete a Query API endpoint
    Delete(QueryApiEndpointTarget),
}

#[derive(Args)]
pub struct QueryApiEndpointTarget {
    /// Service ID
    service_id: String,
    /// Query API endpoint ID
    endpoint_id: String,
}

#[derive(Args)]
pub struct QueryApiEndpointConfigArgs {
    /// Complete JSON definition (file path or - for stdin)
    #[arg(long = "config-file")]
    config_file: String,
}

#[derive(Args)]
pub struct QueryApiEndpointCreateArgs {
    /// Service ID
    service_id: String,
    #[command(flatten)]
    input: QueryApiEndpointConfigArgs,
}

impl QueryApiEndpointArgs {
    pub fn is_write(&self) -> bool {
        match &self.command {
            QueryApiEndpointCommands::List { .. } | QueryApiEndpointCommands::Get(_) => false,
            QueryApiEndpointCommands::Create(_)
            | QueryApiEndpointCommands::Update { .. }
            | QueryApiEndpointCommands::Delete(_) => true,
        }
    }
}

pub async fn run(client: &CloudClient, args: QueryApiEndpointArgs, json: bool) -> CloudResult<()> {
    match args.command {
        QueryApiEndpointCommands::Create(input) => {
            let request =
                build_query_api_endpoint_request(read_config_value(&input.input.config_file)?)?;
            let org = resolve_org_id(client, args.org_id.as_deref()).await?;
            output(
                &client
                    .create_query_api_endpoint(&org, &input.service_id, &request)
                    .await?,
                json,
            )
        }
        QueryApiEndpointCommands::Update { target, input } => {
            let request = build_query_api_endpoint_request(read_config_value(&input.config_file)?)?;
            let org = resolve_org_id(client, args.org_id.as_deref()).await?;
            output(
                &client
                    .update_query_api_endpoint(
                        &org,
                        &target.service_id,
                        &target.endpoint_id,
                        &request,
                    )
                    .await?,
                json,
            )
        }
        QueryApiEndpointCommands::List {
            service_id,
            cursor,
            limit,
        } => {
            let org = resolve_org_id(client, args.org_id.as_deref()).await?;
            let data = client
                .list_query_api_endpoints(&org, &service_id, cursor.as_deref(), limit)
                .await?;
            if json {
                output(&data, true)
            } else {
                print_endpoints(data);
                Ok(())
            }
        }
        QueryApiEndpointCommands::Get(target) => {
            let org = resolve_org_id(client, args.org_id.as_deref()).await?;
            output(
                &client
                    .get_query_api_endpoint(&org, &target.service_id, &target.endpoint_id)
                    .await?,
                json,
            )
        }
        QueryApiEndpointCommands::Delete(target) => {
            let org = resolve_org_id(client, args.org_id.as_deref()).await?;
            let data = client
                .delete_query_api_endpoint(&org, &target.service_id, &target.endpoint_id)
                .await?;
            if json {
                output(&data, true)
            } else {
                crate::cloud::output::print_line(format!(
                    "Deleted Query API endpoint {}",
                    target.endpoint_id
                ));
                Ok(())
            }
        }
    }
}

fn output<T: Serialize>(data: &T, json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(data)?);
    } else {
        print_human(data)?;
    }
    Ok(())
}

fn print_endpoints(data: QueryApiEndpointListResponse) {
    #[derive(Tabled)]
    struct Row {
        id: String,
        name: String,
        database: String,
        owner_type: String,
        url: String,
    }
    if let Some(items) = data.items {
        let rows = items.into_iter().map(|item| Row {
            id: or_absent(item.id),
            name: or_absent(item.name),
            database: or_absent(item.database),
            owner_type: or_absent(item.owner_type),
            url: or_absent(item.url),
        });
        println!("{}", Table::new(rows).with(Style::rounded()));
    } else {
        println!("Query API endpoints: {ABSENT}");
    }
    if let Some(cursor) = data
        .pagination
        .and_then(|pagination| pagination.next_cursor)
    {
        println!("Next cursor: {cursor}");
    }
}

fn build_query_api_endpoint_request(value: Value) -> CloudResult<PublicQueryApiEndpointRequest> {
    let request: PublicQueryApiEndpointRequest =
        deserialize_strict_config(value, "Query API endpoint definition")?;
    for (field, value) in [
        ("name", &request.name),
        ("sql", &request.sql),
        ("database", &request.database),
    ] {
        if value.trim().is_empty() {
            return Err(CloudError::new(format!(
                "`{field}` must contain a non-whitespace character"
            )));
        }
    }
    if request.sql.chars().count() > 4_194_304 {
        return Err(CloudError::new(
            "`sql` must contain at most 4194304 characters",
        ));
    }
    if request.api_key_ids.is_empty() {
        return Err(CloudError::new(
            "`apiKeyIds` must contain at least one API key ID",
        ));
    }
    if request.roles.is_empty() || request.roles.iter().any(String::is_empty) {
        return Err(CloudError::new(
            "`roles` must contain at least one nonempty role and no empty roles",
        ));
    }
    Ok(request)
}

impl CloudClient {
    async fn create_query_api_endpoint(
        &self,
        org: &str,
        service: &str,
        body: &PublicQueryApiEndpointRequest,
    ) -> CloudResult<PublicQueryApiEndpoint> {
        let response = self
            .api()
            .query_api_endpoint_create(org, service, body)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn update_query_api_endpoint(
        &self,
        org: &str,
        service: &str,
        endpoint: &str,
        body: &PublicQueryApiEndpointRequest,
    ) -> CloudResult<PublicQueryApiEndpoint> {
        let response = self
            .api()
            .query_api_endpoint_update(org, service, endpoint, body)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn get_query_api_endpoint(
        &self,
        org: &str,
        service: &str,
        endpoint: &str,
    ) -> CloudResult<PublicQueryApiEndpoint> {
        let response = self
            .api()
            .query_api_endpoint_get(org, service, endpoint)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn list_query_api_endpoints(
        &self,
        org: &str,
        service: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> CloudResult<QueryApiEndpointListResponse> {
        let response = self
            .api()
            .query_api_endpoint_list(org, service, cursor, limit)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn delete_query_api_endpoint(
        &self,
        org: &str,
        service: &str,
        endpoint: &str,
    ) -> CloudResult<DeleteResponse> {
        let response = self
            .api()
            .query_api_endpoint_delete(org, service, endpoint)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use clap::Parser;
    use serde_json::json;

    fn minimal() -> Value {
        json!({"name": "daily totals", "sql": "SELECT 1", "database": "default", "apiKeyIds": ["00000000-0000-4000-8000-000000000001"], "roles": ["reader"]})
    }

    #[test]
    fn query_api_endpoint_builder_minimal() {
        let request = build_query_api_endpoint_request(minimal()).unwrap();
        assert_eq!(request.name, "daily totals");
        assert_eq!(request.sql, "SELECT 1");
        assert_eq!(request.database, "default");
        assert_eq!(
            request.api_key_ids,
            vec![uuid::Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap()]
        );
        assert_eq!(request.roles, vec!["reader"]);
        assert!(request.parameters.is_none());
        assert!(request.allowed_origins.is_none());
    }

    #[test]
    fn query_api_endpoint_builder_maximal() {
        let mut value = minimal();
        value["parameters"] = json!({"kind": "page view", "limit": "10"});
        value["allowedOrigins"] = json!(["https://example.com", "https://other.example.com"]);
        value["roles"] = json!(["reader", "analyst"]);
        value["apiKeyIds"] = json!([
            "00000000-0000-4000-8000-000000000001",
            "00000000-0000-4000-8000-000000000002"
        ]);
        let request = build_query_api_endpoint_request(value).unwrap();
        assert_eq!(request.name, "daily totals");
        assert_eq!(request.sql, "SELECT 1");
        assert_eq!(request.database, "default");
        assert_eq!(request.api_key_ids.len(), 2);
        assert_eq!(
            request.api_key_ids[1].to_string(),
            "00000000-0000-4000-8000-000000000002"
        );
        assert_eq!(request.roles, vec!["reader", "analyst"]);
        let parameters = request.parameters.unwrap();
        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters["kind"], "page view");
        assert_eq!(parameters["limit"], "10");
        assert_eq!(
            request.allowed_origins.unwrap(),
            vec!["https://example.com", "https://other.example.com"]
        );
    }

    #[test]
    fn query_api_endpoint_builder_rejects_invalid_definitions() {
        for field in ["name", "sql", "database", "apiKeyIds", "roles"] {
            let mut value = minimal();
            value.as_object_mut().unwrap().remove(field);
            assert!(
                build_query_api_endpoint_request(value).is_err(),
                "missing {field}"
            );
        }
        for (field, replacement) in [
            ("name", json!(" \n")),
            ("sql", json!("\t")),
            ("database", json!("")),
            ("apiKeyIds", json!([])),
            ("apiKeyIds", json!(["invalid"])),
            ("roles", json!([])),
            ("roles", json!(["reader", ""])),
            ("parameters", json!({"limit": 10})),
            ("allowedOrigins", json!([true])),
            ("unknownField", json!(true)),
        ] {
            let mut value = minimal();
            value[field] = replacement;
            assert!(
                build_query_api_endpoint_request(value).is_err(),
                "invalid {field}"
            );
        }
    }

    #[test]
    fn query_api_endpoint_builder_counts_sql_characters() {
        let mut value = minimal();
        value["sql"] = json!("é".repeat(4_194_304));
        assert!(build_query_api_endpoint_request(value.clone()).is_ok());
        value["sql"] = json!("é".repeat(4_194_305));
        assert!(build_query_api_endpoint_request(value).is_err());
    }

    fn parse(extra: &[&str]) -> QueryApiEndpointArgs {
        let mut args = vec!["chctl", "cloud", "query-api-endpoint"];
        args.extend_from_slice(extra);
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::Cloud(cloud) = cli.command else {
            panic!("cloud command")
        };
        let CloudCommands::QueryApiEndpoint(args) = cloud.command else {
            panic!("endpoint command")
        };
        args
    }

    #[test]
    fn query_api_endpoint_clap_classifies_reads_and_writes() {
        for (args, write) in [
            (vec!["list", "svc"], false),
            (vec!["get", "svc", "endpoint"], false),
            (
                vec!["create", "svc", "--config-file", "definition.json"],
                true,
            ),
            (
                vec!["update", "svc", "endpoint", "--config-file", "-"],
                true,
            ),
            (vec!["delete", "svc", "endpoint"], true),
        ] {
            assert_eq!(parse(&args).is_write(), write);
        }
    }

    #[test]
    fn query_api_endpoint_clap_parses_config_targets_and_org_placement() {
        let args = parse(&[
            "--org-id",
            "org",
            "create",
            "svc",
            "--config-file",
            "definition.json",
        ]);
        assert_eq!(args.org_id.as_deref(), Some("org"));
        let QueryApiEndpointCommands::Create(input) = args.command else {
            panic!("create")
        };
        assert_eq!(input.service_id, "svc");
        assert_eq!(input.input.config_file, "definition.json");
        let args = parse(&[
            "update",
            "svc",
            "endpoint",
            "--config-file",
            "-",
            "--org-id",
            "org",
        ]);
        assert_eq!(args.org_id.as_deref(), Some("org"));
        let QueryApiEndpointCommands::Update { target, input } = args.command else {
            panic!("update")
        };
        assert_eq!(target.service_id, "svc");
        assert_eq!(target.endpoint_id, "endpoint");
        assert_eq!(input.config_file, "-");
        for command in ["get", "delete"] {
            let args = parse(&[command, "svc", "endpoint"]);
            let target = match args.command {
                QueryApiEndpointCommands::Get(target)
                | QueryApiEndpointCommands::Delete(target) => target,
                _ => panic!("target"),
            };
            assert_eq!(target.service_id, "svc");
            assert_eq!(target.endpoint_id, "endpoint");
        }
    }

    #[test]
    fn query_api_endpoint_clap_pagination_defaults_and_bounds() {
        let QueryApiEndpointCommands::List {
            service_id,
            cursor,
            limit,
        } = parse(&["list", "svc"]).command
        else {
            panic!("list")
        };
        assert_eq!(service_id, "svc");
        assert!(cursor.is_none());
        assert!(limit.is_none());
        for limit_value in ["1", "100"] {
            let QueryApiEndpointCommands::List { cursor, limit, .. } =
                parse(&["list", "svc", "--cursor", "next+/=", "--limit", limit_value]).command
            else {
                panic!("list")
            };
            assert_eq!(cursor.as_deref(), Some("next+/="));
            assert_eq!(limit, Some(limit_value.parse().unwrap()));
        }
        for value in ["-1", "0", "101", "not-a-number"] {
            let error = Cli::try_parse_from([
                "chctl",
                "cloud",
                "query-api-endpoint",
                "list",
                "svc",
                "--limit",
                value,
            ])
            .err()
            .unwrap();
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        }
        let negative_equals = Cli::try_parse_from([
            "chctl",
            "cloud",
            "query-api-endpoint",
            "list",
            "svc",
            "--limit=-1",
        ])
        .err()
        .unwrap();
        assert_eq!(
            negative_equals.kind(),
            clap::error::ErrorKind::ValueValidation
        );
        for args in [vec!["create", "svc"], vec!["update", "svc", "endpoint"]] {
            let mut command = vec!["chctl", "cloud", "query-api-endpoint"];
            command.extend(args);
            assert_eq!(
                Cli::try_parse_from(command).err().unwrap().kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
        }
    }
}
