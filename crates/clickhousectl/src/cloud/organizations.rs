use crate::cloud::client::{CloudClient, CloudError, ResourceLookup, Result as CloudResult};
use crate::cloud::output::{ABSENT, or_absent, print_human};
use crate::cloud::shared::{parse_date_only, resolve_org_id};
use crate::cloud::types::DeleteResponse;
use clap::Subcommand;
use clickhouse_cloud_api::models::{
    ByocAvailabilityZoneSuffix, ByocInfrastructurePatchRequest, ByocInfrastructurePostRequest,
    ByocInfrastructurePostRequestRegionid, InvitationPostRequest, MemberPatchRequest,
    OrganizationPatchPrivateEndpoint, OrganizationPatchPrivateEndpointCloudprovider,
    OrganizationPatchPrivateEndpointRegion, OrganizationPatchRequest,
    OrganizationPrivateEndpointsPatch,
};
use tabled::{Table, Tabled, settings::Style};

#[derive(Subcommand)]
pub enum OrgCommands {
    /// List organizations
    List,

    /// Get organization details
    Get {
        /// Organization ID
        org_id: String,
    },

    /// View organization quotas (Beta)
    Quota {
        #[command(subcommand)]
        command: QuotaCommands,
    },

    /// View active credit balances (Beta)
    Balance {
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Manage BYOC infrastructure
    Byoc {
        #[command(subcommand)]
        command: ByocCommands,
    },

    /// Update organization settings
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Only the flags you pass change; everything else is left as-is.
  This can only remove private endpoints; add them with `cloud service update --add-private-endpoint-id`.")]
    Update {
        /// Organization ID
        org_id: String,

        /// New organization name
        #[arg(long)]
        name: Option<String>,

        /// Remove a private endpoint from the org allow list (repeatable)
        ///
        /// Format: id[,description=TEXT][,cloud-provider=aws|gcp|azure][,region=REGION]
        ///
        /// Omitting cloud-provider or region sends gcp / ap-northeast-1, not "unchanged".
        #[arg(long = "remove-private-endpoint")]
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

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long, global = true)]
        org_id: Option<String>,

        /// Organization ID (deprecated positional form; use --org-id)
        #[arg(value_name = "ORG_ID", hide = true, conflicts_with = "org_id")]
        legacy_org_id: Option<String>,

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
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,

        /// Organization ID (deprecated positional form; use --org-id)
        #[arg(value_name = "ORG_ID", hide = true, conflicts_with = "org_id")]
        legacy_org_id: Option<String>,

        /// Report start date in UTC (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date_only)]
        from_date: String,

        /// Report end date in UTC, inclusive (YYYY-MM-DD)
        #[arg(long, value_parser = parse_date_only)]
        to_date: String,

        /// Filter by resource tag: `tag:Key=Value` or `tag:Key` (repeatable)
        #[arg(long)]
        filter: Vec<String>,
    },
}

impl OrgCommands {
    pub fn is_write(&self) -> bool {
        match self {
            OrgCommands::List => false,
            OrgCommands::Get { .. } => false,
            OrgCommands::Quota { .. } => false,
            OrgCommands::Balance { .. } => false,
            OrgCommands::Byoc { command } => command.is_write(),
            OrgCommands::Prometheus { .. } => false,
            OrgCommands::Usage { .. } => false,
            OrgCommands::Update { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum ByocCommands {
    /// Create BYOC infrastructure
    #[command(after_help = "\
CONTEXT FOR AGENTS:
  Wait for `cloud org get <org-id>` to show state `infra-ready` before creating a service.
  Discover profiles with `cloud service profile list --region <region> --byoc-id <id>`.")]
    Create {
        /// Cloud region ID
        #[arg(long)]
        region: String,

        /// Cloud account ID
        #[arg(long)]
        account_id: String,

        /// Availability-zone suffix (repeatable)
        #[arg(long, required = true)]
        availability_zone_suffix: Vec<String>,

        /// VPC CIDR range
        #[arg(long)]
        vpc_cidr_range: String,

        /// Human-readable infrastructure name
        #[arg(long)]
        display_name: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update BYOC infrastructure
    Update {
        /// BYOC infrastructure ID
        byoc_id: String,

        /// New human-readable infrastructure name
        #[arg(long)]
        display_name: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete BYOC infrastructure
    Delete {
        /// BYOC infrastructure ID
        byoc_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl ByocCommands {
    fn is_write(&self) -> bool {
        match self {
            ByocCommands::Create { .. } => true,
            ByocCommands::Update { .. } => true,
            ByocCommands::Delete { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum PrometheusCommands {
    /// List Prometheus scrape targets (Beta)
    Discovery,
}

#[derive(Subcommand)]
pub enum QuotaCommands {
    /// List organization quotas
    List {
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get organization quota details
    Get {
        /// Quota code
        quota_code: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum MemberCommands {
    /// List organization members
    List {
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get member details
    Get {
        /// User ID
        user_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Update member roles
    Update {
        /// User ID
        user_id: String,

        /// Role ID to assign (repeatable; conflicts with --clear-roles)
        #[arg(long, conflicts_with = "clear_roles")]
        role_id: Vec<String>,

        /// Remove all assigned roles; conflicts with --role-id
        #[arg(long, conflicts_with = "role_id")]
        clear_roles: bool,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Remove a member from the organization
    Remove {
        /// User ID
        user_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl MemberCommands {
    pub fn is_write(&self) -> bool {
        match self {
            MemberCommands::List { .. } => false,
            MemberCommands::Get { .. } => false,
            MemberCommands::Update { .. } => true,
            MemberCommands::Remove { .. } => true,
        }
    }
}

#[derive(Subcommand)]
pub enum InvitationCommands {
    /// List pending invitations
    List {
        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Create an invitation
    Create {
        /// Email address to invite (stored lowercased)
        #[arg(long)]
        email: String,

        /// Role ID to assign (repeatable)
        #[arg(long)]
        role_id: Vec<String>,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Get invitation details
    Get {
        /// Invitation ID
        invitation_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },

    /// Delete an invitation
    Delete {
        /// Invitation ID
        invitation_id: String,

        /// Organization ID (auto-detected only if you have one org)
        #[arg(long)]
        org_id: Option<String>,
    },
}

impl InvitationCommands {
    pub fn is_write(&self) -> bool {
        match self {
            InvitationCommands::List { .. } => false,
            InvitationCommands::Get { .. } => false,
            InvitationCommands::Create { .. } => true,
            InvitationCommands::Delete { .. } => true,
        }
    }
}

pub async fn run_org(client: &CloudClient, command: OrgCommands, json: bool) -> CloudResult<()> {
    match command {
        OrgCommands::List => org_list(client, json).await,
        OrgCommands::Get { org_id } => org_get(client, &org_id, json).await,
        OrgCommands::Quota { command } => run_quota(client, command, json).await,
        OrgCommands::Balance { org_id } => org_balance(client, org_id.as_deref(), json).await,
        OrgCommands::Byoc { command } => run_byoc(client, command, json).await,
        OrgCommands::Update {
            org_id,
            name,
            remove_private_endpoint,
            enable_core_dumps,
        } => {
            let options = OrgUpdateOptions {
                name,
                remove_private_endpoints: remove_private_endpoint,
                enable_core_dumps,
            };
            org_update(client, &org_id, options, json).await
        }
        OrgCommands::Prometheus {
            command,
            org_id,
            legacy_org_id,
            filtered_metrics,
        } => {
            let org_id = org_id.as_deref().or(legacy_org_id.as_deref());
            match command {
                Some(PrometheusCommands::Discovery) => {
                    org_prometheus_discovery(client, org_id, filtered_metrics, json).await
                }
                None => org_prometheus(client, org_id, filtered_metrics, json).await,
            }
        }
        OrgCommands::Usage {
            org_id,
            legacy_org_id,
            from_date,
            to_date,
            filter,
        } => {
            let org_id = org_id.as_deref().or(legacy_org_id.as_deref());
            org_usage(client, org_id, &from_date, &to_date, &filter, json).await
        }
    }
}

async fn run_byoc(client: &CloudClient, command: ByocCommands, json: bool) -> CloudResult<()> {
    match command {
        ByocCommands::Create {
            region,
            account_id,
            availability_zone_suffix,
            vpc_cidr_range,
            display_name,
            org_id,
        } => {
            let request = build_byoc_create_request(
                &region,
                &account_id,
                &availability_zone_suffix,
                &vpc_cidr_range,
                &display_name,
            )?;
            byoc_create(client, request, org_id.as_deref(), json).await
        }
        ByocCommands::Update {
            byoc_id,
            display_name,
            org_id,
        } => {
            let request = build_byoc_update_request(&display_name);
            byoc_update(client, &byoc_id, request, org_id.as_deref(), json).await
        }
        ByocCommands::Delete { byoc_id, org_id } => {
            byoc_delete(client, &byoc_id, org_id.as_deref(), json).await
        }
    }
}

async fn run_quota(client: &CloudClient, command: QuotaCommands, json: bool) -> CloudResult<()> {
    match command {
        QuotaCommands::List { org_id } => quota_list(client, org_id.as_deref(), json).await,
        QuotaCommands::Get { quota_code, org_id } => {
            quota_get(client, &quota_code, org_id.as_deref(), json).await
        }
    }
}

pub async fn run_member(
    client: &CloudClient,
    command: MemberCommands,
    json: bool,
) -> CloudResult<()> {
    match command {
        MemberCommands::List { org_id } => member_list(client, org_id.as_deref(), json).await,
        MemberCommands::Get { user_id, org_id } => {
            member_get(client, &user_id, org_id.as_deref(), json).await
        }
        MemberCommands::Update {
            user_id,
            role_id,
            clear_roles,
            org_id,
        } => {
            member_update(
                client,
                &user_id,
                &role_id,
                clear_roles,
                org_id.as_deref(),
                json,
            )
            .await
        }
        MemberCommands::Remove { user_id, org_id } => {
            member_remove(client, &user_id, org_id.as_deref(), json).await
        }
    }
}

pub async fn run_invitation(
    client: &CloudClient,
    command: InvitationCommands,
    json: bool,
) -> CloudResult<()> {
    match command {
        InvitationCommands::List { org_id } => {
            invitation_list(client, org_id.as_deref(), json).await
        }
        InvitationCommands::Create {
            email,
            role_id,
            org_id,
        } => invitation_create(client, &email, &role_id, org_id.as_deref(), json).await,
        InvitationCommands::Get {
            invitation_id,
            org_id,
        } => invitation_get(client, &invitation_id, org_id.as_deref(), json).await,
        InvitationCommands::Delete {
            invitation_id,
            org_id,
        } => invitation_delete(client, &invitation_id, org_id.as_deref(), json).await,
    }
}

#[derive(Default)]
struct OrgUpdateOptions {
    name: Option<String>,
    remove_private_endpoints: Vec<String>,
    enable_core_dumps: Option<bool>,
}

fn parse_org_private_endpoint_remove(value: &str) -> CloudResult<OrganizationPatchPrivateEndpoint> {
    let mut endpoint = OrganizationPatchPrivateEndpoint {
        id: String::new(),
        description: None,
        cloud_provider: OrganizationPatchPrivateEndpointCloudprovider::default(),
        region: OrganizationPatchPrivateEndpointRegion::default(),
    };

    for (index, part) in value.split(',').enumerate() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if index == 0 && !part.contains('=') {
            endpoint.id = part.to_string();
            continue;
        }

        let (key, raw_value) = part.split_once('=').ok_or_else(|| {
            CloudError::new(format!(
                "invalid remove-private-endpoint segment '{}'",
                part
            ))
        })?;

        match key {
            "id" => endpoint.id = raw_value.to_string(),
            "description" => endpoint.description = Some(raw_value.to_string()),
            "cloud-provider" => {
                endpoint.cloud_provider =
                    serde_json::from_value::<OrganizationPatchPrivateEndpointCloudprovider>(
                        serde_json::Value::String(raw_value.to_string()),
                    )
                    .expect("enum with Unknown variant should always deserialize");
            }
            "region" => {
                endpoint.region = serde_json::from_value::<OrganizationPatchPrivateEndpointRegion>(
                    serde_json::Value::String(raw_value.to_string()),
                )
                .expect("enum with Unknown variant should always deserialize");
            }
            _ => {
                return Err(CloudError::new(format!(
                    "invalid remove-private-endpoint key '{}'; expected id, description, cloud-provider, or region",
                    key
                )));
            }
        }
    }

    if endpoint.id.trim().is_empty() {
        return Err(CloudError::new(format!(
            "remove-private-endpoint '{}' requires a non-empty id",
            value
        )));
    }

    Ok(endpoint)
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

fn parse_byoc_region(value: &str) -> CloudResult<ByocInfrastructurePostRequestRegionid> {
    let region = serde_json::from_value::<ByocInfrastructurePostRequestRegionid>(
        serde_json::Value::String(value.to_string()),
    )
    .map_err(|error| CloudError::new(format!("invalid region: {error}")))?;
    if matches!(region, ByocInfrastructurePostRequestRegionid::Unknown(_)) {
        return Err(CloudError::new(format!(
            "invalid region: unsupported BYOC region '{value}'"
        )));
    }
    Ok(region)
}

fn parse_byoc_availability_zone_suffix(value: &str) -> CloudResult<ByocAvailabilityZoneSuffix> {
    let suffix = serde_json::from_value::<ByocAvailabilityZoneSuffix>(serde_json::Value::String(
        value.to_string(),
    ))
    .map_err(|error| CloudError::new(format!("invalid availability zone suffix: {error}")))?;
    if matches!(suffix, ByocAvailabilityZoneSuffix::Unknown(_)) {
        return Err(CloudError::new(format!(
            "invalid availability zone suffix '{value}'"
        )));
    }
    Ok(suffix)
}

fn build_byoc_create_request(
    region: &str,
    account_id: &str,
    availability_zone_suffixes: &[String],
    vpc_cidr_range: &str,
    display_name: &str,
) -> CloudResult<ByocInfrastructurePostRequest> {
    let availability_zone_suffixes = availability_zone_suffixes
        .iter()
        .map(|suffix| parse_byoc_availability_zone_suffix(suffix))
        .collect::<CloudResult<Vec<_>>>()?;
    if availability_zone_suffixes.is_empty() {
        return Err(CloudError::new(
            "at least one --availability-zone-suffix is required",
        ));
    }

    Ok(ByocInfrastructurePostRequest {
        account_id: account_id.to_string(),
        availability_zone_suffixes,
        display_name: display_name.to_string(),
        region_id: parse_byoc_region(region)?,
        vpc_cidr_range: vpc_cidr_range.to_string(),
    })
}

fn build_byoc_update_request(display_name: &str) -> ByocInfrastructurePatchRequest {
    ByocInfrastructurePatchRequest {
        display_name: Some(display_name.to_string()),
    }
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

async fn quota_list(client: &CloudClient, org_id: Option<&str>, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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

async fn quota_get(
    client: &CloudClient,
    quota_code: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let quota = client.get_organization_quota(&org_id, quota_code).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&quota)?);
    } else {
        print_human(&quota)?;
    }
    Ok(())
}

async fn org_balance(client: &CloudClient, org_id: Option<&str>, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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
    org_id: &str,
    options: OrgUpdateOptions,
    json: bool,
) -> CloudResult<()> {
    let request = build_org_update_request(&options)?;
    let organization = client.update_organization(org_id, &request).await?;

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

async fn byoc_create(
    client: &CloudClient,
    request: ByocInfrastructurePostRequest,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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

async fn byoc_delete(
    client: &CloudClient,
    byoc_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let response = client.delete_byoc_infrastructure(&org_id, byoc_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("BYOC infrastructure {byoc_id} deleted");
    }
    Ok(())
}

async fn org_prometheus(
    client: &CloudClient,
    org_id: Option<&str>,
    filtered_metrics: Option<bool>,
    _json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let prometheus = client.get_org_prometheus(&org_id, filtered_metrics).await?;
    println!("{}", prometheus);
    Ok(())
}

async fn org_prometheus_discovery(
    client: &CloudClient,
    org_id: Option<&str>,
    filtered_metrics: Option<bool>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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
    org_id: Option<&str>,
    from_date: &str,
    to_date: &str,
    filters: &[String],
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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

async fn member_list(client: &CloudClient, org_id: Option<&str>, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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

async fn member_get(
    client: &CloudClient,
    user_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = build_member_update_request(role_ids, clear_roles);
    let member = client.update_member(&org_id, user_id, &request).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&member)?);
    } else {
        println!("Member {} updated", or_absent(member.email.as_deref()));
    }
    Ok(())
}

async fn member_remove(
    client: &CloudClient,
    user_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let response = client.delete_member(&org_id, user_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Member {} removed", user_id);
    }
    Ok(())
}

async fn invitation_list(
    client: &CloudClient,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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

async fn invitation_get(
    client: &CloudClient,
    invitation_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
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
                "Multiple organizations found. Specify --org-id to choose one. \
                 Use `clickhousectl cloud org list` to see your organizations.",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use clap::Parser;

    fn parse_cloud_command(args: &[&str]) -> CloudCommands {
        let cli = Cli::try_parse_from(args).expect("parse");
        let Commands::Cloud(cloud_args) = cli.command else {
            panic!("expected cloud command");
        };
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
        let CloudCommands::Org { command } =
            parse_cloud_command(&["clickhousectl", "cloud", "org", "update", "org-1"])
        else {
            panic!("expected org command");
        };
        let OrgCommands::Update {
            org_id,
            name,
            remove_private_endpoint,
            enable_core_dumps,
        } = command
        else {
            panic!("expected org update");
        };
        assert_eq!(org_id, "org-1");
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
            org_id,
        } = command
        else {
            panic!("expected member update");
        };
        assert_eq!(user_id, "user-1");
        assert!(role_id.is_empty());
        assert!(!clear_roles);
        assert!(org_id.is_none());

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
        let InvitationCommands::Create {
            email,
            role_id,
            org_id,
        } = command
        else {
            panic!("expected invitation create");
        };
        assert_eq!(email, "user@example.com");
        assert!(role_id.is_empty());
        assert!(org_id.is_none());
    }

    #[test]
    fn parses_organization_body_command_maximal_and_repeatable_flags() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "update",
            "org-1",
            "--name",
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
            org_id,
            name,
            remove_private_endpoint,
            enable_core_dumps,
        } = command
        else {
            panic!("expected org update");
        };
        assert_eq!(org_id, "org-1");
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
            org_id,
        } = command
        else {
            panic!("expected member update");
        };
        assert_eq!(user_id, "user-1");
        assert_eq!(role_id, vec!["role-1", "role-2"]);
        assert!(!clear_roles);
        assert_eq!(org_id.as_deref(), Some("org-1"));

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
        let InvitationCommands::Create {
            email,
            role_id,
            org_id,
        } = command
        else {
            panic!("expected invitation create");
        };
        assert_eq!(email, "user@example.com");
        assert_eq!(role_id, vec!["role-1", "role-2"]);
        assert_eq!(org_id.as_deref(), Some("org-1"));
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
            org_id,
            legacy_org_id,
            from_date,
            to_date,
            ..
        } = command
        else {
            panic!("expected org usage");
        };
        assert_eq!(org_id, None);
        assert_eq!(legacy_org_id, None);
        assert_eq!(from_date, "2025-01-01");
        assert_eq!(to_date, "2025-01-31");
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
        let OrgCommands::Prometheus { org_id, .. } = command else {
            panic!("expected org prometheus");
        };
        assert_eq!(org_id.as_deref(), Some("org-1"));

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
        let OrgCommands::Usage { org_id, .. } = command else {
            panic!("expected org usage");
        };
        assert_eq!(org_id.as_deref(), Some("org-1"));
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
            org_id,
            filtered_metrics,
            ..
        } = command
        else {
            panic!("expected prometheus discovery");
        };
        assert_eq!(org_id.as_deref(), Some("org-1"));
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
            command: QuotaCommands::List { org_id },
        } = command
        else {
            panic!("expected quota list command");
        };
        assert_eq!(org_id.as_deref(), Some("org-1"));

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
            command: QuotaCommands::Get { quota_code, org_id },
        } = command
        else {
            panic!("expected quota get command");
        };
        assert_eq!(quota_code, "replicas-per-warehouse");
        assert!(org_id.is_none());
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
        let OrgCommands::Balance { org_id } = command else {
            panic!("expected org balance command");
        };
        assert_eq!(org_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn parses_legacy_org_id_positionals() {
        let prometheus =
            Cli::try_parse_from(["clickhousectl", "cloud", "org", "prometheus", "org-1"]).unwrap();
        let Commands::Cloud(args) = prometheus.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Org { command } = args.command else {
            panic!("expected org command");
        };
        let OrgCommands::Prometheus {
            org_id,
            legacy_org_id,
            ..
        } = command
        else {
            panic!("expected org prometheus");
        };
        assert_eq!(org_id, None);
        assert_eq!(legacy_org_id.as_deref(), Some("org-1"));

        let usage = Cli::try_parse_from([
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
        let Commands::Cloud(args) = usage.command else {
            panic!("expected cloud command");
        };
        let crate::cloud::cli::CloudCommands::Org { command } = args.command else {
            panic!("expected org command");
        };
        let OrgCommands::Usage {
            org_id,
            legacy_org_id,
            ..
        } = command
        else {
            panic!("expected org usage");
        };
        assert_eq!(org_id, None);
        assert_eq!(legacy_org_id.as_deref(), Some("org-1"));
    }

    #[test]
    fn rejects_org_id_flag_with_legacy_positional() {
        let prometheus = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "prometheus",
            "org-1",
            "--org-id",
            "org-2",
        ]);
        match prometheus {
            Ok(_) => panic!("expected conflicting org IDs to be rejected"),
            Err(error) => assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict),
        }

        let usage = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "usage",
            "org-1",
            "--org-id",
            "org-2",
            "--from-date",
            "2025-01-01",
            "--to-date",
            "2025-01-31",
        ]);
        match usage {
            Ok(_) => panic!("expected conflicting org IDs to be rejected"),
            Err(error) => assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict),
        }
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
        assert_write(&["clickhousectl", "cloud", "org", "get", "org-1"], false);
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
        assert_write(&["clickhousectl", "cloud", "org", "update", "org-1"], true);

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
    fn parses_byoc_create_update_and_delete_commands() {
        let CloudCommands::Org { command } = parse_cloud_command(&[
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
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Byoc {
            command:
                ByocCommands::Create {
                    region,
                    account_id,
                    availability_zone_suffix,
                    vpc_cidr_range,
                    display_name,
                    org_id,
                },
        } = command
        else {
            panic!("expected BYOC create");
        };
        assert_eq!(region, "us-east-1");
        assert_eq!(account_id, "123456789012");
        assert_eq!(availability_zone_suffix, vec!["a", "b"]);
        assert_eq!(vpc_cidr_range, "10.0.0.0/16");
        assert_eq!(display_name, "production");
        assert_eq!(org_id.as_deref(), Some("org-1"));

        let CloudCommands::Org { command } = parse_cloud_command(&[
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "update",
            "byoc-1",
            "--display-name",
            "renamed",
        ]) else {
            panic!("expected org command");
        };
        let OrgCommands::Byoc {
            command:
                ByocCommands::Update {
                    byoc_id,
                    display_name,
                    org_id,
                },
        } = command
        else {
            panic!("expected BYOC update");
        };
        assert_eq!(byoc_id, "byoc-1");
        assert_eq!(display_name, "renamed");
        assert!(org_id.is_none());

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
    fn byoc_create_requires_an_availability_zone_suffix() {
        let result = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "org",
            "byoc",
            "create",
            "--region",
            "us-east-1",
            "--account-id",
            "123456789012",
            "--vpc-cidr-range",
            "10.0.0.0/16",
            "--display-name",
            "production",
        ]);
        let Err(error) = result else {
            panic!("missing availability zone suffix must fail");
        };
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn build_byoc_create_request_supports_minimal_and_maximal_zones() {
        let minimal = build_byoc_create_request(
            "us-east-1",
            "123456789012",
            &["a".to_string()],
            "10.0.0.0/16",
            "production",
        )
        .unwrap();
        assert_eq!(
            minimal.region_id,
            ByocInfrastructurePostRequestRegionid::Us_east_1
        );
        assert_eq!(minimal.account_id, "123456789012");
        assert_eq!(
            minimal.availability_zone_suffixes,
            vec![ByocAvailabilityZoneSuffix::A]
        );
        assert_eq!(minimal.vpc_cidr_range, "10.0.0.0/16");
        assert_eq!(minimal.display_name, "production");

        let maximal = build_byoc_create_request(
            "eastus",
            "azure-account",
            &[
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string(),
                "f".to_string(),
            ],
            "10.20.0.0/16",
            "all-zones",
        )
        .unwrap();
        assert_eq!(
            maximal.region_id,
            ByocInfrastructurePostRequestRegionid::Eastus
        );
        assert_eq!(maximal.availability_zone_suffixes.len(), 6);
    }

    #[test]
    fn build_byoc_requests_validate_enums_and_update_only_the_name() {
        assert!(
            build_byoc_create_request(
                "future-region",
                "account",
                &["a".to_string()],
                "10.0.0.0/16",
                "name",
            )
            .is_err()
        );
        assert!(
            build_byoc_create_request(
                "us-east-1",
                "account",
                &["z".to_string()],
                "10.0.0.0/16",
                "name",
            )
            .is_err()
        );

        let update = build_byoc_update_request("renamed");
        assert_eq!(update.display_name.as_deref(), Some("renamed"));
        assert_eq!(
            serde_json::to_value(update).unwrap(),
            serde_json::json!({"displayName": "renamed"})
        );
    }
}
