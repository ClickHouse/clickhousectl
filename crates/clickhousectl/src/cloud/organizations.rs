use super::permissions::{Conditional, Declaration as Permission};
use clickhouse_cloud_api::meta::operations as op;

// Declare every API call made by these workflows, including optional lookups.
pub(super) const PERMISSIONS: &[Permission] = &[
    Permission::api("org list", &[&op::ORGANIZATION_GET_LIST]).unscoped(),
    Permission::api("org get", &[&op::ORGANIZATION_GET]),
    Permission::api("org balance", &[&op::CREDIT_BALANCES_GET]),
    Permission::api("org update", &[&op::ORGANIZATION_UPDATE]),
    Permission::api("org prometheus", &[&op::ORGANIZATION_PROMETHEUS_GET]),
    Permission::api(
        "org prometheus discovery",
        &[&op::ORGANIZATION_PROMETHEUS_DISCOVERY_GET],
    ),
    Permission::api("org usage", &[&op::USAGE_COST_GET]),
    Permission::api("org quota list", &[&op::ORGANIZATION_QUOTAS_GET_LIST]),
    Permission::api("org quota get", &[&op::ORGANIZATION_QUOTA_GET]),
    Permission::api(
        "org byoc create",
        &[&op::ORGANIZATION_BYOC_INFRASTRUCTURE_CREATE],
    ),
    Permission::api(
        "org byoc update",
        &[&op::ORGANIZATION_BYOC_INFRASTRUCTURE_UPDATE],
    )
    .when(&[Conditional::flag("name", &[&op::ORGANIZATION_GET])]),
    Permission::api(
        "org byoc delete",
        &[&op::ORGANIZATION_BYOC_INFRASTRUCTURE_DELETE],
    )
    .when(&[Conditional::flag("name", &[&op::ORGANIZATION_GET])]),
    Permission::api("org role list", &[&op::ORGANIZATION_ROLES_GET_LIST]),
    Permission::api("org role get", &[&op::ORGANIZATION_ROLE_GET]).when(&[Conditional::flag(
        "name",
        &[&op::ORGANIZATION_ROLES_GET_LIST],
    )]),
    Permission::api("org role create", &[&op::ORGANIZATION_ROLE_POST]),
    Permission::api("org role update", &[&op::ORGANIZATION_ROLE_PATCH]).when(&[Conditional::flag(
        "name",
        &[&op::ORGANIZATION_ROLES_GET_LIST],
    )]),
    Permission::api("org role delete", &[&op::ORGANIZATION_ROLE_DELETE]).when(&[
        Conditional::flag("name", &[&op::ORGANIZATION_ROLES_GET_LIST]),
    ]),
    Permission::api("member list", &[&op::MEMBER_GET_LIST]),
    Permission::api("member get", &[&op::MEMBER_GET])
        .when(&[Conditional::flag("email", &[&op::MEMBER_GET_LIST])]),
    Permission::api("member update", &[&op::MEMBER_UPDATE])
        .when(&[Conditional::flag("email", &[&op::MEMBER_GET_LIST])]),
    Permission::api("member remove", &[&op::MEMBER_DELETE])
        .when(&[Conditional::flag("email", &[&op::MEMBER_GET_LIST])]),
    Permission::api("invitation list", &[&op::INVITATION_GET_LIST]),
    Permission::api("invitation create", &[&op::INVITATION_CREATE]),
    Permission::api("invitation get", &[&op::INVITATION_GET])
        .when(&[Conditional::flag("email", &[&op::INVITATION_GET_LIST])]),
    Permission::api("invitation delete", &[&op::INVITATION_DELETE])
        .when(&[Conditional::flag("email", &[&op::INVITATION_GET_LIST])]),
];

use crate::cloud::client::{CloudClient, CloudError, ResourceLookup, Result as CloudResult};
use crate::cloud::config::read_typed_config;
use crate::cloud::output::{ABSENT, or_absent, print_human};
use crate::cloud::shared::{EmailSelector, NameSelector, NamedResource};
use crate::cloud::shared::{parse_date_only, parse_tag_filter, resolve_org_id};
use crate::cloud::types::DeleteResponse;
use clap::{Args, Subcommand};
use clickhouse_cloud_api::models::{
    ByocAvailabilityZoneSuffix, ByocInfrastructureDetails, ByocInfrastructurePatchRequest,
    ByocInfrastructurePostRequest, ByocInfrastructurePostRequestRegionid,
    ByocInfrastructureProgress, ByocInfrastructureTags, ByocInfrastructureValidatePostRequest,
    ByocInfrastructureValidatePostRequestRegionid, ByocInfrastructureValidation,
    InvitationPostRequest, MemberPatchRequest, OrganizationPatchPrivateEndpoint,
    OrganizationPatchPrivateEndpointCloudprovider, OrganizationPatchPrivateEndpointRegion,
    OrganizationPatchRequest, OrganizationPrivateEndpointsPatch, RBACPolicyCreateRequest,
    RBACPolicyCreateRequestAllowdeny, RBACPolicyTagsRolev2, RoleCreateRequest, RoleUpdateRequest,
};
use tabled::{Table, Tabled, settings::Style};

#[derive(Subcommand)]
pub enum OrgCommands {
    /// List organizations
    List,

    /// Get organization details
    Get,

    /// View organization quotas (Beta)
    Quota {
        #[command(subcommand)]
        command: QuotaCommands,
    },

    /// View active credit balances (Beta)
    Balance,

    /// Manage BYOC infrastructure
    Byoc {
        #[command(subcommand)]
        command: ByocCommands,
    },

    /// Manage organization roles
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Role IDs from `list` can be used with member, invitation, and API key commands.
  Create/update read JSON request bodies from --file; `-` reads stdin.
  Only custom roles can be updated or deleted.")]
    Role {
        #[command(subcommand)]
        command: RoleCommands,
    },

    /// Update organization settings
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Only the flags you pass change; everything else is left as-is.
  This can only remove private endpoints; add them with `cloud service update --add-private-endpoint-id`.")]
    Update {
        /// New organization name
        #[arg(long = "new-name", id = "new_name")]
        name: Option<String>,

        /// Remove a private endpoint from the org allow list (repeatable)
        ///
        /// Format: id[,description=TEXT],cloud-provider=aws|gcp|azure,region=REGION
        #[arg(
            long = "remove-private-endpoint",
            value_parser = parse_org_private_endpoint_remove_arg
        )]
        remove_private_endpoint: Vec<String>,

        /// Enable or disable core dump collection at the organization level
        #[arg(long)]
        enable_core_dumps: Option<bool>,
    },

    /// Get organization Prometheus configuration
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  With no subcommand, prints raw metrics text from the legacy endpoint; --json is ignored.
  Use `discovery` for Prometheus HTTP service-discovery target groups.")]
    Prometheus {
        #[command(subcommand)]
        command: Option<PrometheusCommands>,

        /// Return the reduced (filtered) metric set
        #[arg(long, global = true)]
        filtered_metrics: Option<bool>,
    },

    /// Get organization usage/billing information
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  The date range is inclusive and may span at most 31 days; longer ranges are rejected.
  Costs are in CHC (ClickHouse Credits), one row per entity per day plus a grand total.")]
    Usage {
        /// Report start date in UTC (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date_only)]
        from_date: String,

        /// Report end date in UTC, inclusive (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date_only)]
        to_date: String,

        /// Filter by resource tag: `tag:KEY=VALUE` or `tag:KEY` (repeatable)
        #[arg(long, value_parser = parse_tag_filter)]
        filter: Vec<String>,
    },
}

impl OrgCommands {
    pub fn is_write(&self) -> bool {
        match self {
            OrgCommands::List => false,
            OrgCommands::Get => false,
            OrgCommands::Quota { .. } => false,
            OrgCommands::Balance => false,
            OrgCommands::Byoc { command } => command.is_write(),
            OrgCommands::Role { command } => command.is_write(),
            OrgCommands::Prometheus { .. } => false,
            OrgCommands::Usage { .. } => false,
            OrgCommands::Update { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum RoleCommands {
    /// List organization roles
    List,

    /// Get organization role details
    Get {
        /// Role ID (from `cloud org role list`)
        #[command(flatten)]
        role_id: NameSelector,
    },

    /// Create a custom organization role
    Create {
        /// JSON request body path, or `-` for stdin
        #[arg(
            long = "file",
            alias = "config-file",
            value_name = "PATH",
            required = true
        )]
        config_file: String,
    },

    /// Update a custom organization role
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Only fields present in the JSON body change; actors and policies replace their whole lists.")]
    Update {
        /// Role ID (from `cloud org role list`)
        #[command(flatten)]
        role_id: NameSelector,

        /// JSON request body path, or `-` for stdin
        #[arg(
            long = "file",
            alias = "config-file",
            value_name = "PATH",
            required = true
        )]
        config_file: String,
    },

    /// Delete a custom organization role
    Delete {
        /// Role ID (from `cloud org role list`)
        #[command(flatten)]
        role_id: NameSelector,
    },
}

impl RoleCommands {
    fn is_write(&self) -> bool {
        match self {
            Self::List | Self::Get { .. } => false,
            Self::Create { .. } | Self::Update { .. } | Self::Delete { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum ByocCommands {
    /// Get BYOC infrastructure details
    Get {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,
    },

    /// Get BYOC infrastructure provisioning progress (Beta)
    Progress {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,
    },

    /// Validate cloud account readiness for BYOC (Beta)
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Preflight only: simulates the cloud permissions ClickHouse needs and creates nothing.
  Requires API key auth, like `create`; OAuth is read-only.
  Takes the same flags as `cloud org byoc create`; a passing payload is accepted by create.
  Exits 1 after printing the result when any check is denied.
  `supported: false` means nothing was verified, not that everything passed.")]
    Validate {
        #[command(flatten)]
        infrastructure: Box<ByocInfrastructureArgs>,
    },

    /// Create BYOC infrastructure
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Preflight the same flags with `cloud org byoc validate` before creating.
  Watch provisioning with `cloud org byoc progress <id>`.
  Wait for `cloud org byoc get <id>` to show state `infra-ready` before creating a service.
  Discover profiles with `cloud service profile list --region <region> --byoc-id <id>`.")]
    Create {
        #[command(flatten)]
        infrastructure: Box<ByocInfrastructureArgs>,

        /// Human-readable infrastructure name
        #[arg(long)]
        display_name: Option<String>,
    },

    /// Update BYOC infrastructure
    Update {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,

        /// New human-readable infrastructure name
        #[arg(long = "new-name", alias = "display-name")]
        display_name: String,
    },

    /// Delete BYOC infrastructure
    Delete {
        /// BYOC infrastructure ID
        #[command(flatten)]
        byoc_id: NameSelector,
    },
}

/// Infrastructure flags shared by `cloud org byoc create` and `validate`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct ByocInfrastructureArgs {
    /// Cloud region ID
    #[arg(long)]
    region: String,

    /// Cloud account ID: AWS account, GCP project, or Azure subscription
    #[arg(long)]
    account_id: String,

    /// Availability-zone suffix (repeatable)
    #[arg(long)]
    availability_zone_suffix: Vec<String>,

    /// CIDR range for a ClickHouse-managed VPC; not with BYO-VPC flags
    #[arg(
        long,
        conflicts_with_all = ["vpc_id", "private_subnet_id", "public_subnet_id"]
    )]
    vpc_cidr_range: Option<String>,

    /// BYO-VPC ID or network name (AWS, GCP); requires --private-subnet-id
    #[arg(long, requires = "private_subnet_id")]
    vpc_id: Option<String>,

    /// BYO-VPC private subnet ID or name (repeatable; AWS 1-6, GCP 1)
    #[arg(long)]
    private_subnet_id: Vec<String>,

    /// AWS BYO-VPC public subnet ID (repeatable; at most 6)
    #[arg(long)]
    public_subnet_id: Vec<String>,

    /// AWS ExternalID in the ClickHouse management role trust policy
    #[arg(long)]
    external_id: Option<String>,

    /// GCP BYO-VPC secondary range name for pod IPs (repeatable)
    #[arg(long)]
    gcp_pod_cidr_range_name: Vec<String>,

    /// GCP Shared VPC host project, when it differs from --account-id
    #[arg(long)]
    gcp_shared_vpc_host_project_id: Option<String>,

    /// Azure Entra tenant ID; required for Azure regions
    #[arg(long)]
    tenant_id: Option<String>,

    /// Azure service principal client ID; required for Azure regions
    #[arg(long)]
    service_principal_client_id: Option<String>,

    /// Tag for the infrastructure's cloud resources (repeatable)
    #[arg(long = "tag", value_name = "KEY=VALUE")]
    tags: Vec<String>,
}

impl ByocCommands {
    fn is_write(&self) -> bool {
        match self {
            ByocCommands::Get { .. } => false,
            ByocCommands::Progress { .. } => false,
            // A preflight that creates nothing, but the endpoint requires the
            // organization manage scope, which OAuth (read-only) never has.
            ByocCommands::Validate { .. } => true,
            ByocCommands::Create { .. } => true,
            ByocCommands::Update { .. } => true,
            ByocCommands::Delete { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum PrometheusCommands {
    /// List Prometheus scrape targets
    Discovery,
}

#[derive(Subcommand)]
pub enum QuotaCommands {
    /// List organization quotas
    List,

    /// Get organization quota details
    Get {
        /// Quota code
        quota_code: String,
    },
}

#[derive(Subcommand)]
pub enum MemberCommands {
    /// List organization members
    List,

    /// Get member details
    Get {
        /// User ID
        #[command(flatten)]
        user_id: EmailSelector,
    },

    /// Update member roles
    Update {
        /// User ID
        #[command(flatten)]
        user_id: EmailSelector,

        /// Role ID to assign (repeatable; conflicts with --clear-roles)
        #[arg(long, conflicts_with = "clear_roles")]
        role_id: Vec<String>,

        /// Remove all assigned roles; conflicts with --role-id
        #[arg(long, conflicts_with = "role_id")]
        clear_roles: bool,
    },

    /// Remove a member from the organization
    Remove {
        /// User ID
        #[command(flatten)]
        user_id: EmailSelector,
    },
}

impl MemberCommands {
    pub fn is_write(&self) -> bool {
        match self {
            MemberCommands::List => false,
            MemberCommands::Get { .. } => false,
            MemberCommands::Update { .. } => true,
            MemberCommands::Remove { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum InvitationCommands {
    /// List pending invitations
    List,

    /// Create an invitation
    Create {
        /// Email address to invite (stored lowercased)
        #[arg(long)]
        email: String,

        /// Role ID to assign (repeatable)
        #[arg(long)]
        role_id: Vec<String>,
    },

    /// Get invitation details
    Get {
        /// Invitation ID
        #[command(flatten)]
        invitation_id: EmailSelector,
    },

    /// Delete an invitation
    Delete {
        /// Invitation ID
        #[command(flatten)]
        invitation_id: EmailSelector,
    },
}

impl InvitationCommands {
    pub fn is_write(&self) -> bool {
        match self {
            InvitationCommands::List => false,
            InvitationCommands::Get { .. } => false,
            InvitationCommands::Create { .. } => true,
            InvitationCommands::Delete { .. } => true,
        }
    }
}

pub async fn run_org(client: &CloudClient, command: OrgCommands, json: bool) -> CloudResult<()> {
    match command {
        OrgCommands::List => org_list(client, json).await,
        OrgCommands::Get => {
            let org_id = resolve_org_id(client).await?;
            org_get(client, &org_id, json).await
        }
        OrgCommands::Quota { command } => run_quota(client, command, json).await,
        OrgCommands::Balance => org_balance(client, json).await,
        OrgCommands::Byoc { command } => run_byoc(client, command, json).await,
        OrgCommands::Role { command } => run_role(client, command, json).await,
        OrgCommands::Update {
            name,
            remove_private_endpoint,
            enable_core_dumps,
        } => {
            let options = OrgUpdateOptions {
                name,
                remove_private_endpoints: remove_private_endpoint,
                enable_core_dumps,
            };
            org_update(client, options, json).await
        }
        OrgCommands::Prometheus {
            command,
            filtered_metrics,
        } => match command {
            Some(PrometheusCommands::Discovery) => {
                org_prometheus_discovery(client, filtered_metrics, json).await
            }
            None => org_prometheus(client, filtered_metrics, json).await,
        },
        OrgCommands::Usage {
            from_date,
            to_date,
            filter,
        } => org_usage(client, &from_date, &to_date, &filter, json).await,
    }
}

async fn run_role(client: &CloudClient, command: RoleCommands, json: bool) -> CloudResult<()> {
    match command {
        RoleCommands::List => role_list(client, json).await,
        RoleCommands::Get { role_id } => {
            role_get(
                client,
                &role_id.resolve(client, NamedResource::Role).await?,
                json,
            )
            .await
        }
        RoleCommands::Create { config_file } => {
            let request = build_role_create_request(&config_file)?;
            role_create(client, request, json).await
        }
        RoleCommands::Update {
            role_id,
            config_file,
        } => {
            let request = build_role_update_request(&config_file)?;
            role_update(
                client,
                &role_id.resolve(client, NamedResource::Role).await?,
                request,
                json,
            )
            .await
        }
        RoleCommands::Delete { role_id } => {
            role_delete(
                client,
                &role_id.resolve(client, NamedResource::Role).await?,
                json,
            )
            .await
        }
    }
}

async fn run_byoc(client: &CloudClient, command: ByocCommands, json: bool) -> CloudResult<()> {
    match command {
        ByocCommands::Get { byoc_id } => {
            byoc_get(
                client,
                &byoc_id.resolve(client, NamedResource::Byoc).await?,
                json,
            )
            .await
        }
        ByocCommands::Progress { byoc_id } => {
            byoc_progress(
                client,
                &byoc_id.resolve(client, NamedResource::Byoc).await?,
                json,
            )
            .await
        }
        ByocCommands::Validate { infrastructure } => {
            let request = build_byoc_validate_request(&infrastructure)?;
            byoc_validate(client, request, json).await
        }
        ByocCommands::Create {
            infrastructure,
            display_name,
        } => {
            let request = build_byoc_create_request(&infrastructure, display_name.as_deref())?;
            byoc_create(client, request, json).await
        }
        ByocCommands::Update {
            byoc_id,
            display_name,
        } => {
            let request = build_byoc_update_request(&display_name);
            byoc_update(
                client,
                &byoc_id.resolve(client, NamedResource::Byoc).await?,
                request,
                json,
            )
            .await
        }
        ByocCommands::Delete { byoc_id } => {
            byoc_delete(
                client,
                &byoc_id.resolve(client, NamedResource::Byoc).await?,
                json,
            )
            .await
        }
    }
}

async fn run_quota(client: &CloudClient, command: QuotaCommands, json: bool) -> CloudResult<()> {
    match command {
        QuotaCommands::List => quota_list(client, json).await,
        QuotaCommands::Get { quota_code } => quota_get(client, &quota_code, json).await,
    }
}

pub async fn run_member(
    client: &CloudClient,
    command: MemberCommands,
    json: bool,
) -> CloudResult<()> {
    match command {
        MemberCommands::List => member_list(client, json).await,
        MemberCommands::Get { user_id } => {
            member_get(client, &user_id.resolve_member(client).await?, json).await
        }
        MemberCommands::Update {
            user_id,
            role_id,
            clear_roles,
        } => {
            member_update(
                client,
                &user_id.resolve_member(client).await?,
                &role_id,
                clear_roles,
                json,
            )
            .await
        }
        MemberCommands::Remove { user_id } => {
            member_remove(client, &user_id.resolve_member(client).await?, json).await
        }
    }
}

pub async fn run_invitation(
    client: &CloudClient,
    command: InvitationCommands,
    json: bool,
) -> CloudResult<()> {
    match command {
        InvitationCommands::List => invitation_list(client, json).await,
        InvitationCommands::Create { email, role_id } => {
            invitation_create(client, &email, &role_id, json).await
        }
        InvitationCommands::Get { invitation_id } => {
            invitation_get(
                client,
                &invitation_id.resolve_invitation(client).await?,
                json,
            )
            .await
        }
        InvitationCommands::Delete { invitation_id } => {
            invitation_delete(
                client,
                &invitation_id.resolve_invitation(client).await?,
                json,
            )
            .await
        }
    }
}

#[derive(Default)]
struct OrgUpdateOptions {
    name: Option<String>,
    remove_private_endpoints: Vec<String>,
    enable_core_dumps: Option<bool>,
}

fn parse_org_private_endpoint_remove(value: &str) -> CloudResult<OrganizationPatchPrivateEndpoint> {
    let mut id = String::new();
    let mut description = None;
    let mut cloud_provider = None;
    let mut region = None;

    for (index, part) in value.split(',').enumerate() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if index == 0 && !part.contains('=') {
            id = part.to_string();
            continue;
        }

        let (key, raw_value) = part.split_once('=').ok_or_else(|| {
            CloudError::new(format!(
                "invalid remove-private-endpoint segment '{}'",
                part
            ))
        })?;

        match key {
            "id" => id = raw_value.to_string(),
            "description" => description = Some(raw_value.to_string()),
            "cloud-provider" => {
                if raw_value.trim().is_empty() {
                    return Err(CloudError::new(format!(
                        "remove-private-endpoint '{}' requires a non-empty cloud-provider",
                        value
                    )));
                }
                cloud_provider = Some(
                    serde_json::from_value::<OrganizationPatchPrivateEndpointCloudprovider>(
                        serde_json::Value::String(raw_value.to_string()),
                    )
                    .expect("enum with Unknown variant should always deserialize"),
                );
            }
            "region" => {
                if raw_value.trim().is_empty() {
                    return Err(CloudError::new(format!(
                        "remove-private-endpoint '{}' requires a non-empty region",
                        value
                    )));
                }
                region = Some(
                    serde_json::from_value::<OrganizationPatchPrivateEndpointRegion>(
                        serde_json::Value::String(raw_value.to_string()),
                    )
                    .expect("enum with Unknown variant should always deserialize"),
                );
            }
            _ => {
                return Err(CloudError::new(format!(
                    "invalid remove-private-endpoint key '{}'; expected id, description, cloud-provider, or region",
                    key
                )));
            }
        }
    }

    if id.trim().is_empty() {
        return Err(CloudError::new(format!(
            "remove-private-endpoint '{}' requires a non-empty id",
            value
        )));
    }

    let (cloud_provider, region) = match (cloud_provider, region) {
        (Some(cloud_provider), Some(region)) => (cloud_provider, region),
        (None, None) => {
            return Err(CloudError::new(format!(
                "remove-private-endpoint '{}' requires cloud-provider and region",
                value
            )));
        }
        (None, Some(_)) => {
            return Err(CloudError::new(format!(
                "remove-private-endpoint '{}' requires cloud-provider",
                value
            )));
        }
        (Some(_), None) => {
            return Err(CloudError::new(format!(
                "remove-private-endpoint '{}' requires region",
                value
            )));
        }
    };

    Ok(OrganizationPatchPrivateEndpoint {
        id,
        description,
        cloud_provider,
        region,
    })
}

/// Validate endpoint removals during clap parsing so incomplete endpoint
/// identities fail as usage errors before credentials or networking are used.
fn parse_org_private_endpoint_remove_arg(value: &str) -> Result<String, String> {
    parse_org_private_endpoint_remove(value)
        .map(|_| value.to_string())
        .map_err(|error| error.message)
}

fn parse_org_private_endpoints_patch(
    remove: &[String],
) -> CloudResult<Option<OrganizationPrivateEndpointsPatch>> {
    if remove.is_empty() {
        return Ok(None);
    }

    let endpoints = remove
        .iter()
        .map(|value| parse_org_private_endpoint_remove(value))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(OrganizationPrivateEndpointsPatch {
        #[cfg(feature = "deprecated-fields")]
        add: None,
        remove: endpoints,
    }))
}

fn build_org_update_request(options: &OrgUpdateOptions) -> CloudResult<OrganizationPatchRequest> {
    Ok(OrganizationPatchRequest {
        name: options.name.clone(),
        private_endpoints: parse_org_private_endpoints_patch(&options.remove_private_endpoints)?,
        enable_core_dumps: options.enable_core_dumps,
    })
}

/// Parse a known value of a generated BYOC enum, rejecting values that only
/// deserialize into its `Unknown` catch-all.
fn parse_known_byoc_value<T: serde::de::DeserializeOwned>(
    field: &str,
    unknown: &str,
    value: &str,
    is_unknown: fn(&T) -> bool,
) -> CloudResult<T> {
    let parsed = serde_json::from_value::<T>(serde_json::Value::String(value.to_string()))
        .map_err(|error| CloudError::new(format!("invalid {field}: {error}")))?;
    if is_unknown(&parsed) {
        return Err(CloudError::new(format!(
            "invalid {field}: {unknown} '{value}'"
        )));
    }
    Ok(parsed)
}

fn parse_byoc_region(value: &str) -> CloudResult<ByocInfrastructurePostRequestRegionid> {
    parse_known_byoc_value("region", "unsupported BYOC region", value, |region| {
        matches!(region, ByocInfrastructurePostRequestRegionid::Unknown(_))
    })
}

fn parse_byoc_validate_region(
    value: &str,
) -> CloudResult<ByocInfrastructureValidatePostRequestRegionid> {
    parse_known_byoc_value("region", "unsupported BYOC region", value, |region| {
        matches!(
            region,
            ByocInfrastructureValidatePostRequestRegionid::Unknown(_)
        )
    })
}

fn parse_byoc_availability_zone_suffix(value: &str) -> CloudResult<ByocAvailabilityZoneSuffix> {
    parse_known_byoc_value(
        "availability zone suffix",
        "unsupported suffix",
        value,
        |suffix| matches!(suffix, ByocAvailabilityZoneSuffix::Unknown(_)),
    )
}

/// Parse repeatable `--tag KEY=VALUE` values into the BYOC tag map. Values
/// may be empty or contain `=`; keys must be nonempty and unique.
fn parse_byoc_tags(values: &[String]) -> CloudResult<Option<ByocInfrastructureTags>> {
    if values.is_empty() {
        return Ok(None);
    }
    let mut tags = ByocInfrastructureTags::new();
    for raw in values {
        let Some((key, value)) = raw.split_once('=') else {
            return Err(CloudError::usage(format!(
                "invalid tag '{raw}': expected KEY=VALUE"
            )));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(CloudError::usage(format!(
                "invalid tag '{raw}': tag key cannot be empty"
            )));
        }
        if tags.insert(key.to_string(), value.to_string()).is_some() {
            return Err(CloudError::usage(format!(
                "invalid tag '{raw}': duplicate tag key '{key}'"
            )));
        }
    }
    Ok(Some(tags))
}

fn non_empty(values: &[String]) -> Option<Vec<String>> {
    (!values.is_empty()).then(|| values.to_vec())
}

/// The region-independent part of a BYOC create/validate body, parsed once so
/// both requests reject the same inputs the same way.
struct ByocInfrastructureFields {
    account_id: String,
    availability_zone_suffixes: Option<Vec<ByocAvailabilityZoneSuffix>>,
    external_id: Option<String>,
    gcp_pod_cidr_range_names: Option<Vec<String>>,
    gcp_shared_vpc_host_project_id: Option<String>,
    private_subnet_ids: Option<Vec<String>>,
    public_subnet_ids: Option<Vec<String>>,
    service_principal_client_id: Option<String>,
    tags: Option<ByocInfrastructureTags>,
    tenant_id: Option<String>,
    vpc_cidr_range: Option<String>,
    vpc_id: Option<String>,
}

fn parse_byoc_infrastructure_fields(
    args: &ByocInfrastructureArgs,
) -> CloudResult<ByocInfrastructureFields> {
    let availability_zone_suffixes = args
        .availability_zone_suffix
        .iter()
        .map(|suffix| parse_byoc_availability_zone_suffix(suffix))
        .collect::<CloudResult<Vec<_>>>()?;
    Ok(ByocInfrastructureFields {
        account_id: args.account_id.clone(),
        availability_zone_suffixes: (!availability_zone_suffixes.is_empty())
            .then_some(availability_zone_suffixes),
        external_id: args.external_id.clone(),
        gcp_pod_cidr_range_names: non_empty(&args.gcp_pod_cidr_range_name),
        gcp_shared_vpc_host_project_id: args.gcp_shared_vpc_host_project_id.clone(),
        private_subnet_ids: non_empty(&args.private_subnet_id),
        public_subnet_ids: non_empty(&args.public_subnet_id),
        service_principal_client_id: args.service_principal_client_id.clone(),
        tags: parse_byoc_tags(&args.tags)?,
        tenant_id: args.tenant_id.clone(),
        vpc_cidr_range: args.vpc_cidr_range.clone(),
        vpc_id: args.vpc_id.clone(),
    })
}

fn build_byoc_create_request(
    args: &ByocInfrastructureArgs,
    display_name: Option<&str>,
) -> CloudResult<ByocInfrastructurePostRequest> {
    let region_id = parse_byoc_region(&args.region)?;
    let fields = parse_byoc_infrastructure_fields(args)?;
    Ok(ByocInfrastructurePostRequest {
        account_id: fields.account_id,
        availability_zone_suffixes: fields.availability_zone_suffixes,
        display_name: display_name.map(str::to_string),
        external_id: fields.external_id,
        gcp_pod_cidr_range_names: fields.gcp_pod_cidr_range_names,
        gcp_shared_vpc_host_project_id: fields.gcp_shared_vpc_host_project_id,
        private_subnet_ids: fields.private_subnet_ids,
        public_subnet_ids: fields.public_subnet_ids,
        region_id,
        service_principal_client_id: fields.service_principal_client_id,
        tags: fields.tags,
        tenant_id: fields.tenant_id,
        vpc_cidr_range: fields.vpc_cidr_range,
        vpc_id: fields.vpc_id,
    })
}

fn build_byoc_validate_request(
    args: &ByocInfrastructureArgs,
) -> CloudResult<ByocInfrastructureValidatePostRequest> {
    let region_id = parse_byoc_validate_region(&args.region)?;
    let fields = parse_byoc_infrastructure_fields(args)?;
    Ok(ByocInfrastructureValidatePostRequest {
        account_id: fields.account_id,
        availability_zone_suffixes: fields.availability_zone_suffixes,
        external_id: fields.external_id,
        gcp_pod_cidr_range_names: fields.gcp_pod_cidr_range_names,
        gcp_shared_vpc_host_project_id: fields.gcp_shared_vpc_host_project_id,
        private_subnet_ids: fields.private_subnet_ids,
        public_subnet_ids: fields.public_subnet_ids,
        region_id,
        service_principal_client_id: fields.service_principal_client_id,
        tags: fields.tags,
        tenant_id: fields.tenant_id,
        vpc_cidr_range: fields.vpc_cidr_range,
        vpc_id: fields.vpc_id,
    })
}

fn build_byoc_update_request(display_name: &str) -> ByocInfrastructurePatchRequest {
    ByocInfrastructurePatchRequest {
        display_name: Some(display_name.to_string()),
    }
}

fn invalid_role_request(source: &str, message: impl std::fmt::Display) -> CloudError {
    CloudError::new(format!(
        "invalid request body in config {source}: {message}"
    ))
}

fn validate_role_policies(policies: &[RBACPolicyCreateRequest], source: &str) -> CloudResult<()> {
    for (index, policy) in policies.iter().enumerate() {
        if let RBACPolicyCreateRequestAllowdeny::Unknown(value) = &policy.allow_deny {
            return Err(invalid_role_request(
                source,
                format!("unknown policies[{index}].allowDeny `{value}`; expected ALLOW or DENY"),
            ));
        }
        if let Some(tags) = &policy.tags
            && let Some(RBACPolicyTagsRolev2::Unknown(value)) = &tags.role_v2
        {
            return Err(invalid_role_request(
                source,
                format!(
                    "unknown policies[{index}].tags.roleV2 `{value}`; expected sql-console-readonly or sql-console-admin"
                ),
            ));
        }
    }
    Ok(())
}

fn build_role_create_request(config_file: &str) -> CloudResult<RoleCreateRequest> {
    let request: RoleCreateRequest = read_typed_config(config_file)?;
    validate_role_policies(&request.policies, config_file)?;
    Ok(request)
}

fn build_role_update_request(config_file: &str) -> CloudResult<RoleUpdateRequest> {
    let request: RoleUpdateRequest = read_typed_config(config_file)?;
    if let Some(policies) = &request.policies {
        validate_role_policies(policies, config_file)?;
    }
    Ok(request)
}

fn build_member_update_request(role_ids: &[String], clear_roles: bool) -> MemberPatchRequest {
    MemberPatchRequest {
        assigned_role_ids: if clear_roles {
            Some(Vec::new())
        } else if role_ids.is_empty() {
            None
        } else {
            Some(role_ids.to_vec())
        },
        #[cfg(feature = "deprecated-fields")]
        role: None,
    }
}

fn build_invitation_create_request(email: &str, role_ids: &[String]) -> InvitationPostRequest {
    InvitationPostRequest {
        email: email.to_string(),
        assigned_role_ids: role_ids.to_vec(),
        #[cfg(feature = "deprecated-fields")]
        role: None,
    }
}

fn join_absent<T>(items: Option<&[T]>, render: impl Fn(&T) -> String) -> String {
    match items {
        Some(items) => items.iter().map(render).collect::<Vec<_>>().join(", "),
        None => ABSENT.to_string(),
    }
}

async fn org_list(client: &CloudClient, json: bool) -> CloudResult<()> {
    let orgs = client.list_organizations().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&orgs)?);
    } else {
        if orgs.is_empty() {
            println!("No organizations found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "ID")]
            id: String,
        }
        let rows: Vec<Row> = orgs
            .into_iter()
            .map(|organization| Row {
                name: or_absent(organization.name.as_deref()),
                id: or_absent(organization.id),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

async fn org_get(client: &CloudClient, org_id: &str, json: bool) -> CloudResult<()> {
    let organization = client.get_organization(org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&organization)?);
    } else {
        print_human(&organization)?;
    }
    Ok(())
}

async fn quota_list(client: &CloudClient, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let quotas = client.list_organization_quotas(&org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&quotas)?);
    } else {
        if quotas.is_empty() {
            println!("No organization quotas found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "Code")]
            code: String,
            #[tabled(rename = "Scope")]
            scope: String,
            #[tabled(rename = "Usage")]
            usage: String,
            #[tabled(rename = "Limit")]
            limit: String,
            #[tabled(rename = "Adjustable")]
            adjustable: String,
        }
        let rows: Vec<Row> = quotas
            .into_iter()
            .map(|quota| Row {
                name: or_absent(quota.name.as_deref()),
                code: or_absent(quota.quota_code.as_ref()),
                scope: or_absent(quota.scope.as_ref()),
                usage: or_absent(quota.usage),
                limit: or_absent(quota.value),
                adjustable: or_absent(quota.adjustable),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

async fn quota_get(client: &CloudClient, quota_code: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let quota = client.get_organization_quota(&org_id, quota_code).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&quota)?);
    } else {
        print_human(&quota)?;
    }
    Ok(())
}

async fn org_balance(client: &CloudClient, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let credit_balances = client.get_credit_balances(&org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&credit_balances)?);
    } else {
        println!(
            "Total remaining credits: {} CHC",
            or_absent(credit_balances.total_remaining_credits)
        );
        let balances = credit_balances.balances.unwrap_or_default();
        if balances.is_empty() {
            println!("No active credit balances found");
            return Ok(());
        }

        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Type")]
            balance_type: String,
            #[tabled(rename = "Remaining (CHC)")]
            remaining: String,
            #[tabled(rename = "Total (CHC)")]
            total: String,
            #[tabled(rename = "Spent (CHC)")]
            spent: String,
            #[tabled(rename = "Start")]
            start: String,
            #[tabled(rename = "Expires")]
            expires: String,
        }
        let rows: Vec<Row> = balances
            .into_iter()
            .map(|balance| Row {
                id: or_absent(balance.id),
                balance_type: or_absent(balance.r#type.as_ref()),
                remaining: or_absent(balance.remaining_credits),
                total: or_absent(balance.total_amount),
                spent: or_absent(balance.amount_spent),
                start: or_absent(balance.start_date),
                expires: or_absent(balance.expiration_date),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

async fn org_update(
    client: &CloudClient,
    options: OrgUpdateOptions,
    json: bool,
) -> CloudResult<()> {
    let request = build_org_update_request(&options)?;
    let org_id = resolve_org_id(client).await?;
    let organization = client.update_organization(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&organization)?);
    } else {
        println!(
            "Organization updated: {} ({})",
            or_absent(organization.name.as_deref()),
            or_absent(organization.id)
        );
    }
    Ok(())
}

async fn byoc_get(client: &CloudClient, byoc_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let infrastructure = client.get_byoc_infrastructure(&org_id, byoc_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&infrastructure)?);
    } else {
        print_human(&infrastructure)?;
    }
    Ok(())
}

async fn byoc_progress(client: &CloudClient, byoc_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let progress = client
        .get_byoc_infrastructure_progress(&org_id, byoc_id)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&progress)?);
    } else {
        print_human(&progress)?;
    }
    Ok(())
}

/// The error a completed validation maps to, or `None` when it passed. Only an
/// explicit `allPassed: false` or an explicitly denied check fails; an
/// unsupported configuration verified nothing and is not a failure.
fn byoc_validation_failure(validation: &ByocInfrastructureValidation) -> Option<CloudError> {
    let checks = validation.checks.as_deref().unwrap_or_default();
    let denied = checks
        .iter()
        .filter(|check| check.allowed == Some(false))
        .count();
    if validation.all_passed != Some(false) && denied == 0 {
        return None;
    }
    Some(CloudError::new(format!(
        "BYOC validation failed: {denied} of {} checks denied",
        checks.len()
    )))
}

async fn byoc_validate(
    client: &CloudClient,
    request: ByocInfrastructureValidatePostRequest,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let validation = client
        .validate_byoc_infrastructure(&org_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&validation)?);
    } else {
        print_human(&validation)?;
        if validation.supported == Some(false) {
            println!(
                "Preflight validation is not supported for this cloud and configuration; nothing was verified."
            );
        }
    }
    match byoc_validation_failure(&validation) {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

async fn byoc_create(
    client: &CloudClient,
    request: ByocInfrastructurePostRequest,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let infrastructure = client.create_byoc_infrastructure(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&infrastructure)?);
    } else {
        print_human(&infrastructure)?;
    }
    Ok(())
}

async fn byoc_update(
    client: &CloudClient,
    byoc_id: &str,
    request: ByocInfrastructurePatchRequest,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let infrastructure = client
        .update_byoc_infrastructure(&org_id, byoc_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&infrastructure)?);
    } else {
        print_human(&infrastructure)?;
    }
    Ok(())
}

async fn byoc_delete(client: &CloudClient, byoc_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.delete_byoc_infrastructure(&org_id, byoc_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("BYOC infrastructure {byoc_id} deleted");
    }
    Ok(())
}

async fn role_list(client: &CloudClient, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let roles = client.list_organization_roles(&org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&roles)?);
    } else {
        if roles.is_empty() {
            println!("No organization roles found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Type")]
            role_type: String,
            #[tabled(rename = "Actors")]
            actors: String,
            #[tabled(rename = "Policies")]
            policies: String,
        }
        let rows = roles
            .into_iter()
            .map(|role| Row {
                name: or_absent(role.name.as_deref()),
                id: or_absent(role.id.as_deref()),
                role_type: or_absent(role.r#type.as_ref()),
                actors: role
                    .actors
                    .as_ref()
                    .map(|actors| actors.len().to_string())
                    .unwrap_or_else(|| ABSENT.to_string()),
                policies: role
                    .policies
                    .as_ref()
                    .map(|policies| policies.len().to_string())
                    .unwrap_or_else(|| ABSENT.to_string()),
            })
            .collect::<Vec<_>>();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

async fn role_get(client: &CloudClient, role_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let role = client.get_organization_role(&org_id, role_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&role)?);
    } else {
        print_human(&role)?;
    }
    Ok(())
}

async fn role_create(
    client: &CloudClient,
    request: RoleCreateRequest,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let role = client.create_organization_role(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&role)?);
    } else {
        print_human(&role)?;
    }
    Ok(())
}

async fn role_update(
    client: &CloudClient,
    role_id: &str,
    request: RoleUpdateRequest,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let role = client
        .update_organization_role(&org_id, role_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&role)?);
    } else {
        print_human(&role)?;
    }
    Ok(())
}

async fn role_delete(client: &CloudClient, role_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.delete_organization_role(&org_id, role_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Organization role {role_id} deleted");
    }
    Ok(())
}

async fn org_prometheus(
    client: &CloudClient,
    filtered_metrics: Option<bool>,
    _json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let prometheus = client.get_org_prometheus(&org_id, filtered_metrics).await?;
    println!("{}", prometheus);
    Ok(())
}

async fn org_prometheus_discovery(
    client: &CloudClient,
    filtered_metrics: Option<bool>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let groups = client
        .discover_org_prometheus_targets(&org_id, filtered_metrics)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&groups)?);
    } else {
        print_human(&groups)?;
    }
    Ok(())
}

async fn org_usage(
    client: &CloudClient,
    from_date: &str,
    to_date: &str,
    filters: &[String],
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let usage = client
        .get_org_usage(&org_id, from_date, to_date, filters)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&usage)?);
    } else {
        println!(
            "Grand Total: {} CHC",
            or_absent(usage.grand_total_chc.map(|total| format!("{total:.2}")))
        );
        let costs = usage.costs.unwrap_or_default();
        if costs.is_empty() {
            println!("No usage cost records found");
            return Ok(());
        }

        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Entity")]
            entity: String,
            #[tabled(rename = "Date")]
            date: String,
            #[tabled(rename = "Total (CHC)")]
            total: String,
        }
        let rows: Vec<Row> = costs
            .iter()
            .map(|cost| Row {
                entity: usage_entity_label(cost.entity_name.as_deref(), cost.entity_id),
                date: or_absent(cost.date.as_deref()),
                total: or_absent(cost.total_chc.map(|total| format!("{total:.2}"))),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

fn usage_entity_label(name: Option<&str>, id: Option<uuid::Uuid>) -> String {
    match (name.filter(|name| !name.is_empty()), id) {
        (Some(name), _) => name.to_string(),
        (None, Some(id)) => format!("{id} (unknown)"),
        (None, None) => ABSENT.to_string(),
    }
}

async fn member_list(client: &CloudClient, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let members = client.list_members(&org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&members)?);
    } else {
        if members.is_empty() {
            println!("No members found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Email")]
            email: String,
            #[tabled(rename = "User ID")]
            user_id: String,
            #[tabled(rename = "Roles")]
            roles: String,
            #[tabled(rename = "Name")]
            name: String,
        }
        let rows: Vec<Row> = members
            .into_iter()
            .map(|member| Row {
                email: or_absent(member.email.as_deref()),
                user_id: or_absent(member.user_id.as_deref()),
                roles: join_absent(member.assigned_roles.as_deref(), |role| {
                    or_absent(role.role_name.as_deref())
                }),
                name: or_absent(member.name.as_deref()),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

async fn member_get(client: &CloudClient, user_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let member = client.get_member(&org_id, user_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&member)?);
    } else {
        print_human(&member)?;
    }
    Ok(())
}

async fn member_update(
    client: &CloudClient,
    user_id: &str,
    role_ids: &[String],
    clear_roles: bool,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let request = build_member_update_request(role_ids, clear_roles);
    let member = client.update_member(&org_id, user_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&member)?);
    } else {
        println!("Member {} updated", or_absent(member.email.as_deref()));
    }
    Ok(())
}

async fn member_remove(client: &CloudClient, user_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.delete_member(&org_id, user_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Member {} removed", user_id);
    }
    Ok(())
}

async fn invitation_list(client: &CloudClient, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let invitations = client.list_invitations(&org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&invitations)?);
    } else {
        if invitations.is_empty() {
            println!("No invitations found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Email")]
            email: String,
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Roles")]
            roles: String,
            #[tabled(rename = "Expires")]
            expires: String,
        }
        let rows: Vec<Row> = invitations
            .into_iter()
            .map(|invitation| Row {
                email: or_absent(invitation.email.as_deref()),
                id: or_absent(invitation.id),
                roles: join_absent(invitation.assigned_roles.as_deref(), |role| {
                    or_absent(role.role_name.as_deref())
                }),
                expires: or_absent(invitation.expire_at.map(|at| at.to_rfc3339())),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

async fn invitation_create(
    client: &CloudClient,
    email: &str,
    role_ids: &[String],
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let request = build_invitation_create_request(email, role_ids);
    let invitation = client.create_invitation(&org_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&invitation)?);
    } else {
        println!(
            "Invitation sent to {} ({})",
            or_absent(invitation.email.as_deref()),
            or_absent(invitation.id)
        );
    }
    Ok(())
}

async fn invitation_get(client: &CloudClient, invitation_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let invitation = client.get_invitation(&org_id, invitation_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&invitation)?);
    } else {
        print_human(&invitation)?;
    }
    Ok(())
}

async fn invitation_delete(
    client: &CloudClient,
    invitation_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.delete_invitation(&org_id, invitation_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Invitation {} deleted", invitation_id);
    }
    Ok(())
}

impl CloudClient {
    pub async fn list_organizations(
        &self,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::Organization>> {
        let response = self
            .api()
            .organization_get_list()
            .await
            .map_err(|error| self.convert_error(error))?;
        Self::unwrap_response(response)
    }

    pub async fn get_organization(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Organization> {
        let response = self.api().organization_get(org_id).await.map_err(|error| {
            // A read by identifier: a 400 over a well-formed UUID is a
            // missing organization, not a bad request (#666).
            self.convert_error_for_lookup(error, ResourceLookup::organization(org_id))
        })?;
        Self::unwrap_response(response)
    }

    pub async fn list_organization_quotas(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::OrganizationQuota>> {
        let response = self
            .api()
            .organization_quotas_get_list(org_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_credit_balances(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::CreditBalances> {
        let response = self
            .api()
            .credit_balances_get(org_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_organization_quota(
        &self,
        org_id: &str,
        quota_code: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::OrganizationQuota> {
        let response = self
            .api()
            .organization_quota_get(org_id, quota_code)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_organization(
        &self,
        org_id: &str,
        request: &OrganizationPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Organization> {
        let response = self
            .api()
            .organization_update(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_byoc_infrastructure(
        &self,
        org_id: &str,
        byoc_id: &str,
    ) -> crate::cloud::client::Result<ByocInfrastructureDetails> {
        let response = self
            .api()
            .organization_byoc_infrastructure_get(org_id, byoc_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_byoc_infrastructure_progress(
        &self,
        org_id: &str,
        byoc_id: &str,
    ) -> crate::cloud::client::Result<ByocInfrastructureProgress> {
        let response = self
            .api()
            .organization_byoc_infrastructure_progress_get(org_id, byoc_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn validate_byoc_infrastructure(
        &self,
        org_id: &str,
        request: &ByocInfrastructureValidatePostRequest,
    ) -> crate::cloud::client::Result<ByocInfrastructureValidation> {
        let response = self
            .api()
            .organization_byoc_infrastructure_validate(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn create_byoc_infrastructure(
        &self,
        org_id: &str,
        request: &ByocInfrastructurePostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ByocConfig> {
        let response = self
            .api()
            .organization_byoc_infrastructure_create(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_byoc_infrastructure(
        &self,
        org_id: &str,
        byoc_id: &str,
        request: &ByocInfrastructurePatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::ByocConfig> {
        let response = self
            .api()
            .organization_byoc_infrastructure_update(org_id, byoc_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_byoc_infrastructure(
        &self,
        org_id: &str,
        byoc_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .organization_byoc_infrastructure_delete(org_id, byoc_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn get_org_prometheus(
        &self,
        org_id: &str,
        filtered_metrics: Option<bool>,
    ) -> crate::cloud::client::Result<String> {
        let filtered_metrics = filtered_metrics.map(|value| if value { "true" } else { "false" });
        self.api()
            .organization_prometheus_get(org_id, filtered_metrics)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))
    }

    pub async fn discover_org_prometheus_targets(
        &self,
        org_id: &str,
        filtered_metrics: Option<bool>,
    ) -> crate::cloud::client::Result<
        Vec<clickhouse_cloud_api::models::PrometheusDiscoveryTargetGroup>,
    > {
        let filtered_metrics = filtered_metrics.map(|value| if value { "true" } else { "false" });
        self.api()
            .organization_prometheus_discovery_get(org_id, filtered_metrics)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))
    }

    pub async fn get_org_usage(
        &self,
        org_id: &str,
        from_date: &str,
        to_date: &str,
        filters: &[String],
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::UsageCost> {
        let filters: Vec<&str> = filters.iter().map(String::as_str).collect();
        let response = self
            .api()
            .usage_cost_get(org_id, from_date, to_date, &filters)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn list_organization_roles(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::RBACRole>> {
        let response = self
            .api()
            .organization_roles_get_list(org_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_organization_role(
        &self,
        org_id: &str,
        role_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::RBACRole> {
        let response = self
            .api()
            .organization_role_get(org_id, role_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn create_organization_role(
        &self,
        org_id: &str,
        request: &RoleCreateRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::RBACRole> {
        let response = self
            .api()
            .organization_role_post(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_organization_role(
        &self,
        org_id: &str,
        role_id: &str,
        request: &RoleUpdateRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::RBACRole> {
        let response = self
            .api()
            .organization_role_patch(org_id, role_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_organization_role(
        &self,
        org_id: &str,
        role_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .organization_role_delete(org_id, role_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn list_members(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::Member>> {
        let response = self
            .api()
            .member_get_list(org_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_member(
        &self,
        org_id: &str,
        user_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Member> {
        let response = self
            .api()
            .member_get(org_id, user_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn update_member(
        &self,
        org_id: &str,
        user_id: &str,
        request: &MemberPatchRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Member> {
        let response = self
            .api()
            .member_update(org_id, user_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_member(
        &self,
        org_id: &str,
        user_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .member_delete(org_id, user_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn list_invitations(
        &self,
        org_id: &str,
    ) -> crate::cloud::client::Result<Vec<clickhouse_cloud_api::models::Invitation>> {
        let response = self
            .api()
            .invitation_get_list(org_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn create_invitation(
        &self,
        org_id: &str,
        request: &InvitationPostRequest,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Invitation> {
        let response = self
            .api()
            .invitation_create(org_id, request)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn get_invitation(
        &self,
        org_id: &str,
        invitation_id: &str,
    ) -> crate::cloud::client::Result<clickhouse_cloud_api::models::Invitation> {
        let response = self
            .api()
            .invitation_get(org_id, invitation_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Self::unwrap_response(response)
    }

    pub async fn delete_invitation(
        &self,
        org_id: &str,
        invitation_id: &str,
    ) -> crate::cloud::client::Result<DeleteResponse> {
        let response = self
            .api()
            .invitation_delete(org_id, invitation_id)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org_id))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }

    pub async fn get_default_org_id(&self) -> crate::cloud::client::Result<String> {
        let organizations = self.list_organizations().await?;
        match organizations.len() {
            0 => Err(CloudError::new("No organization found for this API key")),
            1 => organizations[0]
                .id
                .map(|id| id.to_string())
                .ok_or_else(|| CloudError::new("Organization response is missing its id")),
            _ => Err(CloudError::new(
                "Multiple organizations found. Specify --org-id or --org-name to choose one. \
                 Use `clickhousectl cloud org list` to see your organizations.",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn primary_json_file_argument_contract() {
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "org", "role", "create"],
            "config_file",
            &["config-file"],
        );
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "org", "role", "update", "role-1"],
            "config_file",
            &["config-file"],
        );
    }

    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use clap::Parser;

    fn parse_cloud_command(args: &[&str]) -> CloudCommands {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
        crate::cloud::cli::tests::assert_org_selector(&cloud_args, args);
        cloud_args.command
    }

    fn assert_write(args: &[&str], expected: bool) {
        let command = parse_cloud_command(args);
        assert!(matches!(
            &command,
            CloudCommands::Org { .. }
                | CloudCommands::Member { .. }
                | CloudCommands::Invitation { .. }
        ));
        assert_eq!(
            command.is_write_command(),
            expected,
            "wrong classification for: {}",
            args.join(" ")
        );
    }

    #[test]
    fn parses_organization_body_command_defaults() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "update",
            "--org-id",
            "org-1",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Update {
            name,
            remove_private_endpoint,
            enable_core_dumps,
        } = command
        else {
            panic!("expected org update");
        };

        assert!(name.is_none());
        assert!(remove_private_endpoint.is_empty());
        assert!(enable_core_dumps.is_none());

        let CloudCommands::Member { command } =
            parse_cloud_command(&["clickhousectl", "cloud", "member", "update", "user-1"])
        else {
            panic!("expected member command");
        };
        let MemberCommands::Update {
            user_id,
            role_id,
            clear_roles,
        } = command
        else {
            panic!("expected member update");
        };
        assert_eq!(user_id.id.as_deref(), Some("user-1"));
        assert!(role_id.is_empty());
        assert!(!clear_roles);

        let CloudCommands::Invitation { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "invitation",
            "create",
            "--email",
            "user@example.com",
        ]) else {
            panic!("expected invitation command");
        };
        let InvitationCommands::Create { email, role_id } = command else {
            panic!("expected invitation create");
        };
        assert_eq!(email, "user@example.com");
        assert!(role_id.is_empty());
    }

    #[test]
    fn parses_organization_body_command_maximal_and_repeatable_flags() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "update",
            "--org-id",
            "org-1",
            "--new-name",
            "Updated Org",
            "--remove-private-endpoint",
            "pe-1,description=old,cloud-provider=aws,region=us-east-1",
            "--remove-private-endpoint",
            "pe-2,description=legacy,cloud-provider=azure,region=eastus",
            "--enable-core-dumps",
            "false",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Update {
            name,
            remove_private_endpoint,
            enable_core_dumps,
        } = command
        else {
            panic!("expected org update");
        };

        assert_eq!(name.as_deref(), Some("Updated Org"));
        assert_eq!(
            remove_private_endpoint,
            vec![
                "pe-1,description=old,cloud-provider=aws,region=us-east-1",
                "pe-2,description=legacy,cloud-provider=azure,region=eastus",
            ]
        );
        assert_eq!(enable_core_dumps, Some(false));

        let CloudCommands::Member { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "member",
            "update",
            "user-1",
            "--role-id",
            "role-1",
            "--role-id",
            "role-2",
            "--org-id",
            "org-1",
        ]) else {
            panic!("expected member command");
        };
        let MemberCommands::Update {
            user_id,
            role_id,
            clear_roles,
        } = command
        else {
            panic!("expected member update");
        };
        assert_eq!(user_id.id.as_deref(), Some("user-1"));
        assert_eq!(role_id, vec!["role-1", "role-2"]);
        assert!(!clear_roles);

        let CloudCommands::Invitation { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "invitation",
            "create",
            "--email",
            "user@example.com",
            "--role-id",
            "role-1",
            "--role-id",
            "role-2",
            "--org-id",
            "org-1",
        ]) else {
            panic!("expected invitation command");
        };
        let InvitationCommands::Create { email, role_id } = command else {
            panic!("expected invitation create");
        };
        assert_eq!(email, "user@example.com");
        assert_eq!(role_id, vec!["role-1", "role-2"]);
    }

    #[test]
    fn parses_minimal_private_endpoint_removal() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "update",
            "--org-id",
            "org-1",
            "--remove-private-endpoint",
            "pe-1,cloud-provider=aws,region=us-east-1",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Update {
            remove_private_endpoint,
            ..
        } = command
        else {
            panic!("expected org update");
        };

        assert_eq!(
            remove_private_endpoint,
            ["pe-1,cloud-provider=aws,region=us-east-1"]
        );
    }

    #[test]
    fn rejects_incomplete_private_endpoint_removals_during_clap_parsing() {
        for (value, required) in [
            ("pe-1,region=us-east-1", "requires cloud-provider"),
            ("pe-1,cloud-provider=aws", "requires region"),
            ("pe-1,description=old", "requires cloud-provider and region"),
            (
                "pe-1,cloud-provider=,region=us-east-1",
                "requires a non-empty cloud-provider",
            ),
            (
                "pe-1,cloud-provider=aws,region= ",
                "requires a non-empty region",
            ),
        ] {
            let error = Cli::try_parse_from([
                "clickhousectl",
                "cloud",
                "org",
                "update",
                "--org-id",
                "org-1",
                "--remove-private-endpoint",
                value,
            ])
            .err()
            .expect("incomplete endpoint removal should fail");

            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
            assert!(error.to_string().contains(required), "{error}");
        }
    }

    #[test]
    fn parses_member_clear_roles() {
        let CloudCommands::Member { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "member",
            "update",
            "user-1",
            "--clear-roles",
        ]) else {
            panic!("expected member command");
        };
        let MemberCommands::Update {
            role_id,
            clear_roles,
            ..
        } = command
        else {
            panic!("expected member update");
        };
        assert!(role_id.is_empty());
        assert!(clear_roles);
    }

    #[test]
    fn rejects_conflicting_member_role_changes() {
        for flags in [
            ["--role-id", "role-1", "--clear-roles"],
            ["--clear-roles", "--role-id", "role-1"],
        ] {
            let result = Cli::try_parse_from(
                ["clickhousectl", "cloud", "member", "update", "user-1"]
                    .into_iter()
                    .chain(flags),
            );
            let Err(error) = result else {
                panic!("set and clear flags must conflict");
            };
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
    }

    #[test]
    fn parses_org_usage_date_only_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "usage",
            "--from-date",
            "2025-01-01",
            "--to-date",
            "2025-01-31",
        ])
        .unwrap();

        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Org { command } = args.command else {
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
    fn org_usage_tag_filters_preserve_api_grammar() {
        let base_args = [
            "clickhousectl",
            "cloud",
            "org",
            "usage",
            "--from-date",
            "2025-01-01",
            "--to-date",
            "2025-01-31",
        ];
        let filters = ["tag:env=prod", "tag:active", "tag:empty=", "tag:expr=a=b"];
        let cli = Cli::try_parse_from(
            base_args
                .into_iter()
                .chain(filters.iter().flat_map(|value| ["--filter", *value])),
        )
        .unwrap();
        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Org {
            command: OrgCommands::Usage { filter, .. },
        } = args.command
        else {
            panic!("expected org usage");
        };
        assert_eq!(filter, filters);
        for value in [
            "garbage",
            "state=running",
            "env=prod",
            "tag:",
            "tag:=x",
            "tag: =x",
        ] {
            let error = Cli::try_parse_from(base_args.into_iter().chain(["--filter", value]))
                .err()
                .expect("malformed filter must fail");
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        }
    }

    #[test]
    fn parses_org_prometheus_and_usage_org_id_flags() {
        let prometheus = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "prometheus",
            "--org-id",
            "org-1",
        ])
        .unwrap();
        let Commands::Cloud(args) = prometheus.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Org { command } = args.command else {
            panic!("expected org command");
        };
        let OrgCommands::Prometheus { .. } = command else {
            panic!("expected org prometheus");
        };

        let usage = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "usage",
            "--org-id",
            "org-1",
            "--from-date",
            "2025-01-01",
            "--to-date",
            "2025-01-31",
        ])
        .unwrap();
        let Commands::Cloud(args) = usage.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Org { command } = args.command else {
            panic!("expected org command");
        };
        let OrgCommands::Usage { .. } = command else {
            panic!("expected org usage");
        };
    }

    #[test]
    fn parses_org_prometheus_discovery_flags() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "prometheus",
            "discovery",
            "--org-id",
            "org-1",
            "--filtered-metrics",
            "false",
        ])
        .unwrap();

        let Commands::Cloud(args) = cli.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Org { command } = args.command else {
            panic!("expected org command");
        };
        let OrgCommands::Prometheus {
            command: Some(PrometheusCommands::Discovery),
            filtered_metrics,
            ..
        } = command
        else {
            panic!("expected prometheus discovery");
        };

        assert_eq!(filtered_metrics, Some(false));
    }

    #[test]
    fn parses_org_quota_commands() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "quota",
            "list",
            "--org-id",
            "org-1",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Quota {
            command: QuotaCommands::List,
        } = command
        else {
            panic!("expected quota list command");
        };

        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "quota",
            "get",
            "replicas-per-warehouse",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Quota {
            command: QuotaCommands::Get { quota_code },
        } = command
        else {
            panic!("expected quota get command");
        };
        assert_eq!(quota_code, "replicas-per-warehouse");
    }

    #[test]
    fn parses_org_balance_command() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "balance",
            "--org-id",
            "org-1",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Balance = command else {
            panic!("expected org balance command");
        };
    }

    #[test]
    fn rejects_org_usage_timestamps() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "usage",
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
    fn rejects_invalid_org_usage_calendar_dates() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "usage",
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
    fn top_level_write_classification_covers_every_organization_access_command() {
        assert_write(&["clickhousectl", "cloud", "org", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "org", "get", "--org-id", "org-1"],
            false,
        );
        assert_write(&["clickhousectl", "cloud", "org", "quota", "list"], false);
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "quota",
                "get",
                "services-per-organization",
            ],
            false,
        );
        assert_write(&["clickhousectl", "cloud", "org", "balance"], false);
        assert_write(&["clickhousectl", "cloud", "org", "prometheus"], false);
        assert_write(
            &["clickhousectl", "cloud", "org", "prometheus", "discovery"],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "usage",
                "--from-date",
                "2025-01-01",
                "--to-date",
                "2025-01-31",
            ],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "update",
                "--org-id",
                "org-1",
            ],
            true,
        );

        assert_write(&["clickhousectl", "cloud", "member", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "member", "get", "user-1"],
            false,
        );
        assert_write(
            &["clickhousectl", "cloud", "member", "update", "user-1"],
            true,
        );
        assert_write(
            &["clickhousectl", "cloud", "member", "remove", "user-1"],
            true,
        );

        assert_write(&["clickhousectl", "cloud", "invitation", "list"], false);
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "invitation",
                "get",
                "invitation-1",
            ],
            false,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "invitation",
                "create",
                "--email",
                "user@example.com",
            ],
            true,
        );
        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "invitation",
                "delete",
                "invitation-1",
            ],
            true,
        );
    }

    #[test]
    fn usage_entity_label_distinguishes_named_unknown_and_absent_entities() {
        let id = uuid::Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        assert_eq!(
            usage_entity_label(Some("production"), Some(id)),
            "production"
        );
        assert_eq!(
            usage_entity_label(None, Some(id)),
            "11111111-2222-3333-4444-555555555555 (unknown)"
        );
        assert_eq!(
            usage_entity_label(Some(""), Some(id)),
            format!("{id} (unknown)")
        );
        assert_eq!(usage_entity_label(None, None), ABSENT);
    }

    #[test]
    fn build_org_update_request_supports_minimal_fields() {
        let request = build_org_update_request(&OrgUpdateOptions::default()).unwrap();

        assert!(request.name.is_none());
        assert!(request.private_endpoints.is_none());
        assert!(request.enable_core_dumps.is_none());
    }

    #[test]
    fn build_org_update_request_supports_maximal_fields() {
        let options = OrgUpdateOptions {
            name: Some("Updated Org".to_string()),
            remove_private_endpoints: vec![
                "pe-1,description=old,cloud-provider=aws,region=us-east-1".to_string(),
                "pe-2,description=legacy,cloud-provider=azure,region=eastus".to_string(),
            ],
            enable_core_dumps: Some(false),
        };
        let request = build_org_update_request(&options).unwrap();

        assert_eq!(request.name.as_deref(), Some("Updated Org"));
        assert_eq!(request.enable_core_dumps, Some(false));
        let private_endpoints = request.private_endpoints.as_ref().unwrap();
        #[cfg(feature = "deprecated-fields")]
        assert!(private_endpoints.add.is_none());
        assert_eq!(private_endpoints.remove.len(), 2);
        assert_eq!(private_endpoints.remove[0].id, "pe-1");
        assert_eq!(
            private_endpoints.remove[0].description.as_deref(),
            Some("old")
        );
        assert_eq!(
            private_endpoints.remove[0].cloud_provider,
            OrganizationPatchPrivateEndpointCloudprovider::Aws
        );
        assert_eq!(
            private_endpoints.remove[0].region,
            OrganizationPatchPrivateEndpointRegion::Us_east_1
        );
        assert_eq!(private_endpoints.remove[1].id, "pe-2");
        assert_eq!(
            private_endpoints.remove[1].description.as_deref(),
            Some("legacy")
        );
        assert_eq!(
            private_endpoints.remove[1].cloud_provider,
            OrganizationPatchPrivateEndpointCloudprovider::Azure
        );
        assert_eq!(
            private_endpoints.remove[1].region,
            OrganizationPatchPrivateEndpointRegion::Eastus
        );
    }

    #[test]
    fn build_member_update_request_supports_minimal_fields() {
        let request = build_member_update_request(&[], false);

        assert!(request.assigned_role_ids.is_none());
        #[cfg(feature = "deprecated-fields")]
        assert!(request.role.is_none());
    }

    #[test]
    fn build_member_update_request_supports_maximal_fields() {
        let request =
            build_member_update_request(&["role-1".to_string(), "role-2".to_string()], false);

        assert_eq!(
            request.assigned_role_ids,
            Some(vec!["role-1".to_string(), "role-2".to_string()])
        );
        #[cfg(feature = "deprecated-fields")]
        assert!(request.role.is_none());
    }

    #[test]
    fn build_member_update_request_clears_roles_explicitly() {
        let request = build_member_update_request(&[], true);

        assert_eq!(request.assigned_role_ids, Some(Vec::new()));
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({"assignedRoleIds": []})
        );
    }

    #[test]
    fn build_invitation_create_request_supports_minimal_fields() {
        let request = build_invitation_create_request("user@example.com", &[]);

        assert_eq!(request.email, "user@example.com");
        assert!(request.assigned_role_ids.is_empty());
        #[cfg(feature = "deprecated-fields")]
        assert!(request.role.is_none());
    }

    #[test]
    fn build_invitation_create_request_supports_maximal_fields() {
        let request = build_invitation_create_request(
            "user@example.com",
            &["role-1".to_string(), "role-2".to_string()],
        );

        assert_eq!(request.email, "user@example.com");
        assert_eq!(request.assigned_role_ids, vec!["role-1", "role-2"]);
        #[cfg(feature = "deprecated-fields")]
        assert!(request.role.is_none());
    }

    #[test]
    fn parse_org_private_endpoint_remove_requires_non_empty_id() {
        for value in ["", "description=old", "id="] {
            let error = parse_org_private_endpoint_remove(value).unwrap_err();
            assert!(
                error.to_string().contains("requires a non-empty id"),
                "unexpected error for {value:?}: {error}"
            );
        }
    }

    #[test]
    fn parse_org_private_endpoint_remove_requires_provider_and_region() {
        for (value, required) in [
            ("pe-1,region=us-east-1", "requires cloud-provider"),
            ("pe-1,cloud-provider=aws", "requires region"),
            ("pe-1", "requires cloud-provider and region"),
            (
                "pe-1,cloud-provider= ,region=us-east-1",
                "requires a non-empty cloud-provider",
            ),
            (
                "pe-1,cloud-provider=aws,region= ",
                "requires a non-empty region",
            ),
        ] {
            let error = parse_org_private_endpoint_remove(value).unwrap_err();
            assert!(error.to_string().contains(required), "{error}");
        }
    }

    fn parse_byoc_command(args: &[&str]) -> ByocCommands {
        let CloudCommands::Org { command } = parse_cloud_command(args) else {
            panic!("expected org command");
        };
        let OrgCommands::Byoc { command } = command else {
            panic!("expected BYOC command");
        };
        command
    }

    fn byoc_args(extra: &[&str]) -> Vec<String> {
        let mut args: Vec<String> = [
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "create",
            "--region",
            "us-east-1",
            "--account-id",
            "123456789012",
        ]
        .iter()
        .map(|arg| arg.to_string())
        .collect();
        args.extend(extra.iter().map(|arg| arg.to_string()));
        args
    }

    fn minimal_infrastructure_args() -> ByocInfrastructureArgs {
        ByocInfrastructureArgs {
            region: "us-east-1".into(),
            account_id: "123456789012".into(),
            availability_zone_suffix: vec![],
            vpc_cidr_range: None,
            vpc_id: None,
            private_subnet_id: vec![],
            public_subnet_id: vec![],
            external_id: None,
            gcp_pod_cidr_range_name: vec![],
            gcp_shared_vpc_host_project_id: None,
            tenant_id: None,
            service_principal_client_id: None,
            tags: vec![],
        }
    }

    fn maximal_infrastructure_args() -> ByocInfrastructureArgs {
        ByocInfrastructureArgs {
            region: "eastus".into(),
            account_id: "subscription-1".into(),
            availability_zone_suffix: ["a", "b", "c", "d", "e", "f"].map(String::from).to_vec(),
            vpc_cidr_range: None,
            vpc_id: Some("vpc-1".into()),
            private_subnet_id: vec!["subnet-a".into(), "subnet-b".into()],
            public_subnet_id: vec!["subnet-pub".into()],
            external_id: Some("external-1".into()),
            gcp_pod_cidr_range_name: vec!["pods-1".into(), "pods-2".into()],
            gcp_shared_vpc_host_project_id: Some("host-project".into()),
            tenant_id: Some("tenant-1".into()),
            service_principal_client_id: Some("client-1".into()),
            tags: vec!["team=data".into(), "note=a=b".into(), "empty=".into()],
        }
    }

    #[test]
    fn parses_byoc_create_update_and_delete_commands() {
        let ByocCommands::Create {
            infrastructure,
            display_name,
        } = parse_byoc_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "create",
            "--region",
            "us-east-1",
            "--account-id",
            "123456789012",
            "--availability-zone-suffix",
            "a",
            "--availability-zone-suffix",
            "b",
            "--vpc-cidr-range",
            "10.0.0.0/16",
            "--display-name",
            "production",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected BYOC create");
        };
        assert_eq!(infrastructure.region, "us-east-1");
        assert_eq!(infrastructure.account_id, "123456789012");
        assert_eq!(infrastructure.availability_zone_suffix, vec!["a", "b"]);
        assert_eq!(
            infrastructure.vpc_cidr_range.as_deref(),
            Some("10.0.0.0/16")
        );
        assert_eq!(display_name.as_deref(), Some("production"));

        let ByocCommands::Update {
            byoc_id,
            display_name,
        } = parse_byoc_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "update",
            "byoc-1",
            "--display-name",
            "renamed",
        ])
        else {
            panic!("expected BYOC update");
        };
        assert_eq!(byoc_id.id.as_deref(), Some("byoc-1"));
        assert_eq!(display_name, "renamed");

        assert_write(
            &[
                "clickhousectl",
                "cloud",
                "org",
                "byoc",
                "delete",
                "byoc-1",
                "--org-id",
                "org-1",
            ],
            true,
        );
    }

    #[test]
    fn byoc_get_and_progress_are_reads_selected_by_id_or_name() {
        for subcommand in ["get", "progress"] {
            let command = parse_byoc_command(&[
                "clickhousectl",
                "cloud",
                "org",
                "byoc",
                subcommand,
                "byoc-1",
            ]);
            let byoc_id = match command {
                ByocCommands::Get { byoc_id } | ByocCommands::Progress { byoc_id } => byoc_id,
                _ => panic!("expected BYOC {subcommand}"),
            };
            assert_eq!(byoc_id.id.as_deref(), Some("byoc-1"));
            assert_write(
                &[
                    "clickhousectl",
                    "cloud",
                    "org",
                    "byoc",
                    subcommand,
                    "byoc-1",
                ],
                false,
            );

            let command = parse_byoc_command(&[
                "clickhousectl",
                "cloud",
                "org",
                "byoc",
                subcommand,
                "--name",
                "production",
            ]);
            let (ByocCommands::Get { byoc_id } | ByocCommands::Progress { byoc_id }) = command
            else {
                panic!("expected BYOC {subcommand}");
            };
            assert_eq!(byoc_id.name.as_deref(), Some("production"));

            let error = Cli::try_parse_from(["clickhousectl", "cloud", "org", "byoc", subcommand])
                .err()
                .expect("an ID or --name is required");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
        }
    }

    #[test]
    fn byoc_validate_and_create_are_writes() {
        // Validate creates nothing, but its endpoint needs the organization
        // manage scope, which read-only OAuth never has.
        for subcommand in ["validate", "create"] {
            assert_write(
                &[
                    "clickhousectl",
                    "cloud",
                    "org",
                    "byoc",
                    subcommand,
                    "--region",
                    "us-east-1",
                    "--account-id",
                    "123456789012",
                ],
                true,
            );
        }
    }

    #[test]
    fn byoc_create_needs_only_region_and_account() {
        let ByocCommands::Create {
            infrastructure,
            display_name,
        } = parse_byoc_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "create",
            "--region",
            "us-east-1",
            "--account-id",
            "123456789012",
        ])
        else {
            panic!("expected BYOC create");
        };
        assert_eq!(*infrastructure, minimal_infrastructure_args());
        assert_eq!(display_name, None);

        for missing in ["--region", "--account-id"] {
            let args: Vec<String> = byoc_args(&[]);
            let index = args.iter().position(|arg| arg == missing).unwrap();
            let mut args = args;
            args.drain(index..index + 2);
            let error = Cli::try_parse_from(&args).err().expect("must fail");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument,
                "{missing}"
            );
        }
    }

    #[test]
    fn byoc_validate_shares_create_flags_and_rejects_display_name() {
        let flags = [
            "--region",
            "eastus",
            "--account-id",
            "subscription-1",
            "--availability-zone-suffix",
            "a",
            "--availability-zone-suffix",
            "b",
            "--availability-zone-suffix",
            "c",
            "--availability-zone-suffix",
            "d",
            "--availability-zone-suffix",
            "e",
            "--availability-zone-suffix",
            "f",
            "--vpc-id",
            "vpc-1",
            "--private-subnet-id",
            "subnet-a",
            "--private-subnet-id",
            "subnet-b",
            "--public-subnet-id",
            "subnet-pub",
            "--external-id",
            "external-1",
            "--gcp-pod-cidr-range-name",
            "pods-1",
            "--gcp-pod-cidr-range-name",
            "pods-2",
            "--gcp-shared-vpc-host-project-id",
            "host-project",
            "--tenant-id",
            "tenant-1",
            "--service-principal-client-id",
            "client-1",
            "--tag",
            "team=data",
            "--tag",
            "note=a=b",
            "--tag",
            "empty=",
        ];
        let mut validate = vec!["clickhousectl", "cloud", "org", "byoc", "validate"];
        validate.extend(flags);
        let ByocCommands::Validate { infrastructure } = parse_byoc_command(&validate) else {
            panic!("expected BYOC validate");
        };
        assert_eq!(*infrastructure, maximal_infrastructure_args());

        let mut create = vec!["clickhousectl", "cloud", "org", "byoc", "create"];
        create.extend(flags);
        create.extend(["--display-name", "production"]);
        let ByocCommands::Create {
            infrastructure,
            display_name,
        } = parse_byoc_command(&create)
        else {
            panic!("expected BYOC create");
        };
        assert_eq!(*infrastructure, maximal_infrastructure_args());
        assert_eq!(display_name.as_deref(), Some("production"));

        let mut validate_with_name = validate.clone();
        validate_with_name.extend(["--display-name", "production"]);
        let error = Cli::try_parse_from(&validate_with_name)
            .err()
            .expect("validate has no --display-name");
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn byoc_vpc_cidr_range_conflicts_with_byo_vpc_flags() {
        for byo_vpc in [
            ["--vpc-id", "vpc-1"],
            ["--private-subnet-id", "subnet-a"],
            ["--public-subnet-id", "subnet-pub"],
        ] {
            let mut extra = vec!["--vpc-cidr-range", "10.0.0.0/16"];
            extra.extend(byo_vpc);
            if byo_vpc[0] == "--vpc-id" {
                extra.extend(["--private-subnet-id", "subnet-a"]);
            }
            let error = Cli::try_parse_from(byoc_args(&extra))
                .err()
                .expect("managed and BYO VPC flags conflict");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::ArgumentConflict,
                "{byo_vpc:?}"
            );
        }
    }

    #[test]
    fn byoc_vpc_id_requires_a_private_subnet() {
        let error = Cli::try_parse_from(byoc_args(&["--vpc-id", "vpc-1"]))
            .err()
            .expect("--vpc-id needs --private-subnet-id");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
        assert!(
            Cli::try_parse_from(byoc_args(&[
                "--vpc-id",
                "vpc-1",
                "--private-subnet-id",
                "subnet-a"
            ]))
            .is_ok()
        );
    }

    #[test]
    fn build_byoc_create_request_minimal_sends_only_region_and_account() {
        let minimal = build_byoc_create_request(&minimal_infrastructure_args(), None).unwrap();
        assert_eq!(
            minimal.region_id,
            ByocInfrastructurePostRequestRegionid::Us_east_1
        );
        assert_eq!(minimal.account_id, "123456789012");
        assert_eq!(minimal.availability_zone_suffixes, None);
        assert_eq!(minimal.display_name, None);
        assert_eq!(minimal.vpc_cidr_range, None);
        assert_eq!(minimal.vpc_id, None);
        assert_eq!(minimal.private_subnet_ids, None);
        assert_eq!(minimal.public_subnet_ids, None);
        assert_eq!(minimal.external_id, None);
        assert_eq!(minimal.gcp_pod_cidr_range_names, None);
        assert_eq!(minimal.gcp_shared_vpc_host_project_id, None);
        assert_eq!(minimal.tenant_id, None);
        assert_eq!(minimal.service_principal_client_id, None);
        assert_eq!(minimal.tags, None);
        assert_eq!(
            serde_json::to_value(&minimal).unwrap(),
            serde_json::json!({"accountId": "123456789012", "regionId": "us-east-1"})
        );
    }

    #[test]
    fn build_byoc_create_request_keeps_the_managed_vpc_shape() {
        let args = ByocInfrastructureArgs {
            availability_zone_suffix: vec!["a".into()],
            vpc_cidr_range: Some("10.0.0.0/16".into()),
            ..minimal_infrastructure_args()
        };
        let request = build_byoc_create_request(&args, Some("production")).unwrap();
        assert_eq!(
            request.availability_zone_suffixes,
            Some(vec![ByocAvailabilityZoneSuffix::A])
        );
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({
                "accountId": "123456789012",
                "availabilityZoneSuffixes": ["a"],
                "displayName": "production",
                "regionId": "us-east-1",
                "vpcCidrRange": "10.0.0.0/16"
            })
        );
    }

    fn expected_tags() -> ByocInfrastructureTags {
        ByocInfrastructureTags::from([
            ("team".to_string(), "data".to_string()),
            ("note".to_string(), "a=b".to_string()),
            ("empty".to_string(), String::new()),
        ])
    }

    #[test]
    fn build_byoc_create_request_maximal_maps_every_flag() {
        let maximal =
            build_byoc_create_request(&maximal_infrastructure_args(), Some("all-zones")).unwrap();
        assert_eq!(
            maximal.region_id,
            ByocInfrastructurePostRequestRegionid::Eastus
        );
        assert_eq!(maximal.account_id, "subscription-1");
        assert_eq!(
            maximal.availability_zone_suffixes.as_ref().map(Vec::len),
            Some(6)
        );
        assert_eq!(maximal.display_name.as_deref(), Some("all-zones"));
        assert_eq!(maximal.vpc_cidr_range, None);
        assert_eq!(maximal.vpc_id.as_deref(), Some("vpc-1"));
        assert_eq!(
            maximal.private_subnet_ids,
            Some(vec!["subnet-a".to_string(), "subnet-b".to_string()])
        );
        assert_eq!(
            maximal.public_subnet_ids,
            Some(vec!["subnet-pub".to_string()])
        );
        assert_eq!(maximal.external_id.as_deref(), Some("external-1"));
        assert_eq!(
            maximal.gcp_pod_cidr_range_names,
            Some(vec!["pods-1".to_string(), "pods-2".to_string()])
        );
        assert_eq!(
            maximal.gcp_shared_vpc_host_project_id.as_deref(),
            Some("host-project")
        );
        assert_eq!(maximal.tenant_id.as_deref(), Some("tenant-1"));
        assert_eq!(
            maximal.service_principal_client_id.as_deref(),
            Some("client-1")
        );
        assert_eq!(maximal.tags, Some(expected_tags()));
    }

    #[test]
    fn build_byoc_validate_request_minimal_and_maximal() {
        let minimal = build_byoc_validate_request(&minimal_infrastructure_args()).unwrap();
        assert_eq!(
            minimal.region_id,
            ByocInfrastructureValidatePostRequestRegionid::Us_east_1
        );
        assert_eq!(minimal.account_id, "123456789012");
        assert_eq!(minimal.availability_zone_suffixes, None);
        assert_eq!(minimal.tags, None);
        assert_eq!(
            serde_json::to_value(&minimal).unwrap(),
            serde_json::json!({"accountId": "123456789012", "regionId": "us-east-1"})
        );

        let maximal = build_byoc_validate_request(&maximal_infrastructure_args()).unwrap();
        assert_eq!(
            maximal.region_id,
            ByocInfrastructureValidatePostRequestRegionid::Eastus
        );
        assert_eq!(maximal.account_id, "subscription-1");
        assert_eq!(
            maximal.availability_zone_suffixes.as_ref().map(Vec::len),
            Some(6)
        );
        assert_eq!(maximal.vpc_cidr_range, None);
        assert_eq!(maximal.vpc_id.as_deref(), Some("vpc-1"));
        assert_eq!(
            maximal.private_subnet_ids,
            Some(vec!["subnet-a".to_string(), "subnet-b".to_string()])
        );
        assert_eq!(
            maximal.public_subnet_ids,
            Some(vec!["subnet-pub".to_string()])
        );
        assert_eq!(maximal.external_id.as_deref(), Some("external-1"));
        assert_eq!(
            maximal.gcp_pod_cidr_range_names,
            Some(vec!["pods-1".to_string(), "pods-2".to_string()])
        );
        assert_eq!(
            maximal.gcp_shared_vpc_host_project_id.as_deref(),
            Some("host-project")
        );
        assert_eq!(maximal.tenant_id.as_deref(), Some("tenant-1"));
        assert_eq!(
            maximal.service_principal_client_id.as_deref(),
            Some("client-1")
        );
        assert_eq!(maximal.tags, Some(expected_tags()));

        // Create and validate share one parse, so their bodies match apart
        // from create's display name.
        let mut create = serde_json::to_value(
            build_byoc_create_request(&maximal_infrastructure_args(), None).unwrap(),
        )
        .unwrap();
        assert_eq!(create, serde_json::to_value(&maximal).unwrap());
        create["displayName"] = serde_json::json!("ignored");
        assert_ne!(create, serde_json::to_value(&maximal).unwrap());
    }

    #[test]
    fn build_byoc_requests_reject_bad_tags_as_usage_errors() {
        for (tags, expected) in [
            (vec!["team"], "expected KEY=VALUE"),
            (vec!["=data"], "tag key cannot be empty"),
            (vec![" =data"], "tag key cannot be empty"),
            (vec!["team=a", "team=b"], "duplicate tag key 'team'"),
        ] {
            let args = ByocInfrastructureArgs {
                tags: tags.iter().map(|tag| tag.to_string()).collect(),
                ..minimal_infrastructure_args()
            };
            for error in [
                build_byoc_create_request(&args, None).unwrap_err(),
                build_byoc_validate_request(&args).unwrap_err(),
            ] {
                assert!(error.to_string().contains(expected), "{tags:?}: {error}");
                assert_eq!(
                    error.kind,
                    crate::cloud::client::CloudErrorKind::Usage,
                    "{tags:?}"
                );
            }
        }
    }

    #[test]
    fn build_byoc_requests_validate_enums_and_update_only_the_name() {
        let unknown_region = ByocInfrastructureArgs {
            region: "future-region".into(),
            ..minimal_infrastructure_args()
        };
        assert!(build_byoc_create_request(&unknown_region, None).is_err());
        assert!(build_byoc_validate_request(&unknown_region).is_err());
        let unknown_zone = ByocInfrastructureArgs {
            availability_zone_suffix: vec!["z".into()],
            ..minimal_infrastructure_args()
        };
        assert!(build_byoc_create_request(&unknown_zone, None).is_err());
        assert!(build_byoc_validate_request(&unknown_zone).is_err());

        let update = build_byoc_update_request("renamed");
        assert_eq!(update.display_name.as_deref(), Some("renamed"));
        assert_eq!(
            serde_json::to_value(update).unwrap(),
            serde_json::json!({"displayName": "renamed"})
        );
    }

    fn validation(value: serde_json::Value) -> ByocInfrastructureValidation {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn byoc_validation_fails_only_on_explicit_denial() {
        assert!(
            byoc_validation_failure(&validation(serde_json::json!({
                "allPassed": true,
                "checks": [{"name": "a", "allowed": true}]
            })))
            .is_none()
        );
        // Nothing verified is reported, not failed.
        assert!(
            byoc_validation_failure(&validation(serde_json::json!({
                "supported": false,
                "checks": []
            })))
            .is_none()
        );
        // Absent outcomes are not treated as denials.
        assert!(byoc_validation_failure(&validation(serde_json::json!({}))).is_none());
        assert!(
            byoc_validation_failure(&validation(serde_json::json!({
                "checks": [{"name": "a"}]
            })))
            .is_none()
        );

        let error = byoc_validation_failure(&validation(serde_json::json!({
            "allPassed": false,
            "checks": [{"name": "a", "allowed": true}, {"name": "b", "allowed": false}]
        })))
        .expect("a denied check fails");
        assert_eq!(
            error.message,
            "BYOC validation failed: 1 of 2 checks denied"
        );
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Generic);

        let error = byoc_validation_failure(&validation(serde_json::json!({
            "checks": [{"name": "b", "allowed": false}]
        })))
        .expect("a denied check fails even without allPassed");
        assert_eq!(
            error.message,
            "BYOC validation failed: 1 of 1 checks denied"
        );
        assert!(
            byoc_validation_failure(&validation(serde_json::json!({"allPassed": false}))).is_some()
        );
    }

    #[test]
    fn parses_role_body_commands_and_classifies_access() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "role",
            "create",
            "--file",
            "role.json",
            "--org-id",
            "org-1",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Role {
            command: RoleCommands::Create { config_file },
        } = command
        else {
            panic!("expected role create");
        };
        assert_eq!(config_file, "role.json");

        assert_write(&["clickhousectl", "cloud", "org", "role", "list"], false);
        assert_write(
            &["clickhousectl", "cloud", "org", "role", "get", "role-1"],
            false,
        );
        for verb in ["create", "update", "delete"] {
            let mut args = vec!["clickhousectl", "cloud", "org", "role", verb];
            if verb != "create" {
                args.push("role-1");
            }
            if verb != "delete" {
                args.extend(["--file", "role.json"]);
            }
            assert_write(&args, true);
        }
    }

    #[test]
    fn role_builders_preserve_minimal_and_maximal_bodies() {
        let directory = tempfile::tempdir().unwrap();
        let create_file = directory.path().join("create.json");
        std::fs::write(
            &create_file,
            r#"{"name":"reader","actors":[],"policies":[]}"#,
        )
        .unwrap();
        let minimal = build_role_create_request(create_file.to_str().unwrap()).unwrap();
        assert_eq!(minimal.name, "reader");
        assert!(minimal.actors.is_empty());
        assert!(minimal.policies.is_empty());

        std::fs::write(
            &create_file,
            serde_json::json!({
                "name": "console-admin",
                "actors": ["user/user-1", "apiKey/key-1"],
                "policies": [{
                    "allowDeny": "DENY",
                    "permissions": ["control-plane:organization:update"],
                    "resources": ["organization/org-1", "instance/*"],
                    "tags": {
                        "grants": ["select", "insert"],
                        "roleV2": "sql-console-admin"
                    }
                }, {
                    "allowDeny": "ALLOW",
                    "permissions": ["control-plane:service:view"],
                    "resources": ["instance/*"]
                }]
            })
            .to_string(),
        )
        .unwrap();
        let maximal = build_role_create_request(create_file.to_str().unwrap()).unwrap();
        assert_eq!(maximal.actors.len(), 2);
        assert_eq!(maximal.policies.len(), 2);
        let policy = &maximal.policies[0];
        assert_eq!(policy.allow_deny, RBACPolicyCreateRequestAllowdeny::DENY);
        let tags = policy.tags.as_ref().unwrap();
        assert_eq!(
            tags.grants.as_deref(),
            Some(&["select".into(), "insert".into()][..])
        );
        assert_eq!(tags.role_v2, Some(RBACPolicyTagsRolev2::Sql_console_admin));

        let update_file = directory.path().join("update.json");
        std::fs::write(&update_file, r#"{"name":"renamed"}"#).unwrap();
        let minimal_update = build_role_update_request(update_file.to_str().unwrap()).unwrap();
        assert_eq!(minimal_update.name.as_deref(), Some("renamed"));
        assert!(minimal_update.actors.is_none());
        assert!(minimal_update.policies.is_none());

        std::fs::write(
            &update_file,
            serde_json::json!({
                "name": "full-update",
                "actors": [],
                "policies": [{
                    "allowDeny": "ALLOW",
                    "permissions": [],
                    "resources": [],
                    "tags": {"grants": [], "roleV2": "sql-console-readonly"}
                }]
            })
            .to_string(),
        )
        .unwrap();
        let maximal_update = build_role_update_request(update_file.to_str().unwrap()).unwrap();
        assert_eq!(maximal_update.actors, Some(vec![]));
        assert_eq!(maximal_update.policies.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn role_builders_reject_unknown_nested_fields_and_enums() {
        let directory = tempfile::tempdir().unwrap();
        let config_file = directory.path().join("role.json");
        for (body, expected) in [
            (
                serde_json::json!({"name":"bad","actors":[],"policies":[{"allowDeny":"ALLOW","permissions":[],"resources":[],"tagz":{}}]}),
                "tagz",
            ),
            (
                serde_json::json!({"name":"bad","actors":[],"policies":[{"allowDeny":"AUDIT","permissions":[],"resources":[]}]}),
                "allowDeny",
            ),
            (
                serde_json::json!({"name":"bad","actors":[],"policies":[{"allowDeny":"ALLOW","permissions":[],"resources":[],"tags":{"roleV2":"future-role"}}]}),
                "roleV2",
            ),
        ] {
            std::fs::write(&config_file, body.to_string()).unwrap();
            let error = build_role_create_request(config_file.to_str().unwrap()).unwrap_err();
            assert!(error.message.contains(expected), "{error}");
        }
    }
}
