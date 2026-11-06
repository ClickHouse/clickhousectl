use super::cli::NotificationListArgs;
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{ABSENT, or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use clickhouse_cloud_api::models::{
    Notification, NotificationType, OrganizationPatchPrivateEndpoint,
    OrganizationPatchPrivateEndpointCloudprovider, OrganizationPatchPrivateEndpointRegion,
    OrganizationPatchRequest, OrganizationPrivateEndpointsPatch,
};
use tabled::{Table, Tabled, settings::Style};

#[derive(Default)]
pub(super) struct OrgUpdateOptions {
    pub(super) name: Option<String>,
    pub(super) remove_private_endpoints: Vec<String>,
    pub(super) enable_core_dumps: Option<bool>,
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
pub(super) fn parse_org_private_endpoint_remove_arg(value: &str) -> Result<String, String> {
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

pub(super) async fn org_list(client: &CloudClient, json: bool) -> CloudResult<()> {
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

pub(super) async fn org_get(client: &CloudClient, org_id: &str, json: bool) -> CloudResult<()> {
    let organization = client.get_organization(org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&organization)?);
    } else {
        print_human(&organization)?;
    }
    Ok(())
}

pub(super) async fn quota_list(client: &CloudClient, json: bool) -> CloudResult<()> {
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

pub(super) async fn quota_get(
    client: &CloudClient,
    quota_code: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let quota = client.get_organization_quota(&org_id, quota_code).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&quota)?);
    } else {
        print_human(&quota)?;
    }
    Ok(())
}

#[derive(serde::Serialize)]
pub(super) struct NotificationPage {
    pub(super) result: Vec<Notification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) limit: Option<i64>,
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub(super) next_cursor: Option<String>,
}

pub(super) async fn org_notifications(
    client: &CloudClient,
    args: NotificationListArgs,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let page = client.list_notifications(&org_id, &args).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&page)?);
    } else {
        if page.result.is_empty() {
            println!("No notifications found");
        } else {
            println!("{}", notifications_table(page.result));
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

fn notifications_table(notifications: Vec<Notification>) -> String {
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "ID")]
        id: String,
        #[tabled(rename = "Created At")]
        created_at: String,
        #[tabled(rename = "Severity")]
        severity: String,
        #[tabled(rename = "Type")]
        notification_type: String,
        #[tabled(rename = "Resource Type")]
        resource_type: String,
        #[tabled(rename = "Resource ID")]
        resource_id: String,
        #[tabled(rename = "Title")]
        title: String,
    }
    let rows: Vec<Row> = notifications
        .into_iter()
        .map(|notification| Row {
            id: or_absent(notification.id),
            created_at: or_absent(notification.created_at),
            severity: or_absent(notification.severity),
            notification_type: or_absent(notification.r#type),
            resource_type: or_absent(notification.resource_type),
            resource_id: or_absent(notification.resource_id),
            title: or_absent(notification.title),
        })
        .collect();
    Table::new(rows).with(Style::markdown()).to_string()
}

pub(super) async fn org_notification_types(client: &CloudClient, json: bool) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let types = client.list_notification_types(&org_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&types)?);
    } else {
        if types.is_empty() {
            println!("No notification types found");
            return Ok(());
        }
        println!("{}", notification_types_table(types));
    }
    Ok(())
}

fn notification_types_table(types: Vec<NotificationType>) -> String {
    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Name")]
        name: String,
        #[tabled(rename = "Title")]
        title: String,
        #[tabled(rename = "Category")]
        category: String,
        #[tabled(rename = "Severity")]
        severity: String,
        #[tabled(rename = "Resource Type")]
        resource_type: String,
        #[tabled(rename = "Configurable")]
        configurable: String,
        #[tabled(rename = "Available")]
        available: String,
    }
    let rows: Vec<Row> = types
        .into_iter()
        .map(|kind| Row {
            name: or_absent(kind.name.as_deref()),
            title: or_absent(kind.title.as_deref()),
            category: or_absent(kind.category.as_deref()),
            severity: or_absent(kind.severity.as_ref()),
            resource_type: or_absent(kind.resource_type.as_deref()),
            configurable: or_absent(kind.is_configurable),
            available: or_absent(kind.is_available),
        })
        .collect();
    Table::new(rows).with(Style::markdown()).to_string()
}

pub(super) async fn org_balance(client: &CloudClient, json: bool) -> CloudResult<()> {
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

pub(super) async fn org_update(
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

pub(super) async fn org_prometheus(
    client: &CloudClient,
    filtered_metrics: Option<bool>,
    _json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let prometheus = client.get_org_prometheus(&org_id, filtered_metrics).await?;
    println!("{}", prometheus);
    Ok(())
}

pub(super) async fn org_prometheus_discovery(
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

pub(super) async fn org_usage(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notifications_table_tolerates_absent_fields() {
        let table = notifications_table(vec![
            Notification::default(),
            Notification {
                title: Some("Backup failed".into()),
                severity: Some("future".into()),
                ..Default::default()
            },
        ]);
        assert!(table.contains("Backup failed"));
        assert!(table.contains("future"));
        assert!(table.contains("-"));
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

    #[test]
    fn notification_types_table_renders_absent_fields() {
        let table = notification_types_table(vec![
            serde_json::from_value(serde_json::json!({
                "name": "billing.credit_low",
                "title": "Credits running low",
                "category": "billing",
                "severity": "warning",
                "resourceType": "organization",
                "isConfigurable": true,
                "isAvailable": false
            }))
            .unwrap(),
            NotificationType::default(),
        ]);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 4, "{table}");
        for value in ["billing.credit_low", "warning", "organization", "false"] {
            assert!(lines[2].contains(value), "{value}: {table}");
        }
        assert_eq!(
            lines[3]
                .split('|')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>(),
            vec![ABSENT; 7]
        );
    }
}
