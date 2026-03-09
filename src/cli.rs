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

  Typical local workflow: `clickhousectl install stable && clickhousectl use stable && clickhousectl run server`.

  Use `clickhousectl <command> --help` to get more context for specific commands.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install a ClickHouse version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Downloads a ClickHouse binary to ~/.clickhouse/versions/{version}/.
  Accepts version specs: \"stable\", \"lts\", partial like \"25.12\", or exact like \"25.12.5.44\".
  Optionally set as default with `clickhousectl use <version>`.
  `clickhousectl use <version>` will auto-install if the version is missing and set as default.
  Related: `clickhousectl list --remote` to see downloadable versions.")]
    Install {
        /// Version to install (e.g., 25.1.2.3, 25.1, stable, lts)
        version: String,
    },

    /// List installed versions
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Without flags: shows locally installed versions (exact version strings).
  With --remote: shows versions available for download from GitHub releases.
  Use the exact version strings from this output with `clickhousectl remove` or `clickhousectl use`.
  Related: `clickhousectl install <version>` to install, `clickhousectl which` to see current default.")]
    List {
        /// List versions available for download
        #[arg(long)]
        remote: bool,
    },

    /// Set the default version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Sets the default ClickHouse version used by `clickhousectl run` commands.
  Accepts version specs: \"stable\", \"lts\", partial like \"25.12\", or exact like \"25.12.5.44\".
  Auto-installs the version if not already present.
  Related: `clickhousectl which` to verify, `clickhousectl run server` to start.")]
    Use {
        /// Version to use as default
        version: String,
    },

    /// Remove an installed version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Removes an installed ClickHouse version from ~/.clickhouse/versions/.
  Takes an exact version string as shown by `clickhousectl list` (e.g., \"25.12.5.44\").
  Does NOT accept keywords like \"stable\" — use the exact version number.
  Related: `clickhousectl list` to see installed versions.")]
    Remove {
        /// Version to remove
        version: String,
    },

    /// Show the current default version
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Shows the current default version and binary path. No arguments needed.
  Use this to verify which version is active before running commands.
  Related: `clickhousectl use <version>` to change the default.")]
    Which,

    /// Initialize a project-local ClickHouse configuration
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Creates a .clickhouse/ directory (runtime data, git-ignored) and a clickhouse/ project
  scaffold with subdirs: tables/, materialized_views/, queries/, seed/ (each with .gitkeep).
  The clickhouse/ directory is meant to be committed — organize your SQL files there.
  Related: `clickhousectl run server` to start a server with project-local data.")]
    Init,

    /// Run ClickHouse commands
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Used for interacting with ClickHouse (local and Cloud).
  Gateway to server/client/local subcommands. Requires a default version set via `clickhousectl use`.
  Shortcut: `clickhousectl run --sql 'SELECT 1'` runs a query via clickhouse-local without subcommands to test things that don't need persistence.
  Pass extra ClickHouse args after -- (e.g., `clickhousectl run server -- --http_port=9000`).
  Related: `clickhousectl use <version>` to set default, `clickhousectl which` to check current version.")]
    Run(RunArgs),

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

#[derive(Args)]
pub struct RunArgs {
    /// Execute SQL query using clickhouse local
    #[arg(long, short)]
    pub sql: Option<String>,

    #[command(subcommand)]
    pub command: Option<RunCommands>,
}

#[derive(Subcommand)]
pub enum RunCommands {
    /// Run clickhouse-server
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Starts clickhouse-server with project-local data in .clickhouse/{version}/.
  Auto-initializes the data directory on first run. Replaces the current process (exec).
  Pass extra clickhouse-server args after -- (e.g., `clickhousectl run server -- --http_port=9000`).
  Data persists in .clickhouse/{version}/ between runs.
  Related: `clickhousectl run client` to connect, `clickhousectl use <version>` to change version.")]
    Server {
        /// Arguments to pass to clickhouse-server
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run clickhouse-client
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Connects to a running clickhouse-server. Server must already be running via `clickhousectl run server`.
  Pass clickhouse-client args after -- (e.g., `clickhousectl run client -- --query 'SELECT 1'`).
  Common args: --host, --port, --query, --multiquery, --format.
  Related: `clickhousectl run server` to start a server first.")]
    Client {
        /// Arguments to pass to clickhouse-client
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run clickhouse-local
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Runs clickhouse-local for file/query processing without a server.
  Pass clickhouse-local args after -- (e.g., `clickhousectl run local -- --query 'SELECT 1'`).
  Shortcut: `clickhousectl run --sql 'SELECT 1'` does the same without the local subcommand.
  Useful for processing files, running queries against local data, or testing SQL.
  Related: `clickhousectl run --sql` for quick one-off queries.")]
    Local {
        /// Arguments to pass to clickhouse-local
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
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
