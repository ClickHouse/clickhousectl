use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::config::read_typed_config;
use crate::cloud::output::{ABSENT, or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use clickhouse_cloud_api::models::{
    RBACPolicyCreateRequest, RBACPolicyCreateRequestAllowdeny, RBACPolicyTagsRolev2,
    RoleCreateRequest, RoleUpdateRequest,
};
use tabled::{Table, Tabled, settings::Style};

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

pub(super) fn build_role_create_request(config_file: &str) -> CloudResult<RoleCreateRequest> {
    let request: RoleCreateRequest = read_typed_config(config_file)?;
    validate_role_policies(&request.policies, config_file)?;
    Ok(request)
}

pub(super) fn build_role_update_request(config_file: &str) -> CloudResult<RoleUpdateRequest> {
    let request: RoleUpdateRequest = read_typed_config(config_file)?;
    if let Some(policies) = &request.policies {
        validate_role_policies(policies, config_file)?;
    }
    Ok(request)
}

pub(super) async fn role_list(client: &CloudClient, json: bool) -> CloudResult<()> {
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

pub(super) async fn role_get(client: &CloudClient, role_id: &str, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let role = client.get_organization_role(&org_id, role_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&role)?);
    } else {
        print_human(&role)?;
    }
    Ok(())
}

pub(super) async fn role_create(
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

pub(super) async fn role_update(
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

pub(super) async fn role_delete(
    client: &CloudClient,
    role_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.delete_organization_role(&org_id, role_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Organization role {role_id} deleted");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
