use super::permissions::{Conditional, Declaration as Permission};
use clickhouse_cloud_api::meta::operations as op;

// Declare every API call made by these workflows, including optional lookups.
pub(super) const PERMISSIONS: &[Permission] = &[
    Permission::api("backup list", &[&op::BACKUP_GET_LIST]),
    Permission::api("backup get", &[&op::BACKUP_GET]),
    Permission::api("backup bucket get", &[&op::BACKUP_BUCKET_GET]),
    Permission::api("backup bucket create", &[&op::BACKUP_BUCKET_CREATE]),
    Permission::api("backup bucket update", &[&op::BACKUP_BUCKET_UPDATE]),
    Permission::api("backup bucket delete", &[&op::BACKUP_BUCKET_DELETE]),
    Permission::api("service snapshot list", &[&op::SNAPSHOT_GET_LIST])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api("service snapshot get", &[&op::SNAPSHOT_GET])
        .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service snapshot config get",
        &[&op::SNAPSHOT_CONFIGURATION_GET],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service snapshot config update",
        &[&op::SNAPSHOT_CONFIGURATION_UPDATE],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service backup-config get",
        &[&op::BACKUP_CONFIGURATION_GET],
    )
    .when(&[Conditional::flag("name", &[&op::INSTANCE_GET_LIST])]),
    Permission::api(
        "service backup-config update",
        &[&op::BACKUP_CONFIGURATION_UPDATE],
    )
    .when(&[
        Conditional::flag("name", &[&op::INSTANCE_GET_LIST]),
        Conditional::new(
            "Start time without period",
            &[&op::BACKUP_CONFIGURATION_GET],
        ),
    ]),
];

use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::config::{deserialize_strict_config, read_config_value};
use crate::cloud::output::{or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use crate::cloud::shared::{NameSelector, NamedResource};
use crate::cloud::types::DeleteResponse;
use clap::builder::NonEmptyStringValueParser;
use clap::{ArgGroup, Subcommand};
use clickhouse_cloud_api::models::{
    AwsBackupBucketPatchRequestV1, AwsBackupBucketPostRequestV1, AzureBackupBucketPatchRequestV1,
    AzureBackupBucketPostRequestV1, BackupBucket, BackupBucketPatchRequest,
    BackupBucketPostRequest, BackupConfigurationPatchRequest, GcpBackupBucketPatchRequestV1,
    GcpBackupBucketPostRequestV1,
};
use serde_json::Value;
use tabled::{Table, Tabled, settings::Style};

#[derive(Subcommand)]
pub enum SnapshotCommands {
    /// List service snapshots
    List {
        #[command(flatten)]
        service_id: NameSelector,
    },
    /// Get snapshot details
    Get {
        #[command(flatten)]
        service_id: NameSelector,
        /// Snapshot ID (from `cloud service snapshot list`)
        #[arg(long)]
        snapshot_id: String,
    },
    /// Manage snapshot configuration
    #[command(subcommand)]
    Config(SnapshotConfigCommands),
}

impl SnapshotCommands {
    pub fn is_write(&self) -> bool {
        match self {
            Self::List { .. } | Self::Get { .. } => false,
            Self::Config(command) => command.is_write(),
        }
    }
}

#[derive(Subcommand)]
pub enum SnapshotConfigCommands {
    /// Get snapshot configuration
    Get {
        #[command(flatten)]
        service_id: NameSelector,
    },
    /// Update snapshot configuration
    #[command(after_help = "CONTEXT FOR AGENTS:
  Requires an ADMIN API key.
  Omitted fields stay unchanged; Cloud validates the resulting configuration.")]
    Update {
        #[command(flatten)]
        service_id: NameSelector,
        #[command(flatten)]
        options: SnapshotConfigUpdateOptions,
    },
}

impl SnapshotConfigCommands {
    pub fn is_write(&self) -> bool {
        match self {
            Self::Get { .. } => false,
            Self::Update { .. } => true,
        }
    }
}

#[derive(clap::Args, Default)]
#[group(required = true, multiple = true)]
pub struct SnapshotConfigUpdateOptions {
    /// Enable or disable scheduled snapshots
    #[arg(long, action = clap::ArgAction::Set)]
    enabled: Option<bool>,
    /// Interval in minutes; requires --time-frame
    #[arg(long, requires = "time_frame")]
    gap: Option<u32>,
    /// Retention in minutes; requires --gap
    #[arg(long, requires = "gap")]
    time_frame: Option<u32>,
}

fn build_snapshot_config_update_request(
    options: &SnapshotConfigUpdateOptions,
) -> CloudResult<clickhouse_cloud_api::models::SnapshotConfigurationPatchRequest> {
    if options.enabled.is_none() && options.gap.is_none() && options.time_frame.is_none() {
        return Err(CloudError::usage(
            "provide at least one snapshot configuration change",
        ));
    }
    if options.gap.is_some() != options.time_frame.is_some() {
        return Err(CloudError::usage(
            "--gap and --time-frame must be supplied together",
        ));
    }
    Ok(
        clickhouse_cloud_api::models::SnapshotConfigurationPatchRequest {
            enabled: options.enabled,
            gap: options.gap.map(f64::from),
            time_frame: options.time_frame.map(f64::from),
        },
    )
}

pub async fn run_snapshot(
    client: &CloudClient,
    command: SnapshotCommands,
    json: bool,
) -> CloudResult<()> {
    let org = resolve_org_id(client).await?;
    match command {
        SnapshotCommands::List { service_id } => {
            let id = service_id.resolve(client, NamedResource::Service).await?;
            let snapshots = client.list_snapshots(&org, &id).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&snapshots)?);
            } else if snapshots.is_empty() {
                println!("No snapshots found");
            } else {
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
                let rows = snapshots.into_iter().map(|snapshot| Row {
                    id: or_absent(snapshot.id),
                    status: or_absent(snapshot.status.as_ref()),
                    size: or_absent(snapshot.size_in_bytes.map(format_bytes)),
                    created: format_backup_created(snapshot.started_at),
                });
                println!("{}", Table::new(rows).with(Style::markdown()));
            }
        }
        SnapshotCommands::Get {
            service_id,
            snapshot_id,
        } => {
            let id = service_id.resolve(client, NamedResource::Service).await?;
            let snapshot = client.get_snapshot(&org, &id, &snapshot_id).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                print_human(&snapshot)?;
            }
        }
        SnapshotCommands::Config(command) => {
            let config = match command {
                SnapshotConfigCommands::Get { service_id } => {
                    let id = service_id.resolve(client, NamedResource::Service).await?;
                    client.get_snapshot_config(&org, &id).await?
                }
                SnapshotConfigCommands::Update {
                    service_id,
                    options,
                } => {
                    let request = build_snapshot_config_update_request(&options)?;
                    let id = service_id.resolve(client, NamedResource::Service).await?;
                    client.update_snapshot_config(&org, &id, &request).await?
                }
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                print_human(&config)?;
            }
        }
    }
    Ok(())
}

#[derive(Subcommand)]
pub enum BackupCommands {
    /// List backups for a service
    List {
        /// Service ID (from `cloud service list`)
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,
    },

    /// Get backup details
    Get {
        /// Service ID (from `cloud service list`)
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// Backup ID
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        backup_id: String,
    },

    /// Manage a service backup bucket
    #[command(subcommand)]
    Bucket(BackupBucketCommands),
}

impl BackupCommands {
    pub fn is_write(&self) -> bool {
        match self {
            BackupCommands::List { .. } => false,
            BackupCommands::Get { .. } => false,
            BackupCommands::Bucket(command) => command.is_write(),
        }
    }
}

#[derive(Subcommand)]
pub enum BackupBucketCommands {
    /// Get backup bucket details
    Get {
        /// Service ID (from `cloud service list`)
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,
    },

    /// Create a backup bucket
    Create {
        /// Service ID (from `cloud service list`)
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// Provider-specific JSON file, or - to read from stdin
        #[arg(long = "file", value_name = "PATH", aliases = ["config-file", "config"])]
        config_file: String,
    },

    /// Update a backup bucket
    Update {
        /// Service ID (from `cloud service list`)
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,

        /// Provider-specific update JSON file, or - to read from stdin
        #[arg(long = "file", value_name = "PATH", aliases = ["config-file", "config"])]
        config_file: String,
    },

    /// Delete a backup bucket
    Delete {
        /// Service ID (from `cloud service list`)
        #[arg(value_parser = NonEmptyStringValueParser::new())]
        service_id: String,
    },
}

impl BackupBucketCommands {
    pub fn is_write(&self) -> bool {
        match self {
            BackupBucketCommands::Get { .. } => false,
            BackupBucketCommands::Create { .. }
            | BackupBucketCommands::Update { .. }
            | BackupBucketCommands::Delete { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum BackupConfigCommands {
    /// Get the backup configuration
    Get {
        /// Service ID (from `cloud service list`)
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Update the backup configuration
    #[command(group(
        ArgGroup::new("backup_config_change")
            .required(true)
            .multiple(true)
            .args([
                "backup_period_hours",
                "backup_retention_period_hours",
                "backup_start_time",
                "clear_backup_start_time",
            ])
    ))]
    Update {
        /// Service ID (from `cloud service list`)
        #[command(flatten)]
        service_id: NameSelector,

        /// Interval in hours between backups
        ///
        /// With a backup start time in effect, an explicitly supplied period must be 24 or 48.
        #[arg(long)]
        backup_period_hours: Option<u32>,

        /// Retention in hours (24-1080, in multiples of 24)
        #[arg(long)]
        backup_retention_period_hours: Option<u32>,

        /// Backup start time in UTC, on the hour (HH:00)
        ///
        /// Requires a backup period of 24 or 48 hours, passed in the same call or already stored.
        /// Clear it with --clear-backup-start-time.
        #[arg(long, value_parser = parse_backup_start_time)]
        backup_start_time: Option<String>,

        /// Remove the stored backup start time
        ///
        /// Lifts the 24/48 hour restriction on the period, so it can be combined with
        /// --backup-period-hours; conflicts with --backup-start-time.
        #[arg(long, conflicts_with = "backup_start_time")]
        clear_backup_start_time: bool,
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

pub async fn run(client: &CloudClient, command: BackupCommands, json: bool) -> CloudResult<()> {
    match command {
        BackupCommands::List { service_id } => backup_list(client, &service_id, json).await,
        BackupCommands::Get {
            service_id,
            backup_id,
        } => backup_get(client, &service_id, &backup_id, json).await,
        BackupCommands::Bucket(command) => run_bucket(client, command, json).await,
    }
}

async fn run_bucket(
    client: &CloudClient,
    command: BackupBucketCommands,
    json: bool,
) -> CloudResult<()> {
    match command {
        BackupBucketCommands::Get { service_id } => {
            backup_bucket_get(client, &service_id, json).await
        }
        BackupBucketCommands::Create {
            service_id,
            config_file,
        } => {
            let request =
                build_backup_bucket_create_request(read_config_value(&config_file)?, &config_file)?;
            backup_bucket_create(client, &service_id, request, json).await
        }
        BackupBucketCommands::Update {
            service_id,
            config_file,
        } => {
            let request =
                build_backup_bucket_update_request(read_config_value(&config_file)?, &config_file)?;
            backup_bucket_update(client, &service_id, request, json).await
        }
        BackupBucketCommands::Delete { service_id } => {
            backup_bucket_delete(client, &service_id, json).await
        }
    }
}

pub async fn run_config(
    client: &CloudClient,
    command: BackupConfigCommands,
    json: bool,
) -> CloudResult<()> {
    match command {
        BackupConfigCommands::Get { service_id } => {
            backup_config_get(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                json,
            )
            .await
        }
        BackupConfigCommands::Update {
            service_id,
            backup_period_hours,
            backup_retention_period_hours,
            backup_start_time,
            clear_backup_start_time,
        } => {
            let options = BackupConfigUpdateOptions {
                backup_period_hours,
                backup_retention_period_hours,
                backup_start_time,
                clear_backup_start_time,
            };
            backup_config_update(
                client,
                &service_id.resolve(client, NamedResource::Service).await?,
                options,
                json,
            )
            .await
        }
    }
}

#[derive(Default)]
struct BackupConfigUpdateOptions {
    backup_period_hours: Option<u32>,
    backup_retention_period_hours: Option<u32>,
    backup_start_time: Option<String>,
    clear_backup_start_time: bool,
}

fn parse_backup_start_time(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let valid_syntax = bytes.len() == 5
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b':'
        && bytes[3] == b'0'
        && bytes[4] == b'0';
    let valid_hour = valid_syntax && (bytes[0] - b'0') * 10 + (bytes[1] - b'0') <= 23;

    if !valid_hour {
        return Err(format!(
            "invalid backup start time '{}': expected HH:00 with HH from 00 to 23",
            value
        ));
    }

    Ok(value.to_string())
}

fn build_backup_config_update_request(
    options: &BackupConfigUpdateOptions,
) -> CloudResult<BackupConfigurationPatchRequest> {
    if options.backup_start_time.is_some() && options.clear_backup_start_time {
        return Err(CloudError::usage(
            "--backup-start-time and --clear-backup-start-time cannot be used together",
        ));
    }

    if options.backup_start_time.is_some()
        && matches!(options.backup_period_hours, Some(period) if period != 24 && period != 48)
    {
        return Err(CloudError::usage(
            "--backup-period-hours must be 24 or 48 when --backup-start-time is set",
        ));
    }

    // Clearing is the one way out of the 24/48 hour period restriction, so it
    // travels as an explicit `null` and is deliberately compatible with any
    // period in the same call: the API applies the clear first.
    let backup_start_time = match (&options.backup_start_time, options.clear_backup_start_time) {
        (Some(time), _) => Some(Some(time.clone())),
        (None, true) => Some(None),
        (None, false) => None,
    };

    Ok(BackupConfigurationPatchRequest {
        backup_period_in_hours: options.backup_period_hours.map(f64::from),
        backup_retention_period_in_hours: options.backup_retention_period_hours.map(f64::from),
        backup_start_time,
    })
}

/// A start time sent without a period is validated by the API against the
/// *stored* period, which it keeps rather than defaulting. If that period is
/// not 24 or 48 the PATCH fails with an opaque `BAD_REQUEST`, so check it
/// first and name the flag that fixes it. An absent stored period is not a
/// guess we get to make: proceed and let the API decide.
fn check_stored_period_allows_start_time(stored_period_hours: Option<f64>) -> CloudResult<()> {
    match stored_period_hours {
        Some(period) if period != 24.0 && period != 48.0 => Err(CloudError::new(format!(
            "the stored backup period is {period} hours, but --backup-start-time requires 24 or \
             48. Pass --backup-period-hours 24 or --backup-period-hours 48 in the same call."
        ))),
        _ => Ok(()),
    }
}

fn bucket_provider<'a>(value: &'a Value, source: &str) -> CloudResult<&'a str> {
    value
        .as_object()
        .ok_or_else(|| {
            CloudError::usage(format!(
                "invalid request body in config {source}: expected a JSON object"
            ))
        })?
        .get("bucketProvider")
        .ok_or_else(|| {
            CloudError::usage(format!(
                "invalid request body in config {source}: missing field `bucketProvider`"
            ))
        })?
        .as_str()
        .ok_or_else(|| {
            CloudError::usage(format!(
                "invalid request body in config {source}: `bucketProvider` must be a string"
            ))
        })
}

fn unsupported_bucket_provider(source: &str, provider: &str) -> CloudError {
    CloudError::usage(format!(
        "invalid request body in config {source}: unsupported `bucketProvider` {provider:?}; expected AWS, GCP, or AZURE"
    ))
}

fn build_backup_bucket_create_request(
    value: Value,
    source: &str,
) -> CloudResult<BackupBucketPostRequest> {
    match bucket_provider(&value, source)? {
        "AWS" => deserialize_strict_config::<AwsBackupBucketPostRequestV1>(value, source)
            .map(BackupBucketPostRequest::AwsBackupBucketPostRequestV1),
        "GCP" => deserialize_strict_config::<GcpBackupBucketPostRequestV1>(value, source)
            .map(BackupBucketPostRequest::GcpBackupBucketPostRequestV1),
        "AZURE" => deserialize_strict_config::<AzureBackupBucketPostRequestV1>(value, source)
            .map(BackupBucketPostRequest::AzureBackupBucketPostRequestV1),
        provider => Err(unsupported_bucket_provider(source, provider)),
    }
}

fn build_backup_bucket_update_request(
    value: Value,
    source: &str,
) -> CloudResult<BackupBucketPatchRequest> {
    match bucket_provider(&value, source)? {
        "AWS" => deserialize_strict_config::<AwsBackupBucketPatchRequestV1>(value, source)
            .map(BackupBucketPatchRequest::AwsBackupBucketPatchRequestV1),
        "GCP" => deserialize_strict_config::<GcpBackupBucketPatchRequestV1>(value, source)
            .map(BackupBucketPatchRequest::GcpBackupBucketPatchRequestV1),
        "AZURE" => deserialize_strict_config::<AzureBackupBucketPatchRequestV1>(value, source)
            .map(BackupBucketPatchRequest::AzureBackupBucketPatchRequestV1),
        provider => Err(unsupported_bucket_provider(source, provider)),
    }
}

async fn backup_list(client: &CloudClient, service_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
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
                created: format_backup_created(backup.started_at),
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
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let backup = client.get_backup(&org_id, service_id, backup_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&backup)?);
    } else {
        print_human(&backup)?;
    }
    Ok(())
}

async fn backup_bucket_get(client: &CloudClient, service_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let bucket = client.get_backup_bucket(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&bucket)?);
    } else {
        print_human(&bucket)?;
    }
    Ok(())
}

async fn backup_bucket_create(
    client: &CloudClient,
    service_id: &str,
    request: BackupBucketPostRequest,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let bucket = client
        .create_backup_bucket(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&bucket)?);
    } else {
        print_human(&bucket)?;
    }
    Ok(())
}

async fn backup_bucket_update(
    client: &CloudClient,
    service_id: &str,
    request: BackupBucketPatchRequest,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let bucket = client
        .update_backup_bucket(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&bucket)?);
    } else {
        print_human(&bucket)?;
    }
    Ok(())
}

async fn backup_bucket_delete(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.delete_backup_bucket(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Backup bucket deleted for service {service_id}");
    }
    Ok(())
}

async fn backup_config_get(client: &CloudClient, service_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
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
) -> CloudResult<()> {
    let request = build_backup_config_update_request(&options)?;
    let org_id = resolve_org_id(client).await?;

    // Only a start time being *set* is measured against the stored period.
    // Clearing one is exactly how a service escapes an incompatible period, so
    // it must never be blocked by that check.
    if matches!(request.backup_start_time, Some(Some(_)))
        && request.backup_period_in_hours.is_none()
    {
        let stored = client.get_backup_config(&org_id, service_id).await?;
        check_stored_period_allows_start_time(stored.backup_period_in_hours)?;
    }

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

fn format_backup_created(value: Option<chrono::DateTime<chrono::Utc>>) -> String {
    or_absent(value.map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)))
}

impl CloudClient {
    /// Every snapshot of the service, read across all pages.
    pub async fn list_snapshots(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> CloudResult<Vec<clickhouse_cloud_api::models::Snapshot>> {
        Self::collect_cursor_pages("snapshot", |cursor| async move {
            self.api()
                .snapshot_get_list(org_id, service_id, None, cursor.as_deref())
                .await
                .map_err(|error| self.convert_error_for_organization(error, org_id))
        })
        .await
    }
    pub async fn get_snapshot(
        &self,
        org_id: &str,
        service_id: &str,
        snapshot_id: &str,
    ) -> CloudResult<clickhouse_cloud_api::models::Snapshot> {
        let response = self
            .api()
            .snapshot_get(org_id, service_id, snapshot_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }
    pub async fn get_snapshot_config(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> CloudResult<clickhouse_cloud_api::models::SnapshotConfiguration> {
        let response = self
            .api()
            .snapshot_configuration_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }
    pub async fn update_snapshot_config(
        &self,
        org_id: &str,
        service_id: &str,
        request: &clickhouse_cloud_api::models::SnapshotConfigurationPatchRequest,
    ) -> CloudResult<clickhouse_cloud_api::models::SnapshotConfiguration> {
        let response = self
            .api()
            .snapshot_configuration_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

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

    pub async fn get_backup_bucket(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<BackupBucket> {
        let response = self
            .api()
            .backup_bucket_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn create_backup_bucket(
        &self,
        org_id: &str,
        service_id: &str,
        request: &BackupBucketPostRequest,
    ) -> crate::cloud::client::Result<BackupBucket> {
        let response = self
            .api()
            .backup_bucket_create(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_backup_bucket(
        &self,
        org_id: &str,
        service_id: &str,
        request: &BackupBucketPatchRequest,
    ) -> crate::cloud::client::Result<BackupBucket> {
        let response = self
            .api()
            .backup_bucket_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_backup_bucket(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .backup_bucket_delete(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
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
    #[test]
    fn snapshot_parsing_classifies_reads_and_updates() {
        for (args, write) in [
            (vec!["list", "svc-1"], false),
            (
                vec!["get", "--name", "analytics", "--snapshot-id", "snap-1"],
                false,
            ),
            (vec!["config", "get", "svc-1"], false),
            (
                vec!["config", "update", "svc-1", "--enabled", "false"],
                true,
            ),
            (
                vec![
                    "config",
                    "update",
                    "svc-1",
                    "--enabled",
                    "true",
                    "--gap",
                    "30",
                    "--time-frame",
                    "1440",
                ],
                true,
            ),
        ] {
            let mut full = vec!["chctl", "cloud", "service", "snapshot"];
            full.extend(args);
            let cli = Cli::try_parse_from(full).unwrap();
            let Commands::Cloud(cloud) = cli.command else {
                panic!("cloud");
            };
            assert_eq!(cloud.command.is_write_command(), write);
            if let crate::cloud::cli::CloudCommands::Service {
                command:
                    crate::cloud::services::ServiceCommands::Snapshot {
                        command:
                            SnapshotCommands::Config(SnapshotConfigCommands::Update { options, .. }),
                    },
            } = cloud.command
            {
                assert_eq!(options.enabled, Some(options.gap.is_some()));
                if options.gap.is_some() {
                    assert_eq!(options.gap, Some(30));
                    assert_eq!(options.time_frame, Some(1440));
                }
            }
        }
        for flags in [vec![], vec!["--gap", "30"], vec!["--time-frame", "1440"]] {
            let mut full = vec![
                "chctl", "cloud", "service", "snapshot", "config", "update", "svc-1",
            ];
            full.extend(flags);
            assert_eq!(
                Cli::try_parse_from(full).err().unwrap().kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
        }
    }

    #[test]
    fn snapshot_config_builder_preserves_partial_and_complete_updates() {
        let minimal = build_snapshot_config_update_request(&SnapshotConfigUpdateOptions {
            enabled: Some(false),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(minimal.enabled, Some(false));
        assert!(minimal.gap.is_none());
        assert!(minimal.time_frame.is_none());
        assert_eq!(
            serde_json::to_value(minimal).unwrap(),
            serde_json::json!({"enabled": false})
        );
        for (gap, time_frame) in [(30, 1440), (60, 2880)] {
            let maximal = build_snapshot_config_update_request(&SnapshotConfigUpdateOptions {
                enabled: Some(true),
                gap: Some(gap),
                time_frame: Some(time_frame),
            })
            .unwrap();
            assert_eq!(maximal.enabled, Some(true));
            assert_eq!(maximal.gap, Some(f64::from(gap)));
            assert_eq!(maximal.time_frame, Some(f64::from(time_frame)));
        }
        let pair_only = build_snapshot_config_update_request(&SnapshotConfigUpdateOptions {
            gap: Some(60),
            time_frame: Some(2880),
            ..Default::default()
        })
        .unwrap();
        assert!(pair_only.enabled.is_none());
        assert!(
            build_snapshot_config_update_request(&SnapshotConfigUpdateOptions::default()).is_err()
        );
        assert!(
            build_snapshot_config_update_request(&SnapshotConfigUpdateOptions {
                gap: Some(30),
                ..Default::default()
            })
            .is_err()
        );
    }

    #[test]
    fn primary_json_file_argument_contract() {
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "backup", "bucket", "create", "svc-1"],
            "config_file",
            &["config-file", "config"],
        );
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "backup", "bucket", "update", "svc-1"],
            "config_file",
            &["config-file", "config"],
        );
    }

    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    fn parse_backup(args: &[&str]) -> BackupCommands {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud) = cli.command else {
            panic!("expected cloud command");
        };
        crate::cloud::cli::tests::assert_org_selector(&cloud, args);
        let crate::cloud::cli::CloudCommands::Backup { command } = cloud.command else {
            panic!("expected backup command");
        };
        command
    }

    fn parse_backup_config(args: &[&str]) -> BackupConfigCommands {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud) = cli.command else {
            panic!("expected cloud command");
        };
        crate::cloud::cli::tests::assert_org_selector(&cloud, args);
        let crate::cloud::cli::CloudCommands::Service { command } = cloud.command else {
            panic!("expected service command");
        };
        let crate::cloud::cli::ServiceCommands::BackupConfig { command } = command else {
            panic!("expected backup-config command");
        };
        command
    }

    #[test]
    fn rejects_backup_config_update_without_changes() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "backup-config",
            "update",
            "svc-1",
        ])
        .err()
        .expect("empty backup configuration update should be rejected");

        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn formats_backup_created_as_utc_with_subseconds_and_missing_values() {
        let offset = chrono::DateTime::parse_from_rfc3339("2026-09-15T10:34:56+02:00")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(format_backup_created(Some(offset)), "2026-09-15T08:34:56Z");

        let subsecond = chrono::DateTime::parse_from_rfc3339("2026-09-15T08:34:56.123Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(
            format_backup_created(Some(subsecond)),
            "2026-09-15T08:34:56.123Z"
        );
        assert_eq!(format_backup_created(None), "-");
    }

    #[test]
    fn parses_hourly_backup_start_time_boundaries() {
        for value in ["00:00", "02:00", "23:00"] {
            assert_eq!(parse_backup_start_time(value).unwrap(), value);
        }
    }

    #[test]
    fn rejects_non_hourly_or_out_of_range_backup_start_times() {
        // The rejected value and the format to use instead.
        for value in ["2:00", "02:30", "24:00", "25:00", "02:000", "aa:00"] {
            let error = parse_backup_start_time(value).unwrap_err();
            assert!(error.contains(&format!("'{value}'")), "{value}: {error}");
            assert!(error.contains("HH:00"), "{value}: {error}");
        }

        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--backup-start-time",
            "02:30",
        ]);

        match result {
            Ok(_) => panic!("expected invalid backup start time to be rejected"),
            Err(error) => {
                assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
                assert!(error.to_string().contains("'02:30'"), "{error}");
            }
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
                "--backup-retention-period-hours",
                "0",
            ])
            .is_write()
        );
        assert!(
            !parse_backup(&["clickhousectl", "cloud", "backup", "bucket", "get", "svc-1",])
                .is_write()
        );
        for action in ["create", "update"] {
            assert!(
                parse_backup(&[
                    "clickhousectl",
                    "cloud",
                    "backup",
                    "bucket",
                    action,
                    "svc-1",
                    "--file",
                    "bucket.json",
                ])
                .is_write()
            );
        }
        assert!(
            parse_backup(&[
                "clickhousectl",
                "cloud",
                "backup",
                "bucket",
                "delete",
                "svc-1",
            ])
            .is_write()
        );
    }

    #[test]
    fn builds_all_backup_bucket_create_variants() {
        let cases = [
            serde_json::json!({
                "bucketProvider": "AWS",
                "bucketPath": "s3://backups/prefix",
                "iamRoleArn": "arn:aws:iam::123456789012:role/backups",
                "iamRoleSessionName": "clickhouse-cloud",
                "iamRoleExternalId": "trust-policy-id",
            }),
            serde_json::json!({
                "bucketProvider": "GCP",
                "bucketPath": "gs://backups/prefix",
                "accessKeyId": "gcp-access-key",
                "secretAccessKey": "gcp-secret-key",
            }),
            serde_json::json!({
                "bucketProvider": "AZURE",
                "containerName": "backups",
                "connectionString": "DefaultEndpointsProtocol=https;AccountName=account",
            }),
        ];

        for expected in cases {
            let request =
                build_backup_bucket_create_request(expected.clone(), "test input").unwrap();
            assert_eq!(serde_json::to_value(request).unwrap(), expected);
        }
    }

    #[test]
    fn builds_all_backup_bucket_update_variants() {
        let cases = [
            serde_json::json!({
                "bucketProvider": "AWS",
                "bucketPath": "s3://backups/new-prefix",
                "iamRoleArn": "arn:aws:iam::123456789012:role/backups",
            }),
            serde_json::json!({
                "bucketProvider": "AWS",
                "bucketPath": "s3://backups/new-prefix",
                "iamRoleArn": "arn:aws:iam::123456789012:role/backups",
                "iamRoleSessionName": "rotated-session",
                "iamRoleExternalId": "rotated-external-id",
            }),
            serde_json::json!({
                "bucketProvider": "GCP",
                "bucketPath": "gs://backups/new-prefix",
                "accessKeyId": "new-gcp-access-key",
                "secretAccessKey": "new-gcp-secret-key",
            }),
            serde_json::json!({
                "bucketProvider": "AZURE",
                "containerName": "new-backups",
                "connectionString": "DefaultEndpointsProtocol=https;AccountName=new-account",
            }),
        ];

        for expected in cases {
            let request =
                build_backup_bucket_update_request(expected.clone(), "test input").unwrap();
            assert_eq!(serde_json::to_value(request).unwrap(), expected);
        }
    }

    #[test]
    fn backup_bucket_create_requires_every_provider_field() {
        let cases = [
            serde_json::json!({
                "bucketProvider": "AWS",
                "bucketPath": "s3://backups",
                "iamRoleArn": "arn:aws:iam::123456789012:role/backups",
            }),
            serde_json::json!({
                "bucketProvider": "GCP",
                "bucketPath": "gs://backups",
                "accessKeyId": "key",
            }),
            serde_json::json!({
                "bucketProvider": "AZURE",
                "containerName": "backups",
            }),
        ];

        for value in cases {
            assert!(build_backup_bucket_create_request(value, "test input").is_err());
        }
    }

    #[test]
    fn aws_backup_bucket_external_id_preserves_update_states_and_unknown_field_checks() {
        let base = serde_json::json!({
            "bucketProvider": "AWS", "bucketPath": "s3://backups",
            "iamRoleArn": "arn:aws:iam::123:role/backups"
        });
        for external_id in [
            None,
            Some(serde_json::json!("trust-policy-id")),
            Some(Value::Null),
        ] {
            let mut wire = base.clone();
            if let Some(value) = external_id {
                wire["iamRoleExternalId"] = value;
            }
            let request = build_backup_bucket_update_request(wire.clone(), "test input").unwrap();
            assert_eq!(serde_json::to_value(request).unwrap(), wire);
        }
        for invalid in [serde_json::json!(42), serde_json::json!({"value": "id"})] {
            let mut wire = base.clone();
            wire["iamRoleExternalId"] = invalid;
            assert!(build_backup_bucket_update_request(wire, "test input").is_err());
        }
        let mut unknown = base.clone();
        unknown["iamRoleExternalIdTypo"] = serde_json::json!("id");
        assert!(build_backup_bucket_update_request(unknown, "test input").is_err());

        let mut create = base;
        create["iamRoleSessionName"] = serde_json::json!("backups");
        assert!(build_backup_bucket_create_request(create.clone(), "test input").is_err());
        create["iamRoleExternalId"] = Value::Null;
        assert!(build_backup_bucket_create_request(create, "test input").is_err());
    }

    #[test]
    fn backup_bucket_input_rejects_unknown_providers_and_fields() {
        let unknown_provider = build_backup_bucket_create_request(
            serde_json::json!({"bucketProvider": "S3"}),
            "test input",
        )
        .unwrap_err();
        // The rejected provider, the field and every accepted provider.
        for identifier in ["\"S3\"", "bucketProvider", "AWS", "GCP", "AZURE"] {
            assert!(
                unknown_provider.message.contains(identifier),
                "{unknown_provider}"
            );
        }

        let contradictory_field = build_backup_bucket_update_request(
            serde_json::json!({
                "bucketProvider": "AZURE",
                "containerName": "backups",
                "connectionString": "secret",
                "bucketPath": "s3://wrong-provider",
            }),
            "test input",
        )
        .unwrap_err();
        assert!(contradictory_field.message.contains("bucketPath"));
    }

    /// The exact PATCH body or error for each combination of period,
    /// retention, start time and clear: an untouched start time is omitted,
    /// a clear is an explicit `null` compatible with any period, and a start
    /// time restricts an explicit period to 24 or 48. Each error names
    /// exactly the conflicting flags.
    #[test]
    fn build_backup_config_update_request_sends_exact_bodies_or_names_the_conflict() {
        let options =
            |period: Option<u32>, start: Option<&str>, clear: bool| BackupConfigUpdateOptions {
                backup_period_hours: period,
                backup_retention_period_hours: None,
                backup_start_time: start.map(str::to_owned),
                clear_backup_start_time: clear,
            };
        let maximal = BackupConfigUpdateOptions {
            backup_retention_period_hours: Some(336),
            ..options(Some(48), Some("03:00"), false)
        };
        let cases: Vec<(
            BackupConfigUpdateOptions,
            Result<serde_json::Value, Vec<&str>>,
        )> = vec![
            (options(None, None, false), Ok(serde_json::json!({}))),
            (
                maximal,
                Ok(serde_json::json!({
                    "backupPeriodInHours": 48.0,
                    "backupRetentionPeriodInHours": 336.0,
                    "backupStartTime": "03:00"
                })),
            ),
            (
                options(None, Some("03:00"), false),
                Ok(serde_json::json!({"backupStartTime": "03:00"})),
            ),
            (
                options(Some(24), Some("03:00"), false),
                Ok(serde_json::json!({"backupPeriodInHours": 24.0, "backupStartTime": "03:00"})),
            ),
            (
                options(Some(12), None, false),
                Ok(serde_json::json!({"backupPeriodInHours": 12.0})),
            ),
            (
                options(None, None, true),
                Ok(serde_json::json!({"backupStartTime": null})),
            ),
            (
                options(Some(12), None, true),
                Ok(serde_json::json!({"backupPeriodInHours": 12.0, "backupStartTime": null})),
            ),
            (
                options(Some(12), Some("03:00"), false),
                Err(vec!["--backup-period-hours", "--backup-start-time"]),
            ),
            (
                options(None, Some("03:00"), true),
                Err(vec!["--backup-start-time", "--clear-backup-start-time"]),
            ),
        ];
        for (options, expected) in cases {
            let actual = build_backup_config_update_request(&options)
                .map(|request| serde_json::to_value(request).unwrap())
                .map_err(|error| {
                    let flags = [
                        "--backup-period-hours",
                        "--backup-start-time",
                        "--clear-backup-start-time",
                    ];
                    flags
                        .into_iter()
                        .filter(|flag| {
                            error
                                .message
                                .split(|c: char| c.is_whitespace() || c == ',' || c == '.')
                                .any(|token| token == *flag)
                        })
                        .collect::<Vec<_>>()
                });
            assert_eq!(
                actual,
                expected,
                "period {:?}, start {:?}, clear {}",
                options.backup_period_hours,
                options.backup_start_time,
                options.clear_backup_start_time
            );
        }
    }

    #[test]
    fn stored_period_check_allows_24_48_or_absent_and_names_the_flag_otherwise() {
        for stored in [Some(24.0), Some(48.0), None] {
            check_stored_period_allows_start_time(stored).unwrap();
        }
        // The stored period as stored, and both fixes as flags to pass.
        for (stored, shown) in [(12.0, " 12 "), (12.5, " 12.5 ")] {
            let error = check_stored_period_allows_start_time(Some(stored)).unwrap_err();
            for identifier in [
                shown,
                "--backup-start-time",
                "--backup-period-hours 24",
                "--backup-period-hours 48",
            ] {
                assert!(error.message.contains(identifier), "{error}");
            }
        }
    }

    #[test]
    fn parses_clear_backup_start_time() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--clear-backup-start-time",
            "--backup-period-hours",
            "12",
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
            backup_period_hours,
            backup_start_time,
            clear_backup_start_time,
            ..
        } = command
        else {
            panic!("expected backup-config update");
        };
        assert_eq!(backup_period_hours, Some(12));
        assert!(backup_start_time.is_none());
        assert!(clear_backup_start_time);
    }

    #[test]
    fn rejects_setting_and_clearing_the_start_time_together() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--backup-start-time",
            "02:00",
            "--clear-backup-start-time",
        ])
        .err()
        .expect("conflicting flags should be rejected");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        let message = error.to_string();
        assert!(message.contains("--clear-backup-start-time"), "{message}");
        assert!(message.contains("--backup-start-time <"), "{message}");
    }
}
