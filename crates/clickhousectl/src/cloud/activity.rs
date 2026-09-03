use crate::cloud::client::{CloudClient, Result as CloudResult};
use crate::cloud::output::{or_absent, print_human};
use crate::cloud::shared::{parse_date_only, resolve_org_id};
use clap::Subcommand;
use tabled::{Table, Tabled, settings::Style};

#[derive(Subcommand)]
pub enum ActivityCommands {
    /// List activity log entries
    List {
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,

        /// Start date in UTC (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date_only)]
        from_date: Option<String>,

        /// End date in UTC, inclusive (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date_only)]
        to_date: Option<String>,
    },

    /// Get activity log entry details
    Get {
        /// Activity ID
        activity_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl ActivityCommands {
    pub fn is_write(&self) -> bool {
        match self {
            ActivityCommands::List { .. } => false,
            ActivityCommands::Get { .. } => false,
        }
    }
}

pub async fn run(client: &CloudClient, command: ActivityCommands, json: bool) -> CloudResult<()> {
    match command {
        ActivityCommands::List {
            org_id,
            from_date,
            to_date,
        } => {
            activity_list(
                client,
                org_id.as_deref(),
                from_date.as_deref(),
                to_date.as_deref(),
                json,
            )
            .await
        }
        ActivityCommands::Get {
            activity_id,
            org_id,
        } => activity_get(client, &activity_id, org_id.as_deref(), json).await,
    }
}

async fn activity_list(
    client: &CloudClient,
    org_id: Option<&str>,
    from_date: Option<&str>,
    to_date: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let activities = client.list_activities(&org_id, from_date, to_date).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&activities)?);
    } else {
        if activities.is_empty() {
            println!("No activities found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Type")]
            activity_type: String,
            #[tabled(rename = "Created")]
            created: String,
        }
        let rows: Vec<Row> = activities
            .into_iter()
            .map(|activity| Row {
                id: or_absent(activity.id.as_deref()),
                activity_type: or_absent(activity.r#type.as_ref()),
                created: or_absent(activity.created_at.map(|at| at.to_rfc3339())),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

async fn activity_get(
    client: &CloudClient,
    activity_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let activity = client.get_activity(&org_id, activity_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&activity)?);
    } else {
        print_human(&activity)?;
    }
    Ok(())
}

impl CloudClient {
    pub async fn list_activities(
        &self,
        org_id: &str,
        from_date: Option<&str>,
        to_date: Option<&str>,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::Activity>> {
        let response = self
            .api()
            .activity_get_list(org_id, from_date, to_date)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_activity(
        &self,
        org_id: &str,
        activity_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Activity> {
        let response = self
            .api()
            .activity_get(org_id, activity_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    #[derive(Parser)]
    struct ActivityCli {
        #[command(subcommand)]
        command: ActivityCommands,
    }

    fn parse_activity(args: &[&str]) -> ActivityCommands {
        assert_eq!(args.get(1), Some(&"cloud"));
        assert_eq!(args.get(2), Some(&"activity"));
        ActivityCli::try_parse_from(std::iter::once(args[0]).chain(args.iter().skip(3).copied()))
            .expect("parse")
            .command
    }

    #[test]
    fn parses_activity_list_date_only_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "activity",
            "list",
            "--from-date",
            "2025-01-01",
            "--to-date",
            "2025-01-31",
        ])
        .unwrap();

        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Activity { command } = args.command else {
            panic!("expected activity command");
        };
        let crate::cloud::cli::ActivityCommands::List {
            org_id,
            from_date,
            to_date,
        } = command
        else {
            panic!("expected activity list");
        };
        assert!(org_id.is_none());
        assert_eq!(from_date.as_deref(), Some("2025-01-01"));
        assert_eq!(to_date.as_deref(), Some("2025-01-31"));
    }

    #[test]
    fn rejects_activity_list_timestamps() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "activity",
            "list",
            "--from-date",
            "2025-01-01T00:00:00Z",
            "--to-date",
            "2025-01-31",
        ]);

        match result {
            Ok(_) => panic!("expected timestamp input to be rejected"),
            Err(error) => assert!(error.to_string().contains("expected YYYY-MM-DD")),
        }
    }

    #[test]
    fn rejects_invalid_activity_list_calendar_dates() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "activity",
            "list",
            "--from-date",
            "2025-02-31",
            "--to-date",
            "2025-03-01",
        ]);

        match result {
            Ok(_) => panic!("expected invalid calendar date to be rejected"),
            Err(error) => assert!(error.to_string().contains("expected YYYY-MM-DD")),
        }
    }

    #[test]
    fn every_activity_command_is_read_only() {
        assert!(!parse_activity(&["clickhousectl", "cloud", "activity", "list"]).is_write());
        assert!(
            !parse_activity(&["clickhousectl", "cloud", "activity", "get", "act-1"]).is_write()
        );
    }
}
