use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clickhousectl")]
#[command(about = "ClickHouse version manager", long_about = None)]
#[command(version)]
#[command(after_help = "\
CONTEXT FOR AGENTS:
  clickhousectl is a CLI to work with local ClickHouse and ClickHouse Cloud.

  Two main workflows:
  1. Local: Install and interact with versions of ClickHouse to develop locally.
  2. Cloud: Manage ClickHouse Cloud infrastructure and push local work to cloud.

  You can install the ClickHouse Agent Skills for best practices on using ClikHouse:

  `npx skills add clickhouse/agent-skills`

  Typical local workflow: `clickhousectl local install stable && clickhousectl local use stable && clickhousectl local server start`.

  Use `clickhousectl <command> --help` to get more context for specific commands.")]
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

    /// ClickHouse Cloud API commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Used for managing ClickHouse Cloud infrastructure.
  Gateway to org/service/backup subcommands for ClickHouse Cloud.
  Auth: `clickhousectl cloud auth` to save credentials interactively (stored in .clickhouse/credentials.json).
  Or use env vars CLICKHOUSE_CLOUD_API_KEY + CLICKHOUSE_CLOUD_API_SECRET, or --api-key/--api-secret flags.
  Verify auth with `clickhousectl cloud org list`.
  Add --json to any cloud command for machine-readable output.
  Typical workflow: `cloud org list` → get org ID → `cloud service list` → manage services.
  Related: `clickhousectl cloud org list` to start.")]
    Cloud(CloudArgs),
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
  Connects to a running clickhouse-server. Server must already be running via `clickhousectl local server start`.
  Use --name <name> to connect to a specific named server (default: \"default\").
  --host, --port, and --query are promoted as first-class flags.
  Additional clickhouse-client args can be passed after --.
  Related: `clickhousectl local server start` to start a server first, `clickhousectl local server list` to see servers.")]
    Client {
        /// Server name to connect to (default: "default")
        #[arg(long, short)]
        name: Option<String>,

        /// Host to connect to
        #[arg(long, default_value = "localhost")]
        host: String,

        /// TCP port to connect to (auto-detected from --name if not set)
        #[arg(long, short)]
        port: Option<u16>,

        /// Execute a SQL query
        #[arg(long, short)]
        query: Option<String>,

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
  Manage ClickHouse Cloud organizations. Subcommands: list, get.
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
  Manage ClickHouse Cloud services. Subcommands: list, get, create, delete, start, stop.
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

        /// BYOC region ID
        #[arg(long)]
        byoc_id: Option<String>,

        /// Compliance type: hipaa, pci
        #[arg(long)]
        compliance_type: Option<String>,

        /// Instance profile (enterprise only): v1-default, v1-highmem-xs, etc.
        #[arg(long)]
        profile: Option<String>,

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
