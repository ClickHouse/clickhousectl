use super::cli::{NotificationSettingsListArgs, NotificationSettingsUpdateArgs};
use super::org::print_next_cursor;
use crate::cloud::client::{CloudClient, Result as CloudResult};
use crate::cloud::config::read_typed_config;
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

/// The notification-settings body: the `--file` document as given, or the
/// one rule the flags describe. Clap guarantees exactly one of the two.
pub(super) fn build_notification_settings_update_request(
    args: &NotificationSettingsUpdateArgs,
) -> CloudResult<MemberNotificationSettingsPatchRequest> {
    if let Some(file) = &args.file {
        return read_typed_config(file);
    }
    let Some(effect) = args.effect else {
        return Err(crate::cloud::client::CloudError::usage(
            "supply --effect with --notification and --resource, or --file",
        ));
    };
    Ok(MemberNotificationSettingsPatchRequest {
        rules: Some(vec![NotificationSettingRule {
            channels: (!args.channels.is_empty()).then(|| {
                args.channels
                    .iter()
                    .map(|channel| channel.as_str().to_string())
                    .collect()
            }),
            effect: effect.as_str().to_string(),
            notifications: args.notifications.clone(),
            resources: args.resources.clone(),
        }]),
    })
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

pub(super) async fn notification_settings_list(
    client: &CloudClient,
    user_id: &str,
    args: NotificationSettingsListArgs,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let page = client
        .list_member_notification_settings(&org_id, user_id, &args)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&page)?);
    } else {
        if page.result.is_empty() {
            println!("No notification settings found; role defaults apply");
        } else {
            println!("{}", notification_settings_table(page.result));
        }
        print_next_cursor(page.next_cursor.as_deref())?;
    }
    Ok(())
}

pub(super) async fn notification_settings_update(
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
        let settings = response.settings.unwrap_or_default();
        if settings.is_empty() {
            println!(
                "No notification settings remain for the addressed types; role defaults apply"
            );
        } else {
            println!("{}", notification_settings_table(settings));
        }
    }
    Ok(())
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

    use crate::cloud::organizations::cli::{NotificationChannel, NotificationSettingEffect};

    #[test]
    fn build_notification_settings_update_request_minimal_rule_omits_channels() {
        let request = build_notification_settings_update_request(&NotificationSettingsUpdateArgs {
            effect: Some(NotificationSettingEffect::Inherit),
            notifications: vec!["service_backup_failed".into()],
            resources: vec!["service-1".into()],
            ..Default::default()
        })
        .unwrap();
        let rules = request.rules.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].effect, "inherit");
        assert_eq!(rules[0].notifications, ["service_backup_failed"]);
        assert_eq!(rules[0].resources, ["service-1"]);
        assert_eq!(rules[0].channels, None);
    }

    #[test]
    fn build_notification_settings_update_request_maximal_rule_maps_every_flag() {
        let request = build_notification_settings_update_request(&NotificationSettingsUpdateArgs {
            effect: Some(NotificationSettingEffect::Disabled),
            notifications: vec!["service_backup_failed".into(), "billing.credit_low".into()],
            resources: vec!["service-1".into(), "org-1".into()],
            channels: vec![NotificationChannel::Ui, NotificationChannel::Email],
            file: None,
        })
        .unwrap();
        assert_eq!(
            request.rules,
            Some(vec![NotificationSettingRule {
                channels: Some(vec!["ui".into(), "email".into()]),
                effect: "disabled".into(),
                notifications: vec!["service_backup_failed".into(), "billing.credit_low".into()],
                resources: vec!["service-1".into(), "org-1".into()],
            }])
        );
    }

    #[test]
    fn build_notification_settings_update_request_reads_a_whole_body_from_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let body = serde_json::json!({"rules": [
            {"effect": "enabled", "notifications": ["a"], "resources": ["r1"]},
            {"effect": "future", "notifications": ["b"], "resources": ["r2"], "channels": ["email"]}
        ]});
        std::fs::write(&path, body.to_string()).unwrap();
        let request = build_notification_settings_update_request(&NotificationSettingsUpdateArgs {
            file: Some(path.to_str().unwrap().to_string()),
            ..Default::default()
        })
        .unwrap();
        // Rules pass through in order; open-set values stay for the server to judge.
        assert_eq!(serde_json::to_value(&request).unwrap(), body);

        std::fs::write(&path, r#"{"rules": [], "extra": true}"#).unwrap();
        let error = build_notification_settings_update_request(&NotificationSettingsUpdateArgs {
            file: Some(path.to_str().unwrap().to_string()),
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Usage);
        assert!(error.message.contains("extra"), "{error}");
    }

    #[test]
    fn notification_settings_table_renders_absent_fields() {
        let table = notification_settings_table(vec![
            serde_json::from_value(serde_json::json!({
                "type": "service_backup_failed",
                "resourceType": "service",
                "resourceId": "service-1",
                "channelType": "email",
                "effect": "disabled"
            }))
            .unwrap(),
            NotificationSetting::default(),
        ]);
        let rows: Vec<Vec<&str>> = table
            .lines()
            .skip(2)
            .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
            .collect();
        assert_eq!(
            rows,
            [
                vec![
                    "service_backup_failed",
                    "service",
                    "service-1",
                    "email",
                    "disabled"
                ],
                vec![ABSENT; 5],
            ],
            "{table}"
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
