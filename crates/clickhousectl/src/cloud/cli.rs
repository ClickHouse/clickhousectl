use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime};
use clap::builder::PossibleValuesParser;
use clap::{Args, Subcommand};

// Valid wire values for each ClickPipe enum the CLI accepts as a string argument.
// Kept in sync with the clickhouse-cloud-api library enums; extra variants are
// rejected by clap at parse time with a `[possible values: …]` hint.
const OBJECT_STORAGE_FORMATS: &[&str] = &[
    "JSONEachRow",
    "JSONAsObject",
    "CSV",
    "CSVWithNames",
    "TabSeparated",
    "TabSeparatedWithNames",
    "Parquet",
    "Avro",
];
const OBJECT_STORAGE_COMPRESSIONS: &[&str] = &[
    "none", "gzip", "gz", "brotli", "br", "xz", "LZMA", "zstd", "auto",
];
const OBJECT_STORAGE_TYPES: &[&str] = &[
    "s3",
    "gcs",
    "dospaces",
    "azureblobstorage",
    "cloudflarer2",
    "ovhobjectstorage",
];
const KAFKA_FORMATS: &[&str] = &["JSONEachRow", "Avro", "AvroConfluent", "Protobuf"];
const KAFKA_TYPES: &[&str] = &[
    "kafka",
    "redpanda",
    "msk",
    "gcmk",
    "confluent",
    "warpstream",
    "azureeventhub",
    "dokafka",
];
const KAFKA_AUTHS: &[&str] = &[
    "PLAIN",
    "SCRAM-SHA-256",
    "SCRAM-SHA-512",
    "IAM_ROLE",
    "IAM_USER",
    "MUTUAL_TLS",
];
const KAFKA_OFFSET_STRATEGIES: &[&str] = &["from_beginning", "from_latest", "from_timestamp"];
const KINESIS_FORMATS: &[&str] = &["JSONEachRow", "Avro", "AvroConfluent"];
const KINESIS_AUTHS: &[&str] = &["IAM_ROLE", "IAM_USER"];
const KINESIS_ITERATOR_TYPES: &[&str] = &["TRIM_HORIZON", "LATEST", "AT_TIMESTAMP"];
const POSTGRES_TYPES: &[&str] = &[
    "postgres",
    "supabase",
    "neon",
    "alloydb",
    "planetscale",
    "rdspostgres",
    "aurorapostgres",
    "cloudsqlpostgres",
    "azurepostgres",
    "crunchybridge",
    "tigerdata",
];
const DB_AUTHS: &[&str] = &["basic", "IAM_ROLE"];
const REPLICATION_MODES: &[&str] = &["cdc", "snapshot", "cdc_only"];
const MYSQL_TYPES: &[&str] = &["mysql", "rdsmysql", "auroramysql", "mariadb", "rdsmariadb"];
const MYSQL_REPLICATION_MECHANISMS: &[&str] = &["GTID", "FILE_POS"];
const MONGODB_READ_PREFERENCES: &[&str] = &[
    "primary",
    "primaryPreferred",
    "secondary",
    "secondaryPreferred",
    "nearest",
];

fn parse_date_only(value: &str) -> Result<String, String> {
    if NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() {
        return Err(format!("invalid date '{}': expected YYYY-MM-DD", value));
    }

    Ok(value.to_string())
}

pub(super) fn parse_datetime(value: &str) -> Result<String, String> {
    if DateTime::<FixedOffset>::parse_from_rfc3339(value).is_err() {
        return Err(format!(
            "invalid datetime '{}': expected ISO 8601 / RFC 3339",
            value
        ));
    }

    Ok(value.to_string())
}

fn parse_time_only(value: &str) -> Result<String, String> {
    if NaiveTime::parse_from_str(value, "%H:%M").is_err() {
        return Err(format!("invalid time '{}': expected HH:MM", value));
    }

    Ok(value.to_string())
}
#[derive(Subcommand)]
pub enum AuthCommands {
    /// Log in to ClickHouse Cloud
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Defaults to OAuth device flow (opens browser). OAuth tokens are READ-ONLY.
  For write operations, use API keys via: --api-key/--api-secret flags, or
  CLICKHOUSE_CLOUD_API_KEY / CLICKHOUSE_CLOUD_API_SECRET env vars (exported or in .env).
  Create API keys: https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl
  Related: use `clickhousectl cloud auth status` to verify.")]
    Login {
        /// Log in by entering API key/secret interactively
        #[arg(long)]
        interactive: bool,

        /// API key for non-interactive login (requires --api-secret)
        #[arg(long)]
        api_key: Option<String>,

        /// API secret for non-interactive login (requires --api-key)
        #[arg(long)]
        api_secret: Option<String>,
    },
    /// Log out and clear saved credentials
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  With no flags, clears everything. Use --oauth to keep API keys, or --api-keys to keep OAuth tokens.")]
    Logout {
        /// Clear only OAuth tokens (keep API keys)
        #[arg(long, conflicts_with = "api_keys")]
        oauth: bool,

        /// Clear only API keys (keep OAuth tokens)
        #[arg(long, conflicts_with = "oauth")]
        api_keys: bool,
    },
    /// Show current authentication status
    Status,
    /// Open the ClickHouse Cloud sign-up page in your browser
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Opens the ClickHouse Cloud sign-up page in the user's browser. This is an interactive flow —
  it requires a human to complete sign-up in the browser. Do not use in fully autonomous or CI environments.")]
    Signup,
}

#[derive(Args)]
pub struct CloudArgs {
    /// API key (or set CLICKHOUSE_CLOUD_API_KEY)
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// API secret (or set CLICKHOUSE_CLOUD_API_SECRET)
    #[arg(long, global = true)]
    pub api_secret: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Print debug info (e.g. the credential source used) to stderr before running the command
    #[arg(long, global = true)]
    pub debug: bool,

    /// API base URL (default: auto-detect from OAuth tokens, or https://api.clickhouse.cloud)
    #[cfg_attr(debug_assertions, arg(long, global = true))]
    #[cfg_attr(not(debug_assertions), arg(long, global = true, hide = true))]
    pub url: Option<String>,

    #[command(subcommand)]
    pub command: CloudCommands,
}

#[derive(Subcommand)]
pub enum CloudCommands {
    /// Manage authentication (OAuth login, API keys)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Default `login` opens a browser for OAuth (read-only).
  Use `login --api-key X --api-secret Y` for full read/write access, or set
  CLICKHOUSE_CLOUD_API_KEY / CLICKHOUSE_CLOUD_API_SECRET env vars (exported or in .env).
  Create API keys: https://clickhouse.com/docs/cloud/manage/openapi?referrer=clickhousectl
  `logout` clears all saved credentials (OAuth tokens and API keys).
  Related: `clickhousectl cloud org list` to verify credentials work.")]
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },

    /// Organization commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage ClickHouse Cloud organizations. Subcommands: list, get, update, prometheus, usage.
  Org IDs are needed for most service and backup operations.
  Start with `clickhousectl cloud org list` to discover available org IDs.
  Related: `clickhousectl cloud service list` (uses org ID).")]
    Org {
        #[command(subcommand)]
        command: OrgCommands,
    },

    /// Service commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Most commands need a service ID — get it from `clickhousectl cloud service list`.
  Org ID is auto-detected if you have only one org; otherwise pass --org-id.
  Write commands (create, delete, start, stop, update, scale) require API key auth — OAuth is read-only.
  Use `query` to run SQL against a service over HTTP.
  Related: `clickhousectl cloud org list` for org IDs.")]
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },

    /// Backup commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage ClickHouse Cloud backups. Subcommands: list, get.
  Requires a service ID — get it from `clickhousectl cloud service list`.
  Backup IDs from `backup list` can be used with `service create --backup-id` to restore.
  Related: `clickhousectl cloud service list` for service IDs.")]
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },

    // Clickpipe commands
    #[command(
        name = "clickpipe",
        after_help = "\
CONTEXT FOR AGENTS:
    Manage ClickPipes for ingesting data into ClickHouse Cloud.
    Subcommands: list, get, delete, start, stop, resync, scale, settings, create.
    Requires a service ID — get it from `clickhousectl cloud service list`."
    )]
    ClickPipe {
        #[command(subcommand)]
        command: Box<ClickPipeCommands>,
    },

    /// Manage organization members
    Member {
        #[command(subcommand)]
        command: MemberCommands,
    },

    /// Manage organization invitations
    Invitation {
        #[command(subcommand)]
        command: InvitationCommands,
    },

    /// Manage API keys
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },

    /// View activity log
    Activity {
        #[command(subcommand)]
        command: ActivityCommands,
    },

    /// Manage ClickHouse Cloud Postgres services (beta)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage ClickHouse Cloud managed Postgres services. Subcommands cover CRUD, lifecycle
  (restart/promote/switchover), CA certs, runtime config, password reset, read replicas,
  and point-in-time restore. Service IDs come from `postgres list`.
  Write commands require API key auth — OAuth is read-only.")]
    Postgres {
        #[command(subcommand)]
        command: crate::cloud::postgres::PostgresCommands,
    },
}

impl CloudCommands {
    /// Returns true if this command performs a write/mutating operation.
    /// OAuth (Bearer) auth is read-only and cannot execute write commands.
    ///
    /// Every variant is explicitly matched — no wildcards — so the compiler
    /// will error when a new command is added, forcing the developer to
    /// classify it as read or write.
    pub fn is_write_command(&self) -> bool {
        match self {
            CloudCommands::Auth { .. } => false,
            CloudCommands::Org { command } => match command {
                OrgCommands::List => false,
                OrgCommands::Get { .. } => false,
                OrgCommands::Prometheus { .. } => false,
                OrgCommands::Usage { .. } => false,
                OrgCommands::Update { .. } => true,
            },
            CloudCommands::Service { command } => match command {
                ServiceCommands::List { .. } => false,
                ServiceCommands::Get { .. } => false,
                ServiceCommands::Prometheus { .. } => false,
                ServiceCommands::Query { .. } => false,
                ServiceCommands::Create { .. } => true,
                ServiceCommands::Delete { .. } => true,
                ServiceCommands::Start { .. } => true,
                ServiceCommands::Stop { .. } => true,
                ServiceCommands::Update { .. } => true,
                ServiceCommands::Scale { .. } => true,
                ServiceCommands::ResetPassword { .. } => true,
                ServiceCommands::QueryEndpoint { command } => match command {
                    QueryEndpointCommands::Get { .. } => false,
                    QueryEndpointCommands::Create { .. } => true,
                    QueryEndpointCommands::Delete { .. } => true,
                },
                ServiceCommands::PrivateEndpoint { command } => match command {
                    PrivateEndpointCommands::Create { .. } => true,
                    PrivateEndpointCommands::GetConfig { .. } => false,
                },
                ServiceCommands::BackupConfig { command } => match command {
                    BackupConfigCommands::Get { .. } => false,
                    BackupConfigCommands::Update { .. } => true,
                },
            },
            CloudCommands::Backup { command } => match command {
                BackupCommands::List { .. } => false,
                BackupCommands::Get { .. } => false,
            },
            CloudCommands::Member { command } => match command {
                MemberCommands::List { .. } => false,
                MemberCommands::Get { .. } => false,
                MemberCommands::Update { .. } => true,
                MemberCommands::Remove { .. } => true,
            },
            CloudCommands::Invitation { command } => match command {
                InvitationCommands::List { .. } => false,
                InvitationCommands::Get { .. } => false,
                InvitationCommands::Create { .. } => true,
                InvitationCommands::Delete { .. } => true,
            },
            CloudCommands::Key { command } => match command {
                KeyCommands::List { .. } => false,
                KeyCommands::Get { .. } => false,
                KeyCommands::Create { .. } => true,
                KeyCommands::Update { .. } => true,
                KeyCommands::Delete { .. } => true,
            },
            CloudCommands::Activity { command } => match command {
                ActivityCommands::List { .. } => false,
                ActivityCommands::Get { .. } => false,
            },
            CloudCommands::Postgres { command } => command.is_write(),
            CloudCommands::ClickPipe { command } => match command.as_ref() {
                ClickPipeCommands::List { .. } => false,
                ClickPipeCommands::Get { .. } => false,
                ClickPipeCommands::Delete { .. } => true,
                ClickPipeCommands::Start { .. } => true,
                ClickPipeCommands::Stop { .. } => true,
                ClickPipeCommands::Resync { .. } => true,
                ClickPipeCommands::Scale { .. } => true,
                // Side-effect-free, but the API gateway rejects OAuth/JWT on
                // POST /clickpipes/schemaDiscovery ("This endpoint is not
                // available for JWT authentication"), so classify it as a
                // write to fail fast with the API-key guidance.
                ClickPipeCommands::SchemaDiscover { .. } => true,
                ClickPipeCommands::Create { .. } => true,
                ClickPipeCommands::Settings { command } => match command {
                    ClickPipeSettingsCommands::Get { .. } => false,
                    ClickPipeSettingsCommands::Update { .. } => true,
                },
            },
        }
    }
}

#[derive(Subcommand)]
pub enum OrgCommands {
    /// List organizations
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Returns all organizations accessible with the current API credentials.
  Use this to find org IDs needed by service and backup commands.
  Add --json for machine-readable output.
  Related: `clickhousectl cloud service list` next.")]
    List,

    /// Get organization details
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Returns details for a single organization by ID.
  Get org IDs from `clickhousectl cloud org list`.
  Add --json for machine-readable output.
  Related: `clickhousectl cloud org list` to find org IDs.")]
    Get {
        /// Organization ID
        org_id: String,
    },

    /// Update organization settings
    Update {
        /// Organization ID
        org_id: String,

        /// New organization name
        #[arg(long)]
        name: Option<String>,

        /// Remove a private endpoint from the organization allow list.
        /// Format: id[,description=TEXT][,cloud-provider=aws|gcp|azure][,region=REGION]
        #[arg(long = "remove-private-endpoint")]
        remove_private_endpoint: Vec<String>,

        /// Enable or disable core dump collection at the organization level
        #[arg(long)]
        enable_core_dumps: Option<bool>,
    },

    /// Get organization Prometheus configuration
    Prometheus {
        /// Organization ID
        org_id: String,

        /// Whether to request filtered metrics
        #[arg(long)]
        filtered_metrics: Option<bool>,
    },

    /// Get organization usage/billing information
    Usage {
        /// Organization ID
        org_id: String,

        /// Start date filter in UTC (YYYY-MM-DD, e.g. 2024-01-01)
        #[arg(long, value_parser = parse_date_only)]
        from_date: String,

        /// End date filter in UTC (YYYY-MM-DD, e.g. 2024-01-31)
        #[arg(long, value_parser = parse_date_only)]
        to_date: String,

        /// Filter by entity attributes
        #[arg(long)]
        filter: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum ServiceCommands {
    /// List all services
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Lists all services in the organization. Org ID is auto-detected if only one org exists.
  Returns service IDs needed by get, delete, start, stop, and backup commands.
  Add --json for machine-readable output.
  Related: `clickhousectl cloud service get <id>` for full details.")]
    List {
        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,

        /// Filter by resource tags (e.g., "tag:env=production")
        #[arg(long)]
        filter: Vec<String>,
    },

    /// Get service details
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Returns full service details: status, endpoints, scaling config, IP access list.
  Get the service ID from `clickhousectl cloud service list`.
  Add --json for machine-readable output.
  Related: `clickhousectl cloud service start/stop <id>` to change state.")]
    Get {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Create a new service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Creates a new ClickHouse Cloud service. Only --name is required; other fields have defaults.
  Returns the new service ID and initial password — save these.
  Typical: `clickhousectl cloud service create --name my-svc`.
  Defaults: provider=aws, region=us-east-1. Add --json for machine-readable output.
  Related: `clickhousectl cloud service get <id>` to check status after creation.")]
    Create {
        /// Service name (required)
        #[arg(long)]
        name: String,

        /// Cloud provider: aws, gcp, azure (required)
        #[arg(long, default_value = "aws")]
        provider: String,

        /// Region (required). Examples: us-east-1, eu-west-1, us-central1
        #[arg(long, default_value = "us-east-1")]
        region: String,

        /// Minimum memory per replica in GB (8-356, multiple of 4). Horizontal
        /// autoscaling requires it equal to --max-replica-memory-gb.
        #[arg(long)]
        min_replica_memory_gb: Option<u32>,

        /// Maximum memory per replica in GB (8-356, multiple of 4). Horizontal
        /// autoscaling requires it equal to --min-replica-memory-gb.
        #[arg(long)]
        max_replica_memory_gb: Option<u32>,

        /// Number of replicas (1-20). Vertical autoscaling; mutually exclusive
        /// with the horizontal band (--min-replicas/--max-replicas).
        #[arg(long, conflicts_with_all = ["min_replicas", "max_replicas"])]
        num_replicas: Option<u32>,

        /// Minimum number of replicas for horizontal autoscaling (requires the
        /// horizontal autoscaling org feature). Mutually exclusive with --num-replicas.
        #[arg(long, conflicts_with = "num_replicas")]
        min_replicas: Option<u32>,

        /// Maximum number of replicas for horizontal autoscaling (requires the
        /// horizontal autoscaling org feature). Mutually exclusive with --num-replicas.
        #[arg(long, conflicts_with = "num_replicas")]
        max_replicas: Option<u32>,

        /// Autoscaling mode: vertical (default) or horizontal. Horizontal uses fixed
        /// memory per replica (--min-replica-memory-gb equal to --max-replica-memory-gb)
        /// with a variable replica count (--min-replicas/--max-replicas); vertical uses
        /// a fixed replica count (--num-replicas) with variable memory.
        #[arg(
            long,
            value_parser = PossibleValuesParser::new(
                clickhouse_cloud_api::models::AutoscalingMode::VALUES
            )
        )]
        autoscaling_mode: Option<String>,

        /// Allow scale to zero when idle (default: true)
        #[arg(long)]
        idle_scaling: Option<bool>,

        /// Minimum idle timeout in minutes (>= 5)
        #[arg(long)]
        idle_timeout_minutes: Option<u32>,

        /// IP addresses to allow (CIDR format, e.g., "0.0.0.0/0"). Can be specified multiple times
        #[arg(long = "ip-allow")]
        ip_allow: Vec<String>,

        /// Backup ID to restore from
        #[arg(long)]
        backup_id: Option<String>,

        /// Release channel: slow, default, fast
        #[arg(long)]
        release_channel: Option<String>,

        /// Data warehouse ID (for creating read replicas)
        #[arg(long)]
        data_warehouse_id: Option<String>,

        /// Make service read-only (requires --data-warehouse-id)
        #[arg(long)]
        readonly: bool,

        /// Customer-provided disk encryption key
        #[arg(long)]
        encryption_key: Option<String>,

        /// Role ARN for disk encryption
        #[arg(long)]
        encryption_role: Option<String>,

        /// Enable Transparent Data Encryption (enterprise only)
        #[arg(long)]
        enable_tde: bool,

        /// Compliance type: hipaa, pci
        #[arg(long)]
        compliance_type: Option<String>,

        /// Instance profile (enterprise only): v1-default, v1-highmem-xs, etc.
        #[arg(long)]
        profile: Option<String>,

        /// Tag to attach to the service. Format: key or key=value
        #[arg(long = "tag", value_name = "KEY[=VALUE]")]
        tag: Vec<String>,

        /// Enable a toggleable endpoint protocol. Currently supported: mysql
        #[arg(long = "enable-endpoint")]
        enable_endpoint: Vec<String>,

        /// Disable a toggleable endpoint protocol. Currently supported: mysql
        #[arg(long = "disable-endpoint")]
        disable_endpoint: Vec<String>,

        /// Accept private preview terms for eligible service creation flows
        #[arg(long)]
        private_preview_terms_checked: bool,

        /// Enable or disable service core dump collection
        #[arg(long)]
        enable_core_dumps: Option<bool>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete a service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Permanently deletes a ClickHouse Cloud service. This action is irreversible.
  Use --force to stop a running service before deleting it in one step.
  Related: `clickhousectl cloud service stop <id>` to idle instead of delete.")]
    Delete {
        /// Service ID
        service_id: String,

        /// Stop the service first if it is running, then delete
        #[arg(long)]
        force: bool,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Start a service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Resumes a stopped/idled ClickHouse Cloud service.
  Takes a service ID — get it from `clickhousectl cloud service list`.
  Add --json for machine-readable output.
  Related: `clickhousectl cloud service get <id>` to check status, `clickhousectl cloud service stop <id>` to idle.")]
    Start {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Stop a service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Idles a ClickHouse Cloud service, stopping billing for compute.
  Data is preserved. Takes a service ID — get it from `clickhousectl cloud service list`.
  Add --json for machine-readable output.
  Related: `clickhousectl cloud service start <id>` to resume, `clickhousectl cloud service delete <id>` to remove.")]
    Stop {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update service settings
    Update {
        /// Service ID
        service_id: String,

        /// New service name
        #[arg(long)]
        name: Option<String>,

        /// Add an IP/CIDR entry to the service allow list
        #[arg(long = "add-ip-allow")]
        add_ip_allow: Vec<String>,

        /// Remove an IP/CIDR entry from the service allow list
        #[arg(long = "remove-ip-allow")]
        remove_ip_allow: Vec<String>,

        /// Add a private endpoint ID to the service
        #[arg(long = "add-private-endpoint-id")]
        add_private_endpoint_id: Vec<String>,

        /// Remove a private endpoint ID from the service
        #[arg(long = "remove-private-endpoint-id")]
        remove_private_endpoint_id: Vec<String>,

        /// Release channel: slow, default, fast
        #[arg(long)]
        release_channel: Option<String>,

        /// Enable a toggleable endpoint protocol. Currently supported: mysql
        #[arg(long = "enable-endpoint")]
        enable_endpoint: Vec<String>,

        /// Disable a toggleable endpoint protocol. Currently supported: mysql
        #[arg(long = "disable-endpoint")]
        disable_endpoint: Vec<String>,

        /// Transparent Data Encryption key ID to rotate to
        #[arg(long)]
        transparent_data_encryption_key_id: Option<String>,

        /// Tag to add. Format: key or key=value
        #[arg(long = "add-tag", value_name = "KEY[=VALUE]")]
        add_tag: Vec<String>,

        /// Tag to remove. Format: key or key=value
        #[arg(long = "remove-tag", value_name = "KEY[=VALUE]")]
        remove_tag: Vec<String>,

        /// Enable or disable service core dump collection
        #[arg(long)]
        enable_core_dumps: Option<bool>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update replica scaling settings
    Scale {
        /// Service ID
        service_id: String,

        /// Minimum memory per replica in GB (8-356, multiple of 4). Horizontal
        /// autoscaling requires it equal to --max-replica-memory-gb.
        #[arg(long)]
        min_replica_memory_gb: Option<u32>,

        /// Maximum memory per replica in GB (8-356, multiple of 4). Horizontal
        /// autoscaling requires it equal to --min-replica-memory-gb.
        #[arg(long)]
        max_replica_memory_gb: Option<u32>,

        /// Number of replicas (1-20). Vertical autoscaling; mutually exclusive
        /// with the horizontal band (--min-replicas/--max-replicas).
        #[arg(long, conflicts_with_all = ["min_replicas", "max_replicas"])]
        num_replicas: Option<u32>,

        /// Minimum number of replicas for horizontal autoscaling (requires the
        /// horizontal autoscaling org feature). Mutually exclusive with --num-replicas.
        #[arg(long, conflicts_with = "num_replicas")]
        min_replicas: Option<u32>,

        /// Maximum number of replicas for horizontal autoscaling (requires the
        /// horizontal autoscaling org feature). Mutually exclusive with --num-replicas.
        #[arg(long, conflicts_with = "num_replicas")]
        max_replicas: Option<u32>,

        /// Autoscaling mode: vertical (default) or horizontal. Omit to keep the
        /// service's current mode. See `service create --autoscaling-mode`.
        #[arg(
            long,
            value_parser = PossibleValuesParser::new(
                clickhouse_cloud_api::models::AutoscalingMode::VALUES
            )
        )]
        autoscaling_mode: Option<String>,

        /// Allow scale to zero when idle
        #[arg(long)]
        idle_scaling: Option<bool>,

        /// Minimum idle timeout in minutes (>= 5)
        #[arg(long)]
        idle_timeout_minutes: Option<u32>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Reset a service's default user password
    ResetPassword {
        /// Service ID
        service_id: String,

        /// SHA256 password hash encoded as base64
        #[arg(long)]
        new_password_hash: Option<String>,

        /// MySQL-compatible double SHA1 password hash
        #[arg(long)]
        new_double_sha1_hash: Option<String>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Manage query endpoints
    #[command(name = "query-endpoint")]
    QueryEndpoint {
        #[command(subcommand)]
        command: QueryEndpointCommands,
    },

    /// Manage private endpoints for a service
    #[command(name = "private-endpoint")]
    PrivateEndpoint {
        #[command(subcommand)]
        command: PrivateEndpointCommands,
    },

    /// Manage backup configuration for a service
    #[command(name = "backup-config")]
    BackupConfig {
        #[command(subcommand)]
        command: BackupConfigCommands,
    },

    /// Get service Prometheus metrics
    Prometheus {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,

        /// Whether to request filtered metrics
        #[arg(long)]
        filtered_metrics: Option<bool>,
    },

    /// Run a SQL query against a cloud service over HTTP via the Query API
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Runs SQL over HTTP — no local clickhouse binary or service password required.
  With API key auth: uses a per-service API key (read+write, scoped to this
  service via the query endpoint binding) auto-provisioned on first use and
  stored in .clickhouse/credentials.json.
  With OAuth (cloud auth login): sends your own bearer token — SQL runs as
  your cloud user with read-only access (SELECT only, no writes); no key
  provisioning and no query endpoint required on the service.
  SQL precedence: --query > --queries-file > stdin. Default format: PrettyCompact
  on a TTY, TabSeparated when piped.")]
    Query {
        /// Service name to query (exactly one of --name or --id is required)
        #[arg(long, conflicts_with = "id")]
        name: Option<String>,

        /// Service ID to query
        #[arg(long, conflicts_with = "name")]
        id: Option<String>,

        /// Execute a SQL query
        #[arg(long, short)]
        query: Option<String>,

        /// Execute queries from a SQL file (use "-" for stdin)
        #[arg(long)]
        queries_file: Option<String>,

        /// Target database
        #[arg(long)]
        database: Option<String>,

        /// Response format (e.g. JSONEachRow, CSV, TabSeparated, PrettyCompact)
        #[arg(long)]
        format: Option<String>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,

        /// Fail instead of auto-provisioning the query endpoint + API key
        /// when none is stored locally (API key auth only; with OAuth nothing
        /// is ever provisioned, so this flag has no effect)
        #[arg(long)]
        no_auto_enable: bool,
    },
}

#[derive(Subcommand)]
pub enum QueryEndpointCommands {
    /// Get query endpoint configuration
    Get {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Create/enable query endpoint
    Create {
        /// Service ID
        service_id: String,

        /// Roles to grant access (can be specified multiple times)
        #[arg(long)]
        role: Vec<String>,

        /// OpenAPI key IDs to authorize
        #[arg(long = "open-api-key")]
        open_api_key: Vec<String>,

        /// Allowed origins string for browser access
        #[arg(long)]
        allowed_origins: Option<String>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete/disable query endpoint
    Delete {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum PrivateEndpointCommands {
    /// Create a private endpoint connection
    Create {
        /// Service ID
        service_id: String,

        /// Private endpoint ID (VPC endpoint ID)
        #[arg(long)]
        endpoint_id: String,

        /// Description
        #[arg(long)]
        description: Option<String>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get service private endpoint configuration
    GetConfig {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },
}

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
#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ClickPipeCommands {
    /// List ClickPipes
    List {
        /// Service ID
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get ClickPipe details
    Get {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete a ClickPipe
    Delete {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Start a ClickPipe
    Start {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Stop a ClickPipe
    Stop {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Resync a ClickPipe (CDC pipes only)
    Resync {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update scaling configuration
    Scale {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Number of replicas (1-40)
        #[arg(long)]
        replicas: Option<u32>,

        /// CPU millicores per replica (125-2000, streaming pipes)
        #[arg(long)]
        cpu_millicores: Option<u32>,

        /// Memory GB per replica (0.5-8, streaming pipes)
        #[arg(long)]
        memory_gb: Option<f64>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Manage ClickPipe settings
    Settings {
        #[command(subcommand)]
        command: ClickPipeSettingsCommands,
    },

    /// Discover a source schema without creating a pipe (beta)
    #[command(after_help = "\\
CONTEXT FOR AGENTS:
  Infers the schema (column name + ClickHouse type) for a Kafka or Kinesis source
  without creating a ClickPipe. Useful for filling in --column on `clickpipe create`.
  Related: `clickhousectl cloud clickpipe create kafka|kinesis` to create a pipe with the discovered columns.")]
    SchemaDiscover {
        /// Service ID
        service_id: String,

        #[command(subcommand)]
        command: ClickPipeSchemaDiscoverCommands,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Create a ClickPipe
    Create {
        #[command(subcommand)]
        command: ClickPipeCreateCommands,
    },
}

#[derive(Subcommand)]
pub enum ClickPipeSchemaDiscoverCommands {
    /// Discover schema from a Kafka or Kafka-compatible source
    Kafka(Box<KafkaSourceFields>),

    /// Discover schema from an Amazon Kinesis stream
    Kinesis(Box<KinesisSourceFields>),
}

#[derive(Subcommand)]
pub enum ClickPipeSettingsCommands {
    /// Get ClickPipe settings
    Get {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update ClickPipe settings
    Update {
        /// Service ID
        service_id: String,

        /// ClickPipe ID
        clickpipe_id: String,

        /// Max wait before inserting data (ms, 500-60000)
        #[arg(long)]
        streaming_max_insert_wait_ms: Option<u32>,

        /// Concurrent file processing threads (1-35)
        #[arg(long)]
        object_storage_concurrency: Option<u32>,

        /// Polling interval for continuous ingest (ms, 100-3600000)
        #[arg(long)]
        object_storage_polling_interval_ms: Option<u32>,

        /// Bytes per insert batch
        #[arg(long)]
        object_storage_max_insert_bytes: Option<u64>,

        /// Max files per insert batch (1-10000)
        #[arg(long)]
        object_storage_max_file_count: Option<u32>,

        /// Max concurrent threads for file processing (0-64)
        #[arg(long)]
        clickhouse_max_threads: Option<u32>,

        /// Max concurrent insert threads (0-16)
        #[arg(long)]
        clickhouse_max_insert_threads: Option<u32>,

        /// Use ClickHouse cluster function
        #[arg(long)]
        object_storage_use_cluster_function: Option<bool>,

        /// Push to attached views concurrently
        #[arg(long)]
        clickhouse_parallel_view_processing: Option<bool>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ClickPipeCreateCommands {
    /// Create a ClickPipe from S3, GCS, Azure Blob, or other object storage
    #[command(name = "object-storage")]
    ObjectStorage(ObjectStorageCreateArgs),

    /// Create a ClickPipe from Kafka or Kafka-compatible source
    Kafka(KafkaCreateArgs),

    /// Create a ClickPipe from Amazon Kinesis
    Kinesis(KinesisCreateArgs),

    /// Create a ClickPipe from PostgreSQL
    Postgres(PostgresCreateArgs),

    /// Create a ClickPipe from MySQL
    #[command(name = "mysql")]
    MySQL(MySqlCreateArgs),

    /// Create a ClickPipe from MongoDB
    #[command(name = "mongodb")]
    MongoDB(MongoDbCreateArgs),

    /// Create a ClickPipe from BigQuery
    #[command(name = "bigquery")]
    BigQuery(BigQueryCreateArgs),
}

#[derive(Args, Debug)]
pub struct ObjectStorageCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// Source URL (e.g., https://bucket.s3.region.amazonaws.com/path/*.json)
    #[arg(long)]
    pub source_url: String,

    /// Data format
    #[arg(long, value_parser = PossibleValuesParser::new(OBJECT_STORAGE_FORMATS))]
    pub format: String,

    /// Destination database
    #[arg(long)]
    pub database: String,

    /// Destination table
    #[arg(long)]
    pub table: String,

    /// Destination columns as name:type pairs (e.g., --column "event_id:Int64" --column "name:String")
    #[arg(long = "column")]
    pub columns: Vec<String>,

    /// Storage type
    #[arg(
        long,
        default_value = "s3",
        value_parser = PossibleValuesParser::new(OBJECT_STORAGE_TYPES),
    )]
    pub storage_type: String,

    /// Compression
    #[arg(
        long,
        default_value = "auto",
        value_parser = PossibleValuesParser::new(OBJECT_STORAGE_COMPRESSIONS),
    )]
    pub compression: String,

    /// Enable continuous ingestion
    #[arg(long)]
    pub continuous: bool,

    /// SQS queue URL for continuous ingestion notifications
    #[arg(long)]
    pub queue_url: Option<String>,

    /// Skip the initial load of existing objects and ingest only queue-notification
    /// files. Only applicable when --queue-url is provided.
    #[arg(long, requires = "queue_url")]
    pub skip_initial_load: bool,

    /// Object key to start continuous ingestion after. Mutually exclusive with
    /// --skip-initial-load (the API rejects both being set).
    #[arg(long, conflicts_with = "skip_initial_load")]
    pub start_after: Option<String>,

    /// CSV delimiter character (e.g., ",")
    #[arg(long)]
    pub delimiter: Option<String>,

    /// IAM role ARN for authentication
    #[arg(long)]
    pub iam_role: Option<String>,

    /// Access key ID for authentication
    #[arg(long, requires = "secret_key")]
    pub access_key_id: Option<String>,

    /// Secret key for authentication
    #[arg(long, requires = "access_key_id")]
    pub secret_key: Option<String>,

    /// Azure connection string for authentication
    #[arg(long)]
    pub connection_string: Option<String>,

    /// Azure container name
    #[arg(long)]
    pub azure_container_name: Option<String>,

    /// Object storage path (for Azure)
    #[arg(long)]
    pub path: Option<String>,

    /// Path to GCP service account JSON key file
    #[arg(long)]
    pub service_account_file: Option<String>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

/// Source-connection fields for a Kafka / Kafka-compatible ClickPipe source.
/// Flattened into both `KafkaCreateArgs` (pipe creation) and the schema-discover
/// Kafka subcommand so the source field set has a single definition.
#[derive(Args, Debug)]
pub struct KafkaSourceFields {
    /// Kafka broker(s) (e.g., "broker1:9092,broker2:9092")
    #[arg(long)]
    pub brokers: String,

    /// Topic(s) to consume from
    #[arg(long)]
    pub topics: String,

    /// Data format
    #[arg(long, value_parser = PossibleValuesParser::new(KAFKA_FORMATS))]
    pub format: String,

    /// Kafka type
    #[arg(
        long,
        default_value = "kafka",
        value_parser = PossibleValuesParser::new(KAFKA_TYPES),
    )]
    pub kafka_type: String,

    /// Consumer group name
    #[arg(long)]
    pub consumer_group: Option<String>,

    /// Authentication method
    #[arg(long, value_parser = PossibleValuesParser::new(KAFKA_AUTHS))]
    pub auth: Option<String>,

    /// Username for PLAIN/SCRAM authentication
    #[arg(long, requires = "password")]
    pub username: Option<String>,

    /// Password for PLAIN/SCRAM authentication
    #[arg(long, requires = "username")]
    pub password: Option<String>,

    /// IAM role ARN for MSK IAM authentication
    #[arg(long)]
    pub iam_role: Option<String>,

    /// Access key ID for IAM_USER authentication
    #[arg(long, requires = "secret_key")]
    pub access_key_id: Option<String>,

    /// Secret key for IAM_USER authentication
    #[arg(long, requires = "access_key_id")]
    pub secret_key: Option<String>,

    /// Offset strategy
    #[arg(
        long,
        default_value = "from_beginning",
        value_parser = PossibleValuesParser::new(KAFKA_OFFSET_STRATEGIES),
    )]
    pub offset: String,

    /// Timestamp for from_timestamp offset (e.g., "2021-01-01T00:00")
    #[arg(long)]
    pub offset_timestamp: Option<String>,

    /// Schema registry URL (for Avro/Protobuf formats)
    #[arg(long)]
    pub schema_registry_url: Option<String>,

    /// Schema registry username
    #[arg(long)]
    pub schema_registry_username: Option<String>,

    /// Schema registry password
    #[arg(long)]
    pub schema_registry_password: Option<String>,

    /// Path to broker CA certificate file
    #[arg(long)]
    pub ca_certificate: Option<String>,

    /// Path to client certificate file (for MUTUAL_TLS auth)
    #[arg(long)]
    pub client_certificate: Option<String>,

    /// Path to client private key file (for MUTUAL_TLS auth)
    #[arg(long)]
    pub client_key: Option<String>,

    /// Path to schema registry CA certificate file
    #[arg(long)]
    pub schema_registry_ca_certificate: Option<String>,

    /// Reverse private endpoint IDs (repeatable)
    #[arg(long = "reverse-private-endpoint-id")]
    pub reverse_private_endpoint_ids: Vec<String>,
}

#[derive(Args, Debug)]
pub struct KafkaCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub source: KafkaSourceFields,

    /// Destination database
    #[arg(long)]
    pub database: String,

    /// Destination table
    #[arg(long)]
    pub table: String,

    /// Destination columns as name:type pairs (e.g., --column "event_id:Int64")
    #[arg(long = "column")]
    pub columns: Vec<String>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

/// Source-connection fields for an Amazon Kinesis ClickPipe source.
/// Flattened into both `KinesisCreateArgs` (pipe creation) and the schema-discover
/// Kinesis subcommand so the source field set has a single definition.
#[derive(Args, Debug)]
pub struct KinesisSourceFields {
    /// Kinesis stream name
    #[arg(long)]
    pub stream_name: String,

    /// AWS region (e.g., us-east-1)
    #[arg(long)]
    pub region: String,

    /// Data format
    #[arg(long, value_parser = PossibleValuesParser::new(KINESIS_FORMATS))]
    pub format: String,

    /// Authentication
    #[arg(
        long,
        default_value = "IAM_ROLE",
        value_parser = PossibleValuesParser::new(KINESIS_AUTHS),
    )]
    pub auth: String,

    /// IAM role ARN
    #[arg(long)]
    pub iam_role: Option<String>,

    /// Access key ID for IAM_USER authentication
    #[arg(long, requires = "secret_key")]
    pub access_key_id: Option<String>,

    /// Secret key for IAM_USER authentication
    #[arg(long, requires = "access_key_id")]
    pub secret_key: Option<String>,

    /// Iterator type
    #[arg(
        long,
        default_value = "TRIM_HORIZON",
        value_parser = PossibleValuesParser::new(KINESIS_ITERATOR_TYPES),
    )]
    pub iterator_type: String,

    /// Unix timestamp for AT_TIMESTAMP iterator type
    #[arg(long)]
    pub iterator_timestamp: Option<u64>,

    /// Enable enhanced fan-out
    #[arg(long)]
    pub enhanced_fan_out: bool,
}

#[derive(Args, Debug)]
pub struct KinesisCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    #[command(flatten)]
    pub source: KinesisSourceFields,

    /// Destination database
    #[arg(long)]
    pub database: String,

    /// Destination table
    #[arg(long)]
    pub table: String,

    /// Destination columns as name:type pairs (e.g., --column "event_id:Int64")
    #[arg(long = "column")]
    pub columns: Vec<String>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct PostgresCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// PostgreSQL host
    #[arg(long)]
    pub host: String,

    /// PostgreSQL port
    #[arg(long, default_value = "5432")]
    pub port: u16,

    /// Source database name
    #[arg(long)]
    pub pg_database: String,

    /// Username
    #[arg(long)]
    pub username: String,

    /// Password
    #[arg(long)]
    pub password: String,

    /// Table mappings as schema.table:target_table (repeatable)
    #[arg(long = "table-mapping")]
    pub table_mappings: Vec<String>,

    /// Postgres type
    #[arg(
        long,
        default_value = "postgres",
        value_parser = PossibleValuesParser::new(POSTGRES_TYPES),
    )]
    pub postgres_type: String,

    /// Replication mode
    #[arg(
        long,
        default_value = "cdc",
        value_parser = PossibleValuesParser::new(REPLICATION_MODES),
    )]
    pub replication_mode: String,

    /// Authentication
    #[arg(
        long,
        default_value = "basic",
        value_parser = PossibleValuesParser::new(DB_AUTHS),
    )]
    pub auth: String,

    /// IAM role ARN
    #[arg(long)]
    pub iam_role: Option<String>,

    /// TLS hostname
    #[arg(long)]
    pub tls_host: Option<String>,

    /// Path to CA certificate file
    #[arg(long)]
    pub ca_certificate: Option<String>,

    /// Postgres publication name
    #[arg(long)]
    pub publication_name: Option<String>,

    /// Replication slot name
    #[arg(long)]
    pub replication_slot_name: Option<String>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct MySqlCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// MySQL host
    #[arg(long)]
    pub host: String,

    /// MySQL port
    #[arg(long, default_value = "3306")]
    pub port: u16,

    /// Username
    #[arg(long)]
    pub username: String,

    /// Password
    #[arg(long)]
    pub password: String,

    /// Table mappings as schema.table:target_table (repeatable)
    #[arg(long = "table-mapping")]
    pub table_mappings: Vec<String>,

    /// MySQL type
    #[arg(
        long,
        default_value = "mysql",
        value_parser = PossibleValuesParser::new(MYSQL_TYPES),
    )]
    pub mysql_type: String,

    /// Replication mode
    #[arg(
        long,
        default_value = "cdc",
        value_parser = PossibleValuesParser::new(REPLICATION_MODES),
    )]
    pub replication_mode: String,

    /// Replication mechanism
    #[arg(
        long,
        default_value = "GTID",
        value_parser = PossibleValuesParser::new(MYSQL_REPLICATION_MECHANISMS),
    )]
    pub replication_mechanism: String,

    /// Authentication
    #[arg(
        long,
        default_value = "basic",
        value_parser = PossibleValuesParser::new(DB_AUTHS),
    )]
    pub auth: String,

    /// IAM role ARN
    #[arg(long)]
    pub iam_role: Option<String>,

    /// TLS hostname
    #[arg(long)]
    pub tls_host: Option<String>,

    /// Path to CA certificate file
    #[arg(long)]
    pub ca_certificate: Option<String>,

    /// Disable TLS
    #[arg(long)]
    pub disable_tls: bool,

    /// Skip certificate verification
    #[arg(long)]
    pub skip_cert_verification: bool,

    /// Optional MySQL server_id the pipe declares itself as in the MySQL
    /// replication topology (1-4294967295). Must be unique across replicas
    /// connected to the source. If omitted, one is assigned automatically.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..=4294967295))]
    pub server_id: Option<u64>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct MongoDbCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// MongoDB connection URI (e.g., mongodb+srv://cluster0.example.mongodb.net/mydb)
    #[arg(long)]
    pub uri: String,

    /// Username
    #[arg(long)]
    pub username: String,

    /// Password
    #[arg(long)]
    pub password: String,

    /// Table mappings as database.collection:target_table (repeatable)
    #[arg(long = "table-mapping")]
    pub table_mappings: Vec<String>,

    /// Replication mode
    #[arg(
        long,
        default_value = "cdc",
        value_parser = PossibleValuesParser::new(REPLICATION_MODES),
    )]
    pub replication_mode: String,

    /// Read preference
    #[arg(
        long,
        default_value = "secondaryPreferred",
        value_parser = PossibleValuesParser::new(MONGODB_READ_PREFERENCES),
    )]
    pub read_preference: String,

    /// TLS hostname
    #[arg(long)]
    pub tls_host: Option<String>,

    /// Path to CA certificate file
    #[arg(long)]
    pub ca_certificate: Option<String>,

    /// Disable TLS
    #[arg(long)]
    pub disable_tls: bool,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct BigQueryCreateArgs {
    /// Service ID
    pub service_id: String,

    /// ClickPipe name
    #[arg(long)]
    pub name: String,

    /// Path to GCP service account JSON key file
    #[arg(long)]
    pub service_account_file: String,

    /// GCS staging path for snapshot data
    #[arg(long)]
    pub staging_path: String,

    /// Table mappings as dataset.table:target_table (repeatable)
    #[arg(long = "table-mapping")]
    pub table_mappings: Vec<String>,

    /// Organization ID (auto-detected if not specified)
    #[arg(long)]
    pub org_id: Option<String>,
}

#[derive(Subcommand)]
pub enum MemberCommands {
    /// List organization members
    List {
        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get member details
    Get {
        /// User ID
        user_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update member roles
    Update {
        /// User ID
        user_id: String,

        /// Role IDs to assign (can be specified multiple times)
        #[arg(long)]
        role_id: Vec<String>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Remove a member from the organization
    Remove {
        /// User ID
        user_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum InvitationCommands {
    /// List pending invitations
    List {
        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Create an invitation
    Create {
        /// Email address to invite
        #[arg(long)]
        email: String,

        /// Role IDs to assign (can be specified multiple times)
        #[arg(long)]
        role_id: Vec<String>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get invitation details
    Get {
        /// Invitation ID
        invitation_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete an invitation
    Delete {
        /// Invitation ID
        invitation_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum KeyCommands {
    /// List API keys
    List {
        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Create an API key
    Create {
        /// Key name
        #[arg(long)]
        name: String,

        /// Role IDs to assign (can be specified multiple times)
        #[arg(long)]
        role_id: Vec<String>,

        /// Expiration datetime (ISO 8601 / RFC 3339, e.g. 2025-12-31T23:59:59Z)
        #[arg(long, value_parser = parse_datetime)]
        expires_at: Option<String>,

        /// Key state (enabled or disabled)
        #[arg(long)]
        state: Option<String>,

        /// IP/CIDR entries allowed to use the key
        #[arg(long = "ip-allow")]
        ip_allow: Vec<String>,

        /// Pre-hashed key ID digest
        #[arg(long)]
        hash_key_id: Option<String>,

        /// Suffix of the pre-hashed key ID
        #[arg(long)]
        hash_key_id_suffix: Option<String>,

        /// Pre-hashed key secret digest
        #[arg(long)]
        hash_key_secret: Option<String>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get API key details
    Get {
        /// API key ID
        key_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update an API key
    Update {
        /// API key ID
        key_id: String,

        /// New key name
        #[arg(long)]
        name: Option<String>,

        /// Role IDs to assign (can be specified multiple times)
        #[arg(long)]
        role_id: Vec<String>,

        /// Expiration datetime (ISO 8601 / RFC 3339, e.g. 2025-12-31T23:59:59Z)
        #[arg(long, value_parser = parse_datetime)]
        expires_at: Option<String>,

        /// Key state (e.g., enabled, disabled)
        #[arg(long)]
        state: Option<String>,

        /// IP/CIDR entries allowed to use the key
        #[arg(long = "ip-allow")]
        ip_allow: Vec<String>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete an API key
    Delete {
        /// API key ID
        key_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ActivityCommands {
    /// List activity log entries
    List {
        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,

        /// Start date filter in UTC (YYYY-MM-DD, e.g. 2024-01-01)
        #[arg(long, value_parser = parse_date_only)]
        from_date: Option<String>,

        /// End date filter in UTC (YYYY-MM-DD, e.g. 2024-12-31)
        #[arg(long, value_parser = parse_date_only)]
        to_date: Option<String>,
    },

    /// Get activity log entry details
    Get {
        /// Activity ID
        activity_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
    },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    #[test]
    fn parses_service_update_ga_patch_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "update",
            "svc-1",
            "--add-ip-allow",
            "10.0.0.0/8",
            "--remove-ip-allow",
            "0.0.0.0/0",
            "--add-private-endpoint-id",
            "pe-1",
            "--remove-private-endpoint-id",
            "pe-2",
            "--release-channel",
            "fast",
            "--enable-endpoint",
            "mysql",
            "--add-tag",
            "env=prod",
            "--enable-core-dumps",
            "true",
        ])
        .unwrap();

        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let ServiceCommands::Update {
            service_id,
            add_ip_allow,
            remove_ip_allow,
            add_private_endpoint_id,
            remove_private_endpoint_id,
            release_channel,
            enable_endpoint,
            add_tag,
            enable_core_dumps,
            ..
        } = command
        else {
            panic!("expected service update");
        };

        assert_eq!(service_id, "svc-1");
        assert_eq!(add_ip_allow, vec!["10.0.0.0/8"]);
        assert_eq!(remove_ip_allow, vec!["0.0.0.0/0"]);
        assert_eq!(add_private_endpoint_id, vec!["pe-1"]);
        assert_eq!(remove_private_endpoint_id, vec!["pe-2"]);
        assert_eq!(release_channel.as_deref(), Some("fast"));
        assert_eq!(enable_endpoint, vec!["mysql"]);
        assert_eq!(add_tag, vec!["env=prod"]);
        assert_eq!(enable_core_dumps, Some(true));
    }

    #[test]
    fn parses_service_create_horizontal_autoscaling_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "create",
            "--name",
            "s",
            "--min-replicas",
            "2",
            "--max-replicas",
            "8",
            "--autoscaling-mode",
            "horizontal",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let ServiceCommands::Create {
            min_replicas,
            max_replicas,
            autoscaling_mode,
            num_replicas,
            min_replica_memory_gb,
            max_replica_memory_gb,
            ..
        } = command
        else {
            panic!("expected service create");
        };
        assert_eq!(min_replicas, Some(2));
        assert_eq!(max_replicas, Some(8));
        assert_eq!(autoscaling_mode.as_deref(), Some("horizontal"));
        assert!(num_replicas.is_none());
        assert!(min_replica_memory_gb.is_none());
        assert!(max_replica_memory_gb.is_none());
    }

    #[test]
    fn rejects_service_create_horizontal_vertical_mix() {
        // --min-replicas conflicts with the vertical flags
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "create",
            "--name",
            "s",
            "--min-replicas",
            "2",
            "--max-replicas",
            "8",
            "--num-replicas",
            "3",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_service_create_horizontal_mode_with_memory_bounds() {
        // Horizontal requires equal memory bounds, so the memory flags must
        // combine with the mode and the replica band in one invocation.
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "create",
            "--name",
            "s",
            "--autoscaling-mode",
            "horizontal",
            "--min-replicas",
            "2",
            "--max-replicas",
            "8",
            "--min-replica-memory-gb",
            "16",
            "--max-replica-memory-gb",
            "16",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let ServiceCommands::Create {
            min_replicas,
            max_replicas,
            autoscaling_mode,
            min_replica_memory_gb,
            max_replica_memory_gb,
            ..
        } = command
        else {
            panic!("expected service create");
        };
        assert_eq!(min_replicas, Some(2));
        assert_eq!(max_replicas, Some(8));
        assert_eq!(autoscaling_mode.as_deref(), Some("horizontal"));
        assert_eq!(min_replica_memory_gb, Some(16));
        assert_eq!(max_replica_memory_gb, Some(16));
    }

    #[test]
    fn rejects_service_create_invalid_autoscaling_mode() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "create",
            "--name",
            "s",
            "--min-replicas",
            "2",
            "--max-replicas",
            "8",
            "--autoscaling-mode",
            "turbo",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_service_scale_horizontal_autoscaling_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "scale",
            "svc-1",
            "--min-replicas",
            "2",
            "--max-replicas",
            "8",
            "--autoscaling-mode",
            "horizontal",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let ServiceCommands::Scale {
            service_id,
            min_replicas,
            max_replicas,
            autoscaling_mode,
            num_replicas,
            ..
        } = command
        else {
            panic!("expected service scale");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(min_replicas, Some(2));
        assert_eq!(max_replicas, Some(8));
        assert_eq!(autoscaling_mode.as_deref(), Some("horizontal"));
        assert!(num_replicas.is_none());
    }

    #[test]
    fn parses_service_scale_switch_to_vertical_in_one_call() {
        // Switching a horizontal service back to vertical sends the mode and
        // the target replica count in a single request.
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "scale",
            "svc-1",
            "--autoscaling-mode",
            "vertical",
            "--num-replicas",
            "3",
            "--min-replica-memory-gb",
            "8",
            "--max-replica-memory-gb",
            "32",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let ServiceCommands::Scale {
            autoscaling_mode,
            num_replicas,
            min_replica_memory_gb,
            max_replica_memory_gb,
            min_replicas,
            max_replicas,
            ..
        } = command
        else {
            panic!("expected service scale");
        };
        assert_eq!(autoscaling_mode.as_deref(), Some("vertical"));
        assert_eq!(num_replicas, Some(3));
        assert_eq!(min_replica_memory_gb, Some(8));
        assert_eq!(max_replica_memory_gb, Some(32));
        assert!(min_replicas.is_none());
        assert!(max_replicas.is_none());
    }

    #[test]
    fn rejects_service_scale_num_replicas_with_replica_band() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "scale",
            "svc-1",
            "--num-replicas",
            "3",
            "--min-replicas",
            "2",
            "--max-replicas",
            "8",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_clickpipe_object_storage_ingestion_control_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "t",
            "--source-url",
            "https://b.s3.us-east-1.amazonaws.com/d/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "d",
            "--table",
            "t",
            "--column",
            "id:Int64",
            "--queue-url",
            "https://sqs.us-east-1.amazonaws.com/123/q",
            "--start-after",
            "key1",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::ClickPipe { command } = args.command else {
            panic!("expected clickpipe command");
        };
        let ClickPipeCommands::Create { command } = *command else {
            panic!("expected create");
        };
        let ClickPipeCreateCommands::ObjectStorage(args) = command else {
            panic!("expected object-storage");
        };
        assert!(!args.skip_initial_load);
        assert_eq!(args.start_after.as_deref(), Some("key1"));
        assert_eq!(
            args.queue_url.as_deref(),
            Some("https://sqs.us-east-1.amazonaws.com/123/q")
        );
    }

    #[test]
    fn rejects_skip_initial_load_without_queue_url() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "t",
            "--source-url",
            "https://b.s3.us-east-1.amazonaws.com/d/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "d",
            "--table",
            "t",
            "--column",
            "id:Int64",
            "--skip-initial-load",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_skip_initial_load_with_start_after() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "t",
            "--source-url",
            "https://b.s3.us-east-1.amazonaws.com/d/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "d",
            "--table",
            "t",
            "--column",
            "id:Int64",
            "--queue-url",
            "https://sqs.us-east-1.amazonaws.com/123/q",
            "--skip-initial-load",
            "--start-after",
            "key1",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_clickpipe_mysql_server_id() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "t",
            "--host",
            "h",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "db.t:t",
            "--server-id",
            "4242",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::ClickPipe { command } = args.command else {
            panic!("expected clickpipe command");
        };
        let ClickPipeCommands::Create { command } = *command else {
            panic!("expected create");
        };
        let ClickPipeCreateCommands::MySQL(args) = command else {
            panic!("expected mysql");
        };
        assert_eq!(args.server_id, Some(4242));
    }

    #[test]
    fn rejects_clickpipe_mysql_server_id_out_of_range() {
        // 0 is below the minimum (1)
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "t",
            "--host",
            "h",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "db.t:t",
            "--server-id",
            "0",
        ]);
        assert!(result.is_err());

        // 4294967296 is above the maximum (4294967295)
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "t",
            "--host",
            "h",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "db.t:t",
            "--server-id",
            "4294967296",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_clickpipe_schema_discover_kafka() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "schema-discover",
            "svc-1",
            "kafka",
            "--brokers",
            "b:9092",
            "--topics",
            "t",
            "--format",
            "JSONEachRow",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::ClickPipe { command } = args.command else {
            panic!("expected clickpipe command");
        };
        let ClickPipeCommands::SchemaDiscover {
            service_id,
            command,
            ..
        } = *command
        else {
            panic!("expected schema-discover");
        };
        assert_eq!(service_id, "svc-1");
        assert!(matches!(command, ClickPipeSchemaDiscoverCommands::Kafka(_)));
    }

    #[test]
    fn parses_private_endpoint_config_and_password_hash_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "reset-password",
            "svc-1",
            "--new-password-hash",
            "sha256",
            "--new-double-sha1-hash",
            "sha1",
        ])
        .unwrap();

        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let ServiceCommands::ResetPassword {
            new_password_hash,
            new_double_sha1_hash,
            ..
        } = command
        else {
            panic!("expected reset-password");
        };
        assert_eq!(new_password_hash.as_deref(), Some("sha256"));
        assert_eq!(new_double_sha1_hash.as_deref(), Some("sha1"));

        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "private-endpoint",
            "get-config",
            "svc-1",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let ServiceCommands::PrivateEndpoint { command } = command else {
            panic!("expected private-endpoint command");
        };
        let PrivateEndpointCommands::GetConfig { service_id, .. } = command else {
            panic!("expected get-config");
        };
        assert_eq!(service_id, "svc-1");
    }

    #[test]
    fn parses_key_create_and_backup_config_update_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "key",
            "create",
            "--name",
            "ci-key",
            "--ip-allow",
            "10.0.0.0/8",
            "--hash-key-id",
            "id-hash",
            "--hash-key-id-suffix",
            "abcd",
            "--hash-key-secret",
            "secret-hash",
        ])
        .unwrap();

        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Key { command } = args.command else {
            panic!("expected key command");
        };
        let KeyCommands::Create {
            ip_allow,
            hash_key_id,
            hash_key_id_suffix,
            hash_key_secret,
            ..
        } = command
        else {
            panic!("expected key create");
        };
        assert_eq!(ip_allow, vec!["10.0.0.0/8"]);
        assert_eq!(hash_key_id.as_deref(), Some("id-hash"));
        assert_eq!(hash_key_id_suffix.as_deref(), Some("abcd"));
        assert_eq!(hash_key_secret.as_deref(), Some("secret-hash"));

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
        let CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let ServiceCommands::BackupConfig { command } = command else {
            panic!("expected backup-config");
        };
        let BackupConfigCommands::Update {
            backup_period_hours,
            backup_retention_period_hours,
            backup_start_time,
            ..
        } = command
        else {
            panic!("expected backup-config update");
        };
        assert_eq!(backup_period_hours, Some(12));
        assert_eq!(backup_retention_period_hours, Some(336));
        assert_eq!(backup_start_time.as_deref(), Some("03:00"));
    }

    #[test]
    fn parses_key_expires_at_rfc3339_timestamps() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "key",
            "create",
            "--name",
            "ci-key",
            "--expires-at",
            "2025-12-31T23:59:59Z",
        ])
        .unwrap();

        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Key { command } = args.command else {
            panic!("expected key command");
        };
        let KeyCommands::Create { expires_at, .. } = command else {
            panic!("expected key create");
        };
        assert_eq!(expires_at.as_deref(), Some("2025-12-31T23:59:59Z"));
    }

    #[test]
    fn rejects_invalid_key_expires_at_timestamps() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "key",
            "update",
            "key-1",
            "--expires-at",
            "2025-12-31",
        ]);

        match result {
            Ok(_) => panic!("expected invalid expires-at input to be rejected"),
            Err(err) => assert!(err.to_string().contains("expected ISO 8601 / RFC 3339")),
        }
    }

    #[test]
    fn parses_backup_start_time_hhmm() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "backup-config",
            "update",
            "svc-1",
            "--backup-start-time",
            "03:00",
        ])
        .unwrap();

        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let ServiceCommands::BackupConfig { command } = command else {
            panic!("expected backup-config");
        };
        let BackupConfigCommands::Update {
            backup_start_time, ..
        } = command
        else {
            panic!("expected backup-config update");
        };
        assert_eq!(backup_start_time.as_deref(), Some("03:00"));
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
            Err(err) => assert!(err.to_string().contains("expected HH:MM")),
        }
    }

    #[test]
    fn parses_org_usage_date_only_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "usage",
            "org-1",
            "--from-date",
            "2025-01-01",
            "--to-date",
            "2025-01-31",
        ])
        .unwrap();

        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let CloudCommands::Org { command } = args.command else {
            panic!("expected org command");
        };
        let OrgCommands::Usage {
            from_date, to_date, ..
        } = command
        else {
            panic!("expected org usage");
        };
        assert_eq!(from_date, "2025-01-01");
        assert_eq!(to_date, "2025-01-31");
    }

    #[test]
    fn rejects_org_usage_timestamps() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "usage",
            "org-1",
            "--from-date",
            "2025-01-01T00:00:00Z",
            "--to-date",
            "2025-01-31",
        ]);

        match result {
            Ok(_) => panic!("expected timestamp input to be rejected"),
            Err(err) => assert!(err.to_string().contains("expected YYYY-MM-DD")),
        }
    }

    #[test]
    fn rejects_invalid_org_usage_calendar_dates() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "usage",
            "org-1",
            "--from-date",
            "2025-02-31",
            "--to-date",
            "2025-03-01",
        ]);

        match result {
            Ok(_) => panic!("expected invalid calendar date to be rejected"),
            Err(err) => assert!(err.to_string().contains("expected YYYY-MM-DD")),
        }
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
        let CloudCommands::Activity { command } = args.command else {
            panic!("expected activity command");
        };
        let ActivityCommands::List {
            from_date, to_date, ..
        } = command
        else {
            panic!("expected activity list");
        };
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
            Err(err) => assert!(err.to_string().contains("expected YYYY-MM-DD")),
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
            Err(err) => assert!(err.to_string().contains("expected YYYY-MM-DD")),
        }
    }

    /// Helper to assert a command parsed from CLI args is classified correctly.
    fn assert_write(args: &[&str], expected: bool) {
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
        assert_eq!(
            cloud_args.command.is_write_command(),
            expected,
            "wrong classification for: {}",
            args.join(" ")
        );
    }

    #[test]
    fn is_write_command_read_only_commands() {
        // Org reads
        assert_write(&["clickhousectl", "cloud", "org", "list"], false);
        assert_write(&["clickhousectl", "cloud", "org", "get", "org-1"], false);
        assert_write(
            &["clickhousectl", "cloud", "org", "prometheus", "org-1"],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "usage",
                "org-1",
                "--from-date",
                "2025-01-01",
                "--to-date",
                "2025-01-31",
            ],
            false,
        );

        // Service reads
        assert_write(&["clickhousectl", "cloud", "service", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "service", "get", "svc-1"],
            false,
        );
        assert_write(
            &["clickhousectl", "cloud", "service", "prometheus", "svc-1"],
            false,
        );

        // Backup reads
        assert_write(
            &["clickhousectl", "cloud", "backup", "list", "svc-1"],
            false,
        );
        assert_write(
            &["clickhousectl", "cloud", "backup", "get", "svc-1", "bk-1"],
            false,
        );

        // Backup config read
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "backup-config",
                "get",
                "svc-1",
            ],
            false,
        );

        // Member reads
        assert_write(&["clickhousectl", "cloud", "member", "list"], false);
        assert_write(&["clickhousectl", "cloud", "member", "get", "usr-1"], false);

        // Invitation reads
        assert_write(&["clickhousectl", "cloud", "invitation", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "invitation", "get", "inv-1"],
            false,
        );

        // Key reads
        assert_write(&["clickhousectl", "cloud", "key", "list"], false);
        assert_write(&["clickhousectl", "cloud", "key", "get", "key-1"], false);

        // Activity reads
        assert_write(&["clickhousectl", "cloud", "activity", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "activity", "get", "act-1"],
            false,
        );

        // Query endpoint read
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "query-endpoint",
                "get",
                "svc-1",
            ],
            false,
        );

        // Private endpoint read
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "private-endpoint",
                "get-config",
                "svc-1",
            ],
            false,
        );

        // Postgres reads
        assert_write(&["clickhousectl", "cloud", "postgres", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "postgres", "get", "pg-1"],
            false,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "certs", "get", "pg-1"],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "config",
                "get",
                "pg-1",
            ],
            false,
        );
    }

    #[test]
    fn is_write_command_destructive_commands() {
        // ClickPipe schema discovery is side-effect-free, but the API gateway
        // rejects OAuth/JWT on the endpoint, so it requires API-key auth.
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "clickpipe",
                "schema-discover",
                "svc-1",
                "kafka",
                "--brokers",
                "b:9092",
                "--topics",
                "t",
                "--format",
                "JSONEachRow",
            ],
            true,
        );

        // Org write
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "update",
                "org-1",
                "--name",
                "new",
            ],
            true,
        );

        // Service writes
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "create",
                "--name",
                "s",
                "--provider",
                "aws",
                "--region",
                "us-east-1",
            ],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "service", "delete", "svc-1"],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "service", "start", "svc-1"],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "service", "stop", "svc-1"],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "update",
                "svc-1",
                "--name",
                "new",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "scale",
                "svc-1",
                "--num-replicas",
                "2",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "reset-password",
                "svc-1",
            ],
            true,
        );

        // Backup config write
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "backup-config",
                "update",
                "svc-1",
                "--backup-period-hours",
                "12",
            ],
            true,
        );

        // Member writes
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "member",
                "update",
                "usr-1",
                "--role-id",
                "r1",
            ],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "member", "remove", "usr-1"],
            true,
        );

        // Invitation writes
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "invitation",
                "create",
                "--email",
                "a@b.com",
                "--role-id",
                "r1",
            ],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "invitation", "delete", "inv-1"],
            true,
        );

        // Key writes
        assert_write(
            &["clickhousectl", "cloud", "key", "create", "--name", "k"],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "key",
                "update",
                "key-1",
                "--name",
                "new",
            ],
            true,
        );
        assert_write(&["clickhousectl", "cloud", "key", "delete", "key-1"], true);

        // Query endpoint writes
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "query-endpoint",
                "create",
                "svc-1",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "query-endpoint",
                "delete",
                "svc-1",
            ],
            true,
        );

        // Private endpoint write
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "private-endpoint",
                "create",
                "svc-1",
                "--endpoint-id",
                "ep-1",
            ],
            true,
        );

        // Postgres writes
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "create",
                "--name",
                "pg",
                "--region",
                "us-east-1",
                "--size",
                "m7i.2xlarge",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "update",
                "pg-1",
                "--size",
                "c6gd.large",
            ],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "delete", "pg-1"],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "config",
                "replace",
                "pg-1",
                "--file",
                "/tmp/c.json",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "config",
                "patch",
                "pg-1",
                "--set",
                "max_connections=500",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "reset-password",
                "pg-1",
                "--generate",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "read-replica",
                "create",
                "pg-1",
                "--name",
                "r1",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "postgres",
                "restore",
                "pg-1",
                "--name",
                "r",
                "--restore-target",
                "2026-04-16T12:00:00Z",
            ],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "restart", "pg-1"],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "promote", "pg-1"],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "postgres", "switchover", "pg-1"],
            true,
        );
    }
}
