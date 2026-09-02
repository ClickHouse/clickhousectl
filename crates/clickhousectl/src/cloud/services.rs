use crate::cloud::api_keys::{cleanup_service_query_key, service_query_key_cleanup};
use crate::cloud::backups::BackupConfigCommands;
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::credentials;
use crate::cloud::output::{
    ABSENT, CloudErrorCode, CloudErrorDetail, eprint_line, or_absent, print_human, print_line,
};
use crate::cloud::shared::{parse_serde_enum, parse_tags, resolve_org_id};
use crate::cloud::types::DeleteResponse;
use crate::failure::{self, ApiFailure, FailureKind, FailureStage, ProvisioningState};
use clap::builder::PossibleValuesParser;
use clap::{ArgGroup, Subcommand};
use clickhouse_cloud_api::models::{
    AutoscalingMode, InstancePrivateEndpointsPatch, InstanceServiceQueryApiEndpointsPostRequest,
    InstanceTagsPatch, IpAccessListEntry, IpAccessListPatch, QueryEndpointRole,
    ServicPrivateEndpointePostRequest, Service, ServiceEndpoint, ServiceEndpointChange,
    ServiceEndpointChangeProtocol, ServiceEndpointProtocol, ServicePasswordPatchRequest,
    ServicePatchRequest, ServicePatchRequestReleasechannel, ServicePostRequest,
    ServicePostRequestCompliancetype, ServicePostRequestProfile, ServicePostRequestProvider,
    ServicePostRequestRegion, ServicePostRequestReleasechannel, ServiceReplicaScalingPatchRequest,
    ServiceState, ServiceStatePatchRequest, ServiceStatePatchRequestCommand,
};
#[cfg(test)]
use clickhouse_cloud_api::models::{IpAccessListEntryResponse, ResourceTagsV1Response};
use std::{collections::HashSet, io::IsTerminal, time::Duration};
use tabled::{Table, Tabled, settings::Style};

#[derive(Clone, Copy)]
struct QueryEndpointReadiness {
    timeout: Duration,
    initial_backoff: Duration,
    max_backoff: Duration,
}

/// Gap between `GET service` calls while `service delete --force` waits for a
/// stop to finish. A real stop takes minutes, so the loop can emit dozens of
/// progress lines — see [`crate::cloud::output::eprint_line`] for why none of
/// them may panic.
const STOP_POLL_INTERVAL: Duration = Duration::from_secs(5);

const QUERY_ENDPOINT_READINESS: QueryEndpointReadiness = QueryEndpointReadiness {
    timeout: Duration::from_secs(120),
    initial_backoff: Duration::from_millis(200),
    max_backoff: Duration::from_secs(5),
};

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
        #[arg(long = "add-private-endpoint-id", value_parser = parse_private_endpoint_id_arg)]
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
    #[command(after_help = "\
OUTPUT FORMAT:
  Output is always raw Prometheus exposition text, never JSON. --json is
  accepted for consistency with other commands but is silently ignored.")]
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
    #[command(
        group(ArgGroup::new("service_selector").required(true).args(["name", "id"])),
        after_help = "\
CONTEXT FOR AGENTS:
  Runs SQL over HTTP — no local clickhouse binary or service password required.
  With API key auth: first uses the authenticated key directly when the query
  endpoint already authorizes it. Otherwise, a per-service read+write key is
  auto-provisioned and stored in .clickhouse/credentials.json.
  A stored key rejected with HTTP 401/403 is never replaced automatically: the
  CLI reads the key's management record only to say why. A deleted, disabled,
  expired, unbound or IP-restricted key is reported with its key ID and left
  untouched, and nothing is changed. To replace only that credential
  deliberately, use `cloud service repair-query-key <service-id>`; it also drops
  a deleted key's stale endpoint binding.
  With OAuth (cloud auth login): sends your own bearer token — SQL runs as
  your cloud user with read-only access (SELECT only, no writes); no key
  provisioning and no query endpoint required on the service.
  The Query API times out after about 30 seconds; the statement keeps running
  on the service but the result is lost. For anything longer (large INSERTs,
  backfills, `url()` loads), run it over the native protocol instead:
  `clickhousectl local use latest` puts the standard `clickhouse` binary on
  PATH, then `clickhouse client --host <host> --secure --port 9440 --user
  default --password <password>`.
  SQL sources: --query and --queries-file are mutually exclusive. When neither
  is supplied, SQL is read from stdin. --query never reads stdin, so data
  redirected or piped alongside it is an error rather than a silent no-op:
  send the statement and its data together instead, e.g.
  printf 'INSERT INTO t FORMAT CSV\\n' | cat - data.csv | clickhousectl cloud
  service query --id <id>.
  Default format: PrettyCompact on a TTY, TabSeparated when piped. --json
  selects JSONEachRow and cannot be combined with --format; an explicit
  --format takes precedence over agent auto-JSON.
  The Query API runs exactly one statement per request, so multi-statement SQL
  (a typical ';'-separated .sql script) is rejected by the server. Run
  statements one invocation at a time, or connect with `clickhouse client`
  (see above) to run a script."
    )]
    Query {
        /// Service name to query (exactly one of --name or --id is required)
        #[arg(long, conflicts_with = "id")]
        name: Option<String>,

        /// Service ID to query
        #[arg(long, conflicts_with = "name")]
        id: Option<String>,

        /// Execute a SQL query (a single statement; the Query API does not
        /// accept multi-statement SQL). Does not read stdin: pipe a statement
        /// and its data together instead of redirecting a data file here
        #[arg(long, short, conflicts_with = "queries_file")]
        query: Option<String>,

        /// Execute a query from a SQL file (use "-" for stdin); the file must
        /// contain a single statement — multi-statement files are not
        /// supported
        #[arg(long, conflicts_with = "query")]
        queries_file: Option<String>,

        /// Target database
        #[arg(long)]
        database: Option<String>,

        /// Response format (e.g. JSONEachRow, CSV, TabSeparated, PrettyCompact)
        #[arg(long, conflicts_with = "json")]
        format: Option<String>,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,

        /// Fail instead of auto-provisioning when the authenticated API key
        /// cannot use the query endpoint and no key is stored locally (API
        /// key auth only; with OAuth this flag has no effect)
        #[arg(long)]
        no_auto_enable: bool,
    },

    /// Safely replace clickhousectl's stored Query API key for one service
    #[command(after_help = "\
Repairs only the stored credential for SERVICE_ID. The command verifies the
saved organization, exact management API key ID, and query endpoint ID; it
then replaces only that key's endpoint binding while preserving all other
bindings and project credentials. Legacy or non-owned records without this
metadata are refused. This is an explicit write operation and is never run
automatically after a query fails.")]
    RepairQueryKey {
        /// Service ID whose stored Query API key should be replaced
        service_id: String,

        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,
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
        #[arg(long, value_parser = PossibleValuesParser::new(QueryEndpointRole::VALUES))]
        role: Vec<String>,

        /// OpenAPI key IDs to authorize
        #[arg(long = "open-api-key")]
        open_api_key: Vec<String>,

        /// Allowed origins string for browser access (defaults to "*")
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

        /// Private endpoint ID (AWS VPC endpoint ID, GCP PSC connection ID, or
        /// Azure private endpoint Resource ID / resourceGuid)
        #[arg(long, value_parser = parse_private_endpoint_id_arg)]
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

impl ServiceCommands {
    pub fn is_write(&self) -> bool {
        match self {
            ServiceCommands::List { .. } => false,
            ServiceCommands::Get { .. } => false,
            ServiceCommands::Prometheus { .. } => false,
            ServiceCommands::Query { .. } => false,
            ServiceCommands::RepairQueryKey { .. } => true,
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
            ServiceCommands::BackupConfig { command } => command.is_write(),
        }
    }
}

pub async fn run(client: &CloudClient, command: ServiceCommands, json: bool) -> CloudResult<()> {
    match command {
        ServiceCommands::List { org_id, filter } => {
            service_list(client, org_id.as_deref(), &filter, json).await
        }
        ServiceCommands::Get { service_id, org_id } => {
            service_get(client, &service_id, org_id.as_deref(), json).await
        }
        ServiceCommands::Create {
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
            release_channel,
            data_warehouse_id,
            readonly,
            encryption_key,
            encryption_role,
            enable_tde,
            compliance_type,
            profile,
            tag,
            enable_endpoint,
            disable_endpoint,
            private_preview_terms_checked,
            enable_core_dumps,
            org_id,
        } => {
            let options = CreateServiceOptions {
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
                release_channel,
                data_warehouse_id,
                is_readonly: readonly,
                encryption_key,
                encryption_role,
                enable_tde,
                compliance_type,
                profile,
                tags: tag,
                enable_endpoints: enable_endpoint,
                disable_endpoints: disable_endpoint,
                private_preview_terms_checked,
                enable_core_dumps,
                org_id,
            };
            service_create(client, options, json).await
        }
        ServiceCommands::Delete {
            service_id,
            force,
            org_id,
        } => service_delete(client, &service_id, force, org_id.as_deref(), json).await,
        ServiceCommands::Start { service_id, org_id } => {
            service_start(client, &service_id, org_id.as_deref(), json).await
        }
        ServiceCommands::Stop { service_id, org_id } => {
            service_stop(client, &service_id, org_id.as_deref(), json).await
        }
        ServiceCommands::Update {
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
            org_id,
        } => {
            let options = ServiceUpdateOptions {
                name,
                add_ip_allow,
                remove_ip_allow,
                add_private_endpoint_ids: add_private_endpoint_id,
                remove_private_endpoint_ids: remove_private_endpoint_id,
                release_channel,
                enable_endpoints: enable_endpoint,
                disable_endpoints: disable_endpoint,
                transparent_data_encryption_key_id,
                add_tags: add_tag,
                remove_tags: remove_tag,
                enable_core_dumps,
                org_id,
            };
            service_update(client, &service_id, options, json).await
        }
        ServiceCommands::Scale {
            service_id,
            min_replica_memory_gb,
            max_replica_memory_gb,
            num_replicas,
            min_replicas,
            max_replicas,
            autoscaling_mode,
            idle_scaling,
            idle_timeout_minutes,
            org_id,
        } => {
            service_scale(
                client,
                &service_id,
                ServiceScaleOptions {
                    min_replica_memory_gb,
                    max_replica_memory_gb,
                    num_replicas,
                    min_replicas,
                    max_replicas,
                    autoscaling_mode,
                    idle_scaling,
                    idle_timeout_minutes,
                    org_id,
                },
                json,
            )
            .await
        }
        ServiceCommands::ResetPassword {
            service_id,
            new_password_hash,
            new_double_sha1_hash,
            org_id,
        } => {
            let options = ServiceResetPasswordOptions {
                new_password_hash,
                new_double_sha1_hash,
                org_id,
            };
            service_reset_password(client, &service_id, options, json).await
        }
        ServiceCommands::QueryEndpoint { command } => match command {
            QueryEndpointCommands::Get { service_id, org_id } => {
                query_endpoint_get(client, &service_id, org_id.as_deref(), json).await
            }
            QueryEndpointCommands::Create {
                service_id,
                role,
                open_api_key,
                allowed_origins,
                org_id,
            } => {
                let options = QueryEndpointCreateOptions {
                    roles: role,
                    open_api_keys: open_api_key,
                    allowed_origins,
                    org_id,
                };
                query_endpoint_create(client, &service_id, options, json).await
            }
            QueryEndpointCommands::Delete { service_id, org_id } => {
                query_endpoint_delete(client, &service_id, org_id.as_deref(), json).await
            }
        },
        ServiceCommands::PrivateEndpoint { command } => match command {
            PrivateEndpointCommands::Create {
                service_id,
                endpoint_id,
                description,
                org_id,
            } => {
                private_endpoint_create(
                    client,
                    &service_id,
                    &endpoint_id,
                    description.as_deref(),
                    org_id.as_deref(),
                    json,
                )
                .await
            }
            PrivateEndpointCommands::GetConfig { service_id, org_id } => {
                private_endpoint_get_config(client, &service_id, org_id.as_deref(), json).await
            }
        },
        ServiceCommands::BackupConfig { command } => {
            crate::cloud::backups::run_config(client, command, json).await
        }
        ServiceCommands::Prometheus {
            service_id,
            org_id,
            filtered_metrics,
        } => service_prometheus(client, &service_id, org_id.as_deref(), filtered_metrics).await,
        ServiceCommands::Query {
            name,
            id,
            query,
            queries_file,
            database,
            format,
            org_id,
            no_auto_enable,
        } => {
            let options = ServiceQueryOptions {
                name,
                id,
                query,
                queries_file,
                database,
                format,
                json,
                org_id,
                no_auto_enable,
            };
            service_query(client, options).await
        }
        ServiceCommands::RepairQueryKey { service_id, org_id } => {
            service_query_key_repair(client, &service_id, org_id.as_deref(), json).await
        }
    }
}

/// `host:port` of a service's first endpoint, for list tables.
///
/// Renders [`ABSENT`] when the API returned no endpoints, and keeps whichever
/// half of a partial endpoint it did return (`host` alone, or `-:port`).
fn first_endpoint(endpoints: Option<&[ServiceEndpoint]>) -> String {
    endpoints
        .and_then(|endpoints| endpoints.first())
        .map(|endpoint| match (endpoint.host.as_deref(), endpoint.port) {
            (Some(host), Some(port)) => format!("{host}:{port}"),
            (Some(host), None) => host.to_string(),
            (None, Some(port)) => format!("{ABSENT}:{port}"),
            (None, None) => ABSENT.to_string(),
        })
        .unwrap_or_else(|| ABSENT.to_string())
}

/// Resolve a service by name or ID within the given org.
/// Exactly one of `name` or `id` must be provided.
async fn resolve_service(
    client: &CloudClient,
    org_id: &str,
    name: Option<&str>,
    id: Option<&str>,
) -> CloudResult<Service> {
    match (name, id) {
        (Some(name), None) => {
            let services = client.list_services(org_id).await?;
            let matches: Vec<_> = services
                .into_iter()
                .filter(|service| service.name.as_deref() == Some(name))
                .collect();
            match matches.len() {
                0 => Err(CloudError::new(format!(
                    "no service found with name '{}'",
                    name
                ))),
                1 => Ok(matches.into_iter().next().unwrap()),
                count => Err(CloudError::new(format!(
                    "found {} services named '{}' — use --id to disambiguate",
                    count, name
                ))),
            }
        }
        (None, Some(id)) => Ok(client.get_service(org_id, id).await?),
        (Some(_), Some(_)) => Err(CloudError::new("specify either --name or --id, not both")),
        // Clap rejects this state; retain the check for any future non-CLI caller.
        (None, None) => Err(CloudError::new(
            "specify --name or --id to identify the service",
        )),
    }
}

fn parse_ip_access_entries(values: &[String]) -> Option<Vec<IpAccessListEntry>> {
    (!values.is_empty()).then(|| {
        values
            .iter()
            .map(|value| IpAccessListEntry {
                source: value.clone(),
                description: None,
            })
            .collect()
    })
}

fn parse_ip_access_list_patch(add: &[String], remove: &[String]) -> Option<IpAccessListPatch> {
    let patch = IpAccessListPatch {
        add: parse_ip_access_entries(add).unwrap_or_default(),
        remove: parse_ip_access_entries(remove).unwrap_or_default(),
    };

    (!patch.add.is_empty() || !patch.remove.is_empty()).then_some(patch)
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

fn parse_private_endpoint_ids_patch(
    add: &[String],
    remove: &[String],
) -> Option<InstancePrivateEndpointsPatch> {
    let patch = InstancePrivateEndpointsPatch {
        add: if add.is_empty() { vec![] } else { add.to_vec() },
        remove: if remove.is_empty() {
            vec![]
        } else {
            remove.to_vec()
        },
    };

    (!patch.add.is_empty() || !patch.remove.is_empty()).then_some(patch)
}

fn parse_service_endpoint_changes(
    enable: &[String],
    disable: &[String],
) -> CloudResult<Option<Vec<ServiceEndpointChange>>> {
    let mut changes = Vec::new();

    for protocol in enable {
        changes.push(ServiceEndpointChange {
            protocol: parse_serde_enum::<ServiceEndpointChangeProtocol>(
                protocol,
                "endpoint",
                &["mysql"],
            )?,
            enabled: true,
        });
    }

    for protocol in disable {
        changes.push(ServiceEndpointChange {
            protocol: parse_serde_enum::<ServiceEndpointChangeProtocol>(
                protocol,
                "endpoint",
                &["mysql"],
            )?,
            enabled: false,
        });
    }

    Ok((!changes.is_empty()).then_some(changes))
}

fn parse_instance_tags_patch(
    add: &[String],
    remove: &[String],
) -> CloudResult<Option<InstanceTagsPatch>> {
    let patch = InstanceTagsPatch {
        add: parse_tags(add)?.unwrap_or_default(),
        remove: parse_tags(remove)?.unwrap_or_default(),
    };

    Ok((!patch.add.is_empty() || !patch.remove.is_empty()).then_some(patch))
}

async fn service_list(
    client: &CloudClient,
    org_id: Option<&str>,
    filters: &[String],
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;

    let services = if filters.is_empty() {
        client.list_services(&org_id).await?
    } else {
        client.list_services_filtered(&org_id, filters).await?
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&services)?);
    } else {
        if services.is_empty() {
            println!("No services found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "State")]
            state: String,
            #[tabled(rename = "Provider")]
            provider: String,
            #[tabled(rename = "Region")]
            region: String,
            #[tabled(rename = "Endpoint")]
            endpoint: String,
        }
        let rows: Vec<Row> = services
            .into_iter()
            .map(|service| Row {
                name: or_absent(service.name.as_deref()),
                id: or_absent(service.id),
                state: or_absent(service.state.as_ref()),
                provider: or_absent(service.provider.as_ref()),
                region: or_absent(service.region.as_ref()),
                endpoint: first_endpoint(service.endpoints.as_deref()),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

async fn service_get(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let service = client.get_service(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        print_human(&service)?;
    }
    Ok(())
}

#[derive(Default)]
struct CreateServiceOptions {
    name: String,
    provider: String,
    region: String,
    min_replica_memory_gb: Option<u32>,
    max_replica_memory_gb: Option<u32>,
    num_replicas: Option<u32>,
    min_replicas: Option<u32>,
    max_replicas: Option<u32>,
    autoscaling_mode: Option<String>,
    idle_scaling: Option<bool>,
    idle_timeout_minutes: Option<u32>,
    ip_allow: Vec<String>,
    backup_id: Option<String>,
    release_channel: Option<String>,
    data_warehouse_id: Option<String>,
    is_readonly: bool,
    encryption_key: Option<String>,
    encryption_role: Option<String>,
    enable_tde: bool,
    compliance_type: Option<String>,
    profile: Option<String>,
    tags: Vec<String>,
    enable_endpoints: Vec<String>,
    disable_endpoints: Vec<String>,
    private_preview_terms_checked: bool,
    enable_core_dumps: Option<bool>,
    org_id: Option<String>,
}

#[derive(Default)]
struct ServiceUpdateOptions {
    name: Option<String>,
    add_ip_allow: Vec<String>,
    remove_ip_allow: Vec<String>,
    add_private_endpoint_ids: Vec<String>,
    remove_private_endpoint_ids: Vec<String>,
    release_channel: Option<String>,
    enable_endpoints: Vec<String>,
    disable_endpoints: Vec<String>,
    transparent_data_encryption_key_id: Option<String>,
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
    enable_core_dumps: Option<bool>,
    org_id: Option<String>,
}

#[derive(Default)]
struct ServiceResetPasswordOptions {
    new_password_hash: Option<String>,
    new_double_sha1_hash: Option<String>,
    org_id: Option<String>,
}

#[derive(Default)]
struct QueryEndpointCreateOptions {
    roles: Vec<String>,
    open_api_keys: Vec<String>,
    allowed_origins: Option<String>,
    org_id: Option<String>,
}

struct HorizontalAutoscaling {
    autoscaling_mode: Option<AutoscalingMode>,
    min_replicas: Option<i64>,
    max_replicas: Option<i64>,
}

fn resolve_horizontal_autoscaling(
    autoscaling_mode: Option<&str>,
    min_replicas: Option<u32>,
    max_replicas: Option<u32>,
) -> CloudResult<HorizontalAutoscaling> {
    if min_replicas.is_some() != max_replicas.is_some() {
        return Err(CloudError::new(
            "--min-replicas and --max-replicas must be specified together",
        ));
    }
    let autoscaling_mode = autoscaling_mode
        .map(|value| {
            parse_serde_enum::<AutoscalingMode>(value, "autoscaling_mode", AutoscalingMode::VALUES)
        })
        .transpose()?;
    Ok(HorizontalAutoscaling {
        autoscaling_mode,
        min_replicas: min_replicas.map(i64::from),
        max_replicas: max_replicas.map(i64::from),
    })
}

fn build_create_service_request(options: &CreateServiceOptions) -> CloudResult<ServicePostRequest> {
    let ip_access_list = if options.ip_allow.is_empty() {
        vec![IpAccessListEntry {
            source: "0.0.0.0/0".to_string(),
            description: Some("Allow all (created by clickhousectl)".to_string()),
        }]
    } else {
        parse_ip_access_entries(&options.ip_allow).unwrap_or_default()
    };

    let horizontal = resolve_horizontal_autoscaling(
        options.autoscaling_mode.as_deref(),
        options.min_replicas,
        options.max_replicas,
    )?;

    Ok(ServicePostRequest {
        name: options.name.clone(),
        provider: parse_serde_enum::<ServicePostRequestProvider>(
            &options.provider,
            "provider",
            ServicePostRequestProvider::VALUES,
        )?,
        region: parse_serde_enum::<ServicePostRequestRegion>(
            &options.region,
            "region",
            ServicePostRequestRegion::VALUES,
        )?,
        ip_access_list,
        min_replica_memory_gb: options.min_replica_memory_gb.map(f64::from),
        max_replica_memory_gb: options.max_replica_memory_gb.map(f64::from),
        num_replicas: options.num_replicas.map(i64::from),
        idle_scaling: options.idle_scaling,
        idle_timeout_minutes: options.idle_timeout_minutes.map(f64::from),
        backup_id: options
            .backup_id
            .as_deref()
            .map(uuid::Uuid::parse_str)
            .transpose()
            .map_err(|error| CloudError::new(format!("invalid backup_id: {}", error)))?,
        release_channel: match options.release_channel.as_deref() {
            Some(value) => Some(parse_serde_enum::<ServicePostRequestReleasechannel>(
                value,
                "release_channel",
                ServicePostRequestReleasechannel::VALUES,
            )?),
            None => None,
        },
        tags: parse_tags(&options.tags)?,
        data_warehouse_id: options.data_warehouse_id.clone(),
        is_readonly: if options.is_readonly {
            Some(true)
        } else {
            None
        },
        encryption_key: options.encryption_key.clone(),
        encryption_assumed_role_identifier: options.encryption_role.clone(),
        has_transparent_data_encryption: if options.enable_tde { Some(true) } else { None },
        compliance_type: match options.compliance_type.as_deref() {
            Some(value) => Some(parse_serde_enum::<ServicePostRequestCompliancetype>(
                value,
                "compliance_type",
                ServicePostRequestCompliancetype::VALUES,
            )?),
            None => None,
        },
        profile: match options.profile.as_deref() {
            Some(value) => Some(parse_serde_enum::<ServicePostRequestProfile>(
                value,
                "profile",
                ServicePostRequestProfile::VALUES,
            )?),
            None => None,
        },
        private_preview_terms_checked: if options.private_preview_terms_checked {
            Some(true)
        } else {
            None
        },
        endpoints: parse_service_endpoint_changes(
            &options.enable_endpoints,
            &options.disable_endpoints,
        )?,
        enable_core_dumps: options.enable_core_dumps,
        autoscaling_mode: horizontal.autoscaling_mode,
        byoc_id: None,
        min_replicas: horizontal.min_replicas,
        max_replicas: horizontal.max_replicas,
        #[cfg(feature = "deprecated-fields")]
        max_total_memory_gb: None,
        #[cfg(feature = "deprecated-fields")]
        min_total_memory_gb: None,
        #[cfg(feature = "deprecated-fields")]
        private_endpoint_ids: None,
        #[cfg(feature = "deprecated-fields")]
        tier: None,
    })
}

fn build_update_service_request(
    options: &ServiceUpdateOptions,
) -> CloudResult<ServicePatchRequest> {
    Ok(ServicePatchRequest {
        name: options.name.clone(),
        ip_access_list: parse_ip_access_list_patch(&options.add_ip_allow, &options.remove_ip_allow),
        private_endpoint_ids: parse_private_endpoint_ids_patch(
            &options.add_private_endpoint_ids,
            &options.remove_private_endpoint_ids,
        ),
        release_channel: options
            .release_channel
            .as_deref()
            .map(|value| {
                parse_serde_enum::<ServicePatchRequestReleasechannel>(
                    value,
                    "release_channel",
                    ServicePatchRequestReleasechannel::VALUES,
                )
            })
            .transpose()?,
        endpoints: parse_service_endpoint_changes(
            &options.enable_endpoints,
            &options.disable_endpoints,
        )?,
        transparent_data_encryption_key_id: options.transparent_data_encryption_key_id.clone(),
        tags: parse_instance_tags_patch(&options.add_tags, &options.remove_tags)?,
        enable_core_dumps: options.enable_core_dumps,
    })
}

/// Whether `options` requests removing anything from a service, so callers
/// can decide whether a pre-update `GET` is needed to check the removals
/// against the current state.
fn has_removals(options: &ServiceUpdateOptions) -> bool {
    !options.remove_ip_allow.is_empty()
        || !options.remove_private_endpoint_ids.is_empty()
        || !options.remove_tags.is_empty()
}

/// Requested values that do not match anything in `current` (by exact string
/// equality). The service update API silently no-ops a removal that matches
/// nothing, which is idempotent but can hide a typo, so the caller surfaces
/// each unmatched entry as a warning instead of failing.
fn unmatched<'a>(requested: &'a [String], current: &HashSet<&str>) -> Vec<&'a str> {
    requested
        .iter()
        .map(String::as_str)
        .filter(|value| !current.contains(value))
        .collect()
}

/// Human-readable warnings for every `--remove-*` value on `options` that
/// does not match anything in `current`.
fn unmatched_removal_warnings(options: &ServiceUpdateOptions, current: &Service) -> Vec<String> {
    let mut warnings = Vec::new();

    let current_ip_allow: HashSet<&str> = current
        .ip_access_list
        .iter()
        .flatten()
        .filter_map(|entry| entry.source.as_deref())
        .collect();
    for value in unmatched(&options.remove_ip_allow, &current_ip_allow) {
        warnings.push(format!(
            "--remove-ip-allow {value} did not match any entry in the service's current IP allow list; no entry was removed"
        ));
    }

    let current_private_endpoint_ids: HashSet<&str> = current
        .private_endpoint_ids
        .iter()
        .flatten()
        .map(String::as_str)
        .collect();
    for value in unmatched(
        &options.remove_private_endpoint_ids,
        &current_private_endpoint_ids,
    ) {
        warnings.push(format!(
            "--remove-private-endpoint-id {value} did not match any private endpoint on the service; nothing was removed"
        ));
    }

    let current_tags: HashSet<&str> = current
        .tags
        .iter()
        .flatten()
        .filter_map(|tag| tag.key.as_deref())
        .collect();
    let remove_tag_keys: Vec<String> = options
        .remove_tags
        .iter()
        .map(|value| tag_key(value).to_string())
        .collect();
    for value in unmatched(&remove_tag_keys, &current_tags) {
        warnings.push(format!(
            "--remove-tag {value} did not match any tag on the service; nothing was removed"
        ));
    }

    warnings
}

/// The key portion of a `key` or `key=value` tag argument, matching how
/// [`crate::cloud::shared::parse_tag`] splits removal/addition arguments.
fn tag_key(value: &str) -> &str {
    value.split_once('=').map_or(value, |(key, _)| key).trim()
}

fn build_service_password_patch_request(
    options: &ServiceResetPasswordOptions,
) -> ServicePasswordPatchRequest {
    ServicePasswordPatchRequest {
        new_password_hash: options.new_password_hash.clone(),
        new_double_sha1_hash: options.new_double_sha1_hash.clone(),
    }
}

fn build_query_endpoint_create_request(
    options: &QueryEndpointCreateOptions,
) -> CloudResult<InstanceServiceQueryApiEndpointsPostRequest> {
    Ok(InstanceServiceQueryApiEndpointsPostRequest {
        roles: options
            .roles
            .iter()
            .map(|role| parse_serde_enum(role, "role", QueryEndpointRole::VALUES))
            .collect::<Result<_, _>>()?,
        open_api_keys: options.open_api_keys.clone(),
        allowed_origins: options
            .allowed_origins
            .clone()
            .unwrap_or_else(|| "*".to_string()),
    })
}

fn build_private_endpoint_create_request(
    endpoint_id: &str,
    description: Option<&str>,
) -> ServicPrivateEndpointePostRequest {
    ServicPrivateEndpointePostRequest {
        id: endpoint_id.to_string(),
        description: description.map(String::from).unwrap_or_default(),
    }
}

fn build_service_state_patch_request(
    command: ServiceStatePatchRequestCommand,
) -> ServiceStatePatchRequest {
    ServiceStatePatchRequest {
        command: Some(command),
    }
}

fn service_query_hint(service_id: Option<uuid::Uuid>) -> Option<String> {
    service_id.map(|id| {
        format!(
            "Run SQL with: clickhousectl cloud service query --id {} --query \"SELECT 1\"\n\
             (the Query API endpoint is provisioned automatically on first use)",
            id
        )
    })
}

fn service_credentials_block(password: Option<&str>, service_id: Option<uuid::Uuid>) -> String {
    match (password, service_id) {
        (Some(password), _) => format!(
            "Credentials (save these, password shown only once):\n  Username: default\n  \
             Password: {}",
            password
        ),
        (None, Some(id)) => format!(
            "WARNING: the API response omitted the one-time password, so it cannot be shown.\n\
             The service was created; reset the password to get a usable credential:\n  \
             clickhousectl cloud service reset-password {}",
            id
        ),
        (None, None) => "WARNING: the API response omitted the one-time password, so it cannot be \
                         shown.\nThe service was created; once you have its id, reset the password \
                         with `clickhousectl cloud service reset-password <service-id>` to get a \
                         usable credential."
            .to_string(),
    }
}

async fn service_create(
    client: &CloudClient,
    options: CreateServiceOptions,
    json: bool,
) -> CloudResult<()> {
    let request = build_create_service_request(&options)?;
    let org_id = resolve_org_id(client, options.org_id.as_deref()).await?;
    let response = client.create_service(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        let service_id = response.service.as_ref().and_then(|service| service.id);
        println!("Service created successfully!");
        println!();
        if let Some(service) = &response.service {
            print_human(service)?;
        }
        println!();
        println!(
            "{}",
            service_credentials_block(response.password.as_deref(), service_id)
        );
        if let Some(hint) = service_query_hint(service_id) {
            println!();
            println!("{}", hint);
        }
    }
    Ok(())
}

fn classify_stop_poll_state(state: Option<&ServiceState>) -> CloudResult<bool> {
    let state = state
        .ok_or_else(|| {
            CloudError::new(
                "the API response omitted the service state while waiting for the service to stop, \
             so the stop cannot be confirmed",
            )
        })?
        .to_string();
    if matches!(state.as_str(), "stopped" | "idle") {
        return Ok(true);
    }
    if matches!(state.as_str(), "terminated" | "failed" | "deleted") {
        return Err(CloudError::new(format!(
            "service entered unexpected state '{}' while waiting for stop",
            state
        )));
    }
    Ok(false)
}

#[derive(Default)]
struct StopPollProgress {
    previous_state: Option<String>,
}

impl StopPollProgress {
    fn render(&mut self, state: &str, verbose: bool) -> Option<String> {
        let changed = self.previous_state.as_deref() != Some(state);
        self.previous_state = Some(state.to_string());
        (verbose || changed).then(|| format!("  state: {state}"))
    }
}

fn service_delete_error(error: CloudError, force: bool, service_id: &str) -> CloudError {
    if !force
        && error.message.starts_with("CONFLICT:")
        && error.message.contains("Current state: 'running'")
    {
        CloudError::new(format!(
            "service is running and cannot be deleted. Use --force to stop it first, or \
             `clickhousectl cloud service stop {service_id}`."
        ))
    } else {
        error
    }
}

async fn service_delete(
    client: &CloudClient,
    service_id: &str,
    force: bool,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let (query_key_ids, retain_query_key) = service_query_key_cleanup(&org_id, service_id)?;

    if force {
        let service = client.get_service_if_exists(&org_id, service_id).await?;
        let state = service
            .as_ref()
            .map(|service| or_absent(service.state.as_ref()))
            .unwrap_or_default();
        if matches!(state.as_str(), "running" | "idle" | "starting") {
            // Every write below goes through `eprint_line`/`print_line`, never
            // `eprintln!`/`println!`: this loop streams progress for as long as
            // the stop takes (minutes), so it routinely outlives whatever was
            // reading its output, and a panicking write turned a successful
            // delete into exit 101 (#598).
            eprint_line(format!(
                "Stopping service {} before deletion...",
                service_id
            ));
            client
                .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Stop)
                .await?;

            let verbose_polling =
                std::io::stderr().is_terminal() && !json && std::env::var_os("CI").is_none();
            let mut progress = StopPollProgress::default();
            loop {
                tokio::time::sleep(STOP_POLL_INTERVAL).await;
                let service = client.get_service(&org_id, service_id).await?;
                let state = or_absent(service.state.as_ref());
                if let Some(line) = progress.render(&state, verbose_polling) {
                    eprint_line(line);
                }
                if classify_stop_poll_state(service.state.as_ref())? {
                    break;
                }
            }
        }
    }

    let response = client
        .delete_service(&org_id, service_id)
        .await
        .map_err(|error| service_delete_error(error, force, service_id))?;
    cleanup_service_query_key(client, &org_id, service_id, &query_key_ids).await?;
    if !retain_query_key {
        credentials::remove_service_query_key(service_id)?;
    }
    // The service is already gone by here, so a closed stdout must not turn a
    // completed deletion into a panic — see `print_line` and #598.
    if json {
        print_line(serde_json::to_string_pretty(&response)?);
    } else {
        print_line(format!("Service {} deletion initiated", service_id));
    }
    Ok(())
}

async fn service_start(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let service = client
        .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Start)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        println!(
            "Service {} starting (state: {})",
            or_absent(service.name.as_deref()),
            or_absent(service.state.as_ref())
        );
    }
    Ok(())
}

async fn service_stop(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let service = client
        .change_service_state(&org_id, service_id, ServiceStatePatchRequestCommand::Stop)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        println!(
            "Service {} stopping (state: {})",
            or_absent(service.name.as_deref()),
            or_absent(service.state.as_ref())
        );
    }
    Ok(())
}

async fn service_update(
    client: &CloudClient,
    service_id: &str,
    options: ServiceUpdateOptions,
    json: bool,
) -> CloudResult<()> {
    let request = build_update_service_request(&options)?;
    let org_id = resolve_org_id(client, options.org_id.as_deref()).await?;

    if has_removals(&options) {
        let current = client.get_service(&org_id, service_id).await?;
        for warning in unmatched_removal_warnings(&options, &current) {
            eprint_line(format!("Warning: {warning}"));
        }
    }

    let service = client.update_service(&org_id, service_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        println!("Service {} updated", or_absent(service.name.as_deref()));
        println!("  ID: {}", or_absent(service.id));
        println!("  State: {}", or_absent(service.state.as_ref()));
    }
    Ok(())
}

#[derive(Default)]
struct ServiceScaleOptions {
    min_replica_memory_gb: Option<u32>,
    max_replica_memory_gb: Option<u32>,
    num_replicas: Option<u32>,
    min_replicas: Option<u32>,
    max_replicas: Option<u32>,
    autoscaling_mode: Option<String>,
    idle_scaling: Option<bool>,
    idle_timeout_minutes: Option<u32>,
    org_id: Option<String>,
}

fn build_service_scale_request(
    options: &ServiceScaleOptions,
) -> CloudResult<ServiceReplicaScalingPatchRequest> {
    let horizontal = resolve_horizontal_autoscaling(
        options.autoscaling_mode.as_deref(),
        options.min_replicas,
        options.max_replicas,
    )?;

    Ok(ServiceReplicaScalingPatchRequest {
        autoscaling_mode: horizontal.autoscaling_mode,
        min_replica_memory_gb: options.min_replica_memory_gb.map(f64::from),
        max_replica_memory_gb: options.max_replica_memory_gb.map(f64::from),
        min_replicas: horizontal.min_replicas,
        max_replicas: horizontal.max_replicas,
        num_replicas: options.num_replicas.map(i64::from),
        idle_scaling: options.idle_scaling,
        idle_timeout_minutes: options.idle_timeout_minutes.map(f64::from),
    })
}

async fn service_scale(
    client: &CloudClient,
    service_id: &str,
    options: ServiceScaleOptions,
    json: bool,
) -> CloudResult<()> {
    let request = build_service_scale_request(&options)?;
    let org_id = resolve_org_id(client, options.org_id.as_deref()).await?;
    let service = client
        .update_replica_scaling(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&service)?);
    } else {
        println!(
            "Service {} scaling updated",
            or_absent(service.name.as_deref())
        );
        println!(
            "  Autoscaling Mode: {}",
            or_absent(service.autoscaling_mode.as_ref())
        );
        match service.autoscaling_mode {
            Some(AutoscalingMode::Horizontal) => {
                println!("  Min Replicas: {}", or_absent(service.min_replicas));
                println!("  Max Replicas: {}", or_absent(service.max_replicas));
                println!(
                    "  Memory/Replica: {} GB",
                    or_absent(service.replica_memory_gb)
                );
            }
            Some(AutoscalingMode::Vertical) => {
                println!(
                    "  Min Memory/Replica: {} GB",
                    or_absent(service.min_replica_memory_gb)
                );
                println!(
                    "  Max Memory/Replica: {} GB",
                    or_absent(service.max_replica_memory_gb)
                );
                println!("  Replicas: {}", or_absent(service.num_replicas));
            }
            _ => {}
        }
    }
    Ok(())
}

async fn service_reset_password(
    client: &CloudClient,
    service_id: &str,
    options: ServiceResetPasswordOptions,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, options.org_id.as_deref()).await?;
    let request = build_service_password_patch_request(&options);
    let response = client.reset_password(&org_id, service_id, &request).await?;
    let outcome = resolve_reset_password_outcome(
        generation_requested(&request),
        response.password.as_deref(),
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Password reset for service {}", service_id);
        match outcome {
            ResetPasswordOutcome::Generated(password) => {
                println!("  New password: {}", password)
            }
            ResetPasswordOutcome::HashUpdated => {
                println!("  Password hash updated; no plaintext password returned")
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
enum ResetPasswordOutcome<'a> {
    Generated(&'a str),
    HashUpdated,
}

fn generation_requested(request: &ServicePasswordPatchRequest) -> bool {
    request.new_password_hash.is_none()
}

fn resolve_reset_password_outcome(
    generation_requested: bool,
    password: Option<&str>,
) -> CloudResult<ResetPasswordOutcome<'_>> {
    if !generation_requested {
        return Ok(ResetPasswordOutcome::HashUpdated);
    }
    match password {
        Some(password) => Ok(ResetPasswordOutcome::Generated(password)),
        None => Err(CloudError::new(
            "the API response omitted the generated password, so it cannot be shown: the \
             service password may already have been rotated — run the reset again to get \
             a password you can use",
        )),
    }
}

async fn query_endpoint_get(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let endpoint = client.get_query_endpoint(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        print_human(&endpoint)?;
    }
    Ok(())
}

async fn query_endpoint_create(
    client: &CloudClient,
    service_id: &str,
    options: QueryEndpointCreateOptions,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, options.org_id.as_deref()).await?;
    let request = build_query_endpoint_create_request(&options)?;
    let endpoint = client
        .create_query_endpoint(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        println!("Query endpoint created for service {}", service_id);
        println!("  ID: {}", or_absent(endpoint.id.as_deref()));
        println!(
            "  Roles: {}",
            or_absent(endpoint.roles.as_ref().map(|roles| {
                roles
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            }))
        );
    }
    Ok(())
}

async fn query_endpoint_delete(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let response = client.delete_query_endpoint(&org_id, service_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Query endpoint deleted for service {}", service_id);
    }
    Ok(())
}

async fn service_query_key_repair(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let result =
        crate::cloud::service_query::repair_service_query_key(client, &org_id, service_id).await?;
    // A retired key that could not be deleted is a warning, not a failure: the
    // replacement is active and its ID is stored for the retry (#527). The
    // warning goes to stderr in both modes; the JSON result carries the IDs.
    if let Some(warning) = &result.cleanup_warning {
        eprint_line(warning);
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        match result.status {
            "already_repaired" => println!(
                "Query key for service {} was already repaired by another process (active API key: {})",
                result.service_id, result.api_key_id
            ),
            _ => {
                println!("Query key repaired for service {}", result.service_id);
                println!(
                    "  Replaced API key: {}",
                    or_absent(result.replaced_api_key_id.as_deref())
                );
                println!("  New API key: {}", result.api_key_id);
                println!("  Query endpoint: {}", result.endpoint_id);
                if !result.deleted_api_key_ids.is_empty() {
                    println!(
                        "  Deleted API keys: {}",
                        result.deleted_api_key_ids.join(", ")
                    );
                }
                if !result.pending_cleanup_api_key_ids.is_empty() {
                    println!(
                        "  API keys awaiting deletion: {}",
                        result.pending_cleanup_api_key_ids.join(", ")
                    );
                }
            }
        }
    }
    Ok(())
}

struct ServiceQueryOptions {
    name: Option<String>,
    id: Option<String>,
    query: Option<String>,
    queries_file: Option<String>,
    database: Option<String>,
    format: Option<String>,
    json: bool,
    org_id: Option<String>,
    no_auto_enable: bool,
}

#[allow(clippy::too_many_arguments)]
async fn run_basic_service_query(
    client: &CloudClient,
    service_id: &str,
    key_id: &str,
    key_secret: &str,
    sql: &str,
    database: Option<&str>,
    format: &str,
    service_name: &str,
    confirmed_idle: bool,
) -> Result<reqwest::Response, clickhouse_cloud_api::Error> {
    let run = |wake: bool| {
        client
            .api()
            .run_query(service_id, key_id, key_secret, sql, database, format, wake)
    };
    if confirmed_idle {
        eprint_waking_service(service_name);
        failure::note_retry();
        return run(true).await;
    }

    match run(false).await {
        Err(clickhouse_cloud_api::Error::ServiceIdle) => {
            eprint_waking_service(service_name);
            failure::note_retry();
            run(true).await
        }
        other => other,
    }
}

/// Why the readiness wait ended without the endpoint accepting a probe.
///
/// The deadline is the CLI's own: no HTTP response stands behind it, so it is
/// a variant of its own rather than a synthesized `Error::Api { status: 408 }`
/// — telemetry reports `http_status` as the exact status the server sent, and
/// a made-up one would be a lie there (#450).
#[derive(Debug)]
enum QueryReadinessError {
    /// The endpoint was still rejecting probes when the deadline elapsed.
    TimedOut(Duration),
    /// A probe failed for a reason other than "not ready yet".
    Api(clickhouse_cloud_api::Error),
}

fn query_readiness_timeout_error(timeout: Duration) -> CloudError {
    CloudError::new(format!(
        "Query API endpoint did not become ready within {timeout:?}"
    ))
    .with_failure(ApiFailure::new(FailureKind::Timeout))
}

async fn wait_for_query_endpoint_readiness<T, F, Fut>(
    readiness: QueryEndpointReadiness,
    mut probe: F,
) -> Result<bool, QueryReadinessError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, clickhouse_cloud_api::Error>>,
{
    let deadline = tokio::time::Instant::now() + readiness.timeout;
    let mut backoff = readiness.initial_backoff;
    let mut waiting = false;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(QueryReadinessError::TimedOut(readiness.timeout));
        }

        match tokio::time::timeout(remaining, probe()).await {
            Ok(Ok(_)) => return Ok(false),
            Ok(Err(clickhouse_cloud_api::Error::ServiceIdle)) => return Ok(true),
            Ok(Err(error)) if query_endpoint_readiness_error(&error) => {}
            Ok(Err(error)) => return Err(QueryReadinessError::Api(error)),
            Err(_) => return Err(QueryReadinessError::TimedOut(readiness.timeout)),
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(QueryReadinessError::TimedOut(readiness.timeout));
        }
        if !waiting {
            eprintln!("Waiting for the Query API endpoint to become ready...");
            waiting = true;
        }
        // One more probe is about to be sent: the retry bucket is how a
        // dashboard sees "this run waited on propagation" (#450).
        failure::note_retry();
        tokio::time::sleep(backoff.min(remaining)).await;
        backoff = backoff.saturating_mul(2).min(readiness.max_backoff);
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_just_provisioned_service_query(
    client: &CloudClient,
    key_id: &str,
    key_secret: &str,
    sql: &str,
    database: Option<&str>,
    format: &str,
    target: QueryTarget<'_>,
    readiness: QueryEndpointReadiness,
) -> CloudResult<reqwest::Response> {
    let confirmed_idle = wait_for_query_endpoint_readiness(readiness, || {
        client.api().run_query(
            target.service_id,
            key_id,
            key_secret,
            "SELECT 1",
            None,
            "TabSeparated",
            false,
        )
    })
    .await
    .map_err(|error| match error {
        QueryReadinessError::TimedOut(timeout) => query_readiness_timeout_error(timeout),
        QueryReadinessError::Api(error) => convert_query_error(client, error, target),
    })?;

    run_basic_service_query(
        client,
        target.service_id,
        key_id,
        key_secret,
        sql,
        database,
        format,
        target.service_name,
        confirmed_idle,
    )
    .await
    .map_err(|error| convert_query_error(client, error, target))
}

/// Whether an error means "the Query API endpoint is not (yet) usable by this
/// credential" rather than "the query itself failed".
///
/// The test is structural: a SQL-level rejection arrives as
/// `Error::Sql`, which proves the service ran the request, so the endpoint is
/// demonstrably usable. It used to be told apart by sniffing the formatted
/// message for a `SQL error ` prefix (#450).
fn query_endpoint_readiness_error(error: &clickhouse_cloud_api::Error) -> bool {
    matches!(
        error,
        clickhouse_cloud_api::Error::Api {
            status: 401 | 403 | 404,
            ..
        }
    )
}

fn stored_query_key_rejection_status(error: &clickhouse_cloud_api::Error) -> Option<u16> {
    match error {
        clickhouse_cloud_api::Error::Api {
            status: status @ (401 | 403),
            ..
        } => Some(*status),
        _ => None,
    }
}

/// The `--no-auto-enable` refusal: the Query API rejected the management key
/// (401/403/404) and provisioning would have fixed that, but the caller forbade
/// it. The message is the CLI's; the classification stays the API's actual
/// rejection, carried across from the variant rather than reset to `other`
/// (#450).
fn refused_query_provisioning_error(
    service_id: &str,
    rejection: &clickhouse_cloud_api::Error,
) -> CloudError {
    CloudError::new(format!(
        "the authenticated API key cannot use the Query API endpoint for service {service_id}, \
         and --no-auto-enable prevents provisioning"
    ))
    .with_failure(failure::classify_api_error(rejection))
}

async fn service_query(client: &CloudClient, options: ServiceQueryOptions) -> CloudResult<()> {
    // Anchor the duration bucket of any failure classification (#450) at the
    // start of the run rather than at process start, so it measures the query
    // and not the CLI's own startup.
    failure::start_span();

    let sql = read_query_sql(options.query.as_deref(), options.queries_file.as_deref())
        .map_err(|error| error.at_stage(FailureStage::SqlInput))?;
    let org_id = resolve_org_id(client, options.org_id.as_deref())
        .await
        .map_err(|error| error.at_stage(FailureStage::OrgResolution))?;
    let service = resolve_service(
        client,
        &org_id,
        options.name.as_deref(),
        options.id.as_deref(),
    )
    .await
    .map_err(|error| error.at_stage(FailureStage::ServiceResolution))?;
    let service_id = match service.id {
        Some(id) => id.to_string(),
        None => options.id.clone().ok_or_else(|| {
            CloudError::new("the API response is missing the service id")
                .at_stage(FailureStage::ServiceResolution)
        })?,
    };
    let service_name = or_absent(service.name.as_deref());
    // Resolved once, from the `GET service` response the query already needed:
    // the gateway-timeout hint spells out a `clickhouse client` command, and
    // it must use the service's real native endpoint when the API returned
    // one (#644).
    let native = native_secure_endpoint(service.endpoints.as_deref());
    let target = QueryTarget {
        service_name: &service_name,
        service_id: &service_id,
        org_id: &org_id,
        native: native.as_ref(),
    };

    let format = options.format.unwrap_or_else(|| {
        if options.json {
            "JSONEachRow".to_string()
        } else {
            default_query_format()
        }
    });

    // The `query_request` classifications below are a *fallback*: recording
    // is first-write-wins, so an inner boundary that knows the exact stage
    // (`key_create`, `endpoint_get`, ...) keeps its record and only failures
    // no boundary claimed -- an unusable local credential store, say -- land
    // on the coarse `query_request` stage rather than going unclassified
    // (#450).
    let response = if client.is_bearer_auth() {
        // OAuth carries no write access, so no key or endpoint provisioning is
        // possible on this path at all.
        failure::set_provisioning_state(ProvisioningState::Bearer);
        let run = |wake: bool| {
            client.api().run_query_bearer(
                &service_id,
                &sql,
                options.database.as_deref(),
                &format,
                wake,
            )
        };
        let result = match run(false).await {
            Err(clickhouse_cloud_api::Error::ServiceIdle) => {
                eprint_waking_service(&service_name);
                failure::note_retry();
                run(true).await
            }
            other => other,
        };
        result.map_err(|error| {
            convert_query_error(client, error, target).at_stage(FailureStage::QueryRequest)
        })?
    } else {
        let convert =
            |error: clickhouse_cloud_api::Error| convert_query_error(client, error, target);
        let result = if let Some(key) = credentials::try_get_service_query_key(&service_id)
            .map_err(|error| error.at_stage(FailureStage::QueryRequest))?
        {
            failure::set_provisioning_state(ProvisioningState::StoredKey);
            // A repair that could not delete a superseded key left its ID on
            // the record; finish that here, quietly, before the query (#527).
            // The list is empty on every ordinary run, so this costs nothing
            // unless a retirement is actually outstanding.
            crate::cloud::service_query::retry_pending_query_key_cleanup(
                client,
                &key,
                &service_id,
                &org_id,
            )
            .await;
            match run_basic_service_query(
                client,
                &service_id,
                &key.key_id,
                &key.key_secret,
                &sql,
                options.database.as_deref(),
                &format,
                &service_name,
                false,
            )
            .await
            {
                Err(error) => match stored_query_key_rejection_status(&error) {
                    // The 401/403 alone does not say whether the key is stale
                    // or was deliberately revoked; the management record
                    // decides, and nothing is replaced automatically (#528).
                    Some(status) => {
                        return Err(
                            crate::cloud::service_query::rejected_stored_query_key_error(
                                client,
                                &key,
                                &service_id,
                                &org_id,
                                status,
                            )
                            .await
                            .at_stage(FailureStage::QueryRequest),
                        );
                    }
                    None => Err(convert(error)),
                },
                Ok(response) => Ok(response),
            }
        } else {
            failure::set_provisioning_state(ProvisioningState::ManagementKey);
            let (key_id, key_secret) = client.basic_auth_credentials().ok_or_else(|| {
                CloudError::new("API key credentials are unavailable")
                    .at_stage(FailureStage::QueryRequest)
            })?;
            match run_basic_service_query(
                client,
                &service_id,
                key_id,
                key_secret,
                &sql,
                options.database.as_deref(),
                &format,
                &service_name,
                false,
            )
            .await
            {
                Err(error) if query_endpoint_readiness_error(&error) => {
                    if options.no_auto_enable {
                        failure::set_provisioning_state(ProvisioningState::Refused);
                        return Err(refused_query_provisioning_error(&service_id, &error)
                            .at_stage(FailureStage::QueryRequest));
                    }
                    eprintln!(
                        "Provisioning Query API endpoint + key for service '{}'...",
                        service_name
                    );
                    failure::set_provisioning_state(ProvisioningState::Provisioning);
                    let key = crate::cloud::service_query::ensure_service_query_setup(
                        client,
                        &org_id,
                        &service_id,
                        &service_name,
                    )
                    .await
                    .map_err(|error| error.at_stage(FailureStage::QueryRequest))?;
                    failure::set_provisioning_state(ProvisioningState::Provisioned);
                    run_just_provisioned_service_query(
                        client,
                        &key.key_id,
                        &key.key_secret,
                        &sql,
                        options.database.as_deref(),
                        &format,
                        target,
                        QUERY_ENDPOINT_READINESS,
                    )
                    .await
                }
                other => other.map_err(convert),
            }
        };
        result.map_err(|error| error.at_stage(FailureStage::QueryRequest))?
    };

    use futures_util::StreamExt;
    use std::io::Write as _;
    let mut stream = response.bytes_stream();
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let mut byte_count = 0;
    let mut last_byte = None;
    // Both failure modes of the stream are classified here, at the boundary
    // that knows which one happened (#450): a truncated body is a transport
    // failure, a refused write is local I/O.
    let stream_failure = |error: CloudError| error.at_stage(FailureStage::ResponseStream);
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|error| {
            stream_failure(
                CloudError::new(format!("Failed to read query response: {error}"))
                    .with_failure(ApiFailure::new(FailureKind::Transport)),
            )
        })?;
        handle
            .write_all(&bytes)
            .map_err(|error| stream_failure(CloudError::from(error)))?;
        byte_count += bytes.len();
        if let Some(last) = bytes.last() {
            last_byte = Some(*last);
        }
    }
    match query_output_completion(&format, byte_count, last_byte) {
        QueryOutputCompletion::None => {}
        QueryOutputCompletion::Newline => handle
            .write_all(b"\n")
            .map_err(|error| stream_failure(CloudError::from(error)))?,
        QueryOutputCompletion::Acknowledge => eprintln!("OK"),
    }
    handle
        .flush()
        .map_err(|error| stream_failure(CloudError::from(error)))?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum QueryOutputCompletion {
    None,
    Newline,
    Acknowledge,
}

fn query_output_completion(
    format: &str,
    byte_count: usize,
    last_byte: Option<u8>,
) -> QueryOutputCompletion {
    if byte_count == 0 {
        return QueryOutputCompletion::Acknowledge;
    }
    if query_format_uses_text_lines(format) && last_byte != Some(b'\n') {
        QueryOutputCompletion::Newline
    } else {
        QueryOutputCompletion::None
    }
}

fn query_format_uses_text_lines(format: &str) -> bool {
    matches!(
        format.to_ascii_lowercase().as_str(),
        "jsoneachrow" | "prettycompact" | "tabseparated"
    )
}

fn eprint_waking_service(service_name: &str) {
    eprintln!("Service '{service_name}' is idle; waking it (this may take a minute)...");
}

/// A service's native-protocol (`nativesecure`) endpoint.
///
/// Both halves are required. A hint that named the real host but guessed the
/// port would hand the user a command that does not connect, which is worse
/// than a command that visibly needs filling in.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeEndpoint {
    host: String,
    port: i64,
}

/// What the query path knows about its target service when it has to render
/// an error: enough to name the service, to point at `service start`, and to
/// spell out the native-protocol command that routes around the Query API
/// gateway timeout (#644).
#[derive(Debug, Clone, Copy)]
struct QueryTarget<'a> {
    service_name: &'a str,
    service_id: &'a str,
    org_id: &'a str,
    /// The service's `nativesecure` endpoint, when the `GET service`
    /// response carried a complete one.
    native: Option<&'a NativeEndpoint>,
}

/// The service's `nativesecure` endpoint, when the API returned a complete
/// one.
///
/// Every field of a response model is optional, so a partial endpoint (a host
/// with no port, or the reverse) counts as absent here rather than being
/// half-rendered into a command.
fn native_secure_endpoint(endpoints: Option<&[ServiceEndpoint]>) -> Option<NativeEndpoint> {
    endpoints?.iter().find_map(|endpoint| {
        if endpoint.protocol != Some(ServiceEndpointProtocol::Nativesecure) {
            return None;
        }
        Some(NativeEndpoint {
            host: endpoint.host.clone()?,
            port: endpoint.port?,
        })
    })
}

/// Stand-in for a host the API did not return. A placeholder, never a guess.
const NATIVE_HOST_PLACEHOLDER: &str = "<host>";

/// The standard native-protocol TLS port. Only used in the placeholder
/// command, where there is no endpoint to read a real port from.
const DEFAULT_NATIVE_SECURE_PORT: i64 = 9440;

/// The `clickhouse client` command that reruns the statement over the native
/// protocol.
///
/// The SQL and the password are always literal placeholders: the user's
/// statement is never echoed back into an error message, and no credential is
/// ever rendered.
fn native_client_command(native: Option<&NativeEndpoint>) -> String {
    let (host, port) = match native {
        Some(endpoint) => (endpoint.host.as_str(), endpoint.port),
        None => (NATIVE_HOST_PLACEHOLDER, DEFAULT_NATIVE_SECURE_PORT),
    };
    format!(
        "clickhouse client --host {host} --secure --port {port} --user default \
         --password '<password>' --query '<your SQL>'"
    )
}

/// The Query API gateway stopped waiting for the statement (#644).
///
/// The statement itself is unaffected, so the message leads with that and
/// points at `system.processes` *before* offering the native-protocol route:
/// re-running an `INSERT` that is still executing loads the data twice. There
/// is deliberately no retry anywhere on this path.
fn query_timeout_error(target: QueryTarget<'_>) -> CloudError {
    let command = native_client_command(target.native);
    let endpoint_guidance = if target.native.is_some() {
        String::new()
    } else {
        format!(
            "\nThe API response carried no native endpoint for this service; read the host and \
             port from `clickhousectl cloud service get {} --org-id {}`.",
            target.service_id, target.org_id
        )
    };
    let message = format!(
        "the query timed out at the Query API gateway, which stops waiting after about 30 \
         seconds.\nThe statement may still be running on the service; check `SELECT query_id, \
         elapsed FROM system.processes` before running it again.\n\nHint: run long statements \
         over the native protocol instead. Install the client with\n  clickhousectl local use \
         latest\nthen rerun the query with\n  {command}{endpoint_guidance}\nThe password is the \
         one shown when the service was created; `clickhousectl cloud service reset-password {}` \
         issues a new one.",
        target.service_id
    );

    CloudError::new(message.clone())
        // Rewriting the message must not lose the classification the variant
        // already established, and the classification still comes only from
        // the variant (#450).
        .with_failure(failure::classify_api_error(
            &clickhouse_cloud_api::Error::QueryTimeout,
        ))
        // The same failure, machine-readable, for `--json` mode.
        .with_details(CloudErrorDetail {
            code: CloudErrorCode::QueryTimeout,
            message,
            host: target.native.map(|native| native.host.clone()),
            port: target.native.map(|native| native.port),
            command: Some(command),
            api_key_id: None,
            ip_access_list: None,
        })
}

fn convert_query_error(
    client: &CloudClient,
    error: clickhouse_cloud_api::Error,
    target: QueryTarget<'_>,
) -> CloudError {
    match error {
        clickhouse_cloud_api::Error::ServiceStopped => CloudError::new(format!(
            "service '{}' is stopped; start it with `clickhousectl cloud service start {} --org-id {}` and retry",
            target.service_name, target.service_id, target.org_id
        ))
        // Rewriting the message must not lose the classification the variant
        // already established (#450).
        .with_failure(ApiFailure::new(FailureKind::ServiceStopped)),
        clickhouse_cloud_api::Error::QueryTimeout => query_timeout_error(target),
        other => client.convert_error(other),
    }
}

/// What stdin holds when `--query` is validated (#641). Derived from the file
/// descriptor only, never from anything the user typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StdinInput {
    /// An interactive terminal: nothing was piped or redirected in, and stdin
    /// must not be read at all — a read would block on the human.
    Terminal,
    /// Not a terminal, and no byte arrived within
    /// [`STDIN_FIRST_BYTE_TIMEOUT_MS`]: `< /dev/null`, a closed pipe, or a CI
    /// runner or coding agent that has no tty and piped nothing.
    Empty,
    /// Not a terminal and at least one byte is available to read.
    Pending,
}

/// The message for `--query` plus data on stdin. The Query API takes a single
/// request body, so a statement and a separate data stream can never both be
/// sent; the message therefore leads with what does work.
const INLINE_QUERY_WITH_STDIN_ERROR: &str = "--query cannot be combined with SQL or data on stdin. \
     The Query API sends one request body, so redirected data is never read. Pipe the statement \
     and its data together on stdin instead: \
     printf 'INSERT INTO t FORMAT CSV\\n' | cat - data.csv | clickhousectl cloud service query \
     --id <id>. Or read a whole statement from stdin with --queries-file -.";

/// Whether `--query` conflicts with what is on stdin. Pure, so the decision is
/// unit-tested without a real file descriptor. Only bytes that are actually
/// waiting count: a non-terminal but empty stdin is the normal shape for CI
/// and for coding agents, so it must stay a valid way to run `--query`.
fn inline_query_conflicts_with_stdin(inline: Option<&str>, stdin: StdinInput) -> bool {
    inline.is_some() && matches!(stdin, StdinInput::Pending)
}

/// How long to wait for the first byte before deciding stdin carries nothing.
///
/// A zero timeout would reintroduce #641 as a race: in `cat data.csv |
/// clickhousectl ... --query ...` the shell starts both processes at once, so
/// the CLI can reach `poll` before `cat` has written anything and would go on
/// to send the empty INSERT. A bounded wait closes that window. Nothing that
/// already knows its answer pays it: a regular file, `/dev/null`, a closed
/// pipe (`POLLHUP`) and a pipe that already holds data all make `poll` return
/// at once. The only invocation that waits is `--query` whose stdin is an open
/// pipe with nothing written yet — the harness case that must never hang — and
/// it waits this long rather than forever.
const STDIN_FIRST_BYTE_TIMEOUT_MS: libc::c_int = 250;

/// Classify stdin without consuming it and without blocking indefinitely.
///
/// `poll` answers "would a read block?" without reading; only once it says a
/// read is ready do we `fill_buf`, which performs one syscall and leaves the
/// bytes in the shared stdin buffer, so the other SQL sources are unaffected.
/// A hangup without readable data (`POLLHUP` alone) is an empty stdin, not
/// input. The deliberate residual is now narrow: a producer that has not
/// written its first byte within [`STDIN_FIRST_BYTE_TIMEOUT_MS`] still reads
/// as no input, which is the price of never hanging on a pipe nobody is
/// writing to.
fn stdin_input() -> StdinInput {
    use std::io::BufRead as _;

    if std::io::stdin().is_terminal() {
        return StdinInput::Terminal;
    }

    if !fd_becomes_readable(libc::STDIN_FILENO, STDIN_FIRST_BYTE_TIMEOUT_MS) {
        return StdinInput::Empty;
    }

    match std::io::stdin().lock().fill_buf() {
        Ok(buffered) if !buffered.is_empty() => StdinInput::Pending,
        _ => StdinInput::Empty,
    }
}

/// Whether a read on `fd` would find something within `timeout_ms`. A hangup
/// with nothing readable (`POLLHUP` alone, a closed pipe) reports `false`, and
/// so does a `poll` that is interrupted or errors: the CLI fails open to "no
/// input" rather than refusing a run it cannot justify refusing. Split out
/// from [`stdin_input`] so the timeout is testable against a real pipe without
/// touching the process's own stdin.
fn fd_becomes_readable(fd: libc::c_int, timeout_ms: libc::c_int) -> bool {
    let mut poll_fd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: a single initialized `pollfd` is passed with a matching length,
    // and the bounded timeout keeps the call from borrowing anything beyond
    // its own return.
    let ready = unsafe { libc::poll(&mut poll_fd, 1, timeout_ms) };
    ready > 0 && poll_fd.revents & libc::POLLIN != 0
}

fn read_query_sql(inline: Option<&str>, queries_file: Option<&str>) -> CloudResult<String> {
    use std::io::Read as _;

    if let Some(query) = inline {
        if query.trim().is_empty() {
            return Err(CloudError::new("--query was empty"));
        }
        if inline_query_conflicts_with_stdin(inline, stdin_input()) {
            return Err(CloudError::new(INLINE_QUERY_WITH_STDIN_ERROR));
        }
        return Ok(query.to_string());
    }

    if let Some(path) = queries_file {
        let mut content = String::new();
        if path == "-" {
            std::io::stdin().read_to_string(&mut content)?;
        } else {
            content = std::fs::read_to_string(path)?;
        }
        if content.trim().is_empty() {
            return Err(CloudError::new("queries file was empty"));
        }
        return Ok(content);
    }

    if std::io::stdin().is_terminal() {
        return Err(CloudError::new(
            "no SQL provided. Pass --query, --queries-file, or pipe SQL on stdin.",
        ));
    }

    let mut content = String::new();
    std::io::stdin().read_to_string(&mut content)?;
    if content.trim().is_empty() {
        return Err(CloudError::new("no SQL received on stdin"));
    }
    Ok(content)
}

fn default_query_format() -> String {
    if std::io::stdout().is_terminal() {
        "PrettyCompact".to_string()
    } else {
        "TabSeparated".to_string()
    }
}

async fn private_endpoint_create(
    client: &CloudClient,
    service_id: &str,
    endpoint_id: &str,
    description: Option<&str>,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = build_private_endpoint_create_request(endpoint_id, description);
    let endpoint = client
        .create_private_endpoint(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&endpoint)?);
    } else {
        println!("Private endpoint created for service {}", service_id);
        println!("  Endpoint ID: {}", or_absent(endpoint.id.as_deref()));
        println!(
            "  Description: {}",
            or_absent(endpoint.description.as_deref())
        );
    }
    Ok(())
}

async fn private_endpoint_get_config(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let config = client
        .get_service_private_endpoint_config(&org_id, service_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else {
        print_human(&config)?;
    }
    Ok(())
}

async fn service_prometheus(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    filtered_metrics: Option<bool>,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let prometheus = client
        .get_service_prometheus(&org_id, service_id, filtered_metrics)
        .await?;
    println!("{}", prometheus);
    Ok(())
}

impl CloudClient {
    pub async fn list_services(&self, org_id: &str) -> crate::cloud::client::Result<Vec<Service>> {
        let response = self
            .api()
            .instance_get_list(org_id, &[])
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn list_services_filtered(
        &self,
        org_id: &str,
        filters: &[String],
    ) -> crate::cloud::client::Result<Vec<Service>> {
        let filters: Vec<&str> = filters.iter().map(String::as_str).collect();
        let response = self
            .api()
            .instance_get_list(org_id, &filters)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_service(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<Service> {
        let response = self
            .api()
            .instance_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_service_if_exists(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<Option<Service>> {
        match self.api().instance_get(org_id, service_id).await {
            Ok(response) => Self::unwrap_response(response).map(Some),
            Err(clickhouse_cloud_api::Error::Api { status: 404, .. }) => Ok(None),
            Err(error) => Err(self.convert_error_for_organization(error, org_id)),
        }
    }

    pub async fn create_service(
        &self,
        org_id: &str,
        request: &ServicePostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServicePostResponse> {
        let response = self
            .api()
            .instance_create(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_service(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .instance_delete(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn change_service_state(
        &self,
        org_id: &str,
        service_id: &str,
        command: ServiceStatePatchRequestCommand,
    ) -> crate::cloud::client::Result<Service> {
        let request = build_service_state_patch_request(command);
        let response = self
            .api()
            .instance_state_update(org_id, service_id, &request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_service(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServicePatchRequest,
    ) -> crate::cloud::client::Result<Service> {
        let response = self
            .api()
            .instance_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_replica_scaling(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServiceReplicaScalingPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServiceScalingPatchResponse>
    {
        let response = self
            .api()
            .instance_replica_scaling_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn reset_password(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServicePasswordPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServicePasswordPatchResponse>
    {
        let response = self
            .api()
            .instance_password_update(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint> {
        let response = self
            .api()
            .instance_query_endpoint_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn create_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        request: &InstanceServiceQueryApiEndpointsPostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ServiceQueryAPIEndpoint> {
        let response = self
            .api()
            .instance_query_endpoint_upsert(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_query_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .instance_query_endpoint_delete(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn create_private_endpoint(
        &self,
        org_id: &str,
        service_id: &str,
        request: &ServicPrivateEndpointePostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::InstancePrivateEndpoint> {
        let response = self
            .api()
            .instance_private_endpoint_create(org_id, service_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_service_private_endpoint_config(
        &self,
        org_id: &str,
        service_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::PrivateEndpointConfig> {
        let response = self
            .api()
            .instance_private_endpoint_config_get(org_id, service_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_service_prometheus(
        &self,
        org_id: &str,
        service_id: &str,
        filtered_metrics: Option<bool>,
    ) -> crate::cloud::client::Result<String> {
        let filtered = filtered_metrics.map(|value| value.to_string());
        self.api()
            .instance_prometheus_get(org_id, service_id, filtered.as_deref())
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use clap::Parser;

    fn parse_service(args: &[&str]) -> crate::cloud::cli::ServiceCommands {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
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

    #[test]
    fn service_query_help_documents_query_behavior() {
        let error = Cli::try_parse_from(["clickhousectl", "cloud", "service", "query", "--help"])
            .err()
            .expect("--help should stop parsing");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = error.to_string();

        // The gateway limit is named, not alluded to, and so is what happens
        // to the statement when it fires (#644).
        assert!(help.contains("times out after about 30 seconds"), "{help}");
        assert!(help.contains("the statement keeps running"), "{help}");
        assert!(help.contains("native protocol"), "{help}");
        assert!(help.contains("`clickhousectl local use latest`"), "{help}");
        // The native-client command, which clap wraps across lines.
        assert!(
            help.contains("clickhouse client --host <host> --secure"),
            "{help}"
        );
        assert!(help.contains("--port 9440 --user"), "{help}");
        assert!(help.contains("--password <password>"), "{help}");
        assert!(help.contains("`clickhouse client`"), "{help}");
        assert!(
            help.contains("--query and --queries-file are mutually exclusive"),
            "{help}"
        );
        assert!(help.contains("SQL is read from stdin"), "{help}");
        assert!(help.contains("repair-query-key <service-id>"), "{help}");
        assert!(help.contains("never replaced automatically"), "{help}");
    }

    #[test]
    fn repair_query_key_help_documents_safe_service_scope() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "service",
            "repair-query-key",
            "--help",
        ])
        .err()
        .expect("--help should stop parsing");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = error.to_string();
        assert!(help.contains("SERVICE_ID"), "{help}");
        assert!(help.contains("exact management API key ID"), "{help}");
        assert!(help.contains("preserving all other"), "{help}");
        assert!(help.contains("Legacy or non-owned records"), "{help}");
        assert!(help.contains("is never run"), "{help}");
        assert!(help.contains("automatically after a query fails"), "{help}");
    }

    #[test]
    fn service_prometheus_help_documents_json_is_ignored() {
        let error =
            Cli::try_parse_from(["clickhousectl", "cloud", "service", "prometheus", "--help"])
                .err()
                .expect("--help should stop parsing");
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        let help = error.to_string();
        assert!(
            help.contains("always raw Prometheus exposition text"),
            "{help}"
        );
        assert!(help.contains("--json"), "{help}");
        assert!(help.contains("silently ignored"), "{help}");
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
            release_channel,
            data_warehouse_id,
            readonly,
            encryption_key,
            encryption_role,
            enable_tde,
            compliance_type,
            profile,
            tag,
            enable_endpoint,
            disable_endpoint,
            private_preview_terms_checked,
            enable_core_dumps,
            org_id,
        } = command
        else {
            panic!("expected service create");
        };

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
        assert!(release_channel.is_none());
        assert!(data_warehouse_id.is_none());
        assert!(!readonly);
        assert!(encryption_key.is_none());
        assert!(encryption_role.is_none());
        assert!(!enable_tde);
        assert!(compliance_type.is_none());
        assert!(profile.is_none());
        assert!(tag.is_empty());
        assert!(enable_endpoint.is_empty());
        assert!(disable_endpoint.is_empty());
        assert!(!private_preview_terms_checked);
        assert!(enable_core_dumps.is_none());
        assert!(org_id.is_none());
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
            "10.0.0.0/8",
            "--ip-allow",
            "192.0.2.0/24",
            "--backup-id",
            "backup-1",
            "--release-channel",
            "fast",
            "--data-warehouse-id",
            "dw-1",
            "--readonly",
            "--encryption-key",
            "key-1",
            "--encryption-role",
            "role-1",
            "--enable-tde",
            "--compliance-type",
            "pci",
            "--profile",
            "v1-highmem-xs",
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
            release_channel,
            data_warehouse_id,
            readonly,
            encryption_key,
            encryption_role,
            enable_tde,
            compliance_type,
            profile,
            tag,
            enable_endpoint,
            disable_endpoint,
            private_preview_terms_checked,
            enable_core_dumps,
            org_id,
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
        assert_eq!(ip_allow, vec!["10.0.0.0/8", "192.0.2.0/24"]);
        assert_eq!(backup_id.as_deref(), Some("backup-1"));
        assert_eq!(release_channel.as_deref(), Some("fast"));
        assert_eq!(data_warehouse_id.as_deref(), Some("dw-1"));
        assert!(readonly);
        assert_eq!(encryption_key.as_deref(), Some("key-1"));
        assert_eq!(encryption_role.as_deref(), Some("role-1"));
        assert!(enable_tde);
        assert_eq!(compliance_type.as_deref(), Some("pci"));
        assert_eq!(profile.as_deref(), Some("v1-highmem-xs"));
        assert_eq!(tag, vec!["env=prod", "team=analytics"]);
        assert_eq!(enable_endpoint, vec!["mysql"]);
        assert_eq!(disable_endpoint, vec!["mysql"]);
        assert!(private_preview_terms_checked);
        assert_eq!(enable_core_dumps, Some(false));
        assert_eq!(org_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn parses_service_list_filters_and_org_id() {
        let command = parse_service(&["clickhousectl", "cloud", "service", "list"]);
        let crate::cloud::cli::ServiceCommands::List { org_id, filter } = command else {
            panic!("expected service list");
        };
        assert!(org_id.is_none());
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
        let crate::cloud::cli::ServiceCommands::List { org_id, filter } = command else {
            panic!("expected service list");
        };
        assert_eq!(org_id.as_deref(), Some("org-1"));
        assert_eq!(filter, vec!["tag:env=prod", "tag:team=analytics"]);
    }

    #[test]
    fn parses_service_delete_force_and_org_id() {
        let command = parse_service(&["clickhousectl", "cloud", "service", "delete", "svc-1"]);
        let crate::cloud::cli::ServiceCommands::Delete { force, org_id, .. } = command else {
            panic!("expected service delete");
        };
        assert!(!force);
        assert!(org_id.is_none());

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
        let crate::cloud::cli::ServiceCommands::Delete {
            service_id,
            force,
            org_id,
        } = command
        else {
            panic!("expected service delete");
        };
        assert_eq!(service_id, "svc-1");
        assert!(force);
        assert_eq!(org_id.as_deref(), Some("org-1"));
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
            org_id,
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
        assert!(org_id.is_none());

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
            org_id,
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
        assert!(org_id.is_none());

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
            org_id,
            ..
        } = reset
        else {
            panic!("expected service reset-password");
        };
        assert!(new_password_hash.is_none());
        assert!(new_double_sha1_hash.is_none());
        assert!(org_id.is_none());

        let query_endpoint = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "query-endpoint",
            "create",
            "svc-1",
        ]);
        let crate::cloud::cli::ServiceCommands::QueryEndpoint { command } = query_endpoint else {
            panic!("expected query-endpoint command");
        };
        let crate::cloud::cli::QueryEndpointCommands::Create {
            role,
            open_api_key,
            allowed_origins,
            org_id,
            ..
        } = command
        else {
            panic!("expected query-endpoint create");
        };
        assert!(role.is_empty());
        assert!(open_api_key.is_empty());
        assert!(allowed_origins.is_none());
        assert!(org_id.is_none());

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
        let crate::cloud::cli::PrivateEndpointCommands::Create {
            description,
            org_id,
            ..
        } = command
        else {
            panic!("expected private-endpoint create");
        };
        assert!(description.is_none());
        assert!(org_id.is_none());
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
    fn parses_service_update_maximal_and_repeatable_flags() {
        let command = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "update",
            "svc-1",
            "--name",
            "renamed",
            "--add-ip-allow",
            "10.0.0.0/8",
            "--add-ip-allow",
            "192.0.2.0/24",
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
            "--remove-tag",
            "legacy",
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
            org_id,
        } = command
        else {
            panic!("expected service update");
        };

        assert_eq!(service_id, "svc-1");
        assert_eq!(name.as_deref(), Some("renamed"));
        assert_eq!(add_ip_allow, vec!["10.0.0.0/8", "192.0.2.0/24"]);
        assert_eq!(remove_ip_allow, vec!["0.0.0.0/0"]);
        assert_eq!(add_private_endpoint_id, vec!["pe-1", "pe-2"]);
        assert_eq!(remove_private_endpoint_id, vec!["pe-3"]);
        assert_eq!(release_channel.as_deref(), Some("slow"));
        assert_eq!(enable_endpoint, vec!["mysql"]);
        assert_eq!(disable_endpoint, vec!["mysql"]);
        assert_eq!(transparent_data_encryption_key_id.as_deref(), Some("tde-1"));
        assert_eq!(add_tag, vec!["env=prod", "team=analytics"]);
        assert_eq!(remove_tag, vec!["legacy"]);
        assert_eq!(enable_core_dumps, Some(false));
        assert_eq!(org_id.as_deref(), Some("org-1"));
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
        assert_eq!(service_id, "svc-1");
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
            org_id,
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
        assert_eq!(org_id.as_deref(), Some("org-1"));
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
            org_id,
            ..
        } = command
        else {
            panic!("expected reset-password");
        };
        assert_eq!(new_password_hash.as_deref(), Some("sha256"));
        assert_eq!(new_double_sha1_hash.as_deref(), Some("sha1"));
        assert_eq!(org_id.as_deref(), Some("org-1"));

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
        let PrivateEndpointCommands::GetConfig { service_id, org_id } = command else {
            panic!("expected get-config");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(org_id.as_deref(), Some("org-1"));
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
            org_id,
            no_auto_enable,
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
        assert!(org_id.is_none());
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
            org_id,
            no_auto_enable,
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
        assert_eq!(org_id.as_deref(), Some("org-1"));
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
        let ServiceCommands::RepairQueryKey { service_id, org_id } = command else {
            panic!("expected query-key repair");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(org_id.as_deref(), Some("org-1"));

        let ServiceCommands::RepairQueryKey { org_id, .. } = parse_service(&[
            "clickhousectl",
            "cloud",
            "service",
            "repair-query-key",
            "svc-1",
        ]) else {
            panic!("expected query-key repair");
        };
        assert!(org_id.is_none());
    }

    #[test]
    fn only_a_401_or_403_counts_as_a_stored_query_key_rejection() {
        for status in [401, 403] {
            let error = clickhouse_cloud_api::Error::Api {
                status,
                message: "rejected".into(),
            };
            assert_eq!(stored_query_key_rejection_status(&error), Some(status));
        }
        assert_eq!(
            stored_query_key_rejection_status(&clickhouse_cloud_api::Error::Api {
                status: 404,
                message: "missing".into(),
            }),
            None
        );
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
            org_id,
        } = command
        else {
            panic!("expected query-endpoint create");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(role, vec!["sql_console_read_only", "sql_console_admin"]);
        assert_eq!(open_api_key, vec!["key-1", "key-2"]);
        assert_eq!(allowed_origins.as_deref(), Some("https://example.com"));
        assert_eq!(org_id.as_deref(), Some("org-1"));

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
                crate::cloud::cli::QueryEndpointCommands::Get { service_id, org_id }
                | crate::cloud::cli::QueryEndpointCommands::Delete { service_id, org_id } => {
                    assert_eq!(service_id, "svc-1");
                    assert_eq!(org_id.as_deref(), Some("org-1"));
                }
                crate::cloud::cli::QueryEndpointCommands::Create { .. } => {
                    panic!("expected query-endpoint {action}")
                }
            }
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
            org_id,
        } = command
        else {
            panic!("expected private-endpoint create");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(endpoint_id, "vpce-0123456789abcdef0");
        assert_eq!(description.as_deref(), Some("production"));
        assert_eq!(org_id.as_deref(), Some("org-1"));
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
        let crate::cloud::cli::ServiceCommands::Get { service_id, org_id } = command else {
            panic!("expected service get");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(org_id.as_deref(), Some("org-1"));

        for action in ["start", "stop"] {
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
                crate::cloud::cli::ServiceCommands::Start { service_id, org_id }
                | crate::cloud::cli::ServiceCommands::Stop { service_id, org_id } => {
                    assert_eq!(service_id, "svc-1");
                    assert_eq!(org_id.as_deref(), Some("org-1"));
                }
                _ => panic!("expected service {action}"),
            }
        }

        let command = parse_service(&["clickhousectl", "cloud", "service", "prometheus", "svc-1"]);
        let crate::cloud::cli::ServiceCommands::Prometheus {
            org_id,
            filtered_metrics,
            ..
        } = command
        else {
            panic!("expected service prometheus");
        };
        assert!(org_id.is_none());
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
            org_id,
            filtered_metrics,
        } = command
        else {
            panic!("expected service prometheus");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(org_id.as_deref(), Some("org-1"));
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
            assert_write(
                &[
                    "clickhousectl",
                    "cloud",
                    "service",
                    "query-endpoint",
                    action,
                    "svc-1",
                ],
                true,
            );
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
            ],
            true,
        );
    }

    #[test]
    fn first_endpoint_keeps_the_present_half_of_a_partial_endpoint() {
        let endpoint = |host: Option<&str>, port: Option<i64>| ServiceEndpoint {
            host: host.map(str::to_string),
            port,
            ..Default::default()
        };
        assert_eq!(first_endpoint(None), ABSENT);
        assert_eq!(first_endpoint(Some(&[])), ABSENT);
        assert_eq!(
            first_endpoint(Some(&[endpoint(Some("host"), Some(9440))])),
            "host:9440"
        );
        assert_eq!(
            first_endpoint(Some(&[endpoint(Some("host"), None)])),
            "host"
        );
        assert_eq!(
            first_endpoint(Some(&[endpoint(None, Some(9440))])),
            format!("{ABSENT}:9440")
        );
        assert_eq!(first_endpoint(Some(&[endpoint(None, None)])), ABSENT);
    }

    #[test]
    fn service_query_hint_is_dropped_when_the_id_is_absent() {
        assert_eq!(service_query_hint(None), None);
        let id = uuid::Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap();
        let hint = service_query_hint(Some(id)).unwrap();
        assert!(hint.contains("--id a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6"));
        assert!(hint.contains("provisioned automatically on first use"));
    }

    #[test]
    fn service_credentials_block_shows_the_password_the_api_sent() {
        let id = uuid::Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap();
        assert_eq!(
            service_credentials_block(Some("s3cret"), Some(id)),
            "Credentials (save these, password shown only once):\n  Username: default\n  \
             Password: s3cret"
        );
    }

    #[test]
    fn service_credentials_block_treats_an_empty_password_as_sent() {
        assert_eq!(
            service_credentials_block(Some(""), None),
            "Credentials (save these, password shown only once):\n  Username: default\n  \
             Password: "
        );
    }

    #[test]
    fn service_credentials_block_warns_with_the_reset_command_when_the_password_is_absent() {
        let id = uuid::Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap();
        let block = service_credentials_block(None, Some(id));
        assert!(!block.contains(&format!("Password: {ABSENT}")));
        assert!(block.contains(
            "clickhousectl cloud service reset-password a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6"
        ));
    }

    #[test]
    fn service_credentials_block_warns_generically_when_the_service_id_is_absent() {
        assert!(
            service_credentials_block(None, None)
                .contains("clickhousectl cloud service reset-password <service-id>")
        );
    }

    #[test]
    fn classify_stop_poll_state_fails_on_an_absent_state() {
        let error = classify_stop_poll_state(None).unwrap_err();
        assert!(error.to_string().contains("omitted the service state"));
    }

    #[test]
    fn classify_stop_poll_state_separates_stopped_waiting_and_failed() {
        assert!(classify_stop_poll_state(Some(&ServiceState::Stopped)).unwrap());
        assert!(classify_stop_poll_state(Some(&ServiceState::Idle)).unwrap());
        assert!(!classify_stop_poll_state(Some(&ServiceState::Stopping)).unwrap());
        assert!(!classify_stop_poll_state(Some(&ServiceState::Running)).unwrap());
        assert!(
            !classify_stop_poll_state(Some(&ServiceState::Unknown("hibernating".into()))).unwrap()
        );
        assert!(classify_stop_poll_state(Some(&ServiceState::Failed)).is_err());
        assert!(classify_stop_poll_state(Some(&ServiceState::Unknown("deleted".into()))).is_err());
    }

    #[test]
    fn stop_poll_progress_collapses_repeats_but_keeps_transitions() {
        let mut progress = StopPollProgress::default();
        assert_eq!(
            progress.render("stopping", false).as_deref(),
            Some("  state: stopping")
        );
        assert_eq!(progress.render("stopping", false), None);
        assert_eq!(
            progress.render("running", false).as_deref(),
            Some("  state: running")
        );
        assert_eq!(
            progress.render("stopped", false).as_deref(),
            Some("  state: stopped")
        );
    }

    #[test]
    fn stop_poll_progress_keeps_repeats_in_verbose_mode() {
        let mut progress = StopPollProgress::default();
        assert!(progress.render("stopping", true).is_some());
        assert!(progress.render("stopping", true).is_some());
    }

    #[test]
    fn query_output_completion_acknowledges_empty_responses() {
        assert_eq!(
            query_output_completion("TabSeparated", 0, None),
            QueryOutputCompletion::Acknowledge
        );
        assert_eq!(
            query_output_completion("RowBinary", 0, None),
            QueryOutputCompletion::Acknowledge
        );
    }

    #[test]
    fn query_output_completion_adds_only_a_missing_text_newline() {
        for format in ["TabSeparated", "PrettyCompact", "JSONEachRow"] {
            assert_eq!(
                query_output_completion(format, 2, Some(b'K')),
                QueryOutputCompletion::Newline
            );
        }
        assert_eq!(
            query_output_completion("JSONEachRow", 2, Some(b'\n')),
            QueryOutputCompletion::None
        );
    }

    #[test]
    fn query_output_completion_preserves_exact_and_binary_bodies() {
        for format in [
            "Template",
            "Buffers",
            "BSONEachRow",
            "RowBinary",
            "Native",
            "Parquet",
            "ArrowStream",
            "MsgPack",
        ] {
            assert_eq!(
                query_output_completion(format, 3, Some(0)),
                QueryOutputCompletion::None
            );
        }
    }

    #[test]
    fn query_readiness_is_only_auth_or_generic_not_found() {
        for status in [401, 403, 404] {
            assert!(query_endpoint_readiness_error(
                &clickhouse_cloud_api::Error::Api {
                    status,
                    message: "rejected".into(),
                }
            ));
        }
        for status in [400, 408, 429, 500] {
            assert!(!query_endpoint_readiness_error(
                &clickhouse_cloud_api::Error::Api {
                    status,
                    message: "query failed".into(),
                }
            ));
        }
        // A SQL-level rejection proves the endpoint *is* usable, and it is
        // recognized by its variant rather than by its message text.
        assert!(!query_endpoint_readiness_error(
            &clickhouse_cloud_api::Error::Sql {
                status: 404,
                code: "60".into(),
                details: "Unknown table".into(),
            }
        ));
        assert!(!query_endpoint_readiness_error(
            &clickhouse_cloud_api::Error::ServiceStopped
        ));
    }

    #[tokio::test]
    async fn query_does_not_provision_or_retry_transport_errors() {
        let transport_error = reqwest::Client::new()
            .get("not a url")
            .send()
            .await
            .unwrap_err();
        assert!(!query_endpoint_readiness_error(
            &clickhouse_cloud_api::Error::Http(transport_error)
        ));
    }

    #[tokio::test]
    async fn query_readiness_deadline_bounds_a_stalled_attempt() {
        let readiness = QueryEndpointReadiness {
            timeout: Duration::from_millis(10),
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        };
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            wait_for_query_endpoint_readiness(readiness, || {
                std::future::pending::<Result<(), clickhouse_cloud_api::Error>>()
            }),
        )
        .await
        .expect("stalled attempt exceeded the test guard");

        assert_query_readiness_timeout(result, readiness.timeout);
    }

    #[tokio::test]
    async fn query_readiness_exhaustion_is_not_an_auth_error() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        let readiness = QueryEndpointReadiness {
            timeout: Duration::from_millis(20),
            initial_backoff: Duration::from_millis(20),
            max_backoff: Duration::from_millis(20),
        };
        let attempts = Arc::new(AtomicUsize::new(0));
        let probe_attempts = Arc::clone(&attempts);
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            wait_for_query_endpoint_readiness(readiness, move || {
                probe_attempts.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Err::<(), _>(clickhouse_cloud_api::Error::Api {
                    status: 401,
                    message: "query endpoint is not ready".into(),
                }))
            }),
        )
        .await
        .expect("readiness retries exceeded the test guard");

        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert_query_readiness_timeout(result, readiness.timeout);
    }

    fn assert_query_readiness_timeout<T>(
        result: Result<T, QueryReadinessError>,
        timeout: Duration,
    ) {
        let elapsed = match result {
            Err(QueryReadinessError::TimedOut(elapsed)) => elapsed,
            Err(QueryReadinessError::Api(error)) => panic!("expected the deadline, got {error}"),
            Ok(_) => panic!("readiness should time out"),
        };
        assert_eq!(elapsed, timeout);
        let converted = query_readiness_timeout_error(elapsed);

        assert_eq!(
            converted.kind,
            crate::cloud::client::CloudErrorKind::Generic
        );
        assert_eq!(
            converted.message,
            format!("Query API endpoint did not become ready within {timeout:?}")
        );
        // The readiness deadline is reported as a timeout (#450), not as a
        // generic 4xx: the CLI, not the API, gave up — so there is no HTTP
        // status to report either.
        assert_eq!(
            converted.failure,
            Some(ApiFailure::new(FailureKind::Timeout))
        );
    }

    /// A query target for the error-rendering tests. `native` decides whether
    /// the timeout hint has a real endpoint to name.
    fn test_query_target(native: Option<&NativeEndpoint>) -> QueryTarget<'_> {
        QueryTarget {
            service_name: "demo",
            service_id: "svc-1",
            org_id: "org-1",
            native,
        }
    }

    #[test]
    fn native_secure_endpoint_picks_the_native_protocol_endpoint() {
        let endpoints = vec![
            ServiceEndpoint {
                host: Some("demo.gcp.clickhouse.cloud".into()),
                port: Some(8443),
                protocol: Some(ServiceEndpointProtocol::Https),
                username: None,
            },
            ServiceEndpoint {
                host: Some("demo.gcp.clickhouse.cloud".into()),
                port: Some(9440),
                protocol: Some(ServiceEndpointProtocol::Nativesecure),
                username: Some("default".into()),
            },
        ];
        assert_eq!(
            native_secure_endpoint(Some(&endpoints)),
            Some(NativeEndpoint {
                host: "demo.gcp.clickhouse.cloud".into(),
                port: 9440,
            })
        );
    }

    #[test]
    fn native_secure_endpoint_is_absent_when_the_response_is_incomplete() {
        // No endpoints at all, and an empty list.
        assert_eq!(native_secure_endpoint(None), None);
        assert_eq!(native_secure_endpoint(Some(&[])), None);
        // Only the HTTPS endpoint.
        assert_eq!(
            native_secure_endpoint(Some(&[ServiceEndpoint {
                host: Some("demo.clickhouse.cloud".into()),
                port: Some(8443),
                protocol: Some(ServiceEndpointProtocol::Https),
                username: None,
            }])),
            None
        );
        // Every response field is optional, so a half-returned endpoint is
        // absent rather than half-rendered into a command.
        for partial in [
            ServiceEndpoint {
                host: Some("demo.clickhouse.cloud".into()),
                port: None,
                protocol: Some(ServiceEndpointProtocol::Nativesecure),
                username: None,
            },
            ServiceEndpoint {
                host: None,
                port: Some(9440),
                protocol: Some(ServiceEndpointProtocol::Nativesecure),
                username: None,
            },
            ServiceEndpoint {
                host: Some("demo.clickhouse.cloud".into()),
                port: Some(9440),
                protocol: None,
                username: None,
            },
        ] {
            assert_eq!(
                native_secure_endpoint(Some(std::slice::from_ref(&partial))),
                None
            );
        }
    }

    /// The gateway timeout names the limit, warns before a rerun, and hands
    /// over a native-protocol command built from the service's own endpoint
    /// (#644).
    #[test]
    fn query_timeout_error_hints_the_native_endpoint() {
        let native = NativeEndpoint {
            host: "demo.gcp.clickhouse.cloud".into(),
            port: 9440,
        };
        let error = query_timeout_error(test_query_target(Some(&native)));

        assert!(
            error.message.contains("about 30 seconds"),
            "{}",
            error.message
        );
        assert!(
            error
                .message
                .contains("SELECT query_id, elapsed FROM system.processes"),
            "{}",
            error.message
        );
        assert!(
            error.message.contains("clickhousectl local use latest"),
            "{}",
            error.message
        );
        assert!(
            error.message.contains(
                "clickhouse client --host demo.gcp.clickhouse.cloud --secure --port 9440 \
                 --user default --password '<password>' --query '<your SQL>'"
            ),
            "{}",
            error.message
        );
        assert!(
            error
                .message
                .contains("clickhousectl cloud service reset-password svc-1"),
            "{}",
            error.message
        );
        // With a real endpoint there is nothing to look up.
        assert!(
            !error.message.contains("cloud service get"),
            "{}",
            error.message
        );
        assert!(!error.message.contains("<host>"), "{}", error.message);

        // A timeout is exit code 1 (Generic), never an auth failure, and the
        // classification is the variant's (#450).
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Generic);
        assert_eq!(
            error.failure,
            Some(ApiFailure::with_status(FailureKind::Timeout, 500))
        );

        let details = error.details.as_deref().expect("json details");
        assert_eq!(details.code, CloudErrorCode::QueryTimeout);
        assert_eq!(details.message, error.message);
        assert_eq!(details.host.as_deref(), Some("demo.gcp.clickhouse.cloud"));
        assert_eq!(details.port, Some(9440));
        assert_eq!(
            details.command.as_deref(),
            Some(
                "clickhouse client --host demo.gcp.clickhouse.cloud --secure --port 9440 --user \
                 default --password '<password>' --query '<your SQL>'"
            )
        );
    }

    #[test]
    fn query_timeout_error_falls_back_to_a_host_placeholder() {
        let error = query_timeout_error(test_query_target(None));

        assert!(
            error.message.contains(
                "clickhouse client --host <host> --secure --port 9440 --user default \
                 --password '<password>' --query '<your SQL>'"
            ),
            "{}",
            error.message
        );
        // No endpoint in the response, so the message says where to get one.
        assert!(
            error
                .message
                .contains("clickhousectl cloud service get svc-1 --org-id org-1"),
            "{}",
            error.message
        );

        let details = error.details.as_deref().expect("json details");
        // Absent means omitted: no fabricated host or port.
        assert_eq!(details.host, None);
        assert_eq!(details.port, None);
        assert!(
            details
                .command
                .as_deref()
                .is_some_and(|command| command.contains("--host <host>")),
            "{details:?}"
        );
        let json = serde_json::to_value(details).expect("details serialize");
        assert_eq!(json["code"], "query_timeout");
        assert!(json.get("host").is_none(), "{json}");
        assert!(json.get("port").is_none(), "{json}");
    }

    /// The message is rendered from the target and fixed placeholders only,
    /// so neither the statement nor any credential can reach it.
    #[test]
    fn query_timeout_error_never_echoes_sql_or_credentials() {
        let native = NativeEndpoint {
            host: "demo.gcp.clickhouse.cloud".into(),
            port: 9440,
        };
        for target in [test_query_target(Some(&native)), test_query_target(None)] {
            let error = query_timeout_error(target);
            assert!(error.message.contains("'<your SQL>'"), "{}", error.message);
            assert!(
                error.message.contains("--password '<password>'"),
                "{}",
                error.message
            );
        }
    }

    /// Every query-path error a boundary rewrites keeps a classification
    /// derived from what actually failed, not from its message (#450).
    #[test]
    fn query_errors_carry_their_structural_classification() {
        let client = CloudClient::new(
            Some("test-key"),
            Some("test-secret"),
            Some("https://api.example.com/v1"),
        )
        .unwrap();

        // A stopped service gets a rewritten, actionable message; the
        // classification survives the rewrite.
        let stopped = convert_query_error(
            &client,
            clickhouse_cloud_api::Error::ServiceStopped,
            test_query_target(None),
        );
        assert!(stopped.message.contains("is stopped"));
        assert_eq!(
            stopped.failure,
            Some(ApiFailure::new(FailureKind::ServiceStopped))
        );

        let sql = convert_query_error(
            &client,
            clickhouse_cloud_api::Error::Sql {
                status: 400,
                code: "62".into(),
                details: "Syntax error".into(),
            },
            test_query_target(None),
        );
        assert_eq!(
            sql.failure,
            Some(ApiFailure::with_status(FailureKind::SqlError, 400))
        );

        // `--no-auto-enable` rewrites the message but the failure is still
        // the Query API's own rejection: its class and status survive.
        for status in [401, 403, 404] {
            let refused = refused_query_provisioning_error(
                "svc-1",
                &clickhouse_cloud_api::Error::Api {
                    status,
                    message: "rejected".into(),
                },
            );
            assert!(refused.message.contains("--no-auto-enable"), "{refused}");
            assert_eq!(refused.kind, crate::cloud::client::CloudErrorKind::Generic);
            assert_eq!(
                refused.failure,
                Some(ApiFailure::with_status(FailureKind::Http4xx, status)),
                "status {status}"
            );
        }
    }

    #[test]
    fn service_delete_error_suggests_force_for_a_running_service() {
        let error = CloudError::new(
            "CONFLICT: Only instance in one of the following states can be terminated. \
             Current state: 'running'",
        );
        assert_eq!(
            service_delete_error(error, false, "svc-1").message,
            "service is running and cannot be deleted. Use --force to stop it first, or \
             `clickhousectl cloud service stop svc-1`."
        );
    }

    #[test]
    fn service_delete_error_preserves_unrelated_and_forced_failures() {
        let unrelated = CloudError::new("CONFLICT: service has dependent resources");
        assert_eq!(
            service_delete_error(unrelated, false, "svc-1").message,
            "CONFLICT: service has dependent resources"
        );
        let forced = CloudError::new("CONFLICT: Current state: 'running'");
        assert_eq!(
            service_delete_error(forced, true, "svc-1").message,
            "CONFLICT: Current state: 'running'"
        );
    }

    #[test]
    fn generation_is_requested_unless_a_password_hash_is_sent() {
        let request = |new_password_hash: Option<&str>, new_double_sha1_hash: Option<&str>| {
            ServicePasswordPatchRequest {
                new_password_hash: new_password_hash.map(str::to_string),
                new_double_sha1_hash: new_double_sha1_hash.map(str::to_string),
            }
        };
        assert!(generation_requested(&request(None, None)));
        assert!(generation_requested(&request(None, Some("sha1"))));
        assert!(!generation_requested(&request(Some("sha256"), None)));
        assert!(!generation_requested(&request(
            Some("sha256"),
            Some("sha1")
        )));
    }

    #[test]
    fn resolve_reset_password_outcome_returns_the_generated_password() {
        assert!(matches!(
            resolve_reset_password_outcome(true, Some("s3cret")).unwrap(),
            ResetPasswordOutcome::Generated("s3cret")
        ));
        assert!(matches!(
            resolve_reset_password_outcome(true, Some("")).unwrap(),
            ResetPasswordOutcome::Generated("")
        ));
    }

    #[test]
    fn resolve_reset_password_outcome_reports_hash_updated_regardless_of_response() {
        assert!(matches!(
            resolve_reset_password_outcome(false, None).unwrap(),
            ResetPasswordOutcome::HashUpdated
        ));
        assert!(matches!(
            resolve_reset_password_outcome(false, Some("s3cret")).unwrap(),
            ResetPasswordOutcome::HashUpdated
        ));
    }

    #[test]
    fn resolve_reset_password_outcome_fails_when_the_generated_password_is_absent() {
        let error = resolve_reset_password_outcome(true, None).unwrap_err();
        assert!(error.to_string().contains("omitted the generated password"));
    }

    fn minimal_create_options() -> CreateServiceOptions {
        CreateServiceOptions {
            name: "svc".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn build_create_service_request_supports_minimal_fields() {
        let request = build_create_service_request(&minimal_create_options()).unwrap();

        assert_eq!(request.name, "svc");
        assert_eq!(request.provider, ServicePostRequestProvider::Aws);
        assert_eq!(request.region, ServicePostRequestRegion::Us_east_1);
        assert_eq!(request.ip_access_list.len(), 1);
        assert_eq!(request.ip_access_list[0].source, "0.0.0.0/0");
        assert_eq!(
            request.ip_access_list[0].description.as_deref(),
            Some("Allow all (created by clickhousectl)")
        );
        assert!(request.autoscaling_mode.is_none());
        assert!(request.min_replica_memory_gb.is_none());
        assert!(request.max_replica_memory_gb.is_none());
        assert!(request.num_replicas.is_none());
        assert!(request.min_replicas.is_none());
        assert!(request.max_replicas.is_none());
        assert!(request.idle_scaling.is_none());
        assert!(request.idle_timeout_minutes.is_none());
        assert!(request.backup_id.is_none());
        assert!(request.release_channel.is_none());
        assert!(request.tags.is_none());
        assert!(request.data_warehouse_id.is_none());
        assert!(request.is_readonly.is_none());
        assert!(request.encryption_key.is_none());
        assert!(request.encryption_assumed_role_identifier.is_none());
        assert!(request.has_transparent_data_encryption.is_none());
        assert!(request.compliance_type.is_none());
        assert!(request.profile.is_none());
        assert!(request.private_preview_terms_checked.is_none());
        assert!(request.endpoints.is_none());
        assert!(request.enable_core_dumps.is_none());
        assert!(request.byoc_id.is_none());
    }

    #[test]
    fn build_create_service_request_supports_maximal_fields() {
        let options = CreateServiceOptions {
            min_replica_memory_gb: Some(24),
            max_replica_memory_gb: Some(48),
            num_replicas: Some(3),
            autoscaling_mode: Some("vertical".to_string()),
            idle_scaling: Some(true),
            idle_timeout_minutes: Some(10),
            ip_allow: vec!["10.0.0.0/8".to_string()],
            backup_id: Some("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6".to_string()),
            release_channel: Some("fast".to_string()),
            data_warehouse_id: Some("dw-1".to_string()),
            is_readonly: true,
            encryption_key: Some("key-1".to_string()),
            encryption_role: Some("role-1".to_string()),
            enable_tde: true,
            compliance_type: Some("hipaa".to_string()),
            profile: Some("v1-default".to_string()),
            tags: vec!["env=prod".to_string()],
            enable_endpoints: vec!["mysql".to_string()],
            disable_endpoints: vec!["mysql".to_string()],
            private_preview_terms_checked: true,
            enable_core_dumps: Some(true),
            ..minimal_create_options()
        };
        let request = build_create_service_request(&options).unwrap();

        assert_eq!(request.name, "svc");
        assert_eq!(request.provider, ServicePostRequestProvider::Aws);
        assert_eq!(request.region, ServicePostRequestRegion::Us_east_1);
        assert_eq!(request.autoscaling_mode, Some(AutoscalingMode::Vertical));
        assert_eq!(request.min_replica_memory_gb, Some(24.0));
        assert_eq!(request.max_replica_memory_gb, Some(48.0));
        assert_eq!(request.num_replicas, Some(3));
        assert!(request.min_replicas.is_none());
        assert!(request.max_replicas.is_none());
        assert_eq!(request.idle_scaling, Some(true));
        assert_eq!(request.idle_timeout_minutes, Some(10.0));
        assert_eq!(request.ip_access_list.len(), 1);
        assert_eq!(request.ip_access_list[0].source, "10.0.0.0/8");
        assert!(request.ip_access_list[0].description.is_none());
        assert_eq!(
            request.backup_id,
            Some(uuid::Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-e1e2e3e4e5e6").unwrap())
        );
        assert_eq!(
            request.release_channel,
            Some(ServicePostRequestReleasechannel::Fast)
        );
        assert_eq!(request.data_warehouse_id.as_deref(), Some("dw-1"));
        assert_eq!(request.is_readonly, Some(true));
        assert_eq!(request.encryption_key.as_deref(), Some("key-1"));
        assert_eq!(
            request.encryption_assumed_role_identifier.as_deref(),
            Some("role-1")
        );
        assert_eq!(request.has_transparent_data_encryption, Some(true));
        assert_eq!(
            request.compliance_type,
            Some(ServicePostRequestCompliancetype::Hipaa)
        );
        assert_eq!(request.profile, Some(ServicePostRequestProfile::V1_default));
        let tags = request.tags.as_ref().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key, "env");
        assert_eq!(tags[0].value.as_deref(), Some("prod"));
        let endpoints = request.endpoints.as_ref().unwrap();
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].protocol, ServiceEndpointChangeProtocol::Mysql);
        assert!(endpoints[0].enabled);
        assert_eq!(endpoints[1].protocol, ServiceEndpointChangeProtocol::Mysql);
        assert!(!endpoints[1].enabled);
        assert_eq!(request.private_preview_terms_checked, Some(true));
        assert_eq!(request.enable_core_dumps, Some(true));
        assert!(request.byoc_id.is_none());
    }

    #[test]
    fn build_create_service_request_trims_tag_keys() {
        let request = build_create_service_request(&CreateServiceOptions {
            tags: vec![" env =prod".to_string()],
            ..minimal_create_options()
        })
        .unwrap();
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["tags"][0]["key"], "env");
        assert_eq!(json["tags"][0]["value"], "prod");
    }

    #[test]
    fn build_create_service_request_rejects_empty_tag_keys() {
        let error = build_create_service_request(&CreateServiceOptions {
            tags: vec!["=prod".to_string()],
            ..minimal_create_options()
        })
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid tag '=prod': tag key cannot be empty"
        );
    }

    #[test]
    fn build_update_service_request_supports_minimal_fields() {
        let request = build_update_service_request(&ServiceUpdateOptions::default()).unwrap();

        assert!(request.name.is_none());
        assert!(request.ip_access_list.is_none());
        assert!(request.private_endpoint_ids.is_none());
        assert!(request.release_channel.is_none());
        assert!(request.endpoints.is_none());
        assert!(request.transparent_data_encryption_key_id.is_none());
        assert!(request.tags.is_none());
        assert!(request.enable_core_dumps.is_none());
    }

    #[test]
    fn build_update_service_request_supports_maximal_fields() {
        let options = ServiceUpdateOptions {
            name: Some("updated".to_string()),
            add_ip_allow: vec!["10.0.0.0/8".to_string()],
            remove_ip_allow: vec!["0.0.0.0/0".to_string()],
            add_private_endpoint_ids: vec!["pe-1".to_string()],
            remove_private_endpoint_ids: vec!["pe-2".to_string()],
            release_channel: Some("default".to_string()),
            enable_endpoints: vec!["mysql".to_string()],
            disable_endpoints: vec!["mysql".to_string()],
            transparent_data_encryption_key_id: Some("tde-1".to_string()),
            add_tags: vec!["env=staging".to_string()],
            remove_tags: vec!["old=tag".to_string()],
            enable_core_dumps: Some(false),
            ..Default::default()
        };
        let request = build_update_service_request(&options).unwrap();

        assert_eq!(request.name.as_deref(), Some("updated"));
        let ip_access_list = request.ip_access_list.as_ref().unwrap();
        assert_eq!(ip_access_list.add.len(), 1);
        assert_eq!(ip_access_list.add[0].source, "10.0.0.0/8");
        assert!(ip_access_list.add[0].description.is_none());
        assert_eq!(ip_access_list.remove.len(), 1);
        assert_eq!(ip_access_list.remove[0].source, "0.0.0.0/0");
        assert!(ip_access_list.remove[0].description.is_none());
        let private_endpoint_ids = request.private_endpoint_ids.as_ref().unwrap();
        assert_eq!(private_endpoint_ids.add, vec!["pe-1"]);
        assert_eq!(private_endpoint_ids.remove, vec!["pe-2"]);
        assert_eq!(
            request.release_channel,
            Some(ServicePatchRequestReleasechannel::Default)
        );
        let endpoints = request.endpoints.as_ref().unwrap();
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].protocol, ServiceEndpointChangeProtocol::Mysql);
        assert!(endpoints[0].enabled);
        assert_eq!(endpoints[1].protocol, ServiceEndpointChangeProtocol::Mysql);
        assert!(!endpoints[1].enabled);
        assert_eq!(
            request.transparent_data_encryption_key_id.as_deref(),
            Some("tde-1")
        );
        let tags = request.tags.as_ref().unwrap();
        assert_eq!(tags.add.len(), 1);
        assert_eq!(tags.add[0].key, "env");
        assert_eq!(tags.add[0].value.as_deref(), Some("staging"));
        assert_eq!(tags.remove.len(), 1);
        assert_eq!(tags.remove[0].key, "old");
        assert_eq!(tags.remove[0].value.as_deref(), Some("tag"));
        assert_eq!(request.enable_core_dumps, Some(false));
    }

    #[test]
    fn build_update_service_request_rejects_empty_tag_keys() {
        let error = build_update_service_request(&ServiceUpdateOptions {
            add_tags: vec![" =prod".to_string()],
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid tag ' =prod': tag key cannot be empty"
        );
    }

    #[test]
    fn build_update_service_request_rejects_invalid_release_channel() {
        let error = build_update_service_request(&ServiceUpdateOptions {
            release_channel: Some("turbo".to_string()),
            ..Default::default()
        })
        .unwrap_err();
        assert!(error.to_string().contains("turbo"));
    }

    #[test]
    fn has_removals_is_false_when_no_remove_flags_set() {
        assert!(!has_removals(&ServiceUpdateOptions::default()));
        assert!(!has_removals(&ServiceUpdateOptions {
            add_ip_allow: vec!["10.0.0.0/8".to_string()],
            ..Default::default()
        }));
    }

    #[test]
    fn has_removals_is_true_when_any_remove_flag_set() {
        assert!(has_removals(&ServiceUpdateOptions {
            remove_ip_allow: vec!["10.0.0.0/8".to_string()],
            ..Default::default()
        }));
        assert!(has_removals(&ServiceUpdateOptions {
            remove_private_endpoint_ids: vec!["pe-1".to_string()],
            ..Default::default()
        }));
        assert!(has_removals(&ServiceUpdateOptions {
            remove_tags: vec!["env".to_string()],
            ..Default::default()
        }));
    }

    #[test]
    fn unmatched_returns_only_values_absent_from_current() {
        let current: HashSet<&str> = ["a", "b"].into_iter().collect();
        assert_eq!(
            unmatched(&["a".to_string(), "c".to_string()], &current),
            vec!["c"]
        );
        assert!(unmatched(&["a".to_string(), "b".to_string()], &current).is_empty());
    }

    #[test]
    fn tag_key_strips_value_and_trims() {
        assert_eq!(tag_key("env"), "env");
        assert_eq!(tag_key("env=prod"), "env");
        assert_eq!(tag_key(" env = prod"), "env");
    }

    fn service_with_ip_allow_endpoints_and_tags() -> Service {
        Service {
            ip_access_list: Some(vec![IpAccessListEntryResponse {
                source: Some("10.0.0.0/8".to_string()),
                description: None,
            }]),
            private_endpoint_ids: Some(vec!["pe-1".to_string()]),
            tags: Some(vec![ResourceTagsV1Response {
                key: Some("env".to_string()),
                value: Some("prod".to_string()),
            }]),
            ..Default::default()
        }
    }

    #[test]
    fn unmatched_removal_warnings_is_empty_when_everything_matches() {
        let current = service_with_ip_allow_endpoints_and_tags();
        let options = ServiceUpdateOptions {
            remove_ip_allow: vec!["10.0.0.0/8".to_string()],
            remove_private_endpoint_ids: vec!["pe-1".to_string()],
            remove_tags: vec!["env".to_string()],
            ..Default::default()
        };
        assert!(unmatched_removal_warnings(&options, &current).is_empty());
    }

    #[test]
    fn unmatched_removal_warnings_names_every_unmatched_entry() {
        let current = service_with_ip_allow_endpoints_and_tags();
        let options = ServiceUpdateOptions {
            remove_ip_allow: vec!["10.99.99.99/32".to_string()],
            remove_private_endpoint_ids: vec!["pe-missing".to_string()],
            remove_tags: vec!["missing=value".to_string()],
            ..Default::default()
        };
        let warnings = unmatched_removal_warnings(&options, &current);
        assert_eq!(warnings.len(), 3);
        assert!(
            warnings[0].contains("--remove-ip-allow 10.99.99.99/32")
                && warnings[0].contains("did not match"),
            "{warnings:?}"
        );
        assert!(
            warnings[1].contains("--remove-private-endpoint-id pe-missing"),
            "{warnings:?}"
        );
        assert!(warnings[2].contains("--remove-tag missing"), "{warnings:?}");
    }

    #[test]
    fn unmatched_removal_warnings_matches_tag_removal_by_key_only() {
        // Removing "env=staging" should still match a current "env=prod" tag:
        // the key is what the server matches on for removal.
        let current = service_with_ip_allow_endpoints_and_tags();
        let options = ServiceUpdateOptions {
            remove_tags: vec!["env=staging".to_string()],
            ..Default::default()
        };
        assert!(unmatched_removal_warnings(&options, &current).is_empty());
    }

    #[test]
    fn unmatched_removal_warnings_handles_empty_current_lists() {
        let options = ServiceUpdateOptions {
            remove_ip_allow: vec!["10.0.0.0/8".to_string()],
            ..Default::default()
        };
        let warnings = unmatched_removal_warnings(&options, &Service::default());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("--remove-ip-allow 10.0.0.0/8"));
    }

    #[test]
    fn build_create_service_request_rejects_invalid_provider() {
        let error = build_create_service_request(&CreateServiceOptions {
            provider: "awss".to_string(),
            ..minimal_create_options()
        })
        .unwrap_err();
        assert!(error.to_string().contains("awss"));
    }

    #[test]
    fn build_create_service_request_rejects_invalid_region() {
        let error = build_create_service_request(&CreateServiceOptions {
            region: "us-east-99".to_string(),
            ..minimal_create_options()
        })
        .unwrap_err();
        assert!(error.to_string().contains("us-east-99"));
    }

    #[test]
    fn build_create_service_request_rejects_invalid_release_channel() {
        let error = build_create_service_request(&CreateServiceOptions {
            release_channel: Some("turbo".to_string()),
            ..minimal_create_options()
        })
        .unwrap_err();
        assert!(error.to_string().contains("turbo"));
    }

    #[test]
    fn build_create_service_request_rejects_invalid_autoscaling_mode() {
        let error = build_create_service_request(&CreateServiceOptions {
            min_replicas: Some(2),
            max_replicas: Some(8),
            autoscaling_mode: Some("turbo".to_string()),
            ..minimal_create_options()
        })
        .unwrap_err();
        assert!(error.to_string().contains("turbo"));
    }

    #[test]
    fn build_create_service_request_supports_horizontal_fields() {
        let request = build_create_service_request(&CreateServiceOptions {
            min_replicas: Some(2),
            max_replicas: Some(8),
            autoscaling_mode: Some("horizontal".to_string()),
            ..minimal_create_options()
        })
        .unwrap();
        assert_eq!(request.autoscaling_mode, Some(AutoscalingMode::Horizontal));
        assert_eq!(request.min_replicas, Some(2));
        assert_eq!(request.max_replicas, Some(8));
        assert!(request.num_replicas.is_none());
        assert!(request.min_replica_memory_gb.is_none());
        assert!(request.max_replica_memory_gb.is_none());
    }

    #[test]
    fn build_create_service_request_replica_pair_without_mode_omits_mode() {
        let request = build_create_service_request(&CreateServiceOptions {
            min_replicas: Some(1),
            max_replicas: Some(4),
            ..minimal_create_options()
        })
        .unwrap();
        assert!(request.autoscaling_mode.is_none());
        assert_eq!(request.min_replicas, Some(1));
        assert_eq!(request.max_replicas, Some(4));
    }

    #[test]
    fn build_create_service_request_vertical_omits_horizontal_fields() {
        let request = build_create_service_request(&CreateServiceOptions {
            num_replicas: Some(3),
            min_replica_memory_gb: Some(24),
            max_replica_memory_gb: Some(48),
            ..minimal_create_options()
        })
        .unwrap();
        assert!(request.autoscaling_mode.is_none());
        assert!(request.min_replicas.is_none());
        assert!(request.max_replicas.is_none());
        assert_eq!(request.num_replicas, Some(3));
        assert_eq!(request.min_replica_memory_gb, Some(24.0));
        assert_eq!(request.max_replica_memory_gb, Some(48.0));
    }

    #[test]
    fn build_create_service_request_rejects_min_without_max_replicas() {
        for options in [
            CreateServiceOptions {
                min_replicas: Some(2),
                ..minimal_create_options()
            },
            CreateServiceOptions {
                max_replicas: Some(8),
                ..minimal_create_options()
            },
        ] {
            let error = build_create_service_request(&options).unwrap_err();
            assert!(error.to_string().contains("--min-replicas"));
        }
    }

    #[test]
    fn resolve_horizontal_autoscaling_explicit_vertical_with_no_replicas() {
        let resolved = resolve_horizontal_autoscaling(Some("vertical"), None, None).unwrap();
        assert_eq!(resolved.autoscaling_mode, Some(AutoscalingMode::Vertical));
        assert!(resolved.min_replicas.is_none());
        assert!(resolved.max_replicas.is_none());
    }

    #[test]
    fn build_service_scale_request_supports_minimal_fields() {
        let request = build_service_scale_request(&ServiceScaleOptions::default()).unwrap();

        assert!(request.autoscaling_mode.is_none());
        assert!(request.min_replica_memory_gb.is_none());
        assert!(request.max_replica_memory_gb.is_none());
        assert!(request.num_replicas.is_none());
        assert!(request.min_replicas.is_none());
        assert!(request.max_replicas.is_none());
        assert!(request.idle_scaling.is_none());
        assert!(request.idle_timeout_minutes.is_none());
    }

    #[test]
    fn build_service_scale_request_supports_horizontal_fields() {
        let request = build_service_scale_request(&ServiceScaleOptions {
            min_replicas: Some(2),
            max_replicas: Some(8),
            autoscaling_mode: Some("horizontal".to_string()),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(request.autoscaling_mode, Some(AutoscalingMode::Horizontal));
        assert_eq!(request.min_replicas, Some(2));
        assert_eq!(request.max_replicas, Some(8));
        assert!(request.min_replica_memory_gb.is_none());
        assert!(request.max_replica_memory_gb.is_none());
        assert!(request.num_replicas.is_none());
    }

    #[test]
    fn build_service_scale_request_supports_maximal_vertical_fields() {
        let request = build_service_scale_request(&ServiceScaleOptions {
            autoscaling_mode: Some("vertical".to_string()),
            num_replicas: Some(3),
            min_replica_memory_gb: Some(8),
            max_replica_memory_gb: Some(32),
            idle_scaling: Some(false),
            idle_timeout_minutes: Some(15),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(request.autoscaling_mode, Some(AutoscalingMode::Vertical));
        assert_eq!(request.num_replicas, Some(3));
        assert_eq!(request.min_replica_memory_gb, Some(8.0));
        assert_eq!(request.max_replica_memory_gb, Some(32.0));
        assert!(request.min_replicas.is_none());
        assert!(request.max_replicas.is_none());
        assert_eq!(request.idle_scaling, Some(false));
        assert_eq!(request.idle_timeout_minutes, Some(15.0));
    }

    #[test]
    fn build_service_scale_request_supports_horizontal_memory_fields() {
        let request = build_service_scale_request(&ServiceScaleOptions {
            autoscaling_mode: Some("horizontal".to_string()),
            min_replicas: Some(2),
            max_replicas: Some(8),
            min_replica_memory_gb: Some(16),
            max_replica_memory_gb: Some(16),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(request.autoscaling_mode, Some(AutoscalingMode::Horizontal));
        assert_eq!(request.min_replicas, Some(2));
        assert_eq!(request.max_replicas, Some(8));
        assert_eq!(request.min_replica_memory_gb, Some(16.0));
        assert_eq!(request.max_replica_memory_gb, Some(16.0));
        assert!(request.num_replicas.is_none());
    }

    #[test]
    fn build_service_scale_request_rejects_min_without_max_replicas() {
        let error = build_service_scale_request(&ServiceScaleOptions {
            max_replicas: Some(8),
            ..Default::default()
        })
        .unwrap_err();
        assert!(error.to_string().contains("--min-replicas"));
    }

    #[test]
    fn build_service_password_patch_request_supports_minimal_fields() {
        let request = build_service_password_patch_request(&ServiceResetPasswordOptions::default());

        assert!(request.new_password_hash.is_none());
        assert!(request.new_double_sha1_hash.is_none());
    }

    #[test]
    fn build_service_password_patch_request_supports_maximal_fields() {
        let request = build_service_password_patch_request(&ServiceResetPasswordOptions {
            new_password_hash: Some("sha256".to_string()),
            new_double_sha1_hash: Some("sha1".to_string()),
            org_id: None,
        });

        assert_eq!(request.new_password_hash.as_deref(), Some("sha256"));
        assert_eq!(request.new_double_sha1_hash.as_deref(), Some("sha1"));
    }

    #[test]
    fn build_query_endpoint_create_request_supports_minimal_fields() {
        let request =
            build_query_endpoint_create_request(&QueryEndpointCreateOptions::default()).unwrap();

        assert!(request.roles.is_empty());
        assert!(request.open_api_keys.is_empty());
        assert_eq!(request.allowed_origins, "*");
    }

    #[test]
    fn build_query_endpoint_create_request_supports_maximal_fields() {
        let request = build_query_endpoint_create_request(&QueryEndpointCreateOptions {
            roles: vec![
                "sql_console_read_only".to_string(),
                "sql_console_admin".to_string(),
            ],
            open_api_keys: vec!["key-1".to_string(), "key-2".to_string()],
            allowed_origins: Some("https://example.com".to_string()),
            org_id: None,
        })
        .unwrap();

        assert_eq!(
            request.roles,
            vec![
                QueryEndpointRole::SqlConsoleReadOnly,
                QueryEndpointRole::SqlConsoleAdmin,
            ]
        );
        assert_eq!(request.open_api_keys, vec!["key-1", "key-2"]);
        assert_eq!(request.allowed_origins, "https://example.com");
    }

    #[test]
    fn build_private_endpoint_create_request_supports_minimal_fields() {
        let request = build_private_endpoint_create_request("vpce-0123456789abcdef0", None);

        assert_eq!(request.id, "vpce-0123456789abcdef0");
        assert!(request.description.is_empty());
    }

    #[test]
    fn build_private_endpoint_create_request_supports_maximal_fields() {
        let request =
            build_private_endpoint_create_request("vpce-0123456789abcdef0", Some("production"));

        assert_eq!(request.id, "vpce-0123456789abcdef0");
        assert_eq!(request.description, "production");
    }

    #[test]
    fn build_service_state_patch_request_preserves_start_and_stop() {
        let start = build_service_state_patch_request(ServiceStatePatchRequestCommand::Start);
        let stop = build_service_state_patch_request(ServiceStatePatchRequestCommand::Stop);

        assert_eq!(start.command, Some(ServiceStatePatchRequestCommand::Start));
        assert_eq!(stop.command, Some(ServiceStatePatchRequestCommand::Stop));
    }

    #[test]
    fn inline_query_only_conflicts_with_bytes_waiting_on_stdin() {
        // The whole matrix of stdin states crossed with --query being present
        // (#641). Only "not a terminal and data is already waiting" conflicts:
        // a terminal must never be read, and a non-terminal empty stdin is the
        // normal shape for CI and for coding agents.
        assert!(!inline_query_conflicts_with_stdin(
            Some("SELECT 1"),
            StdinInput::Terminal
        ));
        assert!(!inline_query_conflicts_with_stdin(
            Some("SELECT 1"),
            StdinInput::Empty
        ));
        assert!(inline_query_conflicts_with_stdin(
            Some("SELECT 1"),
            StdinInput::Pending
        ));

        assert!(!inline_query_conflicts_with_stdin(
            None,
            StdinInput::Terminal
        ));
        assert!(!inline_query_conflicts_with_stdin(None, StdinInput::Empty));
        // Without --query, piped bytes are the SQL itself, not a conflict.
        assert!(!inline_query_conflicts_with_stdin(
            None,
            StdinInput::Pending
        ));
    }

    /// A real unnamed pipe as `(read_fd, write_fd)`.
    fn test_pipe() -> (libc::c_int, libc::c_int) {
        let mut fds = [0 as libc::c_int; 2];
        // SAFETY: `fds` is a two-element array, which is what `pipe` writes.
        let created = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(created, 0, "pipe() failed");
        (fds[0], fds[1])
    }

    fn write_byte(fd: libc::c_int) {
        // SAFETY: one byte is written from a live local buffer.
        let written = unsafe { libc::write(fd, b"x".as_ptr().cast(), 1) };
        assert_eq!(written, 1, "write() failed");
    }

    fn close_fd(fd: libc::c_int) {
        // SAFETY: each fd is closed exactly once by its owning test.
        unsafe { libc::close(fd) };
    }

    #[test]
    fn readiness_waits_for_a_producer_that_has_not_written_its_first_byte() {
        // The #641 race: `cat data.csv | clickhousectl ... --query ...` starts
        // both processes at once, so the CLI can look at stdin before the
        // producer has written. A zero timeout calls that "no input" and sends
        // the empty INSERT, which is the whole bug; the bounded wait sees the
        // byte. This is asserted on a real pipe rather than through a
        // subprocess so the interleaving is not left to process startup.
        let (read_fd, write_fd) = test_pipe();
        assert!(
            !fd_becomes_readable(read_fd, 0),
            "nothing has been written yet"
        );

        let producer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            write_byte(write_fd);
            close_fd(write_fd);
        });

        let started = std::time::Instant::now();
        assert!(fd_becomes_readable(read_fd, STDIN_FIRST_BYTE_TIMEOUT_MS));
        assert!(
            started.elapsed() < Duration::from_millis(STDIN_FIRST_BYTE_TIMEOUT_MS as u64 + 500),
            "the wait must stay bounded"
        );

        producer.join().unwrap();
        close_fd(read_fd);
    }

    #[test]
    fn readiness_gives_up_on_a_pipe_nobody_writes_to() {
        // A harness that wired up stdin and produced nothing must not hang the
        // command: the wait expires and the run continues.
        let (read_fd, write_fd) = test_pipe();

        let started = std::time::Instant::now();
        assert!(!fd_becomes_readable(read_fd, STDIN_FIRST_BYTE_TIMEOUT_MS));
        let waited = started.elapsed();
        assert!(
            waited < Duration::from_millis(STDIN_FIRST_BYTE_TIMEOUT_MS as u64 + 500),
            "waited {waited:?}"
        );

        close_fd(write_fd);
        close_fd(read_fd);
    }

    #[test]
    fn readiness_is_immediate_for_data_that_is_already_waiting() {
        let (read_fd, write_fd) = test_pipe();
        write_byte(write_fd);

        assert!(fd_becomes_readable(read_fd, 0));

        close_fd(write_fd);
        close_fd(read_fd);
    }

    #[test]
    fn the_stdin_conflict_message_says_what_does_work() {
        assert!(
            INLINE_QUERY_WITH_STDIN_ERROR
                .starts_with("--query cannot be combined with SQL or data on stdin.")
        );
        assert!(INLINE_QUERY_WITH_STDIN_ERROR.contains("cat - data.csv"));
        assert!(INLINE_QUERY_WITH_STDIN_ERROR.contains("--queries-file -"));
    }

    #[test]
    fn service_query_help_documents_that_query_ignores_stdin() {
        let error = Cli::try_parse_from(["clickhousectl", "cloud", "service", "query", "--help"])
            .err()
            .expect("--help should stop parsing");
        let help = error.to_string();

        assert!(help.contains("--query never reads stdin"), "{help}");
        assert!(help.contains("cat - data.csv"), "{help}");
        assert!(help.contains("Does not read stdin"), "{help}");
    }
}
