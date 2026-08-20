use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{ABSENT, or_absent, print_human};
use crate::cloud::shared::{parse_date_only, resolve_org_id};
use crate::cloud::types::DeleteResponse;
use clap::Subcommand;
use clickhouse_cloud_api::models::{
    InvitationPostRequest, MemberPatchRequest, OrganizationPatchPrivateEndpoint,
    OrganizationPatchPrivateEndpointCloudprovider, OrganizationPatchPrivateEndpointRegion,
    OrganizationPatchRequest, OrganizationPrivateEndpointsPatch,
};
use tabled::{Table, Tabled, settings::Style};

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
        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,

        /// Organization ID (deprecated positional form; use --org-id)
        #[arg(value_name = "ORG_ID", hide = true, conflicts_with = "org_id")]
        legacy_org_id: Option<String>,

        /// Whether to request filtered metrics
        #[arg(long)]
        filtered_metrics: Option<bool>,
    },

    /// Get organization usage/billing information
    Usage {
        /// Organization ID (auto-detected if not specified)
        #[arg(long)]
        org_id: Option<String>,

        /// Organization ID (deprecated positional form; use --org-id)
        #[arg(value_name = "ORG_ID", hide = true, conflicts_with = "org_id")]
        legacy_org_id: Option<String>,

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

impl OrgCommands {
    pub fn is_write(&self) -> bool {
        match self {
            OrgCommands::List => false,
            OrgCommands::Get { .. } => false,
            OrgCommands::Prometheus { .. } => false,
            OrgCommands::Usage { .. } => false,
            OrgCommands::Update { .. } => true,
        }
    }
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
            org_id,
            legacy_org_id,
            filtered_metrics,
        } => {
            let org_id = org_id.as_deref().or(legacy_org_id.as_deref());
            org_prometheus(client, org_id, filtered_metrics, json).await
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
            org_id,
        } => member_update(client, &user_id, &role_id, org_id.as_deref(), json).await,
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

fn build_member_update_request(role_ids: &[String]) -> MemberPatchRequest {
    MemberPatchRequest {
        assigned_role_ids: if role_ids.is_empty() {
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
    org_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = build_member_update_request(role_ids);
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
        let response = self
            .api()
            .organization_get(org_id)
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
            org_id,
        } = command
        else {
            panic!("expected member update");
        };
        assert_eq!(user_id, "user-1");
        assert!(role_id.is_empty());
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
            org_id,
        } = command
        else {
            panic!("expected member update");
        };
        assert_eq!(user_id, "user-1");
        assert_eq!(role_id, vec!["role-1", "role-2"]);
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
        assert_write(&["clickhousectl", "cloud", "org", "prometheus"], false);
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
        let request = build_member_update_request(&[]);

        assert!(request.assigned_role_ids.is_none());
        #[cfg(feature = "deprecated-fields")]
        assert!(request.role.is_none());
    }

    #[test]
    fn build_member_update_request_supports_maximal_fields() {
        let request = build_member_update_request(&["role-1".to_string(), "role-2".to_string()]);

        assert_eq!(
            request.assigned_role_ids,
            Some(vec!["role-1".to_string(), "role-2".to_string()])
        );
        #[cfg(feature = "deprecated-fields")]
        assert!(request.role.is_none());
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
}
