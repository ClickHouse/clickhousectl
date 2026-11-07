use super::cli::{
    MemberNotificationSettingsListArgs, MemberNotificationSettingsUpdateArgs, NotificationChannel,
    NotificationSettingEffect,
};
use crate::cloud::client::{CloudClient, Result as CloudResult};
use crate::cloud::output::{ABSENT, or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use clickhouse_cloud_api::models::{
    InvitationPostRequest, MemberNotificationSettingsPatchRequest, MemberPatchRequest,
    NotificationSetting, NotificationSettingRule,
};
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

#[derive(serde::Serialize)]
pub(super) struct NotificationSettingsPage {
    pub(super) result: Vec<NotificationSetting>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) limit: Option<i64>,
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub(super) next_cursor: Option<String>,
}

fn notification_settings_table(settings: Vec<NotificationSetting>) -> String {
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Type")]
        notification_type: String,
        #[tabled(rename = "Resource Type")]
        resource_type: String,
        #[tabled(rename = "Resource ID")]
        resource_id: String,
        #[tabled(rename = "Channel")]
        channel: String,
        #[tabled(rename = "Effect")]
        effect: String,
    }
    let rows: Vec<Row> = settings
        .into_iter()
        .map(|setting| Row {
            notification_type: or_absent(setting.r#type),
            resource_type: or_absent(setting.resource_type),
            resource_id: or_absent(setting.resource_id),
            channel: or_absent(setting.channel_type),
            effect: or_absent(setting.effect),
        })
        .collect();
    Table::new(rows).with(Style::markdown()).to_string()
}

pub(super) async fn member_notification_settings_list(
    client: &CloudClient,
    user_id: &str,
    args: &MemberNotificationSettingsListArgs,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let page = client
        .list_member_notification_settings(&org_id, user_id, args)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&page)?);
    } else {
        if page.result.is_empty() {
            println!("No explicit notification settings found");
        } else {
            println!("{}", notification_settings_table(page.result));
        }
        if let Some(cursor) = page.next_cursor {
            // JSON escaping keeps opaque tokens visible without suggesting shell execution.
            println!(
                "Next cursor (JSON string): {}",
                serde_json::to_string(&cursor)?
            );
        }
    }
    Ok(())
}

pub(super) fn build_member_notification_settings_request(
    args: &MemberNotificationSettingsUpdateArgs,
) -> MemberNotificationSettingsPatchRequest {
    let effect = match args.effect {
        NotificationSettingEffect::Enabled => "enabled",
        NotificationSettingEffect::Disabled => "disabled",
        NotificationSettingEffect::Inherit => "inherit",
    };
    let channels = args
        .channels
        .iter()
        .map(|channel| match channel {
            NotificationChannel::Ui => "ui".to_string(),
            NotificationChannel::Email => "email".to_string(),
        })
        .collect::<Vec<_>>();
    MemberNotificationSettingsPatchRequest {
        rules: Some(vec![NotificationSettingRule {
            channels: (!channels.is_empty()).then_some(channels),
            effect: effect.to_string(),
            notifications: args.notification_types.clone(),
            resources: args.resource_ids.clone(),
        }]),
    }
}

pub(super) async fn member_notification_settings_update(
    client: &CloudClient,
    user_id: &str,
    request: MemberNotificationSettingsPatchRequest,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client
        .update_member_notification_settings(&org_id, user_id, &request)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        match response.settings {
            Some(settings) if !settings.is_empty() => {
                println!("{}", notification_settings_table(settings));
            }
            _ => println!("No explicit settings remain for the addressed notifications"),
        }
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

    fn update_args(
        effect: NotificationSettingEffect,
        channels: Vec<NotificationChannel>,
    ) -> MemberNotificationSettingsUpdateArgs {
        MemberNotificationSettingsUpdateArgs {
            user_id: crate::cloud::shared::EmailSelector {
                id: Some("user-1".into()),
                email: None,
            },
            effect,
            notification_types: vec!["service_backup_failed".into()],
            resource_ids: vec!["svc-1".into()],
            channels,
        }
    }

    #[test]
    fn build_member_notification_settings_request_minimal_omits_channels() {
        let request = build_member_notification_settings_request(&update_args(
            NotificationSettingEffect::Inherit,
            vec![],
        ));
        let rules = request.rules.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].effect, "inherit");
        assert_eq!(rules[0].notifications, ["service_backup_failed"]);
        assert_eq!(rules[0].resources, ["svc-1"]);
        assert_eq!(rules[0].channels, None);
    }

    #[test]
    fn build_member_notification_settings_request_maximal_maps_every_flag() {
        for (effect, wire) in [
            (NotificationSettingEffect::Enabled, "enabled"),
            (NotificationSettingEffect::Disabled, "disabled"),
        ] {
            let mut args = update_args(
                effect,
                vec![NotificationChannel::Ui, NotificationChannel::Email],
            );
            args.notification_types.push("billing_credit_low".into());
            args.resource_ids.push("org-1".into());
            let rule = build_member_notification_settings_request(&args)
                .rules
                .unwrap()
                .remove(0);
            assert_eq!(rule.effect, wire);
            assert_eq!(
                rule.notifications,
                ["service_backup_failed", "billing_credit_low"]
            );
            assert_eq!(rule.resources, ["svc-1", "org-1"]);
            assert_eq!(
                rule.channels,
                Some(vec!["ui".to_string(), "email".to_string()])
            );
        }
    }

    #[test]
    fn notification_settings_table_renders_absent_fields() {
        let table = notification_settings_table(vec![
            serde_json::from_value(serde_json::json!({
                "type": "service_backup_failed",
                "resourceType": "service",
                "resourceId": "svc-1",
                "channelType": "email",
                "effect": "disabled"
            }))
            .unwrap(),
            NotificationSetting::default(),
        ]);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 4, "{table}");
        for value in ["service_backup_failed", "svc-1", "email", "disabled"] {
            assert!(lines[2].contains(value), "{value}: {table}");
        }
        assert_eq!(
            lines[3]
                .split('|')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>(),
            vec![ABSENT; 5]
        );
    }
}
