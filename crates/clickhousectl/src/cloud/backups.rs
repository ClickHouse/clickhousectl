use crate::cloud::client::CloudClient;
use crate::cloud::output::{or_absent, print_human};
use crate::cloud::shared::{parse_time_only, resolve_org_id};
use clap::Subcommand;
use clickhouse_cloud_api::models::BackupConfigurationPatchRequest;
use tabled::{Table, Tabled, settings::Style};

#[derive(Subcommand)]
pub enum BackupCommands {
    /// List backups for a service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Lists all backups for a given service. Requires a service ID from `clickhousectl cloud service list`.
  Returns backup IDs that can be used with `clickhousectl cloud service create --backup-id` to restore.
  Add --json for machine-readable output.
  Related: `clickhousectl cloud backup get` for details on a specific backup.")]
    List {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get backup details
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Returns details for a specific backup. Requires service ID and backup ID.
  Get service IDs from `clickhousectl cloud service list`, backup IDs from `clickhousectl cloud backup list`.
  Add --json for machine-readable output.
  Related: `clickhousectl cloud service create --backup-id <id>` to restore from this backup.")]
    Get {
        /// Service ID
        service_id: String,

        /// Backup ID
        backup_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl BackupCommands {
    pub fn is_write(&self) -> bool {
        match self {
            BackupCommands::List { .. } => false,
            BackupCommands::Get { .. } => false,
        }
    }
}

#[derive(Subcommand)]
pub enum BackupConfigCommands {
    /// Get backup configuration for a service
    Get {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update backup configuration for a service
    Update {
        /// Service ID
        service_id: String,

        /// The interval in hours between each backup
        #[arg(long)]
        backup_period_hours: Option<u32>,

        /// Retention period in hours
        #[arg(long)]
        backup_retention_period_hours: Option<u32>,

        /// Backup start time in UTC (HH:MM)
        #[arg(long, value_parser = parse_time_only)]
        backup_start_time: Option<String>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl BackupConfigCommands {
    pub fn is_write(&self) -> bool {
        match self {
            BackupConfigCommands::Get { .. } => false,
            BackupConfigCommands::Update { .. } => true,
        }
    }
}

pub async fn run(
    client: &CloudClient,
    command: BackupCommands,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BackupCommands::List { service_id, org_id } => {
            backup_list(client, &service_id, org_id.as_deref(), json).await
        }
        BackupCommands::Get {
            service_id,
            backup_id,
            org_id,
        } => backup_get(client, &service_id, &backup_id, org_id.as_deref(), json).await,
    }
}

pub async fn run_config(
    client: &CloudClient,
    command: BackupConfigCommands,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        BackupConfigCommands::Get { service_id, org_id } => {
            backup_config_get(client, &service_id, org_id.as_deref(), json).await
        }
        BackupConfigCommands::Update {
            service_id,
            backup_period_hours,
            backup_retention_period_hours,
            backup_start_time,
            org_id,
        } => {
            let options = BackupConfigUpdateOptions {
                backup_period_hours,
                backup_retention_period_hours,
                backup_start_time,
                org_id,
            };
            backup_config_update(client, &service_id, options, json).await
        }
    }
}

#[derive(Default)]
struct BackupConfigUpdateOptions {
    backup_period_hours: Option<u32>,
    backup_retention_period_hours: Option<u32>,
    backup_start_time: Option<String>,
    org_id: Option<String>,
}

fn build_backup_config_update_request(
    options: &BackupConfigUpdateOptions,
) -> BackupConfigurationPatchRequest {
    BackupConfigurationPatchRequest {
        backup_period_in_hours: options.backup_period_hours.map(f64::from),
        backup_retention_period_in_hours: options.backup_retention_period_hours.map(f64::from),
        backup_start_time: options.backup_start_time.clone(),
    }
}

async fn backup_list(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let backups = client.list_backups(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&backups)?);
    } else {
        if backups.is_empty() {
            println!("No backups found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Status")]
            status: String,
            #[tabled(rename = "Size")]
            size: String,
            #[tabled(rename = "Created")]
            created: String,
        }
        let rows: Vec<Row> = backups
            .into_iter()
            .map(|backup| Row {
                id: or_absent(backup.id),
                status: or_absent(backup.status.as_ref()),
                size: or_absent(backup.size_in_bytes.map(format_bytes)),
                created: or_absent(backup.started_at.map(|at| at.to_rfc3339())),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

async fn backup_get(
    client: &CloudClient,
    service_id: &str,
    backup_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let backup = client.get_backup(&org_id, service_id, backup_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&backup)?);
    } else {
        print_human(&backup)?;
    }
    Ok(())
}

async fn backup_config_get(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let config = client.get_backup_config(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else {
        print_human(&config)?;
    }
    Ok(())
}

async fn backup_config_update(
    client: &CloudClient,
    service_id: &str,
    options: BackupConfigUpdateOptions,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, options.org_id.as_deref()).await?;
    let request = build_backup_config_update_request(&options);
    let config = client
        .update_backup_config(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else {
        println!("Backup configuration updated for service {}", service_id);
        println!(
            "  Backup period: {} hours",
            or_absent(config.backup_period_in_hours)
        );
        println!(
            "  Retention: {} hours",
            or_absent(config.backup_retention_period_in_hours)
        );
        println!(
            "  Start time: {}",
            or_absent(config.backup_start_time.as_deref())
        );
    }
    Ok(())
}

fn format_bytes(bytes: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes >= GB {
        format!("{:.2} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes / KB)
    } else {
        format!("{} B", bytes)
    }
}

impl CloudClient {
    pub async fn list_backups(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::Backup>> {
        let response = self
            .api()
            .backup_get_list(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_backup(
        &self,
        org_id: &str,
        service_id: &str,
        backup_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Backup> {
        let response = self
            .api()
            .backup_get(org_id, service_id, backup_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_backup_config(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::BackupConfiguration> {
        let response = self
            .api()
            .backup_configuration_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_backup_config(
        &self,
        org_id: &str,
        service_id: &str,
        request: &BackupConfigurationPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::BackupConfiguration> {
        let response = self
            .api()
            .backup_configuration_update(org_id, service_id, request)
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
    struct BackupCli {
        #[command(subcommand)]
        command: BackupCommands,
    }

    #[derive(Parser)]
    struct BackupConfigCli {
        #[command(subcommand)]
        command: BackupConfigCommands,
    }

    fn parse_backup(args: &[&str]) -> BackupCommands {
        assert_eq!(args.get(1), Some(&"cloud"));
        assert_eq!(args.get(2), Some(&"backup"));
        BackupCli::try_parse_from(std::iter::once(args[0]).chain(args.iter().skip(3).copied()))
            .expect("parse")
            .command
    }

    fn parse_backup_config(args: &[&str]) -> BackupConfigCommands {
        assert_eq!(args.get(1), Some(&"cloud"));
        assert_eq!(args.get(2), Some(&"service"));
        assert_eq!(args.get(3), Some(&"backup-config"));
        BackupConfigCli::try_parse_from(
            std::iter::once(args[0]).chain(args.iter().skip(4).copied()),
        )
        .expect("parse")
        .command
    }

    #[test]
    fn parses_backup_config_update_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--backup-period-hours",
            "12",
            "--backup-retention-period-hours",
            "336",
            "--backup-start-time",
            "03:00",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let crate::cloud::cli::ServiceCommands::BackupConfig { command } = command else {
            panic!("expected backup-config command");
        };
        let crate::cloud::cli::BackupConfigCommands::Update {
            service_id,
            backup_period_hours,
            backup_retention_period_hours,
            backup_start_time,
            org_id,
        } = command
        else {
            panic!("expected backup-config update");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(backup_period_hours, Some(12));
        assert_eq!(backup_retention_period_hours, Some(336));
        assert_eq!(backup_start_time.as_deref(), Some("03:00"));
        assert!(org_id.is_none());
    }

    #[test]
    fn parses_backup_list_with_top_level_cli_defaults() {
        let cli =
            Cli::try_parse_from(["clickhousectl", "cloud", "backup", "list", "svc-1"]).unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Backup { command } = args.command else {
            panic!("expected backup command");
        };
        let crate::cloud::cli::BackupCommands::List { service_id, org_id } = command else {
            panic!("expected backup list");
        };
        assert_eq!(service_id, "svc-1");
        assert!(org_id.is_none());
    }

    #[test]
    fn rejects_invalid_backup_start_time() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--backup-start-time",
            "25:00",
        ]);

        match result {
            Ok(_) => panic!("expected invalid backup start time to be rejected"),
            Err(error) => assert!(error.to_string().contains("expected HH:MM")),
        }
    }

    #[test]
    fn backup_write_classification_is_exhaustive() {
        assert!(!parse_backup(&["clickhousectl", "cloud", "backup", "list", "svc-1"]).is_write());
        assert!(
            !parse_backup(&[
                "clickhousectl",
                "cloud",
                "backup",
                "get",
                "svc-1",
                "backup-1",
            ])
            .is_write()
        );
        assert!(
            !parse_backup_config(&[
                "clickhousectl",
                "cloud",
                "service",
                "backup-config",
                "get",
                "svc-1",
            ])
            .is_write()
        );
        assert!(
            parse_backup_config(&[
                "clickhousectl",
                "cloud",
                "service",
                "backup-config",
                "update",
                "svc-1",
            ])
            .is_write()
        );
    }

    #[test]
    fn build_backup_config_update_request_supports_minimal_and_maximal_inputs() {
        let minimal = build_backup_config_update_request(&BackupConfigUpdateOptions::default());
        assert!(minimal.backup_period_in_hours.is_none());
        assert!(minimal.backup_retention_period_in_hours.is_none());
        assert!(minimal.backup_start_time.is_none());

        let maximal = build_backup_config_update_request(&BackupConfigUpdateOptions {
            backup_period_hours: Some(12),
            backup_retention_period_hours: Some(336),
            backup_start_time: Some("03:00".to_string()),
            org_id: None,
        });
        assert_eq!(maximal.backup_period_in_hours, Some(12.0));
        assert_eq!(maximal.backup_retention_period_in_hours, Some(336.0));
        assert_eq!(maximal.backup_start_time.as_deref(), Some("03:00"));
    }
}
