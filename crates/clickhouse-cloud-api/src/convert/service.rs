use super::MissingRequiredFields;
use crate::models::*;

impl TryFrom<ScalingScheduleEntry> for ScalingScheduleEntryRequest {
    type Error = MissingRequiredFields;

    /// Turns a fetched schedule entry into one that can be re-sent.
    ///
    /// An upsert replaces the whole schedule, so a caller that reads a schedule
    /// and writes it back has to send every entry in full: the window bounds,
    /// weekdays and name the API requires cannot be defaulted away.
    fn try_from(value: ScalingScheduleEntry) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.end_hour_utc.is_none() {
            missing.push("endHourUtc");
        }
        if value.name.is_none() {
            missing.push("name");
        }
        if value.start_hour_utc.is_none() {
            missing.push("startHourUtc");
        }
        if value.weekdays.is_none() {
            missing.push("weekdays");
        }
        match (
            value.end_hour_utc,
            value.name,
            value.start_hour_utc,
            value.weekdays,
        ) {
            (Some(end_hour_utc), Some(name), Some(start_hour_utc), Some(weekdays)) => Ok(Self {
                autoscaling_mode: value.autoscaling_mode,
                end_hour_utc,
                idle_scaling: value.idle_scaling,
                idle_timeout_minutes: value.idle_timeout_minutes,
                max_replica_memory_gb: value.max_replica_memory_gb,
                max_replicas: value.max_replicas,
                min_replica_memory_gb: value.min_replica_memory_gb,
                min_replicas: value.min_replicas,
                name,
                // Not part of the response shape; a horizontal entry carries a
                // min/max band instead of a fixed replica count.
                num_replicas: None,
                start_hour_utc,
                weekdays,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}

impl TryFrom<UpgradeWindow> for UpgradeWindowPutRequest {
    type Error = MissingRequiredFields;

    /// Turns a fetched upgrade window into one that can be re-sent.
    ///
    /// `duration` is response-only (the API derives it), so only the window's
    /// start hour and weekday cross over — and both are required.
    fn try_from(value: UpgradeWindow) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.start_hour_utc.is_none() {
            missing.push("startHourUtc");
        }
        if value.weekday.is_none() {
            missing.push("weekday");
        }
        match (value.start_hour_utc, value.weekday) {
            (Some(start_hour_utc), Some(weekday)) => Ok(Self {
                start_hour_utc,
                weekday,
            }),
            _ => Err(MissingRequiredFields::new(missing)),
        }
    }
}
