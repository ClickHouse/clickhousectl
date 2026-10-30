mod byoc;
mod cli;
mod client;
mod members;
mod org;
mod roles;

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
    Permission::api("org notifications", &[&op::NOTIFICATION_GET_LIST]),
    Permission::api("org usage", &[&op::USAGE_COST_GET]),
    Permission::api("org quota list", &[&op::ORGANIZATION_QUOTAS_GET_LIST]),
    Permission::api("org quota get", &[&op::ORGANIZATION_QUOTA_GET]),
    Permission::api(
        "org notification-types",
        &[&op::NOTIFICATION_TYPES_GET_LIST],
    ),
    Permission::api("org byoc get", &[&op::ORGANIZATION_BYOC_INFRASTRUCTURE_GET])
        .when(&[Conditional::flag("name", &[&op::ORGANIZATION_GET])]),
    Permission::api(
        "org byoc progress",
        &[&op::ORGANIZATION_BYOC_INFRASTRUCTURE_PROGRESS_GET],
    )
    .when(&[Conditional::flag("name", &[&op::ORGANIZATION_GET])]),
    Permission::api(
        "org byoc private-endpoint-config",
        &[&op::ORGANIZATION_BYOC_INFRASTRUCTURE_PRIVATE_ENDPOINT_CONFIG_GET],
    )
    .when(&[Conditional::flag("name", &[&op::ORGANIZATION_GET])]),
    Permission::api(
        "org byoc tags",
        &[&op::ORGANIZATION_BYOC_INFRASTRUCTURE_TAGS_GET],
    )
    .when(&[Conditional::flag("name", &[&op::ORGANIZATION_GET])]),
    Permission::api(
        "org byoc validate",
        &[&op::ORGANIZATION_BYOC_INFRASTRUCTURE_VALIDATE],
    ),
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

pub use cli::{
    ByocCommands, InvitationCommands, MemberCommands, OrgCommands, PrometheusCommands,
    QuotaCommands, RoleCommands,
};

use crate::cloud::client::{CloudClient, Result as CloudResult};
use crate::cloud::shared::{NamedResource, resolve_org_id};
use byoc::{
    build_byoc_create_request, build_byoc_update_request, build_byoc_validate_request, byoc_create,
    byoc_delete, byoc_get, byoc_private_endpoint_config, byoc_progress, byoc_tags, byoc_update,
    byoc_validate,
};
use members::{
    invitation_create, invitation_delete, invitation_get, invitation_list, member_get, member_list,
    member_remove, member_update,
};
use org::{
    OrgUpdateOptions, org_balance, org_get, org_list, org_notification_types, org_notifications,
    org_prometheus, org_prometheus_discovery, org_update, org_usage, quota_get, quota_list,
};
use roles::{
    build_role_create_request, build_role_update_request, role_create, role_delete, role_get,
    role_list, role_update,
};

pub async fn run_org(client: &CloudClient, command: OrgCommands, json: bool) -> CloudResult<()> {
    match command {
        OrgCommands::List => org_list(client, json).await,
        OrgCommands::Get => {
            let org_id = resolve_org_id(client).await?;
            org_get(client, &org_id, json).await
        }
        OrgCommands::Quota { command } => run_quota(client, command, json).await,
        OrgCommands::Balance => org_balance(client, json).await,
        OrgCommands::Notifications(args) => org_notifications(client, args, json).await,
        OrgCommands::NotificationTypes => org_notification_types(client, json).await,
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
        ByocCommands::PrivateEndpointConfig { byoc_id } => {
            byoc_private_endpoint_config(
                client,
                &byoc_id.resolve(client, NamedResource::Byoc).await?,
                json,
            )
            .await
        }
        ByocCommands::Tags { byoc_id } => {
            byoc_tags(
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
        ByocCommands::Update { byoc_id, changes } => {
            let request = build_byoc_update_request(&changes)?;
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
