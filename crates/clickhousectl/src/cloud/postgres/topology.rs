use super::config::{load_json_file, load_pg_config_file};
#[cfg(doc)]
use super::lifecycle::postgres_state_change;
use super::lifecycle::{
    build_postgres_update_request, merge_response_tags, postgres_create_ip_access_list,
    render_postgres_connection_guidance, render_postgres_service,
};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{eprint_line, print_line};
use crate::cloud::shared::{parse_tags, resolve_org_id};
use clickhouse_cloud_api::models::{
    PgBouncerConfig, PostgresIpAccessListEntry, PostgresIpAccessListEntryResponse,
    PostgresIpAccessListPatch, PostgresIpAccessListRuleReference, PostgresService,
    PostgresServiceReadReplicaRequest, PostgresServiceRestoreRequest,
    PostgresServiceSetStateCommand, ResourceTagsV1,
};
use std::path::Path;

/// Copy a source service's access rules as create entries, without their IDs.
/// `None` when the source omitted its rule list or a rule's address or ports,
/// since copying a partial list would silently drop restrictions.
fn postgres_entries_from_rules(
    rules: Option<&[PostgresIpAccessListEntryResponse]>,
) -> Option<Vec<PostgresIpAccessListEntry>> {
    rules?
        .iter()
        .map(|rule| {
            Some(PostgresIpAccessListEntry {
                description: rule.description.clone(),
                id: None,
                port_range: rule.port_range.clone()?,
                source: rule.source.clone()?,
            })
        })
        .collect()
}

/// The `ipAccessList` patch that turns a restored service's `current` rules
/// into `desired`: remove each current rule with no exact match, add each
/// desired rule with none. `None` when they already match. A rule matches on
/// address, ports and description; an absent and an empty description match.
fn plan_postgres_restored_access_rules(
    current: &[PostgresIpAccessListEntryResponse],
    desired: &[PostgresIpAccessListEntry],
) -> CloudResult<Option<PostgresIpAccessListPatch>> {
    type Key<'a> = (Option<&'a str>, Option<&'a str>, &'a str);
    fn current_key(rule: &PostgresIpAccessListEntryResponse) -> Key<'_> {
        (
            rule.source.as_deref(),
            rule.port_range.as_deref(),
            rule.description.as_deref().unwrap_or(""),
        )
    }
    fn desired_key(rule: &PostgresIpAccessListEntry) -> Key<'_> {
        (
            Some(rule.source.as_str()),
            Some(rule.port_range.as_str()),
            rule.description.as_deref().unwrap_or(""),
        )
    }
    let mut remove = Vec::new();
    for rule in current {
        if desired.iter().any(|d| desired_key(d) == current_key(rule)) {
            continue;
        }
        let id = rule.id.clone().ok_or_else(|| {
            CloudError::new(
                "the API returned an access rule without an ID, so it cannot be removed",
            )
        })?;
        remove.push(PostgresIpAccessListRuleReference { id });
    }
    let add: Vec<_> = desired
        .iter()
        .filter(|d| {
            !current
                .iter()
                .any(|rule| current_key(rule) == desired_key(d))
        })
        .cloned()
        .collect();
    Ok(
        (!add.is_empty() || !remove.is_empty())
            .then_some(PostgresIpAccessListPatch { add, remove }),
    )
}

pub struct PostgresReadReplicaOptions<'a> {
    pub name: &'a str,
    pub tags: &'a [String],
    pub pg_config_file: Option<&'a Path>,
    pub pg_bouncer_config_file: Option<&'a Path>,
}

pub struct PostgresRestoreOptions<'a> {
    pub name: &'a str,
    pub restore_target: &'a str,
    pub tags: &'a [String],
    pub pg_config_file: Option<&'a Path>,
    pub pg_bouncer_config_file: Option<&'a Path>,
    pub ip_allow: &'a [PostgresIpAccessListEntry],
    pub block_public_access: bool,
}

pub async fn postgres_read_replica_create(
    client: &CloudClient,
    postgres_id: &str,
    opts: PostgresReadReplicaOptions<'_>,
    json: bool,
) -> CloudResult<()> {
    let tags = parse_tags(opts.tags)?;
    let pg_config = opts.pg_config_file.map(load_pg_config_file).transpose()?;
    let pg_bouncer_config = opts
        .pg_bouncer_config_file
        .map(load_json_file::<PgBouncerConfig>)
        .transpose()?;

    let req = PostgresServiceReadReplicaRequest {
        name: opts.name.to_string(),
        tags,
        pg_config,
        pg_bouncer_config,
    };

    let org_id = resolve_org_id(client).await?;
    let svc = client
        .create_postgres_read_replica(&org_id, postgres_id, &req)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Read replica created");
        println!();
        render_postgres_service(&svc);
        render_postgres_connection_guidance(&svc);
    }
    Ok(())
}

pub async fn postgres_restore(
    client: &CloudClient,
    postgres_id: &str,
    opts: PostgresRestoreOptions<'_>,
    json: bool,
) -> CloudResult<()> {
    let tags = parse_tags(opts.tags)?;
    let pg_config = opts.pg_config_file.map(load_pg_config_file).transpose()?;
    let pg_bouncer_config = opts
        .pg_bouncer_config_file
        .map(load_json_file::<PgBouncerConfig>)
        .transpose()?;
    let restore_target = chrono::DateTime::parse_from_rfc3339(opts.restore_target)
        .map_err(|e| CloudError::new(format!("invalid restore-target: {}", e)))?
        .with_timezone(&chrono::Utc);

    let org_id = resolve_org_id(client).await?;

    // The restore endpoint takes no access rules and allows all addresses on
    // 5432 and 6432, so the rules are applied afterwards. Read the source's
    // first, so a source whose rules cannot be copied creates nothing.
    let desired = match postgres_create_ip_access_list(opts.ip_allow, opts.block_public_access) {
        Some(rules) => rules,
        None => {
            let source = client.get_postgres_service(&org_id, postgres_id).await?;
            postgres_entries_from_rules(source.ip_access_list.as_deref()).ok_or_else(|| {
                CloudError::new(format!(
                    "the API did not return complete access rules for source {postgres_id}, so \
                     they cannot be copied and nothing was restored; pass --ip-allow or \
                     --block-public-access to set the restored service's rules"
                ))
            })?
        }
    };

    let req = PostgresServiceRestoreRequest {
        name: opts.name.to_string(),
        restore_target,
        tags: tags.clone(),
        pg_config,
        pg_bouncer_config,
    };
    let svc = client
        .restore_postgres_service(&org_id, postgres_id, &req)
        .await?;

    let svc = apply_restored_access_rules(client, &org_id, svc, &desired, tags).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&svc)?);
    } else {
        println!("Postgres service restore initiated");
        println!();
        render_postgres_service(&svc);
        render_postgres_connection_guidance(&svc);
    }
    Ok(())
}

/// Replace the rules a restore created with `desired`, returning the service
/// to show. Any failure names the restored service, which then still exists
/// with rules that may allow all addresses.
async fn apply_restored_access_rules(
    client: &CloudClient,
    org_id: &str,
    svc: PostgresService,
    desired: &[PostgresIpAccessListEntry],
    requested_tags: Option<Vec<ResourceTagsV1>>,
) -> CloudResult<PostgresService> {
    let Some(restored_id) = svc.id.as_ref().map(ToString::to_string) else {
        return Err(CloudError::new(
            "the restore was accepted, but the API returned no service ID, so its access \
             rules were not applied; find the new service with `cloud postgres list` and \
             set its rules with `cloud postgres update --remove-ip-allow`",
        ));
    };
    let unapplied = |error: CloudError| CloudError {
        message: format!(
            "restored service {restored_id} was created, but its access rules were not fully \
             applied and it may allow all addresses on 5432 and 6432: {}. Read its rules with \
             `cloud postgres get {restored_id}` and fix them with \
             `cloud postgres update {restored_id} --remove-ip-allow <rule-id>`",
            error.message
        ),
        ..error
    };

    // Sparse restore responses omit fields; the PATCH needs the rule IDs, and
    // the complete tag set because the API clears tags a PATCH omits.
    let current = if svc.ip_access_list.is_some() && svc.tags.is_some() {
        svc.clone()
    } else {
        client
            .get_postgres_service(org_id, &restored_id)
            .await
            .map_err(unapplied)?
    };
    let rules = current.ip_access_list.as_deref().ok_or_else(|| {
        unapplied(CloudError::new(
            "the API did not return the restored service's access rules",
        ))
    })?;
    let Some(patch) = plan_postgres_restored_access_rules(rules, desired).map_err(unapplied)?
    else {
        return Ok(svc);
    };
    let tags = match current.tags {
        Some(tags) => merge_response_tags(Some(tags), &[], &[]).map_err(unapplied)?,
        None => requested_tags.unwrap_or_default(),
    };
    let req = build_postgres_update_request(None, None, None, Some(tags), Some(patch));
    let patched = client
        .patch_postgres_service(org_id, &restored_id, &req)
        .await
        .map_err(unapplied)?;
    if patched.ip_access_list.is_some() {
        return Ok(patched);
    }
    client
        .get_postgres_service(org_id, &restored_id)
        .await
        .map_err(|error| CloudError {
            message: format!(
                "access rules were applied to restored service {restored_id}, but reading it \
                 back failed: {}. Check it with `cloud postgres get {restored_id}`",
                error.message
            ),
            ..error
        })
}

// ---------------------------------------------------------------------------
// Role changes: promote / switchover
// ---------------------------------------------------------------------------

/// The `cloud postgres` commands that initiate primary-role operations.
///
/// `restart` is not one of them: it keeps going through
/// [`postgres_state_change`], which needs no role bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostgresRoleCommand {
    Promote,
    Switchover,
}

impl PostgresRoleCommand {
    fn api_command(self) -> PostgresServiceSetStateCommand {
        match self {
            Self::Promote => PostgresServiceSetStateCommand::Promote,
            Self::Switchover => PostgresServiceSetStateCommand::Switchover,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Promote => "promote",
            Self::Switchover => "switchover",
        }
    }
}

/// stderr notes for a role change, in print order.
fn role_change_notes(cmd: PostgresRoleCommand, postgres_id: &str) -> Vec<String> {
    match cmd {
        PostgresRoleCommand::Promote => vec![format!(
            "Promotion is asynchronous. Postgres service {postgres_id} becomes an independent \
             primary and its source remains primary; poll `clickhousectl cloud postgres get \
             {postgres_id} --json` until isPrimary is true."
        )],
        PostgresRoleCommand::Switchover => vec![format!(
            "Switchover is asynchronous inside Postgres service {postgres_id}. Service isPrimary \
             and state, plus database readiness, do not identify the active HA node or confirm \
             completion. Monitor client connectivity separately; these service fields expose no \
             completion check."
        )],
    }
}

/// Handles `cloud postgres promote` and `cloud postgres switchover`.
///
/// The command is issued as-is and returns the API acknowledgment. Completion
/// is asynchronous and must be verified according to the operation.
pub async fn postgres_role_change(
    client: &CloudClient,
    postgres_id: &str,
    cmd: PostgresRoleCommand,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let svc = client
        .set_postgres_service_state(&org_id, postgres_id, cmd.api_command())
        .await?;

    for note in role_change_notes(cmd, postgres_id) {
        eprint_line(note);
    }

    // `print_line`, not `println!`: a closed pipe must not turn an accepted
    // role change into a panic (#598).
    if json {
        print_line(serde_json::to_string_pretty(&svc)?);
    } else {
        print_line(format!("{} accepted", cmd.label()));
        print_line("");
        render_postgres_service(&svc);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::postgres::lifecycle::tests::access_rule;

    fn response_rule(
        id: Option<&str>,
        source: Option<&str>,
        port_range: Option<&str>,
        description: Option<&str>,
    ) -> PostgresIpAccessListEntryResponse {
        PostgresIpAccessListEntryResponse {
            description: description.map(str::to_string),
            id: id.map(str::to_string),
            port_range: port_range.map(str::to_string),
            source: source.map(str::to_string),
        }
    }

    #[test]
    fn postgres_entries_from_rules_copies_complete_rules_without_ids() {
        let rules = vec![
            response_rule(Some("r1"), Some("203.0.113.0/24"), Some("5432"), None),
            response_rule(
                Some("r2"),
                Some("198.51.100.7/32"),
                Some("6432"),
                Some("pooler"),
            ),
        ];
        assert_eq!(
            postgres_entries_from_rules(Some(&rules)),
            Some(vec![
                access_rule("203.0.113.0/24", "5432", None),
                access_rule("198.51.100.7/32", "6432", Some("pooler")),
            ])
        );
        assert_eq!(postgres_entries_from_rules(Some(&[])), Some(Vec::new()));
    }

    #[test]
    fn postgres_entries_from_rules_refuses_incomplete_rules() {
        assert_eq!(postgres_entries_from_rules(None), None);
        for rule in [
            response_rule(Some("r1"), None, Some("5432"), None),
            response_rule(Some("r1"), Some("203.0.113.0/24"), None, None),
        ] {
            assert_eq!(postgres_entries_from_rules(Some(&[rule])), None);
        }
    }

    #[test]
    fn plan_postgres_restored_access_rules_replaces_unmatched_rules() {
        let current = vec![
            response_rule(Some("d1"), Some("0.0.0.0/0"), Some("5432"), None),
            response_rule(Some("d2"), Some("0.0.0.0/0"), Some("6432"), None),
        ];
        let desired = vec![
            access_rule("0.0.0.0/0", "6432", Some("")),
            access_rule("203.0.113.0/24", "5432", Some("office")),
        ];
        let patch = plan_postgres_restored_access_rules(&current, &desired)
            .unwrap()
            .unwrap();
        assert_eq!(
            patch.remove,
            vec![PostgresIpAccessListRuleReference { id: "d1".into() }]
        );
        assert_eq!(
            patch.add,
            vec![access_rule("203.0.113.0/24", "5432", Some("office"))]
        );

        let blocked = plan_postgres_restored_access_rules(&current, &[])
            .unwrap()
            .unwrap();
        assert!(blocked.add.is_empty());
        assert_eq!(blocked.remove.len(), 2);
    }

    #[test]
    fn plan_postgres_restored_access_rules_skips_matching_rules() {
        let current = vec![response_rule(
            Some("d1"),
            Some("0.0.0.0/0"),
            Some("5432"),
            None,
        )];
        let desired = vec![access_rule("0.0.0.0/0", "5432", None)];
        assert_eq!(
            plan_postgres_restored_access_rules(&current, &desired).unwrap(),
            None
        );
        assert_eq!(plan_postgres_restored_access_rules(&[], &[]).unwrap(), None);
    }

    #[test]
    fn plan_postgres_restored_access_rules_refuses_a_rule_without_an_id() {
        let current = vec![response_rule(None, Some("0.0.0.0/0"), Some("5432"), None)];
        let err = plan_postgres_restored_access_rules(&current, &[]).unwrap_err();
        assert!(err.message.contains("without an ID"), "{err}");
    }
}
