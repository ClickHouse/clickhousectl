use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::config::{deserialize_strict_config, read_config_value};
use crate::cloud::output::{print_human, print_line};
use crate::cloud::shared::resolve_org_id;
use clickhouse_cloud_api::models::{
    AutoscalingMode, ScalingScheduleEntryRequest, ScalingSchedulePostRequest,
};

fn build_scaling_schedule_request(
    value: serde_json::Value,
    source: &str,
) -> CloudResult<ScalingSchedulePostRequest> {
    let request: ScalingSchedulePostRequest = deserialize_strict_config(value, source)?;
    for (index, entry) in request.entries.iter().enumerate() {
        validate_scaling_schedule_entry(entry, index, source)?;
    }
    Ok(request)
}

fn validate_scaling_schedule_entry(
    entry: &ScalingScheduleEntryRequest,
    index: usize,
    source: &str,
) -> CloudResult<()> {
    let invalid = |message: String| {
        CloudError::new(format!(
            "invalid request body in config {source}: entries[{index}] {message}"
        ))
    };

    if entry.weekdays.is_empty() {
        return Err(invalid("weekdays must contain at least one day".into()));
    }
    if let Some(day) = entry.weekdays.iter().find(|&&day| !(0..=6).contains(&day)) {
        return Err(invalid(format!(
            "weekdays contains {day}; each day must be between 0 (Sunday) and 6 (Saturday)"
        )));
    }
    if !(0..=23).contains(&entry.start_hour_utc) {
        return Err(invalid(format!(
            "startHourUtc must be between 0 and 23, got {}",
            entry.start_hour_utc
        )));
    }
    if !(1..=24).contains(&entry.end_hour_utc) {
        return Err(invalid(format!(
            "endHourUtc must be between 1 and 24, got {}",
            entry.end_hour_utc
        )));
    }
    if entry.start_hour_utc == entry.end_hour_utc {
        return Err(invalid(
            "startHourUtc and endHourUtc must be different".into(),
        ));
    }

    let memory = match (entry.min_replica_memory_gb, entry.max_replica_memory_gb) {
        (None, None) => None,
        (Some(min), Some(max)) => {
            for (field, value) in [("minReplicaMemoryGb", min), ("maxReplicaMemoryGb", max)] {
                if !(8.0..=356.0).contains(&value) || value % 4.0 != 0.0 {
                    return Err(invalid(format!(
                        "{field} must be a multiple of 4 between 8 and 356, got {value}"
                    )));
                }
            }
            if min > max {
                return Err(invalid(format!(
                    "minReplicaMemoryGb ({min}) must not exceed maxReplicaMemoryGb ({max})"
                )));
            }
            Some((min, max))
        }
        _ => {
            return Err(invalid(
                "minReplicaMemoryGb and maxReplicaMemoryGb must be provided together or both omitted"
                    .into(),
            ));
        }
    };

    let replicas = match (entry.min_replicas, entry.max_replicas) {
        (None, None) => None,
        (Some(min), Some(max)) => {
            if min < 1 || max < 1 {
                return Err(invalid(format!(
                    "minReplicas and maxReplicas must each be at least 1, got {min} and {max}"
                )));
            }
            if min > max {
                return Err(invalid(format!(
                    "minReplicas ({min}) must not exceed maxReplicas ({max})"
                )));
            }
            Some((min, max))
        }
        _ => {
            return Err(invalid(
                "minReplicas and maxReplicas must be provided together or both omitted".into(),
            ));
        }
    };

    if entry.num_replicas.is_some() && replicas.is_some() {
        return Err(invalid(
            "numReplicas is mutually exclusive with minReplicas and maxReplicas".into(),
        ));
    }
    if let Some(count) = entry.num_replicas
        && count < 1
    {
        return Err(invalid(format!(
            "numReplicas must be at least 1, got {count}"
        )));
    }

    match entry.autoscaling_mode.as_ref() {
        Some(AutoscalingMode::Unknown(value)) => {
            return Err(invalid(format!(
                "autoscalingMode must be one of {}, got {value}",
                AutoscalingMode::VALUES.join(", ")
            )));
        }
        Some(AutoscalingMode::Horizontal) => {
            let Some((min_memory, max_memory)) = memory else {
                return Err(invalid(
                    "horizontal autoscaling requires minReplicaMemoryGb and maxReplicaMemoryGb"
                        .into(),
                ));
            };
            if min_memory != max_memory {
                return Err(invalid(
                    "horizontal autoscaling requires equal minReplicaMemoryGb and maxReplicaMemoryGb"
                        .into(),
                ));
            }
            if replicas.is_none() {
                return Err(invalid(
                    "horizontal autoscaling requires minReplicas and maxReplicas".into(),
                ));
            }
            if entry.num_replicas.is_some() {
                return Err(invalid(
                    "horizontal autoscaling does not accept numReplicas".into(),
                ));
            }
        }
        Some(AutoscalingMode::Vertical) | None => {
            if let Some((min, max)) = replicas
                && min != max
            {
                return Err(invalid(
                    "vertical autoscaling accepts minReplicas and maxReplicas only as an equal fixed count"
                        .into(),
                ));
            }
        }
    }

    Ok(())
}

pub(super) async fn scaling_schedule_get(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let schedule = client.get_scaling_schedule(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&schedule)?);
    } else {
        print_human(&schedule)?;
    }
    Ok(())
}

pub(super) async fn scaling_schedule_set(
    client: &CloudClient,
    service_id: &str,
    file: &str,
    json: bool,
) -> CloudResult<()> {
    let request = build_scaling_schedule_request(read_config_value(file)?, file)?;
    let org_id = resolve_org_id(client).await?;
    let schedule = client
        .set_scaling_schedule(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&schedule)?);
    } else {
        print_human(&schedule)?;
    }
    Ok(())
}

pub(super) async fn scaling_schedule_delete(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let response = client.delete_scaling_schedule(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print_line(format!(
            "Scaling schedule deleted from service {service_id}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_scaling_schedule_request_supports_minimal_and_empty_schedules() {
        let request = build_scaling_schedule_request(
            serde_json::json!({
                "entries": [{
                    "name": "Business hours",
                    "weekdays": [1],
                    "startHourUtc": 9,
                    "endHourUtc": 17
                }]
            }),
            "test.json",
        )
        .unwrap();
        assert_eq!(request.entries.len(), 1);
        let entry = &request.entries[0];
        assert_eq!(entry.name, "Business hours");
        assert_eq!(entry.weekdays, vec![1]);
        assert_eq!(entry.start_hour_utc, 9);
        assert_eq!(entry.end_hour_utc, 17);
        assert!(entry.autoscaling_mode.is_none());
        assert!(entry.idle_scaling.is_none());
        assert!(entry.idle_timeout_minutes.is_none());
        assert!(entry.min_replica_memory_gb.is_none());
        assert!(entry.max_replica_memory_gb.is_none());
        assert!(entry.num_replicas.is_none());
        assert!(entry.min_replicas.is_none());
        assert!(entry.max_replicas.is_none());

        let clear = build_scaling_schedule_request(serde_json::json!({"entries": []}), "test.json")
            .unwrap();
        assert!(clear.entries.is_empty());
    }

    #[test]
    fn build_scaling_schedule_request_preserves_all_entry_fields() {
        let request = build_scaling_schedule_request(
            serde_json::json!({
                "entries": [
                    {
                        "name": "Vertical nights",
                        "weekdays": [0, 6],
                        "startHourUtc": 22,
                        "endHourUtc": 6,
                        "autoscalingMode": "vertical",
                        "idleScaling": true,
                        "idleTimeoutMinutes": 5,
                        "minReplicaMemoryGb": 8,
                        "maxReplicaMemoryGb": 32,
                        "numReplicas": 2
                    },
                    {
                        "name": "Horizontal weekdays",
                        "weekdays": [1, 2, 3, 4, 5],
                        "startHourUtc": 8,
                        "endHourUtc": 24,
                        "autoscalingMode": "horizontal",
                        "idleScaling": false,
                        "idleTimeoutMinutes": 0,
                        "minReplicaMemoryGb": 16,
                        "maxReplicaMemoryGb": 16,
                        "minReplicas": 2,
                        "maxReplicas": 7
                    }
                ]
            }),
            "schedule.json",
        )
        .unwrap();

        assert_eq!(request.entries.len(), 2);
        let vertical = &request.entries[0];
        assert_eq!(vertical.autoscaling_mode, Some(AutoscalingMode::Vertical));
        assert_eq!(vertical.idle_scaling, Some(true));
        assert_eq!(vertical.idle_timeout_minutes, Some(5));
        assert_eq!(vertical.min_replica_memory_gb, Some(8.0));
        assert_eq!(vertical.max_replica_memory_gb, Some(32.0));
        assert_eq!(vertical.num_replicas, Some(2));
        assert!(vertical.min_replicas.is_none());
        assert!(vertical.max_replicas.is_none());

        let horizontal = &request.entries[1];
        assert_eq!(
            horizontal.autoscaling_mode,
            Some(AutoscalingMode::Horizontal)
        );
        assert_eq!(horizontal.weekdays, vec![1, 2, 3, 4, 5]);
        assert_eq!(horizontal.end_hour_utc, 24);
        assert_eq!(horizontal.idle_scaling, Some(false));
        assert_eq!(horizontal.idle_timeout_minutes, Some(0));
        assert_eq!(horizontal.min_replica_memory_gb, Some(16.0));
        assert_eq!(horizontal.max_replica_memory_gb, Some(16.0));
        assert_eq!(horizontal.min_replicas, Some(2));
        assert_eq!(horizontal.max_replicas, Some(7));
        assert!(horizontal.num_replicas.is_none());
    }

    #[test]
    fn build_scaling_schedule_request_rejects_unknown_fields_and_modes() {
        for value in [
            serde_json::json!({
                "entries": [{
                    "name": "typo",
                    "weekdays": [1],
                    "startHourUtc": 9,
                    "endHourUtc": 17,
                    "minReplicaMemoryGB": 16
                }]
            }),
            serde_json::json!({
                "entries": [{
                    "name": "future mode",
                    "weekdays": [1],
                    "startHourUtc": 9,
                    "endHourUtc": 17,
                    "autoscalingMode": "diagonal"
                }]
            }),
            serde_json::json!({"entries": [], "baseConfig": {}}),
        ] {
            assert!(build_scaling_schedule_request(value, "bad.json").is_err());
        }
    }

    #[test]
    fn build_scaling_schedule_request_validates_documented_bounds_and_relationships() {
        let invalid_entries = [
            serde_json::json!({"name":"x","weekdays":[],"startHourUtc":0,"endHourUtc":24}),
            serde_json::json!({"name":"x","weekdays":[7],"startHourUtc":0,"endHourUtc":24}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":-1,"endHourUtc":24}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":0,"endHourUtc":25}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":8}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":9,"minReplicaMemoryGb":8}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":9,"minReplicaMemoryGb":12,"maxReplicaMemoryGb":8}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":9,"minReplicaMemoryGb":10,"maxReplicaMemoryGb":12}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":9,"numReplicas":0}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":9,"minReplicas":1}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":9,"minReplicas":3,"maxReplicas":2}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":9,"numReplicas":2,"minReplicas":2,"maxReplicas":2}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":9,"autoscalingMode":"vertical","minReplicas":1,"maxReplicas":2}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":9,"autoscalingMode":"horizontal","minReplicaMemoryGb":16,"maxReplicaMemoryGb":16}),
            serde_json::json!({"name":"x","weekdays":[1],"startHourUtc":8,"endHourUtc":9,"autoscalingMode":"horizontal","minReplicaMemoryGb":16,"maxReplicaMemoryGb":20,"minReplicas":1,"maxReplicas":2}),
        ];
        for entry in invalid_entries {
            let error =
                build_scaling_schedule_request(serde_json::json!({"entries": [entry]}), "bad.json")
                    .unwrap_err();
            assert!(error.message.contains("entries[0]"), "{error}");
        }
    }
}
