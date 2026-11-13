use super::{ClickPipeSourceKind, classify_clickpipe_source};
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::{
    ABSENT, CloudErrorCode, CloudErrorDetail, NextStep, cloud_command, eprint_background_change,
    json_with_next, or_absent, print_human,
};
use crate::cloud::shared::resolve_org_id;
use tabled::{Table, Tabled, settings::Style};

pub(super) async fn clickpipe_context_get(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let context = client
        .get_clickpipe_service_context(&org_id, service_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&context)?);
    } else {
        print_human(&context)?;
    }
    Ok(())
}

pub(super) async fn clickpipe_list(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let clickpipes = client.list_clickpipes(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipes)?);
    } else if clickpipes.is_empty() {
        println!("No ClickPipes found");
    } else {
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "ID")]
            id: String,
            #[tabled(rename = "Source")]
            source: &'static str,
            #[tabled(rename = "Destination")]
            destination: String,
            #[tabled(rename = "State")]
            state: String,
        }
        let rows = clickpipes.iter().map(|clickpipe| {
            let destination = clickpipe.destination.as_ref();
            let database = destination.and_then(|destination| destination.database.as_deref());
            let table = destination.and_then(|destination| destination.table.as_deref());
            Row {
                name: or_absent(clickpipe.name.as_deref()),
                id: or_absent(clickpipe.id.as_ref()),
                source: match classify_clickpipe_source(clickpipe) {
                    ClickPipeSourceKind::Kafka => "Kafka",
                    ClickPipeSourceKind::Kinesis => "Kinesis",
                    ClickPipeSourceKind::PubSub => "Pub/Sub",
                    ClickPipeSourceKind::ObjectStorage => "Object storage",
                    ClickPipeSourceKind::Postgres => "Postgres",
                    ClickPipeSourceKind::MySql => "MySQL",
                    ClickPipeSourceKind::MongoDb => "MongoDB",
                    ClickPipeSourceKind::BigQuery => "BigQuery",
                    ClickPipeSourceKind::Absent => ABSENT,
                },
                destination: match (database, table) {
                    (Some(database), Some(table)) => format!("{database}.{table}"),
                    (Some(database), None) => database.to_string(),
                    (None, Some(table)) => table.to_string(),
                    (None, None) => ABSENT.to_string(),
                },
                state: or_absent(clickpipe.state.as_ref()),
            }
        });
        println!("{}", Table::new(rows).with(Style::markdown()));
    }
    Ok(())
}

pub(super) async fn clickpipe_get(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let clickpipe = client
        .get_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        print_human(&clickpipe)?;
    }
    Ok(())
}

pub(super) async fn clickpipe_delete(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let deleted_name = client
        .delete_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;

    if json {
        println!("{}", serde_json::json!({ "deleted": clickpipe_id }));
    } else {
        println!(
            "ClickPipe {} deleted",
            deleted_name.as_deref().unwrap_or(clickpipe_id)
        );
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum ClickPipeLifecycleCommand {
    Start,
    Stop,
    Resync,
}

impl ClickPipeLifecycleCommand {
    fn api_command(self) -> clickhouse_cloud_api::models::ClickPipeStatePatchRequestCommand {
        use clickhouse_cloud_api::models::ClickPipeStatePatchRequestCommand;

        match self {
            Self::Start => ClickPipeStatePatchRequestCommand::Start,
            Self::Stop => ClickPipeStatePatchRequestCommand::Stop,
            Self::Resync => ClickPipeStatePatchRequestCommand::Resync,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Resync => "resync",
        }
    }

    /// `next` for the accepted state change: poll `clickpipe get` for `state`
    /// (#1162). A stopped CDC pipe reports `Paused`; a one-off load that
    /// finishes after the request reports `Completed`. A start answered with
    /// `Completed` never gets here: `not_applied_in` rejects it.
    fn next(self, org_id: &str, service_id: &str, clickpipe_id: &str) -> NextStep {
        use clickhouse_cloud_api::models::ClickPipeState::*;
        let poll = NextStep::poll(
            &["clickpipe", "get", service_id, clickpipe_id],
            org_id,
            "state",
        )
        .fail_on(&[Failed, InternalError]);
        match self {
            Self::Start | Self::Resync => poll.until(&[Running, Completed]),
            Self::Stop => poll.until(&[Stopped, Paused]),
        }
    }

    /// Whether `state`, returned by an accepted request, shows the request
    /// was not applied (#1228). A start that reaches a CDC pipe still
    /// `Pausing`, or one that has only just reached `Paused`, is answered with
    /// that state and then dropped; an applied start answers `Running`. Only
    /// start: a resync answered with `Paused` is applied. `Stopped` and
    /// `Stopping` are not included: streaming and object-storage pipes halt in
    /// `Stopped`, and no start answered with either has been seen dropped.
    /// A start on a `Completed` pipe is accepted and has no effect (#926): it
    /// does not re-run the pipe, which stays `Completed`.
    fn not_applied_in(self, state: Option<&clickhouse_cloud_api::models::ClickPipeState>) -> bool {
        use clickhouse_cloud_api::models::ClickPipeState::{Completed, Paused, Pausing};
        matches!(self, Self::Start) && matches!(state, Some(Pausing | Paused | Completed))
    }
}

pub(super) async fn clickpipe_state(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    command: ClickPipeLifecycleCommand,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let clickpipe = client
        .change_clickpipe_state(&org_id, service_id, clickpipe_id, command.api_command())
        .await?;
    let name = clickpipe
        .name
        .as_deref()
        .filter(|name| !name.is_empty())
        .unwrap_or(clickpipe_id);

    if command.not_applied_in(clickpipe.state.as_ref()) {
        use clickhouse_cloud_api::models::ClickPipeState::{Completed, Pausing};
        let label = command.label();
        let (code, retry, message) = match clickpipe.state {
            Some(Pausing) => {
                let check = cloud_command(&["clickpipe", "get", service_id, clickpipe_id], &org_id);
                let message = format!(
                    "ClickPipe {name} is still Pausing, so the {label} request was not applied. \
                     Repeat `{check}` until the state is Paused, then run {label} again."
                );
                (CloudErrorCode::ClickPipeStillPausing, Some(check), message)
            }
            Some(Completed) => {
                let message = format!(
                    "ClickPipe {name} has completed, so the {label} request has no effect: \
                     {label} does not re-run a completed pipe."
                );
                (CloudErrorCode::ClickPipeStartNoEffect, None, message)
            }
            _ => {
                let retry = cloud_command(&["clickpipe", label, service_id, clickpipe_id], &org_id);
                let message = format!(
                    "ClickPipe {name} answered the {label} request with state Paused, so it was \
                     not applied. Wait a few seconds, then repeat `{retry}` until it returns \
                     Running."
                );
                (
                    CloudErrorCode::ClickPipeStartNotApplied,
                    Some(retry),
                    message,
                )
            }
        };
        return Err(
            CloudError::new(message.clone()).with_details(CloudErrorDetail {
                command: retry,
                ..CloudErrorDetail::new(code, message)
            }),
        );
    }

    if json {
        let next = command.next(&org_id, service_id, clickpipe_id);
        println!("{}", json_with_next(&clickpipe, Some(next))?);
    } else {
        println!(
            "ClickPipe {} {} request accepted (returned state: {})",
            name,
            command.label(),
            or_absent(clickpipe.state.as_ref())
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn clickpipe_scale(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    replicas: Option<u32>,
    cpu_millicores: Option<u32>,
    memory_gb: Option<f64>,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    // `..Default::default()` fills the `deprecated-fields`-gated fields; without the
    // feature every field is listed, so the update is needless only in that build.
    #[allow(clippy::needless_update)]
    let request = clickhouse_cloud_api::models::ClickPipeScalingPatchRequest {
        replicas: replicas.map(i64::from),
        replica_cpu_millicores: cpu_millicores.map(i64::from),
        replica_memory_gb: memory_gb,
        ..Default::default()
    };
    let clickpipe = client
        .update_clickpipe_scaling(&org_id, service_id, clickpipe_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        let scaling = clickpipe.scaling.unwrap_or_default();
        println!(
            "ClickPipe {} scaling updated",
            or_absent(clickpipe.name.as_deref())
        );
        println!("  Replicas: {}", or_absent(scaling.replicas));
        println!("  CPU: {}m", or_absent(scaling.replica_cpu_millicores));
        println!("  Memory: {} GB", or_absent(scaling.replica_memory_gb));
    }
    Ok(())
}

/// The API sets CPU and memory together: an omitted field reads as 0 and
/// fails the fixed ratio check (#1242), so both flags are required.
#[derive(Debug, PartialEq)]
pub(super) struct CdcScalingValues {
    pub(super) cpu_millicores: u32,
    pub(super) memory_gb: f64,
}

/// `next` for an accepted CDC scaling change, which applies in the background
/// while the PATCH response still shows the old allocation (#1162). CPU and
/// memory change together, so the requested CPU confirms both.
fn cdc_scaling_next(org_id: &str, service_id: &str, values: &CdcScalingValues) -> NextStep {
    let args = ["clickpipe", "cdc-scaling", "get", service_id];
    NextStep::poll(&args, org_id, "replicaCpuMillicores")
        .until(&[serde_json::json!(values.cpu_millicores)])
        .every(30)
}

fn build_cdc_scaling_request(
    values: &CdcScalingValues,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipesCdcScalingPatchRequest> {
    let CdcScalingValues {
        cpu_millicores,
        memory_gb,
    } = *values;
    let required_memory_gb = f64::from(cpu_millicores) / 250.0;
    if memory_gb != required_memory_gb {
        return Err(CloudError::usage(format!(
            "--memory-gb must be {required_memory_gb} when --cpu-millicores is {cpu_millicores}"
        )));
    }

    Ok(
        clickhouse_cloud_api::models::ClickPipesCdcScalingPatchRequest {
            replica_cpu_millicores: Some(i64::from(cpu_millicores)),
            replica_memory_gb: Some(memory_gb),
        },
    )
}

pub(super) async fn clickpipe_cdc_scaling_get(
    client: &CloudClient,
    service_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    let scaling = client
        .get_clickpipe_cdc_scaling(&org_id, service_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&scaling)?);
    } else {
        print_human(&scaling)?;
    }
    Ok(())
}

pub(super) async fn clickpipe_cdc_scaling_update(
    client: &CloudClient,
    service_id: &str,
    values: &CdcScalingValues,
    json: bool,
) -> CloudResult<()> {
    let request = build_cdc_scaling_request(values)?;
    let org_id = resolve_org_id(client).await?;
    let scaling = client
        .update_clickpipe_cdc_scaling(&org_id, service_id, &request)
        .await?;

    if json {
        let next = cdc_scaling_next(&org_id, service_id, values);
        println!("{}", json_with_next(&scaling, Some(next))?);
    } else {
        println!("CDC scaling update accepted");
        println!();
        print_human(&scaling)?;
        eprint_background_change(
            "The new allocation",
            &cloud_command(&["clickpipe", "cdc-scaling", "get", service_id], &org_id),
        );
        eprintln!("Another scaling update before it applies fails with a conflict.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::clickpipes::cli::{
        ClickPipeCdcScalingCommands, ClickPipeCommands, ClickPipeContextCommands,
    };
    use crate::cloud::clickpipes::test_support::{
        assert_names, assert_rejected, clickpipe_parse_error, names_word, parse_clickpipe,
    };

    /// Start and the CPU field are the subprocess tests.
    #[test]
    fn clickpipe_stop_and_resync_next_name_their_end_states() {
        let stop = ClickPipeLifecycleCommand::Stop.next("org", "svc", "pipe");
        assert_eq!(
            stop.until,
            [serde_json::json!("Stopped"), serde_json::json!("Paused")]
        );
        let resync = ClickPipeLifecycleCommand::Resync.next("org", "svc", "pipe");
        assert_eq!(
            resync.until,
            [serde_json::json!("Running"), serde_json::json!("Completed")]
        );
    }

    /// A start answered with `Completed` has no effect (#926); a resync that
    /// finishes as `Completed` is still applied.
    #[test]
    fn only_a_start_answered_with_completed_is_not_applied() {
        use clickhouse_cloud_api::models::ClickPipeState::Completed;
        assert!(ClickPipeLifecycleCommand::Start.not_applied_in(Some(&Completed)));
        assert!(!ClickPipeLifecycleCommand::Resync.not_applied_in(Some(&Completed)));
    }

    #[test]
    fn parses_scale_flags_and_defaults() {
        let ClickPipeCommands::Scale {
            service_id,
            clickpipe_id,
            replicas,
            cpu_millicores,
            memory_gb,
        } = parse_clickpipe(&[
            "scale",
            "svc-1",
            "pipe-1",
            "--replicas",
            "4",
            "--cpu-millicores",
            "500",
            "--memory-gb",
            "1.5",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected scale");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(clickpipe_id.id.as_deref(), Some("pipe-1"));
        assert_eq!(replicas, Some(4));
        assert_eq!(cpu_millicores, Some(500));
        assert_eq!(memory_gb, Some(1.5));
    }

    #[test]
    fn scale_accepts_inclusive_boundaries() {
        for (replicas, cpu, memory) in [("1", "125", "0.5"), ("40", "2000", "8")] {
            let ClickPipeCommands::Scale {
                replicas: parsed_replicas,
                cpu_millicores,
                memory_gb,
                ..
            } = parse_clickpipe(&[
                "scale",
                "svc-1",
                "pipe-1",
                "--replicas",
                replicas,
                "--cpu-millicores",
                cpu,
                "--memory-gb",
                memory,
            ])
            else {
                panic!("expected scale");
            };
            assert_eq!(parsed_replicas, Some(replicas.parse().unwrap()));
            assert_eq!(cpu_millicores, Some(cpu.parse().unwrap()));
            assert_eq!(memory_gb, Some(memory.parse().unwrap()));
        }
    }

    #[test]
    fn scale_rejects_out_of_range_and_non_finite_values() {
        for (flag, values) in [
            ("replicas", &["0", "41", "-1"][..]),
            ("cpu-millicores", &["0", "124", "2001", "-1"][..]),
            (
                "memory-gb",
                &[
                    "0",
                    "0.49",
                    "8.01",
                    "9",
                    "-1",
                    "NaN",
                    "inf",
                    "+inf",
                    "-inf",
                    "infinity",
                    "-infinity",
                ][..],
            ),
        ] {
            for value in values {
                let argument = format!("--{flag}={value}");
                let error = clickpipe_parse_error(&["scale", "svc-1", "pipe-1", &argument]);
                assert_eq!(
                    error.kind(),
                    clap::error::ErrorKind::ValueValidation,
                    "{argument}"
                );
                assert_eq!(error.exit_code(), 2, "{argument}");
            }
        }
    }

    #[test]
    fn scale_requires_at_least_one_of_replicas_cpu_or_memory() {
        assert_rejected(&["scale", "svc-1", "pipe-1"]);
    }

    #[test]
    fn scale_accepts_a_single_flag() {
        let ClickPipeCommands::Scale {
            replicas,
            cpu_millicores,
            memory_gb,
            ..
        } = parse_clickpipe(&["scale", "svc-1", "pipe-1", "--replicas", "4"])
        else {
            panic!("expected scale");
        };
        assert_eq!(replicas, Some(4));
        assert_eq!(cpu_millicores, None);
        assert_eq!(memory_gb, None);

        let ClickPipeCommands::Scale {
            replicas,
            cpu_millicores,
            memory_gb,
            ..
        } = parse_clickpipe(&["scale", "svc-1", "pipe-1", "--cpu-millicores", "500"])
        else {
            panic!("expected scale");
        };
        assert_eq!(replicas, None);
        assert_eq!(cpu_millicores, Some(500));
        assert_eq!(memory_gb, None);

        let ClickPipeCommands::Scale {
            replicas,
            cpu_millicores,
            memory_gb,
            ..
        } = parse_clickpipe(&["scale", "svc-1", "pipe-1", "--memory-gb", "1.5"])
        else {
            panic!("expected scale");
        };
        assert_eq!(replicas, None);
        assert_eq!(cpu_millicores, None);
        assert_eq!(memory_gb, Some(1.5));
    }

    #[test]
    fn scale_accepts_any_combination_of_flags() {
        let ClickPipeCommands::Scale {
            replicas,
            cpu_millicores,
            memory_gb,
            ..
        } = parse_clickpipe(&[
            "scale",
            "svc-1",
            "pipe-1",
            "--replicas",
            "4",
            "--memory-gb",
            "1.5",
        ])
        else {
            panic!("expected scale");
        };
        assert_eq!(replicas, Some(4));
        assert_eq!(cpu_millicores, None);
        assert_eq!(memory_gb, Some(1.5));
    }

    #[test]
    fn cdc_scaling_update_requires_both_values_and_enforces_schema_ranges() {
        // The API reads an omitted value as 0, so one flag alone is a usage error (#1242).
        assert_rejected(&["cdc-scaling", "update", "svc-1"]);
        assert_rejected(&["cdc-scaling", "update", "svc-1", "--cpu-millicores", "1000"]);
        assert_rejected(&["cdc-scaling", "update", "svc-1", "--memory-gb", "4"]);
        for value in ["999", "1500", "33000"] {
            assert_rejected(&[
                "cdc-scaling",
                "update",
                "svc-1",
                "--cpu-millicores",
                value,
                "--memory-gb",
                "4",
            ]);
        }
        for value in ["3", "6", "132", "NaN"] {
            assert_rejected(&[
                "cdc-scaling",
                "update",
                "svc-1",
                "--cpu-millicores",
                "1000",
                "--memory-gb",
                value,
            ]);
        }
        // Both ends of each range are accepted.
        for (cpu, memory) in [(1000, 4.0), (32000, 128.0)] {
            let ClickPipeCommands::CdcScaling {
                command:
                    ClickPipeCdcScalingCommands::Update {
                        cpu_millicores,
                        memory_gb,
                        ..
                    },
            } = parse_clickpipe(&[
                "cdc-scaling",
                "update",
                "svc-1",
                "--cpu-millicores",
                &cpu.to_string(),
                "--memory-gb",
                &memory.to_string(),
            ])
            else {
                panic!("expected CDC scaling update");
            };
            assert_eq!((cpu_millicores, memory_gb), (cpu, memory));
        }
    }

    #[test]
    fn build_cdc_scaling_request_accepts_maximum_balanced_allocation() {
        let request = build_cdc_scaling_request(&CdcScalingValues {
            cpu_millicores: 32000,
            memory_gb: 128.0,
        })
        .unwrap();
        assert_eq!(request.replica_cpu_millicores, Some(32000));
        assert_eq!(request.replica_memory_gb, Some(128.0));
    }

    #[test]
    fn build_cdc_scaling_request_rejects_unbalanced_allocation() {
        let error = build_cdc_scaling_request(&CdcScalingValues {
            cpu_millicores: 2000,
            memory_gb: 4.0,
        })
        .unwrap_err();
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Usage);
        assert_names(&error.message, &["--memory-gb", "--cpu-millicores"]);
        // The user's CPU value and the memory it requires.
        assert!(names_word(&error.message, "2000"), "{}", error.message);
        assert!(names_word(&error.message, "8"), "{}", error.message);
    }

    #[test]
    fn context_get_parses_and_is_read_only() {
        let command = parse_clickpipe(&["context", "get", "svc-1", "--org-id", "org-1"]);
        assert!(!command.is_write());
        let ClickPipeCommands::Context {
            command: ClickPipeContextCommands::Get { service_id },
        } = command
        else {
            panic!("expected context get");
        };
        assert_eq!(service_id, "svc-1");
    }
}
