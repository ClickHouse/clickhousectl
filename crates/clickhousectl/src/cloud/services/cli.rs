use crate::cloud::backups::{BackupConfigCommands, SnapshotCommands};
use crate::cloud::shared::NameSelector;
use crate::cloud::shared::parse_tag_filter;
use clap::builder::PossibleValuesParser;
use clap::{ArgGroup, Subcommand};
use clickhouse_cloud_api::models::QueryEndpointRole;

#[derive(Subcommand)]
pub enum ServiceCommands {
    /// List services in the organization
    List {
        /// Filter by resource tag: `tag:KEY=VALUE` or `tag:KEY` (repeatable)
        #[arg(long, value_parser = parse_tag_filter)]
        filter: Vec<String>,
    },

    /// Get service details
    Get {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Write the service's connection settings to a .env file
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Writes CLICKHOUSE_HOST, CLICKHOUSE_PORT (native TLS), CLICKHOUSE_HTTP_PORT, CLICKHOUSE_USER and
  CLICKHOUSE_SECURE to .env in the current directory (.env.local with --local).
  No password: the API returns it only once, and `cloud service create --dotenv` writes it.
  An existing file is edited in place: only these keys are replaced, so a CLICKHOUSE_PASSWORD
  already in it is kept.")]
    Dotenv {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// Write to .env.local instead of .env
        #[arg(long)]
        local: bool,
    },

    /// Discover available service profiles
    Profile {
        #[command(subcommand)]
        command: ServiceProfileCommands,
    },

    /// Manage ClickHouse settings
    Settings {
        #[command(subcommand)]
        command: ServiceSettingsCommands,
    },

    /// Manage the service scaling schedule
    #[command(name = "scaling-schedule")]
    ScalingSchedule {
        #[command(subcommand)]
        command: ScalingScheduleCommands,
    },

    /// Create a service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Omitting --ip-allow opens the service to 0.0.0.0/0 (\"Allow all\").
  The initial password is printed once — save it; `cloud service reset-password <id>` issues a new one.
  --dotenv writes it and the connection settings to the file instead, keeping it out of the output.
  Returns before the service is up: poll `cloud service get <id>` until state is `running` (minutes).")]
    Create {
        /// Service name
        #[arg(long)]
        name: String,

        /// Cloud provider: aws, gcp, azure
        #[arg(long, default_value = "aws")]
        provider: String,

        /// Region, e.g. us-east-1, eu-west-1, us-central1
        #[arg(long, default_value = "us-east-1")]
        region: String,

        /// Minimum memory per replica in GB
        ///
        /// A multiple of 4, at least 8. Horizontal autoscaling requires it equal to
        /// --max-replica-memory-gb.
        #[arg(long)]
        min_replica_memory_gb: Option<u32>,

        /// Maximum memory per replica in GB
        ///
        /// A multiple of 4; at most 120 for unpaid and 356 for paid services. Horizontal
        /// autoscaling requires it equal to --min-replica-memory-gb.
        #[arg(long)]
        max_replica_memory_gb: Option<u32>,

        /// Fixed replica count for vertical autoscaling (up to 50)
        ///
        /// The first service in a warehouse needs at least 2; existing warehouses allow 1.
        /// Tier and per-warehouse limits may lower the maximum. Conflicts with the horizontal band.
        #[arg(long, conflicts_with_all = ["min_replicas", "max_replicas"])]
        num_replicas: Option<u32>,

        /// Minimum replicas for horizontal autoscaling (1-50)
        ///
        /// Requires --max-replicas and conflicts with --num-replicas. Needs the horizontal
        /// autoscaling org feature unless the band is equal and the mode is vertical. Tier and
        /// per-warehouse limits may lower the maximum.
        #[arg(long, conflicts_with = "num_replicas")]
        min_replicas: Option<u32>,

        /// Maximum replicas for horizontal autoscaling (1-50)
        ///
        /// Requires --min-replicas and conflicts with --num-replicas. Needs the horizontal
        /// autoscaling org feature unless the band is equal and the mode is vertical. Tier and
        /// per-warehouse limits may lower the maximum.
        #[arg(long, conflicts_with = "num_replicas")]
        max_replicas: Option<u32>,

        /// Autoscaling mode (vertical when omitted)
        ///
        /// vertical scales memory between --min-replica-memory-gb and --max-replica-memory-gb at a
        /// fixed --num-replicas count; horizontal scales replicas between --min-replicas and
        /// --max-replicas at a fixed memory (the two memory flags equal).
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

        /// Allowed IP/CIDR, optionally IP_OR_CIDR=DESCRIPTION (repeatable)
        #[arg(long = "ip-allow", value_name = "IP_OR_CIDR[=DESCRIPTION]")]
        ip_allow: Vec<String>,

        /// Backup ID to restore from
        #[arg(long)]
        backup_id: Option<String>,

        /// Backup's encryption_config.json file, or - for stdin (private preview)
        ///
        /// Only with --backup-id, for a backup of a TDE-enabled service in your own
        /// bucket. The restored service has TDE enabled.
        #[arg(long, value_name = "PATH", requires = "backup_id")]
        backup_encryption_config: Option<String>,

        /// Release channel: slow, default, fast (production services only)
        #[arg(long)]
        release_channel: Option<String>,

        /// Data warehouse ID (for creating read replicas)
        #[arg(long)]
        data_warehouse_id: Option<String>,

        /// Make service read-only (requires --data-warehouse-id)
        #[arg(long)]
        readonly: bool,

        /// Enable Transparent Data Encryption (enterprise only)
        #[arg(long)]
        enable_tde: bool,

        /// Compliance type: hipaa, pci
        #[arg(long)]
        compliance_type: Option<String>,

        /// Instance profile (enterprise only): v1-default, v1-highmem-xs, etc.
        #[arg(long)]
        profile: Option<String>,

        /// BYOC infrastructure ID
        #[arg(long)]
        byoc_id: Option<String>,

        /// Tag to attach to the service. Format: key or key=value (repeatable)
        #[arg(long = "tag", value_name = "KEY[=VALUE]")]
        tag: Vec<String>,

        /// Enable a toggleable endpoint protocol; currently mysql (repeatable)
        #[arg(long = "enable-endpoint")]
        enable_endpoint: Vec<String>,

        /// Disable a toggleable endpoint protocol; currently mysql (repeatable)
        #[arg(long = "disable-endpoint")]
        disable_endpoint: Vec<String>,

        /// Accept private preview terms (needed for the org's first service in a
        /// private preview)
        #[arg(long)]
        private_preview_terms_checked: bool,

        /// Enable or disable service core dump collection
        #[arg(long)]
        enable_core_dumps: Option<bool>,

        /// Write connection settings and the password to FILE (.env if no FILE)
        ///
        /// An existing file is edited in place: only the CLICKHOUSE_* keys written are replaced.
        #[arg(
            long,
            value_name = "FILE",
            num_args = 0..=1,
            default_missing_value = ".env"
        )]
        dotenv: Option<std::path::PathBuf>,
    },

    /// Delete a service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Irreversible. A running service cannot be deleted: stop it first, or pass --force.
  --force stops the service and polls until the stop finishes — minutes on a real service, with
    progress lines on stderr.")]
    Delete {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// Stop the service first if it is running, then delete (waits for the stop)
        #[arg(long)]
        force: bool,
    },

    /// Start a service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Resumes a stopped service; an idle service wakes on its own when queried.
  Returns as soon as the start is accepted: poll `cloud service get <id>` until state is `running`.")]
    Start {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Wake an idled service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Wakes an idle service; use `cloud service start <id>` for a stopped service.
  Returns as soon as the wake is accepted: poll `cloud service get <id>` until state is `running`.")]
    Wake {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Stop a service
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Idles the service: compute billing stops, data is preserved.
  Returns as soon as the stop is accepted: poll `cloud service get <id>` until state is `stopped`
    or `idle`.")]
    Stop {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Update service configuration
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Only the flags you pass are changed; everything else is left as it is.
  --remove-ip-allow/--remove-private-endpoint-id/--remove-tag are idempotent: removing something
    absent exits 0 with a warning on stderr.")]
    Update {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// New service name
        #[arg(long = "new-name", id = "new_name")]
        name: Option<String>,

        /// IP/CIDR to add, optionally IP_OR_CIDR=DESCRIPTION (repeatable)
        #[arg(long = "add-ip-allow", value_name = "IP_OR_CIDR[=DESCRIPTION]")]
        add_ip_allow: Vec<String>,

        /// IP or CIDR to remove from the service allow list (repeatable)
        #[arg(long = "remove-ip-allow", value_name = "IP_OR_CIDR")]
        remove_ip_allow: Vec<String>,

        /// Private endpoint ID to add; format-checked before the request (repeatable)
        #[arg(long = "add-private-endpoint-id", value_parser = parse_private_endpoint_id_arg)]
        add_private_endpoint_id: Vec<String>,

        /// Private endpoint ID to remove (repeatable)
        #[arg(long = "remove-private-endpoint-id")]
        remove_private_endpoint_id: Vec<String>,

        /// Release channel: slow, default, fast
        #[arg(long)]
        release_channel: Option<String>,

        /// Enable a toggleable endpoint protocol; currently mysql (repeatable)
        #[arg(long = "enable-endpoint")]
        enable_endpoint: Vec<String>,

        /// Disable a toggleable endpoint protocol; currently mysql (repeatable)
        #[arg(long = "disable-endpoint")]
        disable_endpoint: Vec<String>,

        /// Transparent Data Encryption key ID to rotate to
        #[arg(long)]
        transparent_data_encryption_key_id: Option<String>,

        /// Tag to add (repeatable; cannot be combined with --remove-tag)
        #[arg(
            long = "add-tag",
            value_name = "KEY[=VALUE]",
            conflicts_with = "remove_tag"
        )]
        add_tag: Vec<String>,

        /// Tag to remove (repeatable; cannot be combined with --add-tag)
        #[arg(
            long = "remove-tag",
            value_name = "KEY[=VALUE]",
            conflicts_with = "add_tag"
        )]
        remove_tag: Vec<String>,

        /// Enable or disable service core dump collection
        #[arg(long)]
        enable_core_dumps: Option<bool>,
    },

    /// Update service scaling and idle behaviour
    Scale {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// Minimum memory per replica in GB
        ///
        /// A multiple of 4, at least 8. Horizontal autoscaling requires it equal to
        /// --max-replica-memory-gb.
        #[arg(long)]
        min_replica_memory_gb: Option<u32>,

        /// Maximum memory per replica in GB
        ///
        /// A multiple of 4; at most 120 for unpaid and 356 for paid services. Horizontal
        /// autoscaling requires it equal to --min-replica-memory-gb.
        #[arg(long)]
        max_replica_memory_gb: Option<u32>,

        /// Fixed replica count for vertical autoscaling (up to 50)
        ///
        /// The first service in a warehouse needs at least 2; existing warehouses allow 1.
        /// Tier and per-warehouse limits may lower the maximum. Conflicts with the horizontal band.
        #[arg(long, conflicts_with_all = ["min_replicas", "max_replicas"])]
        num_replicas: Option<u32>,

        /// Minimum replicas for horizontal autoscaling (1-50)
        ///
        /// Requires --max-replicas and conflicts with --num-replicas. Needs the horizontal
        /// autoscaling org feature unless the band is equal and the mode is vertical. Tier and
        /// per-warehouse limits may lower the maximum.
        #[arg(long, conflicts_with = "num_replicas")]
        min_replicas: Option<u32>,

        /// Maximum replicas for horizontal autoscaling (1-50)
        ///
        /// Requires --min-replicas and conflicts with --num-replicas. Needs the horizontal
        /// autoscaling org feature unless the band is equal and the mode is vertical. Tier and
        /// per-warehouse limits may lower the maximum.
        #[arg(long, conflicts_with = "num_replicas")]
        max_replicas: Option<u32>,

        /// Target autoscaling mode
        ///
        /// Omit to keep the service on its current mode. See
        /// `cloud service create --autoscaling-mode` for what each mode scales.
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
    },

    /// Reset the default user password
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Omit --new-password-hash and the API generates a password, printed once — save it.
  With --new-password-hash no plaintext is returned; --new-double-sha1-hash alone still triggers
    generation.")]
    ResetPassword {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// SHA256 password hash encoded as base64
        #[arg(long)]
        new_password_hash: Option<String>,

        /// MySQL-compatible double SHA1 password hash
        #[arg(long)]
        new_double_sha1_hash: Option<String>,
    },

    /// Manage the Query API endpoint
    #[command(
        name = "query-endpoint",
        after_help = "\
CONTEXT FOR AGENTS:
  Only needed to share Query API access with other tools: `cloud service query` binds the
    authenticated API key itself.
  Hand edits can unbind the key `cloud service query` uses; the next query rebinds it unless
    `--no-auto-enable` is passed.
  A stored per-service key is repaired with `cloud service repair-query-key <id>`."
    )]
    QueryEndpoint {
        #[command(subcommand)]
        command: QueryEndpointCommands,
    },

    /// Manage private endpoints
    #[command(name = "private-endpoint")]
    PrivateEndpoint {
        #[command(subcommand)]
        command: PrivateEndpointCommands,
    },

    /// Manage service snapshots (Beta)
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommands,
    },

    /// Manage backup configuration
    #[command(name = "backup-config")]
    BackupConfig {
        #[command(subcommand)]
        command: BackupConfigCommands,
    },

    /// Manage the service upgrade window
    #[command(name = "upgrade-window")]
    UpgradeWindow {
        #[command(subcommand)]
        command: UpgradeWindowCommands,
    },

    /// Manage replica aware routing (Beta)
    #[command(
        name = "replica-aware-routing",
        after_help = "\
CONTEXT FOR AGENTS:
  With HTTP routing enabled, every HTTP client must send the X-ClickHouse-Replica-Tag header.
  Update needs the organization feature enabled; an omitted flag leaves that protocol unchanged."
    )]
    ReplicaAwareRouting {
        #[command(subcommand)]
        command: ReplicaAwareRoutingCommands,
    },

    /// Get Prometheus metrics
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Output is always raw Prometheus exposition text; --json is accepted for consistency but ignored.")]
    Prometheus {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// Whether to request filtered metrics
        #[arg(long)]
        filtered_metrics: Option<bool>,
    },

    /// Run SQL over HTTP via the Query API
    #[command(
        group(ArgGroup::new("service_selector").required(true).args(["name", "id", "service_id"])),
        after_help = "\
CONTEXT FOR AGENTS:
  Run one SQL statement; a timeout may follow execution or wake delays, and is never retried.
  Stored-key 401/403 never rotates keys automatically; deliberate repair: `cloud service repair-query-key <id>`."
    )]
    Query {
        /// Service ID to query
        #[arg(conflicts_with_all = ["name", "id"])]
        service_id: Option<String>,

        /// Service name to query
        #[arg(long, conflicts_with_all = ["id", "service_id"])]
        name: Option<String>,

        /// Service ID to query
        #[arg(long, hide = true, conflicts_with_all = ["name", "service_id"])]
        id: Option<String>,

        /// SQL to run: a single statement, and never read from stdin
        #[arg(long, short, conflicts_with = "queries_file")]
        query: Option<String>,

        /// File containing one SQL statement ("-" reads stdin)
        #[arg(long, conflicts_with = "query")]
        queries_file: Option<String>,

        /// Target database
        #[arg(long)]
        database: Option<String>,

        /// Response format (e.g. JSONEachRow, CSV, TabSeparated, PrettyCompact)
        #[arg(long, conflicts_with = "json")]
        format: Option<String>,

        /// Fail rather than bind the API key to the Query API endpoint
        #[arg(long)]
        no_auto_enable: bool,
    },

    /// Replace clickhousectl's stored Query API key
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Deliberately replaces this service's owned key/binding; preserves other bindings and credentials.
  Records without exact ownership metadata are refused; legacy and external keys cannot be repaired.
  Verification timeout exits 1 but repair stands: retry `cloud service query <id>`, not repair.")]
    RepairQueryKey {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },
}

#[derive(Subcommand)]
pub enum QueryEndpointCommands {
    /// Get the Query API endpoint configuration
    Get {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Create or update the Query API endpoint
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Existing API keys are preserved unless --replace-open-api-keys is set.
  Roles replace the endpoint-wide role list for every authorized key.
  First creation requires --allowed-origins; use '*' explicitly for all.
  Avoid concurrent changes to the same endpoint.")]
    Create {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// Endpoint-wide role to grant (required, repeatable)
        #[arg(long, required = true, value_parser = PossibleValuesParser::new(QueryEndpointRole::VALUES))]
        role: Vec<String>,

        /// API key ID to authorize (repeatable)
        #[arg(long = "open-api-key", value_parser = clap::builder::NonEmptyStringValueParser::new())]
        open_api_key: Vec<String>,

        /// Replace existing API keys (requires --open-api-key)
        #[arg(long, requires = "open_api_key")]
        replace_open_api_keys: bool,

        /// Browser origins; preserve when omitted on an existing endpoint
        #[arg(long, value_parser = clap::builder::NonEmptyStringValueParser::new())]
        allowed_origins: Option<String>,
    },

    /// Delete the Query API endpoint
    Delete {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },
}

#[derive(Subcommand)]
pub enum PrivateEndpointCommands {
    /// Create a private endpoint connection
    Create {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// Private endpoint ID (AWS VPC endpoint ID, GCP PSC connection ID, or
        /// Azure private endpoint Resource ID / resourceGuid)
        #[arg(long, value_parser = parse_private_endpoint_id_arg)]
        endpoint_id: String,

        /// Description
        #[arg(long)]
        description: Option<String>,
    },

    /// Get the private endpoint configuration
    GetConfig {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },
}

#[derive(Subcommand)]
pub enum ServiceProfileCommands {
    /// List available service profiles
    List {
        /// Region ID (required without --byoc-id)
        #[arg(long, required_unless_present = "byoc_id")]
        region: Option<String>,

        /// BYOC infrastructure ID (required without --region)
        ///
        /// Cloud validates any supplied region against this infrastructure.
        #[arg(long, required_unless_present = "region")]
        byoc_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ServiceSettingsCommands {
    /// List configured ClickHouse settings
    List {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Get a ClickHouse setting
    Get {
        #[command(flatten)]
        target: ServiceSettingTarget,
    },

    /// Get the configurable ClickHouse settings schema
    Schema {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Set one or more ClickHouse settings
    #[command(
        group(ArgGroup::new("settings_input").required(true).args(["setting", "settings_file"])),
        after_help = "\
CONTEXT FOR AGENTS:
  Discover supported names and types with `settings schema <service-id>`.
  --setting values are JSON literals; quote string values inside the argument.
  --file reads a JSON settings map; `-` reads stdin. Only named settings change."
    )]
    Set {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// Setting as NAME=JSON_VALUE (repeatable)
        #[arg(long, value_name = "NAME=JSON_VALUE")]
        setting: Vec<String>,

        /// JSON settings map file (`-` reads stdin)
        #[arg(long = "file", alias = "settings-file", value_name = "PATH")]
        settings_file: Option<String>,
    },

    /// Reset a ClickHouse setting to its platform default
    Unset {
        #[command(flatten)]
        target: ServiceSettingTarget,
    },
}

/// Keep the natural setting key required while allowing options between the
/// service ID and key. Clap's allow_missing_positional would shift the ID into
/// the key slot when an option follows it; the explicit --name decides grammar.
#[derive(clap::Args)]
pub struct ServiceSettingTarget {
    /// Service ID, or setting name with --name
    #[arg(value_name = "ID_OR_SETTING_NAME")]
    id_or_setting_name: String,
    /// Setting name when selecting by service ID
    #[arg(
        value_name = "SETTING_NAME",
        required_unless_present = "name",
        conflicts_with = "name"
    )]
    setting_name: Option<String>,
    /// Exact service name within the selected organization
    #[arg(long)]
    name: Option<String>,
}

impl ServiceSettingTarget {
    pub(in crate::cloud) fn parts(&self) -> (NameSelector, &str) {
        match &self.name {
            Some(name) => (
                NameSelector {
                    id: None,
                    name: Some(name.clone()),
                },
                &self.id_or_setting_name,
            ),
            None => (
                NameSelector {
                    id: Some(self.id_or_setting_name.clone()),
                    name: None,
                },
                self.setting_name
                    .as_deref()
                    .expect("clap requires the setting name with a positional ID"),
            ),
        }
    }
}

#[derive(Subcommand)]
pub enum ScalingScheduleCommands {
    /// Get the scaling schedule
    Get {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Create or replace the scaling schedule
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  --file is a JSON object with an entries array; '-' reads it from stdin.
  Set replaces every existing entry. Get, edit only entries, then set to preserve other entries.
  Hours and weekdays are UTC.
  Omit response-only baseConfig and each entry's id and isActiveNow when setting a schedule.")]
    Set {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// JSON request file (use "-" for stdin)
        #[arg(long, value_name = "PATH")]
        file: String,
    },

    /// Delete the scaling schedule
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Deletes every scheduled entry. An active entry returns to the service's base scaling config.")]
    Delete {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },
}

#[derive(Subcommand)]
pub enum UpgradeWindowCommands {
    /// Get the service upgrade window
    Get {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Set the service upgrade window
    Set {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// Start day: 0=Sunday through 6=Saturday
        #[arg(
            long,
            allow_hyphen_values = true,
            value_parser = clap::value_parser!(i64).range(0..=6)
        )]
        weekday: i64,

        /// UTC start hour
        #[arg(long, value_parser = PossibleValuesParser::new(["0", "6", "12", "18"]))]
        start_hour: String,
    },

    /// Delete the service upgrade window
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Restores the default upgrade scheduling behaviour."
    )]
    Delete {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },
}

#[derive(Subcommand)]
pub enum ReplicaAwareRoutingCommands {
    /// Get replica aware routing settings (Beta)
    Get {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,
    },

    /// Update replica aware routing settings (Beta)
    #[command(group(
        ArgGroup::new("routing_change")
            .required(true)
            .multiple(true)
            .args(["http_enabled", "native_enabled"])
    ))]
    Update {
        /// Service ID
        #[command(flatten)]
        service_id: NameSelector,

        /// Turn routing on or off for HTTP connections
        #[arg(long, value_name = "BOOL")]
        http_enabled: Option<bool>,

        /// Turn routing on or off for native protocol connections
        #[arg(long, value_name = "BOOL")]
        native_enabled: Option<bool>,
    },
}

impl ReplicaAwareRoutingCommands {
    fn is_write(&self) -> bool {
        match self {
            ReplicaAwareRoutingCommands::Get { .. } => false,
            ReplicaAwareRoutingCommands::Update { .. } => true,
        }
    }
}

impl UpgradeWindowCommands {
    fn is_write(&self) -> bool {
        match self {
            UpgradeWindowCommands::Get { .. } => false,
            UpgradeWindowCommands::Set { .. } | UpgradeWindowCommands::Delete { .. } => true,
        }
    }
}

impl ServiceProfileCommands {
    fn is_write(&self) -> bool {
        match self {
            ServiceProfileCommands::List { .. } => false,
        }
    }
}

impl ServiceSettingsCommands {
    fn is_write(&self) -> bool {
        match self {
            ServiceSettingsCommands::List { .. }
            | ServiceSettingsCommands::Get { .. }
            | ServiceSettingsCommands::Schema { .. } => false,
            ServiceSettingsCommands::Set { .. } | ServiceSettingsCommands::Unset { .. } => true,
        }
    }
}

impl ScalingScheduleCommands {
    fn is_write(&self) -> bool {
        match self {
            ScalingScheduleCommands::Get { .. } => false,
            ScalingScheduleCommands::Set { .. } | ScalingScheduleCommands::Delete { .. } => true,
        }
    }
}

impl ServiceCommands {
    pub fn is_write(&self) -> bool {
        match self {
            ServiceCommands::List { .. } => false,
            ServiceCommands::Get { .. } => false,
            ServiceCommands::Dotenv { .. } => false,
            ServiceCommands::Profile { command } => command.is_write(),
            ServiceCommands::Settings { command } => command.is_write(),
            ServiceCommands::ScalingSchedule { command } => command.is_write(),
            ServiceCommands::Prometheus { .. } => false,
            ServiceCommands::Query { .. } => false,
            ServiceCommands::RepairQueryKey { .. } => true,
            ServiceCommands::Create { .. } => true,
            ServiceCommands::Delete { .. } => true,
            ServiceCommands::Start { .. } => true,
            ServiceCommands::Wake { .. } => true,
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
            ServiceCommands::BackupConfig { command } => command.is_write(),
            ServiceCommands::Snapshot { command } => command.is_write(),
            ServiceCommands::UpgradeWindow { command } => command.is_write(),
            ServiceCommands::ReplicaAwareRouting { command } => command.is_write(),
        }
    }
}

/// Prefix of an AWS PrivateLink VPC endpoint ID.
const AWS_VPC_ENDPOINT_PREFIX: &str = "vpce-";

/// Clap `value_parser` for flags that *add* a private endpoint ID to a service.
///
/// Adding an ID registers it org-wide and the API accepts any string, so a typo
/// silently installs a dud entry that has to be unpicked from both the service
/// and the organization (issue #611). Reject the cases that cannot be a real
/// endpoint ID before the request is sent; clap reports this as a usage error.
///
/// Removal flags deliberately skip this check — a bogus ID that is already
/// registered must stay removable.
fn parse_private_endpoint_id_arg(value: &str) -> std::result::Result<String, String> {
    validate_private_endpoint_id(value).map(|()| value.to_string())
}

/// The three providers use unrelated endpoint ID formats, and the provider is
/// not known at parse time, so only checks that hold for every provider are
/// applied:
///
/// * AWS PrivateLink IDs are `vpce-` plus 8 or 17 lowercase hex characters. The
///   prefix is unmistakable, so a value carrying it anywhere must be exactly one
///   well-formed AWS ID — that catches truncated IDs, uppercased IDs and a
///   pasted VPC endpoint ARN.
/// * GCP uses a numeric Private Service Connect connection ID and Azure uses a
///   private endpoint Resource ID or `resourceGuid`. Neither has a marker that
///   distinguishes a typo from a valid value, so they are only checked for
///   emptiness and whitespace (illegal in all three formats). An Azure Resource
///   ID is an absolute path and can name a resource `vpce-...`, so it is exempt
///   from the AWS check.
///
/// Whether the endpoint exists and belongs to the caller is not checkable from
/// the client; that remains an upstream API gap.
fn validate_private_endpoint_id(value: &str) -> std::result::Result<(), String> {
    if value.trim().is_empty() {
        return Err("private endpoint ID must not be empty".to_string());
    }
    if value.chars().any(char::is_whitespace) {
        return Err(format!(
            "invalid private endpoint ID '{value}': IDs cannot contain whitespace \
             (pass each endpoint ID as its own flag value)"
        ));
    }
    // Azure Resource IDs are absolute paths and can legitimately name a
    // resource `vpce-...`, so they are exempt from the AWS-shaped check below.
    if value.starts_with('/') {
        return Ok(());
    }
    if value.to_ascii_lowercase().contains(AWS_VPC_ENDPOINT_PREFIX)
        && !is_aws_vpc_endpoint_id(value)
    {
        return Err(format!(
            "invalid AWS VPC endpoint ID '{value}': expected exactly '{AWS_VPC_ENDPOINT_PREFIX}' \
             followed by 8 or 17 lowercase hex characters (for example vpce-0123456789abcdef0). GCP \
             uses the numeric Private Service Connect connection ID and Azure the private \
             endpoint Resource ID or resourceGuid"
        ));
    }
    Ok(())
}

/// `vpce-` plus 8 (legacy) or 17 (current) lowercase hex characters, and nothing
/// else.
fn is_aws_vpc_endpoint_id(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix(AWS_VPC_ENDPOINT_PREFIX) else {
        return false;
    };
    matches!(suffix.len(), 8 | 17)
        && suffix
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
}

#[cfg(test)]
mod tests {
    #[test]
    fn primary_json_file_argument_contract() {
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "service", "settings", "set", "svc-1"],
            "settings_file",
            &["settings-file"],
        );
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "service", "scaling-schedule", "set", "svc-1"],
            "file",
            &[],
        );
    }

    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use clap::Parser;

    fn parse_service(args: &[&str]) -> crate::cloud::cli::ServiceCommands {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
        crate::cloud::cli::tests::assert_org_selector(&cloud_args, args);
        let CloudCommands::Service { command } = cloud_args.command else {
            panic!("expected service command");
        };
        command
    }

    fn assert_write(args: &[&str], expected: bool) {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
        assert!(matches!(&cloud_args.command, CloudCommands::Service { .. }));
        assert_eq!(
            cloud_args.command.is_write_command(),
            expected,
            "wrong classification for: {}",
            args.join(" ")
        );
    }

    /// The `CONTEXT FOR AGENTS:` block on `service query` stays inside its
    /// 8-line budget, whatever the lines say.
    #[test]
    fn service_query_agent_context_block_stays_within_its_line_budget() {
        let error = Cli::try_parse_from(["clickhousectl", "cloud", "service", "query", "--help"])
            .err()
            .expect("--help should stop parsing");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = error.to_string();

        let block = help
            .split_once("CONTEXT FOR AGENTS:\n")
            .expect("after_help block")
            .1;
        let lines = block.lines().filter(|line| !line.trim().is_empty()).count();
        assert!(lines <= 8, "{lines} lines: {help}");
    }

    #[test]
    fn repair_query_key_has_a_positional_id_selector() {
        use clap::CommandFactory;
        let mut root = Cli::command();
        root.build();
        let command = root
            .find_subcommand("cloud")
            .unwrap()
            .find_subcommand("service")
            .unwrap()
            .find_subcommand("repair-query-key")
            .unwrap();
        let id = command
            .get_arguments()
            .find(|arg| arg.get_id() == "resource_id")
            .unwrap();
        assert!(id.is_positional());
        assert_eq!(id.get_value_names().unwrap(), &["ID"]);
    }

    #[test]
    fn parses_service_create_defaults() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "create",
            "--name",
            "default-service",
        ]);
        let crate::cloud::cli::ServiceCommands::Create {
            name,
            provider,
            region,
            min_replica_memory_gb,
            max_replica_memory_gb,
            num_replicas,
            min_replicas,
            max_replicas,
            autoscaling_mode,
            idle_scaling,
            idle_timeout_minutes,
            ip_allow,
            backup_id,
            backup_encryption_config,
            release_channel,
            data_warehouse_id,
            readonly,
            enable_tde,
            compliance_type,
            profile,
            byoc_id,
            tag,
            enable_endpoint,
            disable_endpoint,
            private_preview_terms_checked,
            enable_core_dumps,
            dotenv,
        } = command
        else {
            panic!("expected service create");
        };

        assert!(dotenv.is_none());
        assert_eq!(name, "default-service");
        assert_eq!(provider, "aws");
        assert_eq!(region, "us-east-1");
        assert!(min_replica_memory_gb.is_none());
        assert!(max_replica_memory_gb.is_none());
        assert!(num_replicas.is_none());
        assert!(min_replicas.is_none());
        assert!(max_replicas.is_none());
        assert!(autoscaling_mode.is_none());
        assert!(idle_scaling.is_none());
        assert!(idle_timeout_minutes.is_none());
        assert!(ip_allow.is_empty());
        assert!(backup_id.is_none());
        assert!(backup_encryption_config.is_none());
        assert!(release_channel.is_none());
        assert!(data_warehouse_id.is_none());
        assert!(!readonly);
        assert!(!enable_tde);
        assert!(compliance_type.is_none());
        assert!(profile.is_none());
        assert!(byoc_id.is_none());
        assert!(tag.is_empty());
        assert!(enable_endpoint.is_empty());
        assert!(disable_endpoint.is_empty());
        assert!(!private_preview_terms_checked);
        assert!(enable_core_dumps.is_none());
    }

    #[test]
    fn parses_service_create_maximal_vertical_flags() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "create",
            "--name",
            "maximal-service",
            "--provider",
            "azure",
            "--region",
            "eastus",
            "--min-replica-memory-gb",
            "16",
            "--max-replica-memory-gb",
            "32",
            "--num-replicas",
            "3",
            "--autoscaling-mode",
            "vertical",
            "--idle-scaling",
            "false",
            "--idle-timeout-minutes",
            "10",
            "--ip-allow",
            "10.0.0.0/8=office",
            "--ip-allow",
            "2001:db8::/32=\u{6771}\u{4eac}",
            "--backup-id",
            "backup-1",
            "--backup-encryption-config",
            "-",
            "--release-channel",
            "fast",
            "--data-warehouse-id",
            "dw-1",
            "--readonly",
            "--enable-tde",
            "--compliance-type",
            "pci",
            "--profile",
            "v1-highmem-xs",
            "--byoc-id",
            "byoc-1",
            "--tag",
            "env=prod",
            "--tag",
            "team=analytics",
            "--enable-endpoint",
            "mysql",
            "--disable-endpoint",
            "mysql",
            "--private-preview-terms-checked",
            "--enable-core-dumps",
            "false",
            "--org-id",
            "org-1",
        ]);
        let crate::cloud::cli::ServiceCommands::Create {
            name,
            provider,
            region,
            min_replica_memory_gb,
            max_replica_memory_gb,
            num_replicas,
            autoscaling_mode,
            idle_scaling,
            idle_timeout_minutes,
            ip_allow,
            backup_id,
            backup_encryption_config,
            release_channel,
            data_warehouse_id,
            readonly,
            enable_tde,
            compliance_type,
            profile,
            byoc_id,
            tag,
            enable_endpoint,
            disable_endpoint,
            private_preview_terms_checked,
            enable_core_dumps,
            ..
        } = command
        else {
            panic!("expected service create");
        };

        assert_eq!(name, "maximal-service");
        assert_eq!(provider, "azure");
        assert_eq!(region, "eastus");
        assert_eq!(min_replica_memory_gb, Some(16));
        assert_eq!(max_replica_memory_gb, Some(32));
        assert_eq!(num_replicas, Some(3));
        assert_eq!(autoscaling_mode.as_deref(), Some("vertical"));
        assert_eq!(idle_scaling, Some(false));
        assert_eq!(idle_timeout_minutes, Some(10));
        assert_eq!(
            ip_allow,
            vec!["10.0.0.0/8=office", "2001:db8::/32=\u{6771}\u{4eac}"]
        );
        assert_eq!(backup_id.as_deref(), Some("backup-1"));
        assert_eq!(backup_encryption_config.as_deref(), Some("-"));
        assert_eq!(release_channel.as_deref(), Some("fast"));
        assert_eq!(data_warehouse_id.as_deref(), Some("dw-1"));
        assert!(readonly);
        assert!(enable_tde);
        assert_eq!(compliance_type.as_deref(), Some("pci"));
        assert_eq!(profile.as_deref(), Some("v1-highmem-xs"));
        assert_eq!(byoc_id.as_deref(), Some("byoc-1"));
        assert_eq!(tag, vec!["env=prod", "team=analytics"]);
        assert_eq!(enable_endpoint, vec!["mysql"]);
        assert_eq!(disable_endpoint, vec!["mysql"]);
        assert!(private_preview_terms_checked);
        assert_eq!(enable_core_dumps, Some(false));
    }

    #[test]
    fn parses_service_create_dynamic_byoc_profile_flags() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "create",
            "--name",
            "byoc-service",
            "--profile",
            "v1-standard-byoc-4",
            "--byoc-id",
            "byoc-1",
            "--min-replica-memory-gb",
            "48",
            "--max-replica-memory-gb",
            "48",
        ]);
        let ServiceCommands::Create {
            profile,
            byoc_id,
            min_replica_memory_gb,
            max_replica_memory_gb,
            ..
        } = command
        else {
            panic!("expected service create");
        };
        assert_eq!(profile.as_deref(), Some("v1-standard-byoc-4"));
        assert_eq!(byoc_id.as_deref(), Some("byoc-1"));
        assert_eq!(min_replica_memory_gb, Some(48));
        assert_eq!(max_replica_memory_gb, Some(48));
    }

    #[test]
    fn parses_service_list_filters_and_org_id() {
        let command = parse_service(&["clickhousectl", "cloud", "service", "list"]);
        let crate::cloud::cli::ServiceCommands::List { filter } = command else {
            panic!("expected service list");
        };

        assert!(filter.is_empty());

        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "list",
            "--filter",
            "tag:env=prod",
            "--filter",
            "tag:team=analytics",
            "--org-id",
            "org-1",
        ]);
        let crate::cloud::cli::ServiceCommands::List { filter } = command else {
            panic!("expected service list");
        };

        assert_eq!(filter, vec!["tag:env=prod", "tag:team=analytics"]);
    }

    #[test]
    fn service_list_tag_filters_preserve_api_grammar() {
        for value in ["tag:env=prod", "tag:active", "tag:empty=", "tag:expr=a=b"] {
            let command = parse_service(&[
                "clickhousectl",
                "cloud",
                "service",
                "list",
                "--filter",
                value,
            ]);
            let ServiceCommands::List { filter, .. } = command else {
                panic!("expected service list");
            };
            assert_eq!(filter, [value]);
        }
        for value in [
            "garbage",
            "state=running",
            "env=prod",
            "tag:",
            "tag:=x",
            "tag: =x",
        ] {
            let error = Cli::try_parse_from([
                "clickhousectl",
                "cloud",
                "service",
                "list",
                "--filter",
                value,
            ])
            .err()
            .expect("malformed filter must fail");
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        }
    }

    #[test]
    fn parses_service_dotenv_selector_and_local_flag() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "dotenv",
            "svc-1",
            "--org-id",
            "org-1",
        ]);
        assert!(!command.is_write());
        let ServiceCommands::Dotenv { service_id, local } = command else {
            panic!("expected service dotenv");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
        assert!(!local);

        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "dotenv",
            "--name",
            "analytics",
            "--local",
        ]);
        let ServiceCommands::Dotenv { service_id, local } = command else {
            panic!("expected service dotenv");
        };
        assert_eq!(service_id.name.as_deref(), Some("analytics"));
        assert!(service_id.id.is_none());
        assert!(local);
    }

    #[test]
    fn parses_service_create_dotenv_with_and_without_a_file() {
        for (flags, expected) in [
            (vec!["--dotenv"], ".env"),
            (vec!["--dotenv", ".env.local"], ".env.local"),
            (vec!["--dotenv=config/app.env"], "config/app.env"),
            // A following flag is not taken as the file.
            (vec!["--dotenv", "--org-id", "org-1"], ".env"),
        ] {
            let mut args = vec![
                "clickhousectl",
                "cloud",
                "service",
                "create",
                "--name",
                "svc",
            ];
            args.extend(flags);
            let ServiceCommands::Create { dotenv, .. } = parse_service(&args) else {
                panic!("expected service create");
            };
            assert_eq!(dotenv.as_deref(), Some(std::path::Path::new(expected)));
        }

        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "create",
            "--name",
            "svc",
            "--dotenv",
            "a.env",
            "--dotenv",
            "b.env",
        ])
        .err()
        .expect("--dotenv is not repeatable");
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn parses_service_profile_list_region_or_byoc_flags() {
        for (flags, expected_region, expected_byoc) in [
            (vec!["--region", "eu-west-1"], Some("eu-west-1"), None),
            (vec!["--byoc-id", "byoc-1"], None, Some("byoc-1")),
            (
                vec!["--region", "eu-west-1", "--byoc-id", "byoc-1"],
                Some("eu-west-1"),
                Some("byoc-1"),
            ),
        ] {
            let mut args = vec!["clickhousectl", "cloud", "service", "profile", "list"];
            args.extend(flags);
            let command = parse_service(&args);
            assert!(!command.is_write());
            let ServiceCommands::Profile {
                command: ServiceProfileCommands::List { region, byoc_id },
            } = command
            else {
                panic!("expected service profile list");
            };
            assert_eq!(region.as_deref(), expected_region);
            assert_eq!(byoc_id.as_deref(), expected_byoc);
        }

        let error = Cli::try_parse_from(["clickhousectl", "cloud", "service", "profile", "list"])
            .err()
            .expect("missing both region and BYOC should fail");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn parses_scaling_schedule_commands_and_classifies_writes() {
        let get = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "scaling-schedule",
            "get",
            "svc-1",
        ]);
        let ServiceCommands::ScalingSchedule {
            command: ScalingScheduleCommands::Get { service_id },
        } = get
        else {
            panic!("expected scaling schedule get");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));

        let set = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "scaling-schedule",
            "set",
            "svc-1",
            "--file",
            "-",
            "--org-id",
            "org-1",
        ]);
        let ServiceCommands::ScalingSchedule {
            command: ScalingScheduleCommands::Set { service_id, file },
        } = set
        else {
            panic!("expected scaling schedule set");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
        assert_eq!(file, "-");

        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "scaling-schedule",
                "get",
                "svc-1",
            ],
            false,
        );
        for leaf in ["set", "delete"] {
            let mut args = vec![
                "clickhousectl",
                "cloud",
                "service",
                "scaling-schedule",
                leaf,
                "svc-1",
            ];
            if leaf == "set" {
                args.extend(["--file", "schedule.json"]);
            }
            assert_write(&args, true);
        }
    }

    #[test]
    fn scaling_schedule_set_requires_file() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "scaling-schedule",
            "set",
            "svc-1",
        ])
        .err()
        .expect("missing --file should fail");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn parses_upgrade_window_commands_and_classifies_writes() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "upgrade-window",
            "set",
            "svc-1",
            "--weekday",
            "3",
            "--start-hour",
            "12",
            "--org-id",
            "org-1",
        ]);
        let ServiceCommands::UpgradeWindow {
            command:
                UpgradeWindowCommands::Set {
                    service_id,
                    weekday,
                    start_hour,
                },
        } = command
        else {
            panic!("expected upgrade-window set");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
        assert_eq!(weekday, 3);
        assert_eq!(start_hour, "12");

        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "upgrade-window",
                "get",
                "svc-1",
            ],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "upgrade-window",
                "set",
                "svc-1",
                "--weekday",
                "0",
                "--start-hour",
                "0",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "upgrade-window",
                "delete",
                "svc-1",
            ],
            true,
        );
    }

    #[test]
    fn parses_replica_aware_routing_commands_and_classifies_writes() {
        let base = ["clickhousectl", "cloud", "service", "replica-aware-routing"];
        let parse_update = |flags: &[&str]| {
            let mut args = base.to_vec();
            args.extend(["update", "svc-1"]);
            args.extend(flags);
            match parse_service(&args) {
                ServiceCommands::ReplicaAwareRouting {
                    command:
                        ReplicaAwareRoutingCommands::Update {
                            service_id,
                            http_enabled,
                            native_enabled,
                        },
                } => {
                    assert_eq!(service_id.id.as_deref(), Some("svc-1"));
                    (http_enabled, native_enabled)
                }
                _ => panic!("expected replica-aware-routing update"),
            }
        };
        assert_eq!(
            parse_update(&["--http-enabled", "true"]),
            (Some(true), None)
        );
        assert_eq!(
            parse_update(&["--native-enabled", "false"]),
            (None, Some(false))
        );
        assert_eq!(
            parse_update(&["--http-enabled", "false", "--native-enabled", "true"]),
            (Some(false), Some(true))
        );

        let ServiceCommands::ReplicaAwareRouting {
            command: ReplicaAwareRoutingCommands::Get { service_id },
        } = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "replica-aware-routing",
            "get",
            "--name",
            "analytics",
        ])
        else {
            panic!("expected replica-aware-routing get");
        };
        assert_eq!(service_id.name.as_deref(), Some("analytics"));

        let mut get = base.to_vec();
        get.extend(["get", "svc-1"]);
        assert_write(&get, false);
        let mut update = base.to_vec();
        update.extend(["update", "svc-1", "--native-enabled", "true"]);
        assert_write(&update, true);
    }

    #[test]
    fn replica_aware_routing_update_requires_a_change_and_boolean_values() {
        let base = [
            "clickhousectl",
            "cloud",
            "service",
            "replica-aware-routing",
            "update",
            "svc-1",
        ];
        let error = Cli::try_parse_from(base)
            .err()
            .expect("no change should fail");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        for flag in ["--http-enabled", "--native-enabled"] {
            let mut args = base.to_vec();
            args.extend([flag, "yes"]);
            let error = Cli::try_parse_from(args)
                .err()
                .expect("non-bool should fail");
            assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
            let mut args = base.to_vec();
            args.push(flag);
            assert!(Cli::try_parse_from(args).is_err(), "{flag} needs a value");
        }
    }

    #[test]
    fn upgrade_window_set_rejects_missing_and_out_of_range_values() {
        for args in [
            vec![
                "clickhousectl",
                "cloud",
                "service",
                "upgrade-window",
                "set",
                "svc-1",
                "--start-hour",
                "6",
            ],
            vec![
                "clickhousectl",
                "cloud",
                "service",
                "upgrade-window",
                "set",
                "svc-1",
                "--weekday",
                "2",
            ],
        ] {
            let error = Cli::try_parse_from(args).err().expect("should fail");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
        }

        for (flag, value) in [
            ("--weekday", "7"),
            ("--weekday", "-1"),
            ("--start-hour", "3"),
        ] {
            let mut args = vec![
                "clickhousectl",
                "cloud",
                "service",
                "upgrade-window",
                "set",
                "svc-1",
                "--weekday",
                "2",
                "--start-hour",
                "6",
            ];
            let index = args.iter().position(|arg| *arg == flag).unwrap() + 1;
            args[index] = value;
            let error = Cli::try_parse_from(args).err().expect("should fail");
            assert!(matches!(
                error.kind(),
                clap::error::ErrorKind::ValueValidation | clap::error::ErrorKind::InvalidValue
            ));
        }
    }

    #[test]
    fn parses_service_delete_force_and_org_id() {
        let command = parse_service(&["clickhousectl", "cloud", "service", "delete", "svc-1"]);
        let crate::cloud::cli::ServiceCommands::Delete { force, .. } = command else {
            panic!("expected service delete");
        };
        assert!(!force);

        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "delete",
            "svc-1",
            "--force",
            "--org-id",
            "org-1",
        ]);
        let crate::cloud::cli::ServiceCommands::Delete { service_id, force } = command else {
            panic!("expected service delete");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
        assert!(force);
    }

    #[test]
    fn parses_service_body_command_defaults() {
        let update = parse_service(&["clickhousectl", "cloud", "service", "update", "svc-1"]);
        let crate::cloud::cli::ServiceCommands::Update {
            name,
            add_ip_allow,
            remove_ip_allow,
            add_private_endpoint_id,
            remove_private_endpoint_id,
            release_channel,
            enable_endpoint,
            disable_endpoint,
            transparent_data_encryption_key_id,
            add_tag,
            remove_tag,
            enable_core_dumps,
            ..
        } = update
        else {
            panic!("expected service update");
        };
        assert!(name.is_none());
        assert!(add_ip_allow.is_empty());
        assert!(remove_ip_allow.is_empty());
        assert!(add_private_endpoint_id.is_empty());
        assert!(remove_private_endpoint_id.is_empty());
        assert!(release_channel.is_none());
        assert!(enable_endpoint.is_empty());
        assert!(disable_endpoint.is_empty());
        assert!(transparent_data_encryption_key_id.is_none());
        assert!(add_tag.is_empty());
        assert!(remove_tag.is_empty());
        assert!(enable_core_dumps.is_none());

        let scale = parse_service(&["clickhousectl", "cloud", "service", "scale", "svc-1"]);
        let crate::cloud::cli::ServiceCommands::Scale {
            min_replica_memory_gb,
            max_replica_memory_gb,
            num_replicas,
            min_replicas,
            max_replicas,
            autoscaling_mode,
            idle_scaling,
            idle_timeout_minutes,
            ..
        } = scale
        else {
            panic!("expected service scale");
        };
        assert!(min_replica_memory_gb.is_none());
        assert!(max_replica_memory_gb.is_none());
        assert!(num_replicas.is_none());
        assert!(min_replicas.is_none());
        assert!(max_replicas.is_none());
        assert!(autoscaling_mode.is_none());
        assert!(idle_scaling.is_none());
        assert!(idle_timeout_minutes.is_none());

        let reset = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "reset-password",
            "svc-1",
        ]);
        let crate::cloud::cli::ServiceCommands::ResetPassword {
            new_password_hash,
            new_double_sha1_hash,
            ..
        } = reset
        else {
            panic!("expected service reset-password");
        };
        assert!(new_password_hash.is_none());
        assert!(new_double_sha1_hash.is_none());

        let query_endpoint = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "query-endpoint",
            "create",
            "svc-1",
            "--role",
            "sql_console_read_only",
        ]);
        let crate::cloud::cli::ServiceCommands::QueryEndpoint { command } = query_endpoint else {
            panic!("expected query-endpoint command");
        };
        let crate::cloud::cli::QueryEndpointCommands::Create {
            role,
            open_api_key,
            allowed_origins,
            replace_open_api_keys,
            ..
        } = command
        else {
            panic!("expected query-endpoint create");
        };
        assert_eq!(role, ["sql_console_read_only"]);
        assert!(!replace_open_api_keys);
        assert!(open_api_key.is_empty());
        assert!(allowed_origins.is_none());

        let private_endpoint = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "private-endpoint",
            "create",
            "svc-1",
            "--endpoint-id",
            "vpce-0123456789abcdef0",
        ]);
        let crate::cloud::cli::ServiceCommands::PrivateEndpoint { command } = private_endpoint
        else {
            panic!("expected private-endpoint command");
        };
        let crate::cloud::cli::PrivateEndpointCommands::Create { description, .. } = command else {
            panic!("expected private-endpoint create");
        };
        assert!(description.is_none());
    }

    #[test]
    fn parses_service_query_json_mode() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "query",
            "--id",
            "svc-1",
            "--query",
            "SELECT 1",
            "--json",
        ])
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        assert!(args.json);
        let CloudCommands::Service { command } = args.command else {
            panic!("expected service command");
        };
        let ServiceCommands::Query { format, .. } = command else {
            panic!("expected service query");
        };
        assert!(format.is_none());
    }

    #[test]
    fn rejects_service_query_json_with_explicit_format() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "query",
            "--id",
            "svc-1",
            "--query",
            "SELECT 1",
            "--json",
            "--format",
            "CSV",
        ])
        .err()
        .expect("--json and --format should conflict");
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        assert!(error.to_string().contains("--json"));
        assert!(error.to_string().contains("--format"));
    }

    #[test]
    fn parses_service_query_explicit_format_without_json() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "query",
            "--id",
            "svc-1",
            "--query",
            "SELECT 1",
            "--format",
            "CSV",
        ]);
        let ServiceCommands::Query { format, .. } = command else {
            panic!("expected service query");
        };
        assert_eq!(format.as_deref(), Some("CSV"));
    }

    #[test]
    fn parses_service_update_ga_patch_flags() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "update",
            "svc-1",
            "--add-ip-allow",
            "2001:db8::/32=office",
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
        ]);
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
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
        assert_eq!(add_ip_allow, vec!["2001:db8::/32=office"]);
        assert_eq!(remove_ip_allow, vec!["0.0.0.0/0"]);
        assert_eq!(add_private_endpoint_id, vec!["pe-1"]);
        assert_eq!(remove_private_endpoint_id, vec!["pe-2"]);
        assert_eq!(release_channel.as_deref(), Some("fast"));
        assert_eq!(enable_endpoint, vec!["mysql"]);
        assert_eq!(add_tag, vec!["env=prod"]);
        assert_eq!(enable_core_dumps, Some(true));
    }

    #[test]
    fn parses_service_update_maximal_and_repeatable_add_flags() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "update",
            "svc-1",
            "--new-name",
            "renamed",
            "--add-ip-allow",
            "10.0.0.0/8=office",
            "--add-ip-allow",
            "2001:db8::/32=\u{6771}\u{4eac}",
            "--remove-ip-allow",
            "0.0.0.0/0",
            "--add-private-endpoint-id",
            "pe-1",
            "--add-private-endpoint-id",
            "pe-2",
            "--remove-private-endpoint-id",
            "pe-3",
            "--release-channel",
            "slow",
            "--enable-endpoint",
            "mysql",
            "--disable-endpoint",
            "mysql",
            "--transparent-data-encryption-key-id",
            "tde-1",
            "--add-tag",
            "env=prod",
            "--add-tag",
            "team=analytics",
            "--enable-core-dumps",
            "false",
            "--org-id",
            "org-1",
        ]);
        let crate::cloud::cli::ServiceCommands::Update {
            service_id,
            name,
            add_ip_allow,
            remove_ip_allow,
            add_private_endpoint_id,
            remove_private_endpoint_id,
            release_channel,
            enable_endpoint,
            disable_endpoint,
            transparent_data_encryption_key_id,
            add_tag,
            remove_tag,
            enable_core_dumps,
        } = command
        else {
            panic!("expected service update");
        };

        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
        assert_eq!(name.as_deref(), Some("renamed"));
        assert_eq!(
            add_ip_allow,
            vec!["10.0.0.0/8=office", "2001:db8::/32=\u{6771}\u{4eac}"]
        );
        assert_eq!(remove_ip_allow, vec!["0.0.0.0/0"]);
        assert_eq!(add_private_endpoint_id, vec!["pe-1", "pe-2"]);
        assert_eq!(remove_private_endpoint_id, vec!["pe-3"]);
        assert_eq!(release_channel.as_deref(), Some("slow"));
        assert_eq!(enable_endpoint, vec!["mysql"]);
        assert_eq!(disable_endpoint, vec!["mysql"]);
        assert_eq!(transparent_data_encryption_key_id.as_deref(), Some("tde-1"));
        assert_eq!(add_tag, vec!["env=prod", "team=analytics"]);
        assert!(remove_tag.is_empty());
        assert_eq!(enable_core_dumps, Some(false));
    }

    #[test]
    fn parses_service_update_repeatable_tag_removals_with_other_removals() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "update",
            "svc-1",
            "--remove-tag",
            "legacy",
            "--remove-tag",
            "owner=former-team",
            "--remove-ip-allow",
            "10.0.0.0/8",
            "--remove-private-endpoint-id",
            "pe-1",
        ]);
        let ServiceCommands::Update {
            add_tag,
            remove_tag,
            remove_ip_allow,
            remove_private_endpoint_id,
            ..
        } = command
        else {
            panic!("expected service update");
        };

        assert!(add_tag.is_empty());
        assert_eq!(remove_tag, vec!["legacy", "owner=former-team"]);
        assert_eq!(remove_ip_allow, vec!["10.0.0.0/8"]);
        assert_eq!(remove_private_endpoint_id, vec!["pe-1"]);
    }

    #[test]
    fn rejects_combining_service_update_tag_additions_and_removals() {
        for (first_flag, first_value, second_flag, second_value) in [
            ("--add-tag", "env=prod", "--remove-tag", "env=prod"),
            ("--remove-tag", "env=prod", "--add-tag", "env=prod"),
            ("--add-tag", "env=prod", "--remove-tag", "env=staging"),
            ("--remove-tag", "env=staging", "--add-tag", "env=prod"),
            ("--add-tag", "env=prod", "--remove-tag", "team=analytics"),
            ("--remove-tag", "team=analytics", "--add-tag", "env=prod"),
            ("--add-tag", "env", "--remove-tag", "team"),
            ("--remove-tag", "team", "--add-tag", "env"),
        ] {
            let error = Cli::try_parse_from([
                "clickhousectl",
                "cloud",
                "service",
                "update",
                "svc-1",
                first_flag,
                first_value,
                second_flag,
                second_value,
            ])
            .err()
            .expect("mixed tag operations must conflict");

            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "{first_flag} {first_value} followed by {second_flag} {second_value}"
            );
        }
    }

    #[test]
    fn parses_service_create_horizontal_autoscaling_flags() {
        let command = parse_service(&[
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
        ]);
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
        assert!(
            Cli::try_parse_from([
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
            ])
            .is_err()
        );
    }

    #[test]
    fn parses_service_create_horizontal_mode_with_memory_bounds() {
        let command = parse_service(&[
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
        ]);
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
        assert!(
            Cli::try_parse_from([
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
            ])
            .is_err()
        );
    }

    #[test]
    fn parses_service_scale_horizontal_autoscaling_flags() {
        let command = parse_service(&[
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
        ]);
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
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
        assert_eq!(min_replicas, Some(2));
        assert_eq!(max_replicas, Some(8));
        assert_eq!(autoscaling_mode.as_deref(), Some("horizontal"));
        assert!(num_replicas.is_none());
    }

    #[test]
    fn parses_service_scale_switch_to_vertical_in_one_call() {
        let command = parse_service(&[
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
            "--idle-scaling",
            "false",
            "--idle-timeout-minutes",
            "15",
            "--org-id",
            "org-1",
        ]);
        let ServiceCommands::Scale {
            autoscaling_mode,
            num_replicas,
            min_replica_memory_gb,
            max_replica_memory_gb,
            min_replicas,
            max_replicas,
            idle_scaling,
            idle_timeout_minutes,
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
        assert_eq!(idle_scaling, Some(false));
        assert_eq!(idle_timeout_minutes, Some(15));
    }

    #[test]
    fn rejects_service_scale_num_replicas_with_replica_band() {
        assert!(
            Cli::try_parse_from([
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
            ])
            .is_err()
        );
    }

    #[test]
    fn parses_private_endpoint_config_and_password_hash_flags() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "reset-password",
            "svc-1",
            "--new-password-hash",
            "sha256",
            "--new-double-sha1-hash",
            "sha1",
            "--org-id",
            "org-1",
        ]);
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

        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "private-endpoint",
            "get-config",
            "svc-1",
            "--org-id",
            "org-1",
        ]);
        let ServiceCommands::PrivateEndpoint { command } = command else {
            panic!("expected private-endpoint command");
        };
        let PrivateEndpointCommands::GetConfig { service_id } = command else {
            panic!("expected get-config");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
    }

    #[test]
    fn parses_service_query_options() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "query",
            "--id",
            "svc-1",
        ]);
        let crate::cloud::cli::ServiceCommands::Query {
            name,
            id,
            query,
            queries_file,
            database,
            format,
            no_auto_enable,
            ..
        } = command
        else {
            panic!("expected service query");
        };
        assert!(name.is_none());
        assert_eq!(id.as_deref(), Some("svc-1"));
        assert!(query.is_none());
        assert!(queries_file.is_none());
        assert!(database.is_none());
        assert!(format.is_none());

        assert!(!no_auto_enable);

        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "query",
            "--name",
            "analytics",
            "--query",
            "SELECT 1",
            "--database",
            "default",
            "--format",
            "CSV",
            "--org-id",
            "org-1",
            "--no-auto-enable",
        ]);
        let crate::cloud::cli::ServiceCommands::Query {
            name,
            id,
            query,
            queries_file,
            database,
            format,
            no_auto_enable,
            ..
        } = command
        else {
            panic!("expected service query");
        };

        assert_eq!(name.as_deref(), Some("analytics"));
        assert!(id.is_none());
        assert_eq!(query.as_deref(), Some("SELECT 1"));
        assert!(queries_file.is_none());
        assert_eq!(database.as_deref(), Some("default"));
        assert_eq!(format.as_deref(), Some("CSV"));

        assert!(no_auto_enable);
    }

    #[test]
    fn parses_service_query_key_repair_scope() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "repair-query-key",
            "svc-1",
            "--org-id",
            "org-1",
        ]);
        let ServiceCommands::RepairQueryKey { service_id } = command else {
            panic!("expected query-key repair");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));

        let ServiceCommands::RepairQueryKey { .. } = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "repair-query-key",
            "svc-1",
        ]) else {
            panic!("expected query-key repair");
        };
    }

    #[test]
    fn parses_service_query_file_source() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "query",
            "--name",
            "analytics",
            "--queries-file",
            "queries.sql",
        ]);
        let ServiceCommands::Query {
            query,
            queries_file,
            ..
        } = command
        else {
            panic!("expected service query");
        };

        assert!(query.is_none());
        assert_eq!(queries_file.as_deref(), Some("queries.sql"));
    }

    #[test]
    fn rejects_service_query_with_inline_and_file_sources() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "query",
            "--id",
            "svc-1",
            "--query",
            "SELECT 1",
            "--queries-file",
            "queries.sql",
        ])
        .err()
        .expect("service query with conflicting SQL sources should fail");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        let message = error.to_string();
        assert!(message.contains("--query <QUERY>"), "{message}");
        assert!(
            message.contains("--queries-file <QUERIES_FILE>"),
            "{message}"
        );
    }

    #[test]
    fn rejects_service_query_without_name_or_id() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "query",
            "--query",
            "SELECT 1",
        ])
        .err()
        .expect("service query without a selector should fail");

        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        let message = error.to_string();
        assert!(message.contains("--name <NAME>"), "{message}");
        assert!(message.contains("--id <ID>"), "{message}");
    }

    #[test]
    fn rejects_service_query_name_with_id() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "query",
            "--name",
            "analytics",
            "--id",
            "svc-1",
            "--query",
            "SELECT 1",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parses_query_endpoint_arguments() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "query-endpoint",
            "create",
            "svc-1",
            "--role",
            "sql_console_read_only",
            "--role",
            "sql_console_admin",
            "--open-api-key",
            "key-1",
            "--open-api-key",
            "key-2",
            "--replace-open-api-keys",
            "--allowed-origins",
            "https://example.com",
            "--org-id",
            "org-1",
        ]);
        let crate::cloud::cli::ServiceCommands::QueryEndpoint { command } = command else {
            panic!("expected query-endpoint command");
        };
        let crate::cloud::cli::QueryEndpointCommands::Create {
            service_id,
            role,
            open_api_key,
            allowed_origins,
            replace_open_api_keys,
        } = command
        else {
            panic!("expected query-endpoint create");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
        assert!(replace_open_api_keys);
        assert_eq!(role, vec!["sql_console_read_only", "sql_console_admin"]);
        assert_eq!(open_api_key, vec!["key-1", "key-2"]);
        assert_eq!(allowed_origins.as_deref(), Some("https://example.com"));

        for action in ["get", "delete"] {
            let command = parse_service(&[
                "clickhousectl",
                "cloud",
                "service",
                "query-endpoint",
                action,
                "svc-1",
                "--org-id",
                "org-1",
            ]);
            let crate::cloud::cli::ServiceCommands::QueryEndpoint { command } = command else {
                panic!("expected query-endpoint command");
            };
            match command {
                crate::cloud::cli::QueryEndpointCommands::Get { service_id }
                | crate::cloud::cli::QueryEndpointCommands::Delete { service_id } => {
                    assert_eq!(service_id.id.as_deref(), Some("svc-1"));
                }
                crate::cloud::cli::QueryEndpointCommands::Create { .. } => {
                    panic!("expected query-endpoint {action}")
                }
            }
        }
    }

    #[test]
    fn query_endpoint_create_requires_roles_and_nonempty_explicit_values() {
        let prefix = [
            "clickhousectl",
            "cloud",
            "service",
            "query-endpoint",
            "create",
            "svc-1",
        ];
        let missing_role = Cli::try_parse_from(prefix).err().unwrap();
        assert_eq!(
            missing_role.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        for (flags, expected) in [
            (
                vec!["--role", "sql_console_read_only", "--replace-open-api-keys"],
                clap::error::ErrorKind::MissingRequiredArgument,
            ),
            (
                vec!["--role", "sql_console_read_only", "--open-api-key", ""],
                clap::error::ErrorKind::InvalidValue,
            ),
            (
                vec!["--role", "sql_console_read_only", "--allowed-origins", ""],
                clap::error::ErrorKind::InvalidValue,
            ),
        ] {
            let error = Cli::try_parse_from(prefix.into_iter().chain(flags))
                .err()
                .unwrap();
            assert_eq!(error.kind(), expected);
        }
    }

    #[test]
    fn rejects_unknown_query_endpoint_role() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "query-endpoint",
            "create",
            "svc-1",
            "--role",
            "admin",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_private_endpoint_create_values() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "private-endpoint",
            "create",
            "svc-1",
            "--endpoint-id",
            "vpce-0123456789abcdef0",
            "--description",
            "production",
            "--org-id",
            "org-1",
        ]);
        let crate::cloud::cli::ServiceCommands::PrivateEndpoint { command } = command else {
            panic!("expected private-endpoint command");
        };
        let crate::cloud::cli::PrivateEndpointCommands::Create {
            service_id,
            endpoint_id,
            description,
        } = command
        else {
            panic!("expected private-endpoint create");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
        assert_eq!(endpoint_id, "vpce-0123456789abcdef0");
        assert_eq!(description.as_deref(), Some("production"));
    }

    /// Non-AWS endpoint IDs have no marker that separates a typo from a valid
    /// value, so they must pass through untouched (issue #611).
    #[test]
    fn parses_non_aws_private_endpoint_ids_unchanged() {
        for id in [
            // GCP Private Service Connect connection ID.
            "102600141743718403",
            // Azure private endpoint Resource ID.
            "/subscriptions/11111111-2222-3333-4444-555555555555/resourceGroups/rg/providers/\
             Microsoft.Network/privateEndpoints/pe-demo",
            // Azure resourceGuid (legacy form).
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            // Legacy 8-character AWS VPC endpoint ID.
            "vpce-0123abcd",
        ] {
            let command = parse_service(&[
                "clickhousectl",
                "cloud",
                "service",
                "private-endpoint",
                "create",
                "svc-1",
                "--endpoint-id",
                id,
            ]);
            let crate::cloud::cli::ServiceCommands::PrivateEndpoint { command } = command else {
                panic!("expected private-endpoint command");
            };
            let crate::cloud::cli::PrivateEndpointCommands::Create { endpoint_id, .. } = command
            else {
                panic!("expected private-endpoint create");
            };
            assert_eq!(endpoint_id, id, "{id} must be accepted verbatim");
        }
    }

    #[test]
    fn rejects_malformed_private_endpoint_create_endpoint_id() {
        for id in ["vpce-1", "vpce-bogus", "VPCE-0123456789ABCDEF0", "", " "] {
            let error = Cli::try_parse_from([
                "clickhousectl",
                "cloud",
                "service",
                "private-endpoint",
                "create",
                "svc-1",
                "--endpoint-id",
                id,
            ])
            .err()
            .unwrap_or_else(|| panic!("--endpoint-id {id:?} must be rejected"));
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::ValueValidation,
                "--endpoint-id {id:?} must fail as a usage error"
            );
        }
    }

    #[test]
    fn rejects_malformed_added_private_endpoint_id_on_update() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "update",
            "svc-1",
            "--add-private-endpoint-id",
            "vpce-nope",
        ])
        .err()
        .expect("malformed --add-private-endpoint-id must be rejected");
        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
    }

    /// Removing an ID must never be format-checked: a bogus ID that is already
    /// registered has to stay removable (issue #611).
    #[test]
    fn accepts_malformed_removed_private_endpoint_id_on_update() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "update",
            "svc-1",
            "--remove-private-endpoint-id",
            "vpce-nope",
        ]);
        let crate::cloud::cli::ServiceCommands::Update {
            remove_private_endpoint_id,
            ..
        } = command
        else {
            panic!("expected service update");
        };
        assert_eq!(remove_private_endpoint_id, vec!["vpce-nope"]);
    }

    #[test]
    fn validate_private_endpoint_id_accepts_every_provider_format() {
        for id in [
            "vpce-0123456789abcdef0",
            "vpce-0123abcd",
            "102600141743718403",
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "/subscriptions/sub/resourceGroups/rg/providers/Microsoft.Network/privateEndpoints/pe",
            // An Azure resource may itself be named `vpce-...`.
            "/subscriptions/sub/resourceGroups/rg/providers/Microsoft.Network/privateEndpoints/\
             vpce-mine",
        ] {
            assert!(
                validate_private_endpoint_id(id).is_ok(),
                "{id} must be accepted"
            );
        }
    }

    #[test]
    fn validate_private_endpoint_id_rejects_empty_values() {
        for id in ["", "   ", "\t"] {
            let error = validate_private_endpoint_id(id).expect_err("empty must be rejected");
            assert!(error.contains("must not be empty"), "{error}");
        }
    }

    #[test]
    fn validate_private_endpoint_id_rejects_malformed_values() {
        for id in [
            // Two IDs squeezed into one quoted flag value.
            "vpce-0123456789abcdef0 vpce-0123abcd",
            "vpce-",
            "vpce-1",
            "vpce-xyz",
            // 16 hex characters: one short of the current AWS length.
            "vpce-0123456789abcdef",
            // 18 hex characters: one too many.
            "vpce-0123456789abcdef01",
            // Non-hex character inside an otherwise correct-length ID.
            "vpce-0123456789abcdefg",
            // AWS never issues uppercase IDs, so this would register a dud.
            "VPCE-0123456789ABCDEF0",
            // Endpoint *service* name pasted instead of the endpoint ID.
            "com.amazonaws.vpce.us-east-1.vpce-svc-0123456789abcdef0",
            // Full ARN pasted instead of the endpoint ID.
            "arn:aws:ec2:us-east-1:123456789012:vpc-endpoint/vpce-0123456789abcdef0",
        ] {
            let error = validate_private_endpoint_id(id)
                .expect_err(&format!("{id} must be rejected, not accepted"));
            // The message names the offending value so the fix is obvious.
            assert!(
                error.contains(id),
                "error for {id} should quote the value: {error}"
            );
        }
    }

    #[test]
    fn parses_service_identity_lifecycle_and_prometheus_options() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ]);
        let crate::cloud::cli::ServiceCommands::Get { service_id } = command else {
            panic!("expected service get");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));

        for action in ["start", "wake", "stop"] {
            let command = parse_service(&[
                "clickhousectl",
                "cloud",
                "service",
                action,
                "svc-1",
                "--org-id",
                "org-1",
            ]);
            match command {
                crate::cloud::cli::ServiceCommands::Start { service_id }
                | crate::cloud::cli::ServiceCommands::Wake { service_id }
                | crate::cloud::cli::ServiceCommands::Stop { service_id } => {
                    assert_eq!(service_id.id.as_deref(), Some("svc-1"));
                }
                _ => panic!("expected service {action}"),
            }
        }

        let command = parse_service(&["clickhousectl", "cloud", "service", "prometheus", "svc-1"]);
        let crate::cloud::cli::ServiceCommands::Prometheus {
            filtered_metrics, ..
        } = command
        else {
            panic!("expected service prometheus");
        };

        assert!(filtered_metrics.is_none());

        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "prometheus",
            "svc-1",
            "--filtered-metrics",
            "true",
            "--org-id",
            "org-1",
        ]);
        let crate::cloud::cli::ServiceCommands::Prometheus {
            service_id,
            filtered_metrics,
        } = command
        else {
            panic!("expected service prometheus");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));

        assert_eq!(filtered_metrics, Some(true));
    }

    #[test]
    fn top_level_write_classification_covers_every_service_command() {
        assert_write(&["clickhousectl", "cloud", "service", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "service", "get", "svc-1"],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "profile",
                "list",
                "--region",
                "us-east-1",
            ],
            false,
        );
        for action in ["list", "schema"] {
            assert_write(
                &[
                    "clickhousectl",
                    "cloud",
                    "service",
                    "settings",
                    action,
                    "svc-1",
                ],
                false,
            );
        }
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "settings",
                "get",
                "svc-1",
                "compatibility",
            ],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "settings",
                "set",
                "svc-1",
                "--setting",
                "compatibility=\"24.8\"",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "settings",
                "unset",
                "svc-1",
                "compatibility",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "repair-query-key",
                "svc-1",
            ],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "service", "prometheus", "svc-1"],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "query",
                "--id",
                "svc-1",
                "--query",
                "SELECT 1",
            ],
            false,
        );
        assert_write(
            &["clickhousectl", "cloud", "service", "create", "--name", "s"],
            true,
        );
        for action in [
            "delete",
            "start",
            "wake",
            "stop",
            "update",
            "scale",
            "reset-password",
        ] {
            assert_write(
                &["clickhousectl", "cloud", "service", action, "svc-1"],
                true,
            );
        }
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
        for action in ["create", "delete"] {
            let mut args = vec![
                "clickhousectl",
                "cloud",
                "service",
                "query-endpoint",
                action,
                "svc-1",
            ];
            if action == "create" {
                args.extend(["--role", "sql_console_read_only"]);
            }
            assert_write(&args, true);
        }
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
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "service",
                "backup-config",
                "update",
                "svc-1",
                "--backup-period-hours",
                "24",
            ],
            true,
        );
    }

    #[test]
    fn backup_encryption_config_requires_backup_id() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "create",
            "--name",
            "svc",
            "--backup-encryption-config",
            "config.json",
        ])
        .err()
        .expect("--backup-encryption-config without --backup-id should fail");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn parses_service_settings_set_inputs() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "settings",
            "set",
            "svc-1",
            "--setting",
            "compatibility=\"24.8\"",
            "--setting",
            "enable_analyzer=1",
            "--org-id",
            "org-1",
        ]);
        let ServiceCommands::Settings {
            command:
                ServiceSettingsCommands::Set {
                    service_id,
                    setting,
                    settings_file,
                },
        } = command
        else {
            panic!("expected settings set");
        };
        assert_eq!(service_id.id.as_deref(), Some("svc-1"));
        assert_eq!(setting, ["compatibility=\"24.8\"", "enable_analyzer=1"]);
        assert!(settings_file.is_none());

        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "settings",
            "set",
            "svc-1",
            "--file",
            "-",
        ]);
        let ServiceCommands::Settings {
            command: ServiceSettingsCommands::Set { settings_file, .. },
        } = command
        else {
            panic!("expected settings set");
        };
        assert_eq!(settings_file.as_deref(), Some("-"));
    }

    #[test]
    fn service_settings_set_requires_exactly_one_input_kind() {
        let missing = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "settings",
            "set",
            "svc-1",
        ])
        .err()
        .expect("missing settings input should fail");
        assert_eq!(
            missing.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );

        let conflict = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "settings",
            "set",
            "svc-1",
            "--setting",
            "compatibility=\"24.8\"",
            "--file",
            "settings.json",
        ])
        .err()
        .expect("conflicting settings inputs should fail");
        assert_eq!(conflict.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}
