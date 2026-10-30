use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{ABSENT, or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use clickhouse_cloud_api::models::{
    ReplicaAwareRouting, ReplicaAwareRoutingPatchRequest, UpgradeWindow, UpgradeWindowPutRequest,
    UpgradeWindowStartHourUtc,
};
use tabled::{Table, Tabled, settings::Style};

fn build_upgrade_window_request(
    weekday: i64,
    start_hour: &str,
) -> CloudResult<UpgradeWindowPutRequest> {
    let start_hour_utc = match start_hour {
        "0" => UpgradeWindowStartHourUtc::Hour0,
        "6" => UpgradeWindowStartHourUtc::Hour6,
        "12" => UpgradeWindowStartHourUtc::Hour12,
        "18" => UpgradeWindowStartHourUtc::Hour18,
        value => {
            return Err(CloudError::new(format!(
                "invalid start_hour '{value}': expected one of 0, 6, 12, 18"
            )));
        }
    };
    if !(0..=6).contains(&weekday) {
        return Err(CloudError::new(format!(
            "invalid weekday '{weekday}': expected a number from 0 through 6"
        )));
    }
    Ok(UpgradeWindowPutRequest {
        start_hour_utc,
        weekday,
    })
}

pub(super) fn build_replica_aware_routing_request(
    http_enabled: Option<bool>,
    native_enabled: Option<bool>,
) -> ReplicaAwareRoutingPatchRequest {
    ReplicaAwareRoutingPatchRequest {
        http_enabled,
        native_enabled,
    }
}

#[derive(serde::Serialize)]
struct UpgradeWindowHumanView {
    duration: String,
    #[serde(rename = "startHourUtc")]
    start_hour_utc: String,
    weekday: String,
}

fn upgrade_window_human_view(window: &UpgradeWindow) -> UpgradeWindowHumanView {
    let duration = window
        .duration
        .map(|duration| {
            let hours = i64::from(duration);
            format!("{hours} hour{}", if hours == 1 { "" } else { "s" })
        })
        .unwrap_or_else(|| ABSENT.to_string());
    let weekday = window
        .weekday
        .map(|weekday| match weekday {
            0 => "Sunday (0)".to_string(),
            1 => "Monday (1)".to_string(),
            2 => "Tuesday (2)".to_string(),
            3 => "Wednesday (3)".to_string(),
            4 => "Thursday (4)".to_string(),
            5 => "Friday (5)".to_string(),
            6 => "Saturday (6)".to_string(),
            unknown => unknown.to_string(),
        })
        .unwrap_or_else(|| ABSENT.to_string());

    UpgradeWindowHumanView {
        duration,
        start_hour_utc: or_absent(window.start_hour_utc.as_ref()),
        weekday,
    }
}

pub(super) async fn upgrade_window_get(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let window = client.get_upgrade_window(&org_id, service_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&window)?);
    } else {
        print_human(&upgrade_window_human_view(&window))?;
    }
    Ok(())
}

pub(super) async fn upgrade_window_set(
    client: &CloudClient,
    service_id: &str,
    weekday: i64,
    start_hour: &str,
    json: bool,
) -> CloudResult<()> {
    let request = build_upgrade_window_request(weekday, start_hour)?;
    let org_id = resolve_org_id(client).await?;
    let window = client
        .set_upgrade_window(&org_id, service_id, &request)
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&window)?);
    } else {
        print_human(&upgrade_window_human_view(&window))?;
    }
    Ok(())
}

pub(super) async fn replica_aware_routing_get(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let routing = client
        .get_replica_aware_routing(&org_id, service_id)
        .await?;
    print_replica_aware_routing(&routing, json)
}

pub(super) async fn replica_aware_routing_update(
    client: &CloudClient,
    service_id: &str,
    request: ReplicaAwareRoutingPatchRequest,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let routing = client
        .update_replica_aware_routing(&org_id, service_id, &request)
        .await?;
    print_replica_aware_routing(&routing, json)
}

fn print_replica_aware_routing(routing: &ReplicaAwareRouting, json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(routing)?);
    } else {
        print_human(routing)?;
    }
    Ok(())
}

pub(super) async fn upgrade_window_delete(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.delete_upgrade_window(&org_id, service_id).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Upgrade window deleted for service {service_id}");
    }
    Ok(())
}

pub(super) async fn service_profile_list(
    client: &CloudClient,
    region_id: Option<&str>,
    byoc_id: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let profiles = client
        .list_service_profiles(&org_id, region_id, byoc_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&profiles)?);
    } else {
        if profiles.is_empty() {
            println!("No service profiles found");
            return Ok(());
        }
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Profile")]
            profile: String,
            #[tabled(rename = "CPU cores")]
            cpu_cores: String,
            #[tabled(rename = "Memory GiB")]
            memory_gi: String,
        }
        let rows: Vec<Row> = profiles
            .into_iter()
            .map(|profile| Row {
                profile: or_absent(profile.profile),
                cpu_cores: or_absent(profile.cpu_cores),
                memory_gi: or_absent(profile.memory_gi),
            })
            .collect();
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_replica_aware_routing_request_supports_minimal_and_maximal_values() {
        let minimal = build_replica_aware_routing_request(None, Some(false));
        assert_eq!(minimal.http_enabled, None);
        assert_eq!(minimal.native_enabled, Some(false));
        assert_eq!(
            serde_json::to_value(&minimal).unwrap(),
            serde_json::json!({"nativeEnabled": false})
        );

        let maximal = build_replica_aware_routing_request(Some(true), Some(true));
        assert_eq!(maximal.http_enabled, Some(true));
        assert_eq!(maximal.native_enabled, Some(true));
    }

    #[test]
    fn build_upgrade_window_request_supports_minimal_and_maximal_values() {
        let earliest = build_upgrade_window_request(0, "0").unwrap();
        assert_eq!(earliest.weekday, 0);
        assert_eq!(earliest.start_hour_utc, UpgradeWindowStartHourUtc::Hour0);

        let latest = build_upgrade_window_request(6, "18").unwrap();
        assert_eq!(latest.weekday, 6);
        assert_eq!(latest.start_hour_utc, UpgradeWindowStartHourUtc::Hour18);
    }

    #[test]
    fn upgrade_window_human_view_names_every_documented_weekday() {
        let expected = [
            "Sunday (0)",
            "Monday (1)",
            "Tuesday (2)",
            "Wednesday (3)",
            "Thursday (4)",
            "Friday (5)",
            "Saturday (6)",
        ];

        for (weekday, expected) in expected.into_iter().enumerate() {
            let view = upgrade_window_human_view(&UpgradeWindow {
                duration: None,
                start_hour_utc: None,
                weekday: Some(weekday as i64),
            });
            assert_eq!(view.weekday, expected);
        }
    }

    #[test]
    fn upgrade_window_human_view_labels_duration_and_preserves_sparse_unknowns() {
        use clickhouse_cloud_api::models::UpgradeWindowDuration;

        let known = upgrade_window_human_view(&UpgradeWindow {
            duration: Some(UpgradeWindowDuration::SixHours),
            start_hour_utc: Some(UpgradeWindowStartHourUtc::Hour12),
            weekday: Some(3),
        });
        assert_eq!(
            serde_json::to_value(known).unwrap(),
            serde_json::json!({
                "duration": "6 hours",
                "startHourUtc": "12",
                "weekday": "Wednesday (3)"
            })
        );

        let sparse_unknown = upgrade_window_human_view(&UpgradeWindow {
            duration: Some(UpgradeWindowDuration::Unknown(1)),
            start_hour_utc: None,
            weekday: Some(9),
        });
        assert_eq!(
            serde_json::to_value(sparse_unknown).unwrap(),
            serde_json::json!({
                "duration": "1 hour",
                "startHourUtc": "-",
                "weekday": "9"
            })
        );
    }
}
