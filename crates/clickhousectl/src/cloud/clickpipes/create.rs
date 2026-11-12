use super::cli::{ClickPipeCreateRequestArgs, ClickPipeCreateValidationArgs};
use super::{ClickPipeSourceKind, non_empty};
use crate::cloud::client::{CloudError, Result as CloudResult};
use crate::cloud::output::or_absent;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ClickPipeFieldMappingJson {
    #[serde(rename = "sourceField")]
    source_field: String,
    #[serde(rename = "destinationField")]
    destination_field: String,
}

fn parse_create_field_mappings(
    raw_mappings: &[String],
) -> CloudResult<Vec<clickhouse_cloud_api::models::ClickPipeFieldMapping>> {
    raw_mappings
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let mapping: ClickPipeFieldMappingJson =
                serde_json::from_str(raw).map_err(|error| {
                    CloudError::usage(format!(
                        "--field-mapping #{}: invalid JSON object: {error}",
                        index + 1
                    ))
                })?;
            if mapping.source_field.is_empty() {
                return Err(CloudError::usage(format!(
                    "--field-mapping #{}: sourceField must not be empty",
                    index + 1
                )));
            }
            if mapping.destination_field.is_empty() {
                return Err(CloudError::usage(format!(
                    "--field-mapping #{}: destinationField must not be empty",
                    index + 1
                )));
            }
            Ok(clickhouse_cloud_api::models::ClickPipeFieldMapping {
                source_field: mapping.source_field,
                destination_field: mapping.destination_field,
            })
        })
        .collect()
}

#[derive(Debug)]
pub(super) struct BuiltCreateRequestArgs {
    start_paused: bool,
    validate_samples: Option<bool>,
    scaling: Option<clickhouse_cloud_api::models::ClickPipeScaling>,
    settings: Option<clickhouse_cloud_api::models::ClickPipeSettings>,
    field_mappings: Vec<clickhouse_cloud_api::models::ClickPipeFieldMapping>,
}

pub(super) fn build_create_request_args(
    args: &ClickPipeCreateRequestArgs,
    source: ClickPipeSourceKind,
) -> CloudResult<BuiltCreateRequestArgs> {
    if args.start_paused && source.database_source_label().is_some() {
        return Err(CloudError::usage(
            "--start-paused is not supported for database ClickPipes",
        ));
    }
    let scaling = match (args.replicas, args.cpu_millicores, args.memory_gb) {
        (None, None, None) => None,
        (Some(replicas), Some(cpu), Some(memory)) => {
            if !(1..=40).contains(&replicas) {
                return Err(CloudError::usage("--replicas must be in the range 1..=40"));
            }
            if !(125..=2000).contains(&cpu) {
                return Err(CloudError::usage(
                    "--cpu-millicores must be in the range 125..=2000",
                ));
            }
            if !memory.is_finite() || !(0.5..=8.0).contains(&memory) {
                return Err(CloudError::usage(
                    "--memory-gb must be in the range 0.5..=8",
                ));
            }
            Some(clickhouse_cloud_api::models::ClickPipeScaling {
                replicas: i64::from(replicas),
                replica_cpu_millicores: i64::from(cpu),
                replica_memory_gb: memory,
                #[cfg(feature = "deprecated-fields")]
                concurrency: 0,
            })
        }
        _ => {
            return Err(CloudError::usage(
                "initial scaling requires --replicas, --cpu-millicores, and --memory-gb together",
            ));
        }
    };

    if args.settings.kafka_read_committed.is_some() && !source.is_kafka() {
        return Err(CloudError::usage(
            "--kafka-read-committed can only be used with clickpipe create kafka",
        ));
    }
    if args.settings.has_object_storage_setting() && source != ClickPipeSourceKind::ObjectStorage {
        return Err(CloudError::usage(
            "--object-storage-* settings can only be used with clickpipe create object-storage",
        ));
    }
    let settings = args.settings.to_create_settings();

    Ok(BuiltCreateRequestArgs {
        start_paused: args.start_paused,
        validate_samples: args.validation.validate_samples,
        scaling,
        settings,
        field_mappings: parse_create_field_mappings(&args.field_mappings)?,
    })
}

pub(super) fn build_create_validation_args(
    args: &ClickPipeCreateValidationArgs,
) -> BuiltCreateRequestArgs {
    BuiltCreateRequestArgs {
        start_paused: false,
        validate_samples: args.validate_samples,
        scaling: None,
        settings: None,
        field_mappings: Vec::new(),
    }
}

pub(super) fn apply_create_request_args(
    request: &mut clickhouse_cloud_api::models::ClickPipePostRequest,
    args: BuiltCreateRequestArgs,
) {
    // `startPaused` and `fieldMappings` are optional in the spec; omit `false`
    // and an empty list so existing create requests stay unchanged.
    request.start_paused = args.start_paused.then_some(true);
    request.source.validate_samples = args.validate_samples;
    request.scaling = args.scaling;
    request.settings = args.settings;
    request.field_mappings = non_empty(args.field_mappings);
}

/// Print the standard "created" confirmation for any create_* handler.
pub(super) fn print_created(
    clickpipe: &clickhouse_cloud_api::models::ClickPipe,
    json: bool,
) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(clickpipe)?);
    } else {
        println!("ClickPipe created successfully!");
        println!("  Name: {}", or_absent(clickpipe.name.as_deref()));
        println!("  ID: {}", or_absent(clickpipe.id.as_ref()));
        println!("  State: {}", or_absent(clickpipe.state.as_ref()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::cloud::clickpipes::cli::{
        ClickPipeCommands, ClickPipeCreateCommands, ClickPipeSettingsValues,
    };
    use crate::cloud::clickpipes::destination::build_destination;
    use crate::cloud::clickpipes::test_support::{assert_names, parse_clickpipe};

    #[test]
    fn start_paused_parsing_and_shared_builder_match_supported_sources() {
        use clap::CommandFactory;
        let cli = Cli::command();
        let create = cli
            .find_subcommand("cloud")
            .unwrap()
            .find_subcommand("clickpipe")
            .unwrap()
            .find_subcommand("create")
            .unwrap();
        for (source, kind) in [
            ("kafka", ClickPipeSourceKind::Kafka),
            ("kinesis", ClickPipeSourceKind::Kinesis),
            ("object-storage", ClickPipeSourceKind::ObjectStorage),
            ("pubsub", ClickPipeSourceKind::PubSub),
        ] {
            assert!(
                create
                    .find_subcommand(source)
                    .unwrap()
                    .get_arguments()
                    .any(|arg| arg.get_long() == Some("start-paused"))
            );
            for paused in [false, true] {
                let args = ClickPipeCreateRequestArgs {
                    start_paused: paused,
                    ..Default::default()
                };
                let built = build_create_request_args(&args, kind).unwrap();
                let mut request = clickhouse_cloud_api::models::ClickPipePostRequest {
                    name: String::new(),
                    source: Default::default(),
                    destination: build_destination("db", "", Vec::new(), None),
                    field_mappings: None,
                    scaling: None,
                    settings: None,
                    start_paused: None,
                };
                apply_create_request_args(&mut request, built);
                assert_eq!(request.start_paused, paused.then_some(true));
                let json = serde_json::to_value(&request).unwrap();
                assert_eq!(
                    json.get("startPaused"),
                    paused.then_some(&serde_json::Value::Bool(true))
                );
            }
        }
        for kind in [
            ClickPipeSourceKind::Postgres,
            ClickPipeSourceKind::MySql,
            ClickPipeSourceKind::MongoDb,
            ClickPipeSourceKind::BigQuery,
        ] {
            let args = ClickPipeCreateRequestArgs {
                start_paused: true,
                ..Default::default()
            };
            assert!(build_create_request_args(&args, kind).is_err());
        }
        let ClickPipeCommands::Create {
            command: ClickPipeCreateCommands::Kinesis(args),
        } = parse_clickpipe(&[
            "create",
            "kinesis",
            "svc-1",
            "--name",
            "pipe",
            "--stream-name",
            "events",
            "--region",
            "us-east-1",
            "--format",
            "JSONEachRow",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:UInt64",
            "--start-paused",
        ])
        else {
            panic!("kinesis");
        };
        assert!(args.request.start_paused);
    }

    #[test]
    fn build_create_request_args_preserves_omission_and_explicit_values() {
        let minimal = build_create_request_args(
            &ClickPipeCreateRequestArgs::default(),
            ClickPipeSourceKind::Kafka,
        )
        .unwrap();
        assert!(!minimal.start_paused);
        assert_eq!(minimal.validate_samples, None);
        assert_eq!(minimal.scaling, None);
        assert_eq!(minimal.settings, None);
        assert!(minimal.field_mappings.is_empty());

        let maximal = build_create_request_args(
            &ClickPipeCreateRequestArgs {
                start_paused: true,
                validation: ClickPipeCreateValidationArgs {
                    validate_samples: Some(false),
                },
                replicas: Some(1),
                cpu_millicores: Some(125),
                memory_gb: Some(0.5),
                field_mappings: vec![
                    r#"{"sourceField":"source:a=b","destinationField":"destination:x=y"}"#.into(),
                ],
                settings: ClickPipeSettingsValues {
                    streaming_max_insert_wait_ms: Some(500),
                    clickhouse_max_threads: Some(0),
                    clickhouse_max_insert_threads: Some(0),
                    clickhouse_max_download_threads: Some(0),
                    clickhouse_min_insert_block_size_bytes: Some(0),
                    clickhouse_parallel_distributed_insert_select: Some(0),
                    kafka_read_committed: Some(false),
                    clickhouse_parallel_view_processing: Some(false),
                    ..Default::default()
                },
            },
            ClickPipeSourceKind::Kafka,
        )
        .unwrap();
        assert!(maximal.start_paused);
        assert_eq!(maximal.validate_samples, Some(false));
        let scaling = maximal.scaling.unwrap();
        assert_eq!(scaling.replicas, 1);
        assert_eq!(scaling.replica_cpu_millicores, 125);
        assert_eq!(scaling.replica_memory_gb, 0.5);
        let settings = maximal.settings.unwrap();
        assert_eq!(settings.clickhouse_max_threads, Some(0));
        assert_eq!(settings.kafka_read_committed, Some(false));
        assert_eq!(maximal.field_mappings.len(), 1);
        assert_eq!(maximal.field_mappings[0].source_field, "source:a=b");
        assert_eq!(
            maximal.field_mappings[0].destination_field,
            "destination:x=y"
        );

        let object_storage = build_create_request_args(
            &ClickPipeCreateRequestArgs {
                settings: ClickPipeSettingsValues {
                    object_storage_concurrency: Some(1),
                    object_storage_polling_interval_ms: Some(100),
                    object_storage_max_insert_bytes: Some(524_288_000),
                    object_storage_max_file_count: Some(1),
                    object_storage_max_file_size: Some(1_048_576),
                    object_storage_use_cluster_function: Some(false),
                    ..Default::default()
                },
                ..Default::default()
            },
            ClickPipeSourceKind::ObjectStorage,
        )
        .unwrap();
        let settings = object_storage.settings.unwrap();
        assert_eq!(settings.object_storage_concurrency, Some(1));
        assert_eq!(settings.object_storage_max_file_size, Some(1_048_576));
        assert_eq!(settings.object_storage_use_cluster_function, Some(false));
        assert_eq!(settings.kafka_read_committed, None);
    }

    #[test]
    fn build_create_request_args_validates_shape_and_source_compatibility() {
        let partial_scaling = ClickPipeCreateRequestArgs {
            replicas: Some(1),
            ..Default::default()
        };
        assert_names(
            &build_create_request_args(&partial_scaling, ClickPipeSourceKind::Kafka)
                .unwrap_err()
                .message,
            &["--replicas", "--cpu-millicores", "--memory-gb"],
        );

        let unknown_mapping_field = ClickPipeCreateRequestArgs {
            field_mappings: vec![
                r#"{"sourceField":"source","destinationField":"target","typo":true}"#.into(),
            ],
            ..Default::default()
        };
        assert_names(
            &build_create_request_args(&unknown_mapping_field, ClickPipeSourceKind::Kafka)
                .unwrap_err()
                .message,
            &["--field-mapping #1: ", "`typo`"],
        );

        let non_kafka = ClickPipeCreateRequestArgs {
            settings: ClickPipeSettingsValues {
                kafka_read_committed: Some(false),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_names(
            &build_create_request_args(&non_kafka, ClickPipeSourceKind::PubSub)
                .unwrap_err()
                .message,
            &["--kafka-read-committed", "clickpipe create kafka"],
        );

        let non_object_storage = ClickPipeCreateRequestArgs {
            settings: ClickPipeSettingsValues {
                object_storage_max_file_size: Some(1_048_576),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_names(
            &build_create_request_args(&non_object_storage, ClickPipeSourceKind::Kafka)
                .unwrap_err()
                .message,
            &["clickpipe create object-storage"],
        );
    }
}
