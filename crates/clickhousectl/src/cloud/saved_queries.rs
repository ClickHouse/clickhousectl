use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{or_absent, print_human, print_line};
use crate::cloud::shared::{NameSelector, resolve_org_id, select_named_id};
use crate::cloud::types::DeleteResponse;
use crate::failure::{ApiFailure, FailureKind};
use clap::{Args, Subcommand};
use clickhouse_cloud_api::models::{
    PublicSavedQuery, PublicSavedQueryListItem, PublicSavedQueryRequest,
};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use tabled::{Table, Tabled, settings::Style};

/// Upper bound the API enforces on `sql`, counted in characters.
const MAX_SQL_CHARS: usize = 4_194_304;

#[derive(Args)]
pub struct SavedQueryArgs {
    #[command(subcommand)]
    command: SavedQueryCommands,
}

#[derive(Subcommand)]
pub enum SavedQueryCommands {
    /// List saved queries
    List {
        /// Service ID
        service_id: String,
        /// Cursor from nextCursor
        #[arg(long)]
        cursor: Option<String>,
        /// Maximum records per page (1–100)
        #[arg(long, allow_negative_numbers = true, value_parser = clap::value_parser!(i64).range(1..=100))]
        limit: Option<i64>,
    },
    /// Get saved query details
    Get(SavedQueryTarget),
    /// Create a saved query
    Create {
        /// Service ID
        service_id: String,
        /// Saved query name, unique within the service
        #[arg(long)]
        name: String,
        #[command(flatten)]
        body: SavedQueryBodyArgs,
    },
    /// Update a saved query
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Replaces the whole saved query; pass --new-name with the current name to keep it.\n  Omitted --param values reset to empty; nothing is merged from the stored query."
    )]
    Update {
        #[command(flatten)]
        target: SavedQueryTarget,
        /// Saved query name to store, unique within the service
        #[arg(long = "new-name", id = "new_name", value_name = "NEW_NAME")]
        name: String,
        #[command(flatten)]
        body: SavedQueryBodyArgs,
    },
    /// Delete a saved query
    Delete(SavedQueryTarget),
}

#[derive(Args)]
pub struct SavedQueryTarget {
    /// Service ID
    service_id: String,
    /// Saved query ID
    #[command(flatten)]
    query_id: NameSelector,
}

#[derive(Args)]
pub struct SavedQueryBodyArgs {
    /// SQL text of the saved query
    #[arg(
        long,
        required_unless_present = "sql_file",
        conflicts_with = "sql_file"
    )]
    sql: Option<String>,
    /// File containing the SQL ("-" reads stdin)
    #[arg(long, value_name = "PATH")]
    sql_file: Option<String>,
    /// Database the saved query runs against
    #[arg(long)]
    database: String,
    /// Default query parameter (repeatable)
    #[arg(long = "param", value_name = "KEY=VALUE", value_parser = parse_param)]
    params: Vec<(String, String)>,
}

impl SavedQueryArgs {
    pub fn is_write(&self) -> bool {
        match &self.command {
            SavedQueryCommands::List { .. } | SavedQueryCommands::Get(_) => false,
            SavedQueryCommands::Create { .. }
            | SavedQueryCommands::Update { .. }
            | SavedQueryCommands::Delete(_) => true,
        }
    }
}

fn parse_param(raw: &str) -> Result<(String, String), String> {
    let (key, value) = raw
        .split_once('=')
        .ok_or_else(|| format!("invalid parameter '{raw}': expected KEY=VALUE"))?;
    if key.is_empty() {
        return Err(format!("invalid parameter '{raw}': KEY must not be empty"));
    }
    Ok((key.to_owned(), value.to_owned()))
}

pub async fn run(client: &CloudClient, args: SavedQueryArgs, json: bool) -> CloudResult<()> {
    match args.command {
        SavedQueryCommands::Create {
            service_id,
            name,
            body,
        } => {
            let request = build_saved_query_request(name, read_sql(&body)?, body)?;
            let org = resolve_org_id(client).await?;
            output(
                &client
                    .create_saved_query(&org, &service_id, &request)
                    .await?,
                json,
            )
        }
        SavedQueryCommands::Update { target, name, body } => {
            let request = build_saved_query_request(name, read_sql(&body)?, body)?;
            let query_id = resolve_query_id(client, &target).await?;
            let org = resolve_org_id(client).await?;
            output(
                &client
                    .update_saved_query(&org, &target.service_id, &query_id, &request)
                    .await?,
                json,
            )
        }
        SavedQueryCommands::List {
            service_id,
            cursor,
            limit,
        } => {
            let org = resolve_org_id(client).await?;
            let page = client
                .list_saved_queries_page(&org, &service_id, cursor.as_deref(), limit)
                .await?;
            if json {
                output(&page, true)
            } else {
                print_saved_queries(page);
                Ok(())
            }
        }
        SavedQueryCommands::Get(target) => {
            let query_id = resolve_query_id(client, &target).await?;
            let org = resolve_org_id(client).await?;
            output(
                &client
                    .get_saved_query(&org, &target.service_id, &query_id)
                    .await?,
                json,
            )
        }
        SavedQueryCommands::Delete(target) => {
            let query_id = resolve_query_id(client, &target).await?;
            let org = resolve_org_id(client).await?;
            let data = client
                .delete_saved_query(&org, &target.service_id, &query_id)
                .await?;
            if json {
                output(&data, true)
            } else {
                print_line(format!("Deleted saved query {query_id}"));
                Ok(())
            }
        }
    }
}

/// Read the SQL body from `--sql` or `--sql-file` (clap guarantees exactly one).
fn read_sql(body: &SavedQueryBodyArgs) -> CloudResult<String> {
    use std::io::Read as _;

    if let Some(sql) = &body.sql {
        return Ok(sql.clone());
    }
    let path = body
        .sql_file
        .as_deref()
        .ok_or_else(|| CloudError::usage("supply --sql or --sql-file"))?;
    let (bytes, source) = if path == "-" {
        let mut bytes = Vec::new();
        std::io::stdin().read_to_end(&mut bytes).map_err(|error| {
            CloudError::new(format!("failed to read SQL from stdin: {error}"))
                .with_failure(ApiFailure::new(FailureKind::Io))
        })?;
        (bytes, "stdin".to_owned())
    } else {
        let bytes = std::fs::read(path).map_err(|error| {
            CloudError::new(format!("failed to read SQL file {path}: {error}"))
                .with_failure(ApiFailure::new(FailureKind::Io))
        })?;
        (bytes, format!("SQL file {path}"))
    };
    String::from_utf8(bytes)
        .map_err(|_| CloudError::usage(format!("SQL from {source} is not valid UTF-8")))
}

/// Build the full-replacement body shared by create and update.
fn build_saved_query_request(
    name: String,
    sql: String,
    body: SavedQueryBodyArgs,
) -> CloudResult<PublicSavedQueryRequest> {
    for (flag, value) in [("name", &name), ("sql", &sql), ("database", &body.database)] {
        if value.trim().is_empty() {
            return Err(CloudError::usage(format!(
                "{flag} must contain a non-whitespace character"
            )));
        }
    }
    if sql.chars().count() > MAX_SQL_CHARS {
        return Err(CloudError::usage(format!(
            "sql must contain at most {MAX_SQL_CHARS} characters"
        )));
    }
    let parameters = if body.params.is_empty() {
        None
    } else {
        let mut map = BTreeMap::new();
        for (key, value) in body.params {
            if map.contains_key(&key) {
                return Err(CloudError::usage(format!(
                    "--param '{key}' was given more than once"
                )));
            }
            map.insert(key, value);
        }
        Some(map)
    };
    Ok(PublicSavedQueryRequest {
        name,
        sql,
        database: body.database,
        parameters,
    })
}

async fn resolve_query_id(client: &CloudClient, target: &SavedQueryTarget) -> CloudResult<String> {
    let name = match (&target.query_id.id, &target.query_id.name) {
        (Some(id), None) => return Ok(id.clone()),
        (None, Some(name)) => name,
        _ => {
            return Err(CloudError::usage(
                "supply exactly one positional saved query ID or --name",
            ));
        }
    };
    let org = resolve_org_id(client).await?;
    let mut rows = Vec::new();
    let mut cursor: Option<String> = None;
    let mut seen_cursors = HashSet::new();
    let mut seen_ids = HashSet::new();
    let mut total_count = None;
    loop {
        let page = client
            .list_saved_queries_page(&org, &target.service_id, cursor.as_deref(), Some(100))
            .await?;
        for item in &page.result {
            let id = item
                .id
                .ok_or_else(|| CloudError::new("saved query list contains a missing ID"))?;
            if !seen_ids.insert(id) {
                return Err(CloudError::new(
                    "saved query pagination repeated a resource ID",
                ));
            }
        }
        rows.extend(page.result);
        if let Some(total) = page.total_count {
            if total < 0 || total_count.is_some_and(|previous| previous != total) {
                return Err(CloudError::new(
                    "saved query list returned inconsistent totalCount",
                ));
            }
            total_count = Some(total);
        }
        match page.next_cursor {
            None => {
                if total_count.is_some_and(|total| total as usize != rows.len()) {
                    return Err(CloudError::new(
                        "saved query list ended before all records were received",
                    ));
                }
                break;
            }
            Some(next) if next.is_empty() || !seen_cursors.insert(next.clone()) => {
                return Err(CloudError::new(
                    "saved query list returned an empty or repeated cursor",
                ));
            }
            Some(next) => cursor = Some(next),
        }
    }
    select_named_id(
        "saved query",
        name,
        rows.iter().map(|r| (r.name.as_deref(), r.id.as_ref())),
    )
}

fn output<T: Serialize>(data: &T, json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(data)?);
    } else {
        print_human(data)?;
    }
    Ok(())
}

/// One list page, keeping the envelope's continuation metadata for `--json`.
#[derive(Serialize)]
struct SavedQueryPage {
    result: Vec<PublicSavedQueryListItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<i64>,
    #[serde(rename = "totalCount", skip_serializing_if = "Option::is_none")]
    total_count: Option<i64>,
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

fn print_saved_queries(page: SavedQueryPage) {
    #[derive(Tabled)]
    struct Row {
        id: String,
        name: String,
        database: String,
    }
    if page.result.is_empty() {
        println!("No saved queries found");
    } else {
        let rows = page.result.into_iter().map(|item| Row {
            id: or_absent(item.id),
            name: or_absent(item.name),
            database: or_absent(item.database),
        });
        println!("{}", Table::new(rows).with(Style::rounded()));
    }
    if let Some(cursor) = page.next_cursor {
        println!("Next cursor: {cursor}");
    }
}

impl CloudClient {
    async fn create_saved_query(
        &self,
        org: &str,
        service: &str,
        body: &PublicSavedQueryRequest,
    ) -> CloudResult<PublicSavedQuery> {
        let response = self
            .api()
            .saved_query_create(org, service, body)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }

    async fn update_saved_query(
        &self,
        org: &str,
        service: &str,
        query: &str,
        body: &PublicSavedQueryRequest,
    ) -> CloudResult<PublicSavedQuery> {
        let response = self
            .api()
            .saved_query_update(org, service, query, body)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }

    async fn get_saved_query(
        &self,
        org: &str,
        service: &str,
        query: &str,
    ) -> CloudResult<PublicSavedQuery> {
        let response = self
            .api()
            .saved_query_get(org, service, query)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }

    async fn list_saved_queries_page(
        &self,
        org: &str,
        service: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> CloudResult<SavedQueryPage> {
        let mut response = self
            .api()
            .saved_query_list(org, service, cursor, limit)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        let limit = response.limit;
        let total_count = response.total_count;
        let next_cursor = response.next_cursor.take();
        Ok(SavedQueryPage {
            result: Self::unwrap_response(response)?,
            limit,
            total_count,
            next_cursor,
        })
    }

    async fn delete_saved_query(
        &self,
        org: &str,
        service: &str,
        query: &str,
    ) -> CloudResult<DeleteResponse> {
        let response = self
            .api()
            .saved_query_delete(org, service, query)
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
    use crate::cloud::client::CloudErrorKind;
    use clap::Parser;
    use clap::error::ErrorKind;

    fn body(database: &str, params: &[(&str, &str)]) -> SavedQueryBodyArgs {
        SavedQueryBodyArgs {
            sql: None,
            sql_file: None,
            database: database.to_owned(),
            params: params
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        }
    }

    #[test]
    fn saved_query_builder_minimal() {
        let request =
            build_saved_query_request("daily".into(), "SELECT 1".into(), body("default", &[]))
                .unwrap();
        assert_eq!(request.name, "daily");
        assert_eq!(request.sql, "SELECT 1");
        assert_eq!(request.database, "default");
        assert!(request.parameters.is_none());
    }

    #[test]
    fn saved_query_builder_maximal() {
        let request = build_saved_query_request(
            "daily totals".into(),
            "SELECT * FROM t WHERE d = {day:Date} AND r = {region:String}".into(),
            body(
                "analytics",
                &[("day", "2026-09-30"), ("region", "eu=west"), ("empty", "")],
            ),
        )
        .unwrap();
        assert_eq!(request.name, "daily totals");
        assert_eq!(
            request.sql,
            "SELECT * FROM t WHERE d = {day:Date} AND r = {region:String}"
        );
        assert_eq!(request.database, "analytics");
        let parameters = request.parameters.unwrap();
        assert_eq!(parameters.len(), 3);
        assert_eq!(parameters["day"], "2026-09-30");
        assert_eq!(parameters["region"], "eu=west");
        assert_eq!(parameters["empty"], "");
    }

    #[test]
    fn saved_query_builder_rejects_invalid_input_as_usage() {
        for (name, sql, database, params) in [
            (" ", "SELECT 1", "default", vec![]),
            ("daily", "\n\t", "default", vec![]),
            ("daily", "SELECT 1", "", vec![]),
            (
                "daily",
                "SELECT 1",
                "default",
                vec![("id", "1"), ("id", "2")],
            ),
        ] {
            let error = build_saved_query_request(name.into(), sql.into(), body(database, &params))
                .unwrap_err();
            assert_eq!(error.kind, CloudErrorKind::Usage, "{}", error.message);
        }
    }

    #[test]
    fn saved_query_builder_counts_sql_characters() {
        let ok = "é".repeat(MAX_SQL_CHARS);
        assert!(build_saved_query_request("q".into(), ok, body("default", &[])).is_ok());
        let too_long = "é".repeat(MAX_SQL_CHARS + 1);
        assert!(build_saved_query_request("q".into(), too_long, body("default", &[])).is_err());
    }

    #[test]
    fn saved_query_param_parser_rejects_missing_equals_and_empty_key() {
        assert_eq!(parse_param("k=v").unwrap(), ("k".into(), "v".into()));
        assert_eq!(parse_param("k=").unwrap(), ("k".into(), String::new()));
        assert_eq!(parse_param("k=a=b").unwrap(), ("k".into(), "a=b".into()));
        assert!(parse_param("novalue").is_err());
        assert!(parse_param("=v").is_err());
    }

    fn try_parse(extra: &[&str]) -> Result<Cli, clap::Error> {
        let mut args = vec!["chctl", "cloud", "saved-query"];
        args.extend_from_slice(extra);
        Cli::try_parse_from(args)
    }

    fn parse(extra: &[&str]) -> SavedQueryArgs {
        let cli = try_parse(extra).unwrap();
        let Commands::Cloud(cloud) = cli.command else {
            panic!("cloud command")
        };
        crate::cloud::cli::tests::assert_org_selector(&cloud, extra);
        let CloudCommands::SavedQuery(args) = cloud.command else {
            panic!("saved-query command")
        };
        args
    }

    #[test]
    fn saved_query_clap_classifies_reads_and_writes() {
        for (args, write) in [
            (vec!["list", "svc"], false),
            (vec!["get", "svc", "query"], false),
            (vec!["get", "svc", "--name", "daily"], false),
            (
                vec![
                    "create",
                    "svc",
                    "--name",
                    "daily",
                    "--sql",
                    "SELECT 1",
                    "--database",
                    "default",
                ],
                true,
            ),
            (
                vec![
                    "update",
                    "svc",
                    "query",
                    "--new-name",
                    "daily",
                    "--sql-file",
                    "-",
                    "--database",
                    "default",
                ],
                true,
            ),
            (vec!["delete", "svc", "query"], true),
        ] {
            assert_eq!(parse(&args).is_write(), write, "{args:?}");
        }
    }

    #[test]
    fn saved_query_clap_parses_create_body_flags() {
        let args = parse(&[
            "--org-id",
            "org",
            "create",
            "svc",
            "--name",
            "daily",
            "--sql",
            "SELECT {id:UInt32}",
            "--database",
            "analytics",
            "--param",
            "id=7",
            "--param",
            "region=eu=west",
        ]);
        let SavedQueryCommands::Create {
            service_id,
            name,
            body,
        } = args.command
        else {
            panic!("create")
        };
        assert_eq!(service_id, "svc");
        assert_eq!(name, "daily");
        assert_eq!(body.sql.as_deref(), Some("SELECT {id:UInt32}"));
        assert!(body.sql_file.is_none());
        assert_eq!(body.database, "analytics");
        assert_eq!(
            body.params,
            vec![
                ("id".to_owned(), "7".to_owned()),
                ("region".to_owned(), "eu=west".to_owned())
            ]
        );
    }

    #[test]
    fn saved_query_clap_parses_update_target_and_rename() {
        let args = parse(&[
            "update",
            "svc",
            "--name",
            "old",
            "--new-name",
            "new",
            "--sql-file",
            "query.sql",
            "--database",
            "default",
            "--org-id",
            "org",
        ]);
        let SavedQueryCommands::Update { target, name, body } = args.command else {
            panic!("update")
        };
        assert_eq!(target.service_id, "svc");
        assert!(target.query_id.id.is_none());
        assert_eq!(target.query_id.name.as_deref(), Some("old"));
        assert_eq!(name, "new");
        assert!(body.sql.is_none());
        assert_eq!(body.sql_file.as_deref(), Some("query.sql"));
        assert!(body.params.is_empty());
        for command in ["get", "delete"] {
            let target = match parse(&[command, "svc", "query"]).command {
                SavedQueryCommands::Get(target) | SavedQueryCommands::Delete(target) => target,
                _ => panic!("target"),
            };
            assert_eq!(target.service_id, "svc");
            assert_eq!(target.query_id.id.as_deref(), Some("query"));
        }
    }

    #[test]
    fn saved_query_clap_body_constraints() {
        let create = ["create", "svc", "--name", "q", "--database", "default"];
        // SQL is required, from exactly one source.
        assert_eq!(
            try_parse(&create).err().unwrap().kind(),
            ErrorKind::MissingRequiredArgument
        );
        let mut both = create.to_vec();
        both.extend(["--sql", "SELECT 1", "--sql-file", "q.sql"]);
        assert_eq!(
            try_parse(&both).err().unwrap().kind(),
            ErrorKind::ArgumentConflict
        );
        // Name and database are required on create; update requires --new-name.
        for args in [
            vec![
                "create",
                "svc",
                "--sql",
                "SELECT 1",
                "--database",
                "default",
            ],
            vec!["create", "svc", "--name", "q", "--sql", "SELECT 1"],
            vec![
                "update",
                "svc",
                "query",
                "--sql",
                "SELECT 1",
                "--database",
                "default",
            ],
            vec!["update", "svc", "--new-name", "q", "--sql", "SELECT 1"],
        ] {
            assert_eq!(
                try_parse(&args).err().unwrap().kind(),
                ErrorKind::MissingRequiredArgument,
                "{args:?}"
            );
        }
        for bad in ["novalue", "=v"] {
            let mut args = create.to_vec();
            args.extend(["--sql", "SELECT 1", "--param", bad]);
            assert_eq!(
                try_parse(&args).err().unwrap().kind(),
                ErrorKind::ValueValidation,
                "{bad}"
            );
        }
    }

    #[test]
    fn saved_query_clap_pagination_defaults_and_bounds() {
        let SavedQueryCommands::List {
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
            let SavedQueryCommands::List { cursor, limit, .. } =
                parse(&["list", "svc", "--cursor", "next+/=", "--limit", limit_value]).command
            else {
                panic!("list")
            };
            assert_eq!(cursor.as_deref(), Some("next+/="));
            assert_eq!(limit, Some(limit_value.parse().unwrap()));
        }
        for value in ["-1", "0", "101", "not-a-number"] {
            assert_eq!(
                try_parse(&["list", "svc", "--limit", value])
                    .err()
                    .unwrap()
                    .kind(),
                ErrorKind::ValueValidation,
                "{value}"
            );
        }
    }
}
