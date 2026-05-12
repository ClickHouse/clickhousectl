use crate::cloud::client::CloudClient;
use crate::cloud::commands::{parse_serde_enum, resolve_org_id};
use chrono::{DateTime, Utc};
use clickhouse_cloud_api::models::{
    ClickStackAlertChannel, ClickStackAlertChannelEmail, ClickStackAlertChannelEmailType,
    ClickStackAlertChannelWebhook, ClickStackAlertChannelWebhookSeverity,
    ClickStackAlertChannelWebhookType, ClickStackAlertResponse, ClickStackCreateAlertRequest,
    ClickStackCreateAlertRequestInterval, ClickStackCreateAlertRequestSource,
    ClickStackCreateAlertRequestThresholdtype, ClickStackCreateDashboardRequest,
    ClickStackDashboardResponse, ClickStackUpdateAlertRequest,
    ClickStackUpdateAlertRequestInterval, ClickStackUpdateAlertRequestSource,
    ClickStackUpdateAlertRequestThresholdtype, ClickStackUpdateDashboardRequest,
};
use std::io::Read;
use tabled::{Table, Tabled, settings::Style};

const KNOWN_INTERVALS: &[&str] = &["1m", "5m", "15m", "30m", "1h", "6h", "12h", "1d"];
const KNOWN_ALERT_SOURCES: &[&str] = &["saved_search", "tile"];
const KNOWN_THRESHOLD_TYPES: &[&str] = &["above", "below"];
const KNOWN_CHANNEL_TYPES: &[&str] = &["email", "webhook"];
const KNOWN_WEBHOOK_SEVERITIES: &[&str] = &["critical", "error", "warning", "info"];

// ===========================================================================
// Dashboards
// ===========================================================================

pub async fn clickstack_dashboard_list(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let dashboards = client
        .clickstack_list_dashboards(&org_id, service_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&dashboards)?);
    } else if dashboards.is_empty() {
        println!("No dashboards found");
    } else {
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "Tags")]
            tags: String,
            #[tabled(rename = "Tiles")]
            tiles: usize,
        }
        let rows: Vec<Row> = dashboards
            .into_iter()
            .map(|d| Row {
                id: d.id,
                name: d.name,
                tags: d.tags.join(", "),
                tiles: d.tiles.len(),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub async fn clickstack_dashboard_get(
    client: &CloudClient,
    service_id: &str,
    dashboard_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let dashboard = client
        .clickstack_get_dashboard(&org_id, service_id, dashboard_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&dashboard)?);
    } else {
        print_dashboard(&dashboard);
    }
    Ok(())
}

pub struct DashboardWriteArgs<'a> {
    pub from_file: &'a str,
    pub name_override: Option<&'a str>,
    pub tag_overrides: &'a [String],
}

pub async fn clickstack_dashboard_create(
    client: &CloudClient,
    service_id: &str,
    args: DashboardWriteArgs<'_>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request =
        load_dashboard_create_request(args.from_file, args.name_override, args.tag_overrides)?;
    let dashboard = client
        .clickstack_create_dashboard(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&dashboard)?);
    } else {
        println!("Dashboard created");
        print_dashboard(&dashboard);
    }
    Ok(())
}

pub async fn clickstack_dashboard_update(
    client: &CloudClient,
    service_id: &str,
    dashboard_id: &str,
    args: DashboardWriteArgs<'_>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request =
        load_dashboard_update_request(args.from_file, args.name_override, args.tag_overrides)?;
    let dashboard = client
        .clickstack_update_dashboard(&org_id, service_id, dashboard_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&dashboard)?);
    } else {
        println!("Dashboard updated");
        print_dashboard(&dashboard);
    }
    Ok(())
}

pub async fn clickstack_dashboard_delete(
    client: &CloudClient,
    service_id: &str,
    dashboard_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let response = client
        .clickstack_delete_dashboard(&org_id, service_id, dashboard_id)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Dashboard deleted: {dashboard_id}");
    }
    Ok(())
}

// ===========================================================================
// Alerts
// ===========================================================================

#[allow(clippy::too_many_arguments)]
pub struct AlertCreateArgs<'a> {
    pub name: Option<&'a str>,
    pub threshold: f64,
    pub threshold_max: Option<f64>,
    pub threshold_type: &'a str,
    pub interval: &'a str,
    pub source: &'a str,
    pub group_by: Option<&'a str>,
    pub message: Option<&'a str>,
    pub dashboard_id: Option<&'a str>,
    pub tile_id: Option<&'a str>,
    pub saved_search_id: Option<&'a str>,
    pub schedule_offset_minutes: Option<i64>,
    pub schedule_start_at: Option<&'a str>,
    pub channel_type: &'a str,
    pub emails: &'a [String],
    pub webhook_id: Option<&'a str>,
    pub webhook_service: Option<&'a str>,
    pub severity: Option<&'a str>,
    pub slack_channel_id: Option<&'a str>,
}

pub async fn clickstack_alert_list(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let alerts = client.clickstack_list_alerts(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&alerts)?);
    } else if alerts.is_empty() {
        println!("No alerts found");
    } else {
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "Source")]
            source: String,
            #[tabled(rename = "Threshold")]
            threshold: String,
            #[tabled(rename = "Interval")]
            interval: String,
            #[tabled(rename = "Channel")]
            channel: String,
        }
        let rows: Vec<Row> = alerts
            .into_iter()
            .map(|a| Row {
                id: a.id,
                name: a.name.unwrap_or_default(),
                source: a.source.to_string(),
                threshold: format!("{} {}", a.threshold_type, a.threshold),
                interval: a.interval.to_string(),
                channel: summarise_channel(&a.channel),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub async fn clickstack_alert_get(
    client: &CloudClient,
    service_id: &str,
    alert_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let alert = client
        .clickstack_get_alert(&org_id, service_id, alert_id)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&alert)?);
    } else {
        print_alert(&alert);
    }
    Ok(())
}

pub async fn clickstack_alert_create(
    client: &CloudClient,
    service_id: &str,
    args: AlertCreateArgs<'_>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = build_alert_create_request(args)?;
    let alert = client
        .clickstack_create_alert(&org_id, service_id, &request)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&alert)?);
    } else {
        println!("Alert created");
        print_alert(&alert);
    }
    Ok(())
}

pub async fn clickstack_alert_update(
    client: &CloudClient,
    service_id: &str,
    alert_id: &str,
    args: AlertCreateArgs<'_>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = build_alert_update_request(args)?;
    let alert = client
        .clickstack_update_alert(&org_id, service_id, alert_id, &request)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&alert)?);
    } else {
        println!("Alert updated");
        print_alert(&alert);
    }
    Ok(())
}

pub async fn clickstack_alert_delete(
    client: &CloudClient,
    service_id: &str,
    alert_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let response = client
        .clickstack_delete_alert(&org_id, service_id, alert_id)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Alert deleted: {alert_id}");
    }
    Ok(())
}

// ===========================================================================
// Sources & Webhooks (read-only)
// ===========================================================================

pub async fn clickstack_source_list(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let sources = client.clickstack_list_sources(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&sources)?);
    } else if sources.is_empty() {
        println!("No sources found");
    } else {
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Kind")]
            kind: String,
            #[tabled(rename = "Name")]
            name: String,
        }
        let rows: Vec<Row> = sources
            .iter()
            .map(|s| {
                use clickhouse_cloud_api::models::ClickStackSource;
                match s {
                    ClickStackSource::ClickStackLogSource(s) => Row {
                        id: s.id.clone(),
                        kind: s.kind.to_string(),
                        name: s.name.clone(),
                    },
                    ClickStackSource::ClickStackTraceSource(s) => Row {
                        id: s.id.clone(),
                        kind: s.kind.to_string(),
                        name: s.name.clone(),
                    },
                    ClickStackSource::ClickStackMetricSource(s) => Row {
                        id: s.id.clone(),
                        kind: s.kind.to_string(),
                        name: s.name.clone(),
                    },
                    ClickStackSource::ClickStackSessionSource(s) => Row {
                        id: s.id.clone(),
                        kind: s.kind.to_string(),
                        name: s.name.clone(),
                    },
                    ClickStackSource::Unknown(s) => Row {
                        id: String::new(),
                        kind: String::from("unknown"),
                        name: s.clone(),
                    },
                }
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub async fn clickstack_webhook_list(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let webhooks = client.clickstack_list_webhooks(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&webhooks)?);
    } else if webhooks.is_empty() {
        println!("No webhooks found");
    } else {
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Service")]
            service: String,
            #[tabled(rename = "Name")]
            name: String,
        }
        let rows: Vec<Row> = webhooks
            .iter()
            .map(|w| {
                use clickhouse_cloud_api::models::ClickStackWebhook;
                match w {
                    ClickStackWebhook::ClickStackSlackWebhook(w) => Row {
                        id: w.id.clone(),
                        service: "slack".into(),
                        name: w.name.clone(),
                    },
                    ClickStackWebhook::ClickStackIncidentIOWebhook(w) => Row {
                        id: w.id.clone(),
                        service: "incident.io".into(),
                        name: w.name.clone(),
                    },
                    ClickStackWebhook::ClickStackGenericWebhook(w) => Row {
                        id: w.id.clone(),
                        service: "generic".into(),
                        name: w.name.clone(),
                    },
                    ClickStackWebhook::ClickStackSlackAPIWebhook(w) => Row {
                        id: w.id.clone(),
                        service: "slack-api".into(),
                        name: w.name.clone(),
                    },
                    ClickStackWebhook::ClickStackPagerDutyAPIWebhook(w) => Row {
                        id: w.id.clone(),
                        service: "pagerduty".into(),
                        name: w.name.clone(),
                    },
                    ClickStackWebhook::Unknown(s) => Row {
                        id: String::new(),
                        service: "unknown".into(),
                        name: s.clone(),
                    },
                }
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

// ===========================================================================
// Builders & formatters
// ===========================================================================

fn read_payload(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    if path == "-" {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        Ok(std::fs::read_to_string(path)?)
    }
}

fn load_dashboard_create_request(
    path: &str,
    name_override: Option<&str>,
    tag_overrides: &[String],
) -> Result<ClickStackCreateDashboardRequest, Box<dyn std::error::Error>> {
    let body = read_payload(path)?;
    let mut req: ClickStackCreateDashboardRequest =
        serde_json::from_str(&body).map_err(|e| format!("failed to parse {}: {}", path, e))?;
    if let Some(name) = name_override {
        req.name = name.to_string();
    }
    if !tag_overrides.is_empty() {
        let mut tags = req.tags.unwrap_or_default();
        tags.extend(tag_overrides.iter().cloned());
        req.tags = Some(tags);
    }
    Ok(req)
}

fn load_dashboard_update_request(
    path: &str,
    name_override: Option<&str>,
    tag_overrides: &[String],
) -> Result<ClickStackUpdateDashboardRequest, Box<dyn std::error::Error>> {
    let body = read_payload(path)?;
    let mut req: ClickStackUpdateDashboardRequest =
        serde_json::from_str(&body).map_err(|e| format!("failed to parse {}: {}", path, e))?;
    if let Some(name) = name_override {
        req.name = name.to_string();
    }
    if !tag_overrides.is_empty() {
        let mut tags = req.tags.unwrap_or_default();
        tags.extend(tag_overrides.iter().cloned());
        req.tags = Some(tags);
    }
    Ok(req)
}

fn build_alert_create_request(
    args: AlertCreateArgs<'_>,
) -> Result<ClickStackCreateAlertRequest, Box<dyn std::error::Error>> {
    let interval: ClickStackCreateAlertRequestInterval =
        parse_serde_enum(args.interval, "interval", KNOWN_INTERVALS)?;
    let source: ClickStackCreateAlertRequestSource =
        parse_serde_enum(args.source, "source", KNOWN_ALERT_SOURCES)?;
    let threshold_type: ClickStackCreateAlertRequestThresholdtype =
        parse_serde_enum(args.threshold_type, "threshold-type", KNOWN_THRESHOLD_TYPES)?;
    let channel = build_alert_channel(
        args.channel_type,
        args.emails,
        args.webhook_id,
        args.webhook_service,
        args.severity,
        args.slack_channel_id,
    )?;
    validate_source_args(
        args.source,
        args.dashboard_id,
        args.tile_id,
        args.saved_search_id,
    )?;
    let schedule_start_at = parse_schedule_start_at(args.schedule_start_at)?;

    Ok(ClickStackCreateAlertRequest {
        name: args.name.map(str::to_string),
        threshold: args.threshold,
        threshold_max: args.threshold_max,
        threshold_type,
        interval,
        source,
        group_by: args.group_by.map(str::to_string),
        message: args.message.map(str::to_string),
        dashboard_id: args.dashboard_id.map(str::to_string),
        tile_id: args.tile_id.map(str::to_string),
        saved_search_id: args.saved_search_id.map(str::to_string),
        schedule_offset_minutes: args.schedule_offset_minutes,
        schedule_start_at,
        channel,
    })
}

fn build_alert_update_request(
    args: AlertCreateArgs<'_>,
) -> Result<ClickStackUpdateAlertRequest, Box<dyn std::error::Error>> {
    let interval: ClickStackUpdateAlertRequestInterval =
        parse_serde_enum(args.interval, "interval", KNOWN_INTERVALS)?;
    let source: ClickStackUpdateAlertRequestSource =
        parse_serde_enum(args.source, "source", KNOWN_ALERT_SOURCES)?;
    let threshold_type: ClickStackUpdateAlertRequestThresholdtype =
        parse_serde_enum(args.threshold_type, "threshold-type", KNOWN_THRESHOLD_TYPES)?;
    let channel = build_alert_channel(
        args.channel_type,
        args.emails,
        args.webhook_id,
        args.webhook_service,
        args.severity,
        args.slack_channel_id,
    )?;
    validate_source_args(
        args.source,
        args.dashboard_id,
        args.tile_id,
        args.saved_search_id,
    )?;
    let schedule_start_at = parse_schedule_start_at(args.schedule_start_at)?;

    Ok(ClickStackUpdateAlertRequest {
        name: args.name.map(str::to_string),
        threshold: args.threshold,
        threshold_max: args.threshold_max,
        threshold_type,
        interval,
        source,
        group_by: args.group_by.map(str::to_string),
        message: args.message.map(str::to_string),
        dashboard_id: args.dashboard_id.map(str::to_string),
        tile_id: args.tile_id.map(str::to_string),
        saved_search_id: args.saved_search_id.map(str::to_string),
        schedule_offset_minutes: args.schedule_offset_minutes,
        schedule_start_at,
        channel,
    })
}

fn build_alert_channel(
    channel_type: &str,
    emails: &[String],
    webhook_id: Option<&str>,
    webhook_service: Option<&str>,
    severity: Option<&str>,
    slack_channel_id: Option<&str>,
) -> Result<ClickStackAlertChannel, Box<dyn std::error::Error>> {
    if !KNOWN_CHANNEL_TYPES.contains(&channel_type) {
        return Err(format!(
            "invalid channel-type: unknown value '{}', expected one of: {}",
            channel_type,
            KNOWN_CHANNEL_TYPES.join(", ")
        )
        .into());
    }
    match channel_type {
        "email" => {
            if emails.is_empty() {
                return Err("--channel-type=email requires at least one --email".into());
            }
            Ok(ClickStackAlertChannel::ClickStackAlertChannelEmail(
                ClickStackAlertChannelEmail {
                    email_recipients: emails.to_vec(),
                    r#type: ClickStackAlertChannelEmailType::Email,
                },
            ))
        }
        "webhook" => {
            let webhook_id = webhook_id
                .ok_or("--channel-type=webhook requires --webhook-id")?
                .to_string();
            let severity = severity
                .map(|s| parse_serde_enum(s, "severity", KNOWN_WEBHOOK_SEVERITIES))
                .transpose()?;
            let severity: Option<ClickStackAlertChannelWebhookSeverity> = severity;
            Ok(ClickStackAlertChannel::ClickStackAlertChannelWebhook(
                ClickStackAlertChannelWebhook {
                    webhook_id,
                    webhook_service: webhook_service.map(str::to_string),
                    severity,
                    slack_channel_id: slack_channel_id.map(str::to_string),
                    r#type: ClickStackAlertChannelWebhookType::Webhook,
                },
            ))
        }
        _ => unreachable!("channel type already validated"),
    }
}

fn validate_source_args(
    source: &str,
    dashboard_id: Option<&str>,
    tile_id: Option<&str>,
    saved_search_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    match source {
        "tile" if dashboard_id.is_none() => {
            Err("--source=tile requires --dashboard-id".into())
        }
        "tile" if tile_id.is_none() => Err("--source=tile requires --tile-id".into()),
        "saved_search" if saved_search_id.is_none() => {
            Err("--source=saved_search requires --saved-search-id".into())
        }
        _ => Ok(()),
    }
}

fn parse_schedule_start_at(
    value: Option<&str>,
) -> Result<Option<DateTime<Utc>>, Box<dyn std::error::Error>> {
    match value {
        Some(s) => Ok(Some(
            DateTime::parse_from_rfc3339(s)
                .map_err(|e| format!("invalid schedule-start-at: {}", e))?
                .with_timezone(&Utc),
        )),
        None => Ok(None),
    }
}

fn summarise_channel(channel: &ClickStackAlertChannel) -> String {
    match channel {
        ClickStackAlertChannel::ClickStackAlertChannelEmail(_) => "email".to_string(),
        ClickStackAlertChannel::ClickStackAlertChannelWebhook(w) => {
            format!("webhook({})", w.webhook_id)
        }
        ClickStackAlertChannel::Unknown(s) => s.clone(),
    }
}

fn print_dashboard(d: &ClickStackDashboardResponse) {
    println!("Dashboard: {}", d.id);
    println!("  Name: {}", d.name);
    println!("  Tags: {}", d.tags.join(", "));
    println!("  Tiles: {}", d.tiles.len());
    println!("  Filters: {}", d.filters.len());
}

fn print_alert(a: &ClickStackAlertResponse) {
    println!("Alert: {}", a.id);
    if let Some(name) = &a.name {
        println!("  Name: {}", name);
    }
    println!("  Source: {}", a.source);
    println!("  Interval: {}", a.interval);
    println!("  Threshold: {} {}", a.threshold_type, a.threshold);
    println!("  Channel: {}", summarise_channel(&a.channel));
    if let Some(dashboard_id) = &a.dashboard_id {
        println!("  Dashboard ID: {}", dashboard_id);
    }
    if let Some(tile_id) = &a.tile_id {
        println!("  Tile ID: {}", tile_id);
    }
    if let Some(saved_search_id) = &a.saved_search_id {
        println!("  Saved Search ID: {}", saved_search_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    fn email_args<'a>() -> AlertCreateArgs<'a> {
        AlertCreateArgs {
            name: Some("disk"),
            threshold: 90.0,
            threshold_max: None,
            threshold_type: "above",
            interval: "5m",
            source: "saved_search",
            group_by: None,
            message: None,
            dashboard_id: None,
            tile_id: None,
            saved_search_id: Some("ss-1"),
            schedule_offset_minutes: None,
            schedule_start_at: None,
            channel_type: "email",
            emails: EMAIL_LIST.as_ref(),
            webhook_id: None,
            webhook_service: None,
            severity: None,
            slack_channel_id: None,
        }
    }

    // `emails` is a borrowed slice — keep one statically allocated for tests
    // so the test arg builders can hand it out by reference without lifetime gymnastics.
    static EMAIL_LIST: std::sync::LazyLock<Vec<String>> =
        std::sync::LazyLock::new(|| vec!["a@b.com".to_string()]);

    #[test]
    fn build_alert_request_email_channel() {
        let req = build_alert_create_request(email_args()).unwrap();
        assert_eq!(req.threshold, 90.0);
        assert!(matches!(
            req.channel,
            ClickStackAlertChannel::ClickStackAlertChannelEmail(_)
        ));
        if let ClickStackAlertChannel::ClickStackAlertChannelEmail(email) = req.channel {
            assert_eq!(email.email_recipients, vec!["a@b.com".to_string()]);
        }
    }

    #[test]
    fn build_alert_request_webhook_channel_with_severity() {
        let args = AlertCreateArgs {
            source: "tile",
            dashboard_id: Some("dash-1"),
            tile_id: Some("tile-1"),
            saved_search_id: None,
            channel_type: "webhook",
            emails: &[],
            webhook_id: Some("wh-1"),
            severity: Some("critical"),
            slack_channel_id: Some("C123"),
            ..email_args()
        };
        let req = build_alert_create_request(args).unwrap();
        if let ClickStackAlertChannel::ClickStackAlertChannelWebhook(w) = req.channel {
            assert_eq!(w.webhook_id, "wh-1");
            assert_eq!(w.slack_channel_id.as_deref(), Some("C123"));
            assert!(matches!(
                w.severity,
                Some(ClickStackAlertChannelWebhookSeverity::Critical)
            ));
        } else {
            panic!("expected webhook channel");
        }
    }

    #[test]
    fn build_alert_request_email_without_recipient_errors() {
        let args = AlertCreateArgs {
            emails: &[],
            ..email_args()
        };
        let err = build_alert_create_request(args).unwrap_err().to_string();
        assert!(err.contains("--email"), "got error: {err}");
    }

    #[test]
    fn build_alert_request_webhook_without_id_errors() {
        let args = AlertCreateArgs {
            source: "tile",
            dashboard_id: Some("dash-1"),
            tile_id: Some("tile-1"),
            saved_search_id: None,
            channel_type: "webhook",
            emails: &[],
            webhook_id: None,
            ..email_args()
        };
        let err = build_alert_create_request(args).unwrap_err().to_string();
        assert!(err.contains("--webhook-id"), "got error: {err}");
    }

    #[test]
    fn build_alert_request_tile_source_missing_dashboard_id_errors() {
        let args = AlertCreateArgs {
            source: "tile",
            dashboard_id: None,
            tile_id: Some("tile-1"),
            saved_search_id: None,
            ..email_args()
        };
        let err = build_alert_create_request(args).unwrap_err().to_string();
        assert!(err.contains("--dashboard-id"), "got error: {err}");
    }

    #[test]
    fn build_alert_request_saved_search_source_missing_id_errors() {
        let args = AlertCreateArgs {
            source: "saved_search",
            saved_search_id: None,
            ..email_args()
        };
        let err = build_alert_create_request(args).unwrap_err().to_string();
        assert!(err.contains("--saved-search-id"), "got error: {err}");
    }

    #[test]
    fn build_alert_request_invalid_interval_errors() {
        let args = AlertCreateArgs {
            interval: "7m",
            ..email_args()
        };
        let err = build_alert_create_request(args).unwrap_err().to_string();
        assert!(err.contains("interval"), "got error: {err}");
        assert!(err.contains("5m"), "expected error to list known values");
    }

    #[test]
    fn build_alert_request_invalid_channel_type_errors() {
        let args = AlertCreateArgs {
            channel_type: "sms",
            ..email_args()
        };
        let err = build_alert_create_request(args).unwrap_err().to_string();
        assert!(err.contains("channel-type"), "got error: {err}");
    }

    #[test]
    fn load_dashboard_request_applies_name_and_tag_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dash.json");
        let body = r#"{
            "name": "from-file",
            "tiles": [],
            "tags": ["original"]
        }"#;
        std::fs::write(&path, body).unwrap();
        let req = load_dashboard_create_request(
            path.to_str().unwrap(),
            Some("renamed"),
            &["extra".to_string()],
        )
        .unwrap();
        assert_eq!(req.name, "renamed");
        assert_eq!(
            req.tags.as_deref(),
            Some(["original".to_string(), "extra".to_string()].as_slice())
        );
    }

    #[test]
    fn load_dashboard_request_rejects_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dash.json");
        std::fs::write(&path, "{not json").unwrap();
        let err = load_dashboard_create_request(path.to_str().unwrap(), None, &[])
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed to parse"), "got error: {err}");
    }

    #[test]
    fn cli_parses_clickstack_dashboard_create() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickstack",
            "dashboard",
            "create",
            "svc-1",
            "--from-file",
            "dash.json",
            "--name",
            "renamed",
            "--tag",
            "smoke",
        ])
        .expect("dashboard create should parse");
        let _ = cli;
    }

    #[test]
    fn cli_parses_clickstack_alert_create_webhook() {
        let cli = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickstack",
            "alert",
            "create",
            "svc-1",
            "--threshold",
            "1",
            "--threshold-type",
            "above",
            "--interval",
            "5m",
            "--source",
            "tile",
            "--dashboard-id",
            "dash-1",
            "--tile-id",
            "tile-1",
            "--channel-type",
            "webhook",
            "--webhook-id",
            "wh-1",
        ])
        .expect("alert create webhook should parse");
        let _ = cli;
    }

    #[test]
    fn parse_schedule_start_at_round_trip() {
        let dt = parse_schedule_start_at(Some("2026-05-12T10:00:00Z"))
            .unwrap()
            .unwrap();
        assert_eq!(dt.to_rfc3339(), "2026-05-12T10:00:00+00:00");
    }

    #[test]
    fn parse_schedule_start_at_rejects_invalid() {
        let err = parse_schedule_start_at(Some("not a date"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("schedule-start-at"), "got error: {err}");
    }
}
