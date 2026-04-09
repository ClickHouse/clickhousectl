use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime};
use clap::{Args, Subcommand};

fn parse_date_only(value: &str) -> Result<String, String> {
    if NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() {
        return Err(format!("invalid date '{}': expected YYYY-MM-DD", value));
    }

    Ok(value.to_string())
}

fn parse_datetime(value: &str) -> Result<String, String> {
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
  Defaults to OAuth device flow (opens browser). OAuth tokens are READ-ONLY — they can list and
  inspect resources but cannot create, modify, or delete.
  For write operations (create, update, delete services, etc.), use --api-key and --api-secret.
  Agents: use API key auth for any mutating operations. OAuth is only suitable for read-only exploration.
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
By default, clears ALL credentials (OAuth tokens and API keys).
Use --oauth or --api-keys to clear only one type.")]
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

    /// API base URL override (internal use only)
    #[arg(long, global = true, hide = true)]
    pub url: Option<String>,

    #[command(subcommand)]
    pub command: CloudCommands,
}

#[derive(Subcommand)]
pub enum CloudCommands {
    /// Manage authentication (OAuth login, API keys)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Use `login --api-key X --api-secret Y` for full read/write access.
  Default `login` opens a browser for OAuth (read-only access only — cannot create, modify, or delete resources).
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
  Use `client` to open a clickhouse-client session to a service.
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
                ServiceCommands::Client { .. } => false,
                ServiceCommands::Prometheus { .. } => false,
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

        /// Minimum memory per replica in GB (8-356, multiple of 4)
        #[arg(long)]
        min_replica_memory_gb: Option<u32>,

        /// Maximum memory per replica in GB (8-356, multiple of 4)
        #[arg(long)]
        max_replica_memory_gb: Option<u32>,

        /// Number of replicas (1-20)
        #[arg(long)]
        num_replicas: Option<u32>,

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

        /// Minimum memory per replica in GB (8-356, multiple of 4)
        #[arg(long)]
        min_replica_memory_gb: Option<u32>,

        /// Maximum memory per replica in GB (8-356, multiple of 4)
        #[arg(long)]
        max_replica_memory_gb: Option<u32>,

        /// Number of replicas (1-20)
        #[arg(long)]
        num_replicas: Option<u32>,

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

    /// Connect to a cloud service with clickhouse-client
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Mirrors `clickhousectl local client` but for cloud services. Auto-downloads the matching
  ClickHouse version. Use CLICKHOUSE_PASSWORD env var to avoid interactive prompts.
  Related: `clickhousectl cloud service list` to find service names/IDs.")]
    Client {
        /// Service name to connect to
        #[arg(long)]
        name: Option<String>,

        /// Service ID to connect to
        #[arg(long)]
        id: Option<String>,

        /// Execute a SQL query
        #[arg(long, short)]
        query: Option<String>,

        /// Execute queries from a SQL file
        #[arg(long)]
        queries_file: Option<String>,

        /// Database user (default: "default")
        #[arg(long, default_value = "default")]
        user: String,

        /// Database password (or set CLICKHOUSE_PASSWORD env var)
        #[arg(long)]
        password: Option<String>,

        /// Use current local default version instead of the service's version
        #[arg(long)]
        allow_mismatched_client_version: bool,

        /// Reset the service password via API and use it for this connection (destructive)
        #[arg(long, hide = true)]
        generate_password: bool,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,

        /// Additional arguments to pass to clickhouse-client
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
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
        assert_write(&["clickhousectl", "cloud", "org", "prometheus", "org-1"], false);
        assert_write(&["clickhousectl", "cloud", "org", "usage", "org-1", "--from-date", "2025-01-01", "--to-date", "2025-01-31"], false);

        // Service reads
        assert_write(&["clickhousectl", "cloud", "service", "list"], false);
        assert_write(&["clickhousectl", "cloud", "service", "get", "svc-1"], false);
        assert_write(&["clickhousectl", "cloud", "service", "client", "--id", "svc-1"], false);
        assert_write(&["clickhousectl", "cloud", "service", "prometheus", "svc-1"], false);

        // Backup reads
        assert_write(&["clickhousectl", "cloud", "backup", "list", "svc-1"], false);
        assert_write(&["clickhousectl", "cloud", "backup", "get", "svc-1", "bk-1"], false);

        // Backup config read
        assert_write(&["clickhousectl", "cloud", "service", "backup-config", "get", "svc-1"], false);

        // Member reads
        assert_write(&["clickhousectl", "cloud", "member", "list"], false);
        assert_write(&["clickhousectl", "cloud", "member", "get", "usr-1"], false);

        // Invitation reads
        assert_write(&["clickhousectl", "cloud", "invitation", "list"], false);
        assert_write(&["clickhousectl", "cloud", "invitation", "get", "inv-1"], false);

        // Key reads
        assert_write(&["clickhousectl", "cloud", "key", "list"], false);
        assert_write(&["clickhousectl", "cloud", "key", "get", "key-1"], false);

        // Activity reads
        assert_write(&["clickhousectl", "cloud", "activity", "list"], false);
        assert_write(&["clickhousectl", "cloud", "activity", "get", "act-1"], false);

        // Query endpoint read
        assert_write(&["clickhousectl", "cloud", "service", "query-endpoint", "get", "svc-1"], false);

        // Private endpoint read
        assert_write(&["clickhousectl", "cloud", "service", "private-endpoint", "get-config", "svc-1"], false);
    }

    #[test]
    fn is_write_command_destructive_commands() {
        // Org write
        assert_write(&["clickhousectl", "cloud", "org", "update", "org-1", "--name", "new"], true);

        // Service writes
        assert_write(&["clickhousectl", "cloud", "service", "create", "--name", "s", "--provider", "aws", "--region", "us-east-1"], true);
        assert_write(&["clickhousectl", "cloud", "service", "delete", "svc-1"], true);
        assert_write(&["clickhousectl", "cloud", "service", "start", "svc-1"], true);
        assert_write(&["clickhousectl", "cloud", "service", "stop", "svc-1"], true);
        assert_write(&["clickhousectl", "cloud", "service", "update", "svc-1", "--name", "new"], true);
        assert_write(&["clickhousectl", "cloud", "service", "scale", "svc-1", "--num-replicas", "2"], true);
        assert_write(&["clickhousectl", "cloud", "service", "reset-password", "svc-1"], true);

        // Backup config write
        assert_write(&["clickhousectl", "cloud", "service", "backup-config", "update", "svc-1", "--backup-period-hours", "12"], true);

        // Member writes
        assert_write(&["clickhousectl", "cloud", "member", "update", "usr-1", "--role-id", "r1"], true);
        assert_write(&["clickhousectl", "cloud", "member", "remove", "usr-1"], true);

        // Invitation writes
        assert_write(&["clickhousectl", "cloud", "invitation", "create", "--email", "a@b.com", "--role-id", "r1"], true);
        assert_write(&["clickhousectl", "cloud", "invitation", "delete", "inv-1"], true);

        // Key writes
        assert_write(&["clickhousectl", "cloud", "key", "create", "--name", "k"], true);
        assert_write(&["clickhousectl", "cloud", "key", "update", "key-1", "--name", "new"], true);
        assert_write(&["clickhousectl", "cloud", "key", "delete", "key-1"], true);

        // Query endpoint writes
        assert_write(&["clickhousectl", "cloud", "service", "query-endpoint", "create", "svc-1"], true);
        assert_write(&["clickhousectl", "cloud", "service", "query-endpoint", "delete", "svc-1"], true);

        // Private endpoint write
        assert_write(&["clickhousectl", "cloud", "service", "private-endpoint", "create", "svc-1", "--endpoint-id", "ep-1"], true);
    }
}
