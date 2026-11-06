use crate::cloud::client::{CloudClient, Result as CloudResult};
use crate::cloud::output::{ABSENT, or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use clickhouse_cloud_api::models::{InvitationPostRequest, MemberPatchRequest};
use tabled::{Table, Tabled, settings::Style};

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
        // The spec makes `assignedRoleIds` optional; keep sending it (possibly
        // `[]`) so the create request is unchanged.
        assigned_role_ids: Some(role_ids.to_vec()),
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

pub(super) async fn member_list(client: &CloudClient, json: bool) -> CloudResult<()> {
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

pub(super) async fn member_get(client: &CloudClient, user_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let member = client.get_member(&org_id, user_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&member)?);
    } else {
        print_human(&member)?;
    }
    Ok(())
}

pub(super) async fn member_update(
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

pub(super) async fn member_remove(
    client: &CloudClient,
    user_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.delete_member(&org_id, user_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Member {} removed", user_id);
    }
    Ok(())
}

pub(super) async fn invitation_list(client: &CloudClient, json: bool) -> CloudResult<()> {
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

pub(super) async fn invitation_create(
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

pub(super) async fn invitation_get(
    client: &CloudClient,
    invitation_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let invitation = client.get_invitation(&org_id, invitation_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&invitation)?);
    } else {
        print_human(&invitation)?;
    }
    Ok(())
}

pub(super) async fn invitation_delete(
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(request.assigned_role_ids, Some(vec![]));
        // The spec makes `assignedRoleIds` optional; the wire request still sends `[]`.
        assert_eq!(
            serde_json::to_value(&request).unwrap()["assignedRoleIds"],
            serde_json::json!([])
        );
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
        assert_eq!(
            request.assigned_role_ids.as_deref(),
            Some(&["role-1".to_string(), "role-2".to_string()][..])
        );
        #[cfg(feature = "deprecated-fields")]
        assert!(request.role.is_none());
    }
}
