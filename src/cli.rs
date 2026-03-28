use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime};
use clap::{Args, Parser, Subcommand};

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

#[derive(Parser)]
#[command(name = "clickhousectl")]
#[command(about = "The official CLI for ClickHouse: local and cloud", long_about = None)]
#[command(version)]
#[command(after_help = "\
CONTEXT FOR AGENTS:
  clickhousectl is a CLI to work with local ClickHouse and ClickHouse Cloud.

  Two main workflows:
  1. Local: Install and interact with versions of ClickHouse to develop locally.
  2. Cloud: Manage ClickHouse Cloud infrastructure and push local work to cloud.

  You can install the ClickHouse Agent Skills with:

  `clickhousectl skills`

  Typical local workflow: `clickhousectl local install stable && clickhousectl local use stable && clickhousectl local server start`.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Work with local ClickHouse installations
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage local ClickHouse installations: install versions, run queries, manage servers.
  Typical workflow: `clickhousectl local install stable && clickhousectl local use stable && clickhousectl local server start`.
  Use `clickhousectl local <command> --help` for details on each subcommand.")]
    Local {
        #[command(subcommand)]
        command: LocalCommands,
    },

    /// Work with serverless ClickHouse in ClickHouse Cloud
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Used for managing ClickHouse Cloud infrastructure.
  Gateway to org/service/backup/member/invitation/key/activity subcommands for ClickHouse Cloud.
  Auth: `clickhousectl cloud auth` to save credentials interactively (stored in .clickhouse/credentials.json).
  Or use env vars CLICKHOUSE_CLOUD_API_KEY + CLICKHOUSE_CLOUD_API_SECRET, or --api-key/--api-secret flags.
  Verify auth with `clickhousectl cloud org list`.
  Add --json to any cloud command for machine-readable output.
  Typical workflow: `cloud org list` → get org ID → `cloud service list` → manage services and related resources.
  Related: `clickhousectl cloud org list` to start.")]
    Cloud(CloudArgs),

    /// Install ClickHouse agent skills into supported coding agents
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Installs the official ClickHouse agent skills. These skills contain knowledge on how to use
  ClickHouse and this CLI.
  Using any flags skips interactive mode. Project scope is the default. Universal `.agents/skills`
  is always included.")]
    Skills(SkillsArgs),
}

#[derive(Subcommand)]
pub enum LocalCommands {
    /// Install a ClickHouse version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Downloads a ClickHouse binary to ~/.clickhouse/versions/{version}/.
  Accepts version specs: \"stable\", \"lts\", partial like \"25.12\", or exact like \"25.12.5.44\".
  Optionally set as default with `clickhousectl local use <version>`.
  `clickhousectl local use <version>` will auto-install if the version is missing and set as default.
  Related: `clickhousectl local list --remote` to see downloadable versions.")]
    Install {
        /// Version to install (e.g., 25.1.2.3, 25.1, stable, lts)
        version: String,
    },

    /// List installed versions
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Without flags: shows locally installed versions (exact version strings).
  With --remote: shows versions available for download from GitHub releases.
  Use the exact version strings from this output with `clickhousectl local remove` or `clickhousectl local use`.
  Related: `clickhousectl local install <version>` to install, `clickhousectl local which` to see current default.")]
    List {
        /// List versions available for download
        #[arg(long)]
        remote: bool,
    },

    /// Set the default version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Sets the default ClickHouse version used by `clickhousectl local client` and `clickhousectl local server`.
  Accepts version specs: \"stable\", \"lts\", partial like \"25.12\", or exact like \"25.12.5.44\".
  Auto-installs the version if not already present.
  Related: `clickhousectl local which` to verify, `clickhousectl local server start` to start a server.")]
    Use {
        /// Version to use as default
        version: String,
    },

    /// Remove an installed version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Removes an installed ClickHouse version from ~/.clickhouse/versions/.
  Takes an exact version string as shown by `clickhousectl local list` (e.g., \"25.12.5.44\").
  Does NOT accept keywords like \"stable\" — use the exact version number.
  Related: `clickhousectl local list` to see installed versions.")]
    Remove {
        /// Version to remove
        version: String,
    },

    /// Show the current default version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Shows the current default version and binary path. No arguments needed.
  Use this to verify which version is active before running commands.
  Related: `clickhousectl local use <version>` to change the default.")]
    Which,

    /// Initialize a project-local ClickHouse configuration
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Creates a .clickhouse/ directory (runtime data, git-ignored) and a clickhouse/ project
  scaffold with subdirs: tables/, materialized_views/, queries/, seed/ (each with .gitkeep).
  The clickhouse/ directory is meant to be committed — organize your SQL files there.
  Related: `clickhousectl local server start` to start a server with project-local data.")]
    Init,

    /// Connect to a running ClickHouse server with clickhouse-client
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Two connection modes:
  1. Named server: `clickhousectl local client --name dev` — looks up port and version from a
     locally managed server started via `clickhousectl local server start`. Defaults to \"default\".
  2. Explicit host/port: `clickhousectl local client --host myhost --port 9000` — connects to any
     ClickHouse server directly, bypassing local server lookup.
  --query and --queries-file execute SQL inline or from a file.
  Additional clickhouse-client args can be passed after --.
  Related: `clickhousectl local server start` to start a local server, `clickhousectl local server list` to see servers.")]
    Client {
        /// Server name to connect to (default: "default")
        #[arg(long, short)]
        name: Option<String>,

        /// Host to connect to (bypasses local server lookup)
        #[arg(long)]
        host: Option<String>,

        /// TCP port to connect to (bypasses local server lookup if set)
        #[arg(long, short)]
        port: Option<u16>,

        /// Execute a SQL query
        #[arg(long, short)]
        query: Option<String>,

        /// Execute queries from a SQL file
        #[arg(long)]
        queries_file: Option<String>,

        /// Additional arguments to pass to clickhouse-client
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Manage local ClickHouse server instances
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Manage named ClickHouse server instances. Each server has its own data directory.
  Subcommands: start, list, stop, stop-all, remove.
  Data is stored in .clickhouse/servers/<name>/data/ and persists between restarts.
  Typical: `clickhousectl local server start` (starts \"default\"), `clickhousectl local server start --name test`.
  Related: `clickhousectl local client` to connect to a running server.")]
    Server {
        #[command(subcommand)]
        command: ServerCommands,
    },
}

#[derive(Args, Debug)]
pub struct SkillsArgs {
    /// Install into specific agents (repeatable or comma-separated).
    #[arg(
        long = "agent",
        value_name = "AGENT",
        value_delimiter = ',',
        long_help = "Install into specific agents (repeatable or comma-separated).\n\nValid names:\n  claude, cursor, opencode, codex\n  agent, roo, trae, windsurf\n  zencoder, neovate, pochi, adal\n  openclaw, cline, command-code\n  kiro-cli, agents"
    )]
    pub agents: Vec<String>,

    /// Install into every supported agent in the selected scope without prompting
    #[arg(long, conflicts_with_all = ["agents", "detected_only"])]
    pub all: bool,

    /// Install only into agents detected from your home directory without prompting
    #[arg(long = "detected-only", conflicts_with_all = ["agents", "all"])]
    pub detected_only: bool,

    /// Install into global agent config directories in your home directory
    #[arg(long)]
    pub global: bool,
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

    #[command(subcommand)]
    pub command: CloudCommands,
}

#[derive(Subcommand)]
pub enum CloudCommands {
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
  Manage ClickHouse Cloud services. Subcommands: list, get, create, delete, start, stop, update,
  scale, reset-password, query-endpoint, private-endpoint, backup-config, prometheus.
  Most commands need a service ID — get it from `clickhousectl cloud service list`.
  Org ID is auto-detected if you have only one org; otherwise pass --org-id.
  Add --json for machine-readable output. All write operations are immediate.
  Related: `clickhousectl cloud org list` for org IDs, `clickhousectl cloud backup list` for service backups.")]
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },

    /// Save API credentials interactively
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Prompts for API key and secret, saves to .clickhouse/credentials.json (project-local).
  Credentials are auto-loaded by subsequent cloud commands.
  Fallback order: --api-key/--api-secret flags > credentials file > env vars.
  Related: `clickhousectl cloud org list` to verify credentials work.")]
    Auth,

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
  Takes a service ID — get it from `clickhousectl cloud service list`.
  Add --json for machine-readable output.
  Related: `clickhousectl cloud service stop <id>` to idle instead of delete.")]
    Delete {
        /// Service ID
        service_id: String,

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
pub enum ServerCommands {
    /// Start a ClickHouse server instance
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Starts a named clickhouse-server instance with its own data directory.
  Data is stored in .clickhouse/servers/<name>/data/ and persists between restarts.
  Without --name, the first server is called \"default\"; if \"default\" is already running,
  a random name is generated (e.g., \"bold-crane\").
  Use --name to give a server a stable identity (e.g., --name dev, --name test).
  Ports default to 8123 (HTTP) and 9000 (TCP). If they're in use, free ports are auto-assigned.
  Use --http-port and --tcp-port to set explicit ports.
  Runs in background by default. Use --foreground (-F / --fg) to run in foreground.
  If --name is given and that server is already running, the command will error.
  Shows count of already-running servers before starting.
  Related: `clickhousectl local server list` to see servers, `clickhousectl local server stop <name>` to stop one.")]
    Start {
        /// Server name (default: \"default\", or random if default is already running)
        #[arg(long)]
        name: Option<String>,

        /// HTTP port (default: 8123, auto-assigns a free port if in use)
        #[arg(long)]
        http_port: Option<u16>,

        /// TCP port (default: 9000, auto-assigns a free port if in use)
        #[arg(long)]
        tcp_port: Option<u16>,

        /// Run server in foreground (default: background)
        #[arg(long, alias = "fg", short = 'F')]
        foreground: bool,

        /// Arguments to pass to clickhouse-server
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// List all server instances (running and stopped)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Shows all named ClickHouse server instances and their status.
  Automatically cleans up stale entries for processes that are no longer running.
  Shows name, status (running/stopped), PID, version, and ports.
  Related: `clickhousectl local server start` to start a server, `clickhousectl local server stop <name>` to stop one.")]
    List,

    /// Stop a running server by name
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Stops a named ClickHouse server. Use the name from `clickhousectl local server list`.
  Sends SIGTERM first, then SIGKILL if the process doesn't exit gracefully.
  The server's data directory is preserved — restart with `clickhousectl local server start --name <name>`.
  Related: `clickhousectl local server list` to see servers.")]
    Stop {
        /// Name of the server to stop
        name: String,
    },

    /// Stop all running server instances
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Stops all running ClickHouse server instances.
  Sends SIGTERM first, then SIGKILL if processes don't exit.
  Data directories are preserved.
  Related: `clickhousectl local server list` to see servers.")]
    StopAll,

    /// Remove a stopped server and its data
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Permanently deletes a server's data directory. The server must be stopped first.
  This is irreversible — all data for this server instance will be lost.
  Related: `clickhousectl local server stop <name>` to stop first, `clickhousectl local server list` to see servers.")]
    Remove {
        /// Name of the server to remove
        name: String,
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

    #[test]
    fn parses_skills_all_and_agent_flags() {
        let cli = Cli::try_parse_from(["clickhousectl", "skills", "--all"]).unwrap();
        let Commands::Skills(args) = cli.command else {
            panic!("expected skills command");
        };
        assert!(args.all);
        assert!(args.agents.is_empty());
        assert!(!args.detected_only);
        assert!(!args.global);

        let cli = Cli::try_parse_from(["clickhousectl", "skills", "--global"]).unwrap();
        let Commands::Skills(args) = cli.command else {
            panic!("expected skills command");
        };
        assert!(args.global);
        assert!(!args.all);
        assert!(!args.detected_only);
        assert!(args.agents.is_empty());

        let cli = Cli::try_parse_from(["clickhousectl", "skills", "--detected-only"]).unwrap();
        let Commands::Skills(args) = cli.command else {
            panic!("expected skills command");
        };
        assert!(args.detected_only);
        assert!(!args.all);
        assert!(!args.global);
        assert!(args.agents.is_empty());

        let cli = Cli::try_parse_from([
            "clickhousectl",
            "skills",
            "--global",
            "--agent",
            "claude,codex",
            "--agent",
            "agents",
        ])
        .unwrap();
        let Commands::Skills(args) = cli.command else {
            panic!("expected skills command");
        };
        assert!(!args.all);
        assert!(!args.detected_only);
        assert!(args.global);
        assert_eq!(args.agents, vec!["claude", "codex", "agents"]);
    }
}
