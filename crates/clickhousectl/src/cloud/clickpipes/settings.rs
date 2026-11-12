use super::classify_clickpipe_source;
use super::cli::ClickPipeSettingsValues;
use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::output::print_human;
use crate::cloud::shared::resolve_org_id;

pub(super) async fn clickpipe_settings_get(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    // The settings endpoint only exists for streaming and object-storage pipes,
    // so the pipe is fetched first to classify its source and refuse a database
    // CDC pipe with an applicability error rather than the API's NOT_FOUND.
    let clickpipe = client
        .get_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;
    ensure_clickpipe_has_ingestion_settings(&clickpipe, service_id, clickpipe_id)?;
    let settings = client
        .get_clickpipe_settings(&org_id, service_id, clickpipe_id)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&settings)?);
    } else {
        print_human(&settings)?;
    }
    Ok(())
}

impl ClickPipeSettingsValues {
    pub(super) fn has_any(&self) -> bool {
        self.streaming_max_insert_wait_ms.is_some()
            || self.object_storage_concurrency.is_some()
            || self.object_storage_polling_interval_ms.is_some()
            || self.object_storage_max_insert_bytes.is_some()
            || self.object_storage_max_file_count.is_some()
            || self.object_storage_max_file_size.is_some()
            || self.clickhouse_max_threads.is_some()
            || self.clickhouse_max_insert_threads.is_some()
            || self.clickhouse_max_download_threads.is_some()
            || self.clickhouse_min_insert_block_size_bytes.is_some()
            || self.clickhouse_parallel_distributed_insert_select.is_some()
            || self.kafka_read_committed.is_some()
            || self.object_storage_use_cluster_function.is_some()
            || self.clickhouse_parallel_view_processing.is_some()
    }

    pub(super) fn has_object_storage_setting(&self) -> bool {
        self.object_storage_concurrency.is_some()
            || self.object_storage_polling_interval_ms.is_some()
            || self.object_storage_max_insert_bytes.is_some()
            || self.object_storage_max_file_count.is_some()
            || self.object_storage_max_file_size.is_some()
            || self.object_storage_use_cluster_function.is_some()
    }
}

/// Refuse a `clickpipe settings` command that cannot apply to this pipe.
///
/// The ingestion settings endpoints exist for streaming and object-storage
/// pipes only. Calling them for a database CDC pipe returns a bare `NOT_FOUND`
/// about the pipe, so the CLI classifies the source first and explains
/// applicability instead of relaying that (#643).
fn ensure_clickpipe_has_ingestion_settings(
    clickpipe: &clickhouse_cloud_api::models::ClickPipe,
    service_id: &str,
    clickpipe_id: &str,
) -> CloudResult<()> {
    let Some(label) = classify_clickpipe_source(clickpipe).database_source_label() else {
        return Ok(());
    };
    Err(CloudError::new(format!(
        "ClickPipe {clickpipe_id} is a {label} pipe; `clickpipe settings get` and \
         `settings update` apply only to streaming (Kafka, Kinesis) and object-storage \
         pipes. CDC pipe settings (sync interval, pull batch size) live on the pipe \
         itself: see `clickhousectl cloud clickpipe get {service_id} {clickpipe_id}`."
    )))
}

/// Build the settings PUT body from the flags the user passed.
///
/// `kafka_read_committed` is Kafka-only: the API rejects the key for every other
/// source, so callers reject an explicit Kafka-only flag on other sources and
/// pass `None` for the preserved value. An explicitly requested value takes
/// precedence over the current Kafka setting, including `false`.
fn build_clickpipe_settings_request(
    values: &ClickPipeSettingsValues,
    kafka_read_committed: Option<bool>,
) -> clickhouse_cloud_api::models::ClickPipeSettingsPutRequest {
    clickhouse_cloud_api::models::ClickPipeSettingsPutRequest {
        streaming_max_insert_wait_ms: values.streaming_max_insert_wait_ms.map(i64::from),
        object_storage_concurrency: values.object_storage_concurrency.map(i64::from),
        object_storage_polling_interval_ms: values
            .object_storage_polling_interval_ms
            .map(i64::from),
        object_storage_max_insert_bytes: values
            .object_storage_max_insert_bytes
            .map(|value| value as i64),
        object_storage_max_file_count: values.object_storage_max_file_count.map(i64::from),
        object_storage_max_file_size: values.object_storage_max_file_size,
        clickhouse_max_threads: values.clickhouse_max_threads.map(i64::from),
        clickhouse_max_insert_threads: values.clickhouse_max_insert_threads.map(i64::from),
        object_storage_use_cluster_function: values.object_storage_use_cluster_function,
        clickhouse_parallel_view_processing: values.clickhouse_parallel_view_processing,
        kafka_read_committed: values.kafka_read_committed.or(kafka_read_committed),
        clickhouse_max_download_threads: values.clickhouse_max_download_threads.map(i64::from),
        clickhouse_min_insert_block_size_bytes: values
            .clickhouse_min_insert_block_size_bytes
            .map(|value| value as i64),
        clickhouse_parallel_distributed_insert_select: values
            .clickhouse_parallel_distributed_insert_select
            .map(i64::from),
    }
}

pub(super) async fn clickpipe_settings_update(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    values: &ClickPipeSettingsValues,
    json: bool,
) -> CloudResult<()> {
    let org_id = resolve_org_id(client).await?;
    // Live verification for #682 showed that object-storage PUT merges omitted
    // keys, including nondefault values of the three previously hidden settings.
    // Kafka-specific omission was not live-verified, so retain read-before-write
    // preservation for its non-nullable boolean without fabricating a default.
    let clickpipe = client
        .get_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;
    ensure_clickpipe_has_ingestion_settings(&clickpipe, service_id, clickpipe_id)?;
    let source = classify_clickpipe_source(&clickpipe);
    if values.kafka_read_committed.is_some() && !source.is_kafka() {
        return Err(CloudError::new(
            "--kafka-read-committed requires a ClickPipe with a confirmed Kafka source",
        ));
    }
    let kafka_read_committed = if source.is_kafka() && values.kafka_read_committed.is_none() {
        Some(
            client
                .get_clickpipe_settings(&org_id, service_id, clickpipe_id)
                .await?
                .kafka_read_committed
                .ok_or_else(|| {
                    CloudError::new(
                        "Cannot preserve Kafka read-committed: the settings response omitted \
                         kafka_read_committed; pass --kafka-read-committed true or false explicitly",
                    )
                })?,
        )
    } else {
        None
    };
    let request = build_clickpipe_settings_request(values, kafka_read_committed);
    let settings = client
        .update_clickpipe_settings(&org_id, service_id, clickpipe_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&settings)?);
    } else {
        print_human(&settings)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::cloud::clickpipes::ClickPipeSourceKind;
    use crate::cloud::clickpipes::cli::{
        ClickPipeCommands, ClickPipeCreateCommands, ClickPipeSettingsCommands,
    };
    use crate::cloud::clickpipes::test_support::{assert_names, parse_clickpipe};
    use clap::Parser;

    #[test]
    fn object_storage_insert_size_boundaries_match_create_and_settings_update() {
        for size in [
            524_287_999_u64,
            524_288_000,
            536_870_912,
            10_737_418_240,
            10_737_418_241,
            10_485_760,
            53_687_091_200,
        ] {
            let size_arg = size.to_string();
            for mut args in [
                vec![
                    "create",
                    "object-storage",
                    "--column",
                    "id:Int64",
                    "svc-1",
                    "--name",
                    "pipe-1",
                    "--source-url",
                    "https://example.test/data.csv",
                    "--format",
                    "CSVWithNames",
                    "--database",
                    "default",
                    "--table",
                    "events",
                ],
                vec!["settings", "update", "svc-1", "pipe-1"],
            ] {
                args.extend(["--object-storage-max-insert-bytes", &size_arg]);
                let result = Cli::try_parse_from(
                    ["chctl", "cloud", "clickpipe"]
                        .into_iter()
                        .chain(args.iter().copied()),
                );
                if [524_288_000, 536_870_912, 10_737_418_240].contains(&size) {
                    assert!(result.is_ok(), "{args:?}");
                    let settings = match parse_clickpipe(&args) {
                        ClickPipeCommands::Create {
                            command: ClickPipeCreateCommands::ObjectStorage(args),
                        } => args.request.settings,
                        ClickPipeCommands::Settings {
                            command: ClickPipeSettingsCommands::Update { settings, .. },
                        } => settings,
                        _ => panic!("unexpected command"),
                    };
                    assert_eq!(settings.object_storage_max_insert_bytes, Some(size));
                    let request = build_clickpipe_settings_request(&settings, None);
                    assert_eq!(request.object_storage_max_insert_bytes, Some(size as i64));
                } else {
                    assert_eq!(
                        result.err().expect("out-of-range size must fail").kind(),
                        clap::error::ErrorKind::ValueValidation,
                        "{args:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn parses_settings_commands_flags_and_defaults() {
        let ClickPipeCommands::Settings {
            command:
                ClickPipeSettingsCommands::Get {
                    service_id,
                    clickpipe_id,
                },
        } = parse_clickpipe(&["settings", "get", "svc-1", "pipe-1", "--org-id", "org-1"])
        else {
            panic!("expected settings get");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(clickpipe_id.id.as_deref(), Some("pipe-1"));

        let ClickPipeCommands::Settings {
            command:
                ClickPipeSettingsCommands::Update {
                    service_id,
                    clickpipe_id,
                    settings,
                },
        } = parse_clickpipe(&[
            "settings",
            "update",
            "svc-1",
            "pipe-1",
            "--streaming-max-insert-wait-ms",
            "1000",
            "--object-storage-concurrency",
            "2",
            "--object-storage-polling-interval-ms",
            "3000",
            "--object-storage-max-insert-bytes",
            "524288000",
            "--object-storage-max-file-count",
            "5",
            "--object-storage-max-file-size",
            "1048576",
            "--clickhouse-max-threads",
            "6",
            "--clickhouse-max-insert-threads",
            "7",
            "--clickhouse-max-download-threads",
            "8",
            "--clickhouse-min-insert-block-size-bytes",
            "20971520",
            "--clickhouse-parallel-distributed-insert-select",
            "1",
            "--kafka-read-committed",
            "false",
            "--object-storage-use-cluster-function",
            "true",
            "--clickhouse-parallel-view-processing",
            "false",
            "--org-id",
            "org-1",
        ])
        else {
            panic!("expected settings update");
        };
        assert_eq!(service_id, "svc-1");
        assert_eq!(clickpipe_id.id.as_deref(), Some("pipe-1"));
        assert_eq!(settings.streaming_max_insert_wait_ms, Some(1000));
        assert_eq!(settings.object_storage_concurrency, Some(2));
        assert_eq!(settings.object_storage_polling_interval_ms, Some(3000));
        assert_eq!(settings.object_storage_max_insert_bytes, Some(524_288_000));
        assert_eq!(settings.object_storage_max_file_count, Some(5));
        assert_eq!(settings.object_storage_max_file_size, Some(1_048_576));
        assert_eq!(settings.clickhouse_max_threads, Some(6));
        assert_eq!(settings.clickhouse_max_insert_threads, Some(7));
        assert_eq!(settings.clickhouse_max_download_threads, Some(8));
        assert_eq!(
            settings.clickhouse_min_insert_block_size_bytes,
            Some(20971520)
        );
        assert_eq!(
            settings.clickhouse_parallel_distributed_insert_select,
            Some(1)
        );
        assert_eq!(settings.kafka_read_committed, Some(false));
        assert_eq!(settings.object_storage_use_cluster_function, Some(true));
        assert_eq!(settings.clickhouse_parallel_view_processing, Some(false));

        let ClickPipeCommands::Settings {
            command: ClickPipeSettingsCommands::Update { settings, .. },
        } = parse_clickpipe(&[
            "settings",
            "update",
            "svc-1",
            "pipe-1",
            "--object-storage-max-file-count",
            "200",
        ])
        else {
            panic!("expected settings update");
        };
        assert_eq!(settings.streaming_max_insert_wait_ms, None);
        assert_eq!(settings.object_storage_concurrency, None);
        assert_eq!(settings.object_storage_polling_interval_ms, None);
        assert_eq!(settings.object_storage_max_insert_bytes, None);
        assert_eq!(settings.object_storage_max_file_count, Some(200));
        assert_eq!(settings.object_storage_max_file_size, None);
        assert_eq!(settings.clickhouse_max_threads, None);
        assert_eq!(settings.clickhouse_max_insert_threads, None);
        assert_eq!(settings.clickhouse_max_download_threads, None);
        assert_eq!(settings.clickhouse_min_insert_block_size_bytes, None);
        assert_eq!(settings.clickhouse_parallel_distributed_insert_select, None);
        assert_eq!(settings.kafka_read_committed, None);
        assert_eq!(settings.object_storage_use_cluster_function, None);
        assert_eq!(settings.clickhouse_parallel_view_processing, None);

        // The max file size alone satisfies the required settings group.
        let ClickPipeCommands::Settings {
            command: ClickPipeSettingsCommands::Update { settings, .. },
        } = parse_clickpipe(&[
            "settings",
            "update",
            "svc-1",
            "pipe-1",
            "--object-storage-max-file-size",
            "0",
        ])
        else {
            panic!("expected settings update");
        };
        assert_eq!(
            settings,
            ClickPipeSettingsValues {
                object_storage_max_file_size: Some(0),
                ..Default::default()
            }
        );
    }

    // A non-Kafka pipe must not carry `kafka_read_committed`: the API rejects
    // the key for every other source, which broke every non-Kafka settings
    // update (#602).
    #[test]
    fn builds_settings_request_without_kafka_only_settings_for_non_kafka_pipes() {
        let values = ClickPipeSettingsValues {
            object_storage_max_file_count: Some(200),
            ..Default::default()
        };
        let request = build_clickpipe_settings_request(&values, None);
        assert_eq!(request.kafka_read_committed, None);
        assert_eq!(request.object_storage_max_file_count, Some(200));
        assert_eq!(request.object_storage_max_file_size, None);
        assert_eq!(request.streaming_max_insert_wait_ms, None);
        assert_eq!(request.object_storage_concurrency, None);
        assert_eq!(request.object_storage_polling_interval_ms, None);
        assert_eq!(request.object_storage_max_insert_bytes, None);
        assert_eq!(request.clickhouse_max_threads, None);
        assert_eq!(request.clickhouse_max_insert_threads, None);
        assert_eq!(request.object_storage_use_cluster_function, None);
        assert_eq!(request.clickhouse_parallel_view_processing, None);
        assert_eq!(request.clickhouse_max_download_threads, None);
        assert_eq!(request.clickhouse_min_insert_block_size_bytes, None);
        assert_eq!(request.clickhouse_parallel_distributed_insert_select, None);
    }

    #[test]
    fn builds_settings_request_with_every_flag_and_kafka_read_committed() {
        let values = ClickPipeSettingsValues {
            streaming_max_insert_wait_ms: Some(1000),
            object_storage_concurrency: Some(2),
            object_storage_polling_interval_ms: Some(3000),
            object_storage_max_insert_bytes: Some(4000),
            object_storage_max_file_count: Some(5),
            object_storage_max_file_size: Some(1_048_576),
            clickhouse_max_threads: Some(6),
            clickhouse_max_insert_threads: Some(7),
            clickhouse_max_download_threads: Some(8),
            clickhouse_min_insert_block_size_bytes: Some(20971520),
            clickhouse_parallel_distributed_insert_select: Some(1),
            kafka_read_committed: None,
            object_storage_use_cluster_function: Some(true),
            clickhouse_parallel_view_processing: Some(false),
        };
        let request = build_clickpipe_settings_request(&values, Some(true));
        assert_eq!(request.streaming_max_insert_wait_ms, Some(1000));
        assert_eq!(request.object_storage_concurrency, Some(2));
        assert_eq!(request.object_storage_polling_interval_ms, Some(3000));
        assert_eq!(request.object_storage_max_insert_bytes, Some(4000));
        assert_eq!(request.object_storage_max_file_count, Some(5));
        assert_eq!(request.object_storage_max_file_size, Some(1_048_576));
        assert_eq!(request.clickhouse_max_threads, Some(6));
        assert_eq!(request.clickhouse_max_insert_threads, Some(7));
        assert_eq!(request.clickhouse_max_download_threads, Some(8));
        assert_eq!(
            request.clickhouse_min_insert_block_size_bytes,
            Some(20971520)
        );
        assert_eq!(
            request.clickhouse_parallel_distributed_insert_select,
            Some(1)
        );
        assert_eq!(request.object_storage_use_cluster_function, Some(true));
        assert_eq!(request.clickhouse_parallel_view_processing, Some(false));
        assert_eq!(request.kafka_read_committed, Some(true));
        // A Kafka pipe with the setting disabled still sends it explicitly, so
        // the PUT does not silently flip it.
        let request = build_clickpipe_settings_request(&values, Some(false));
        assert_eq!(request.kafka_read_committed, Some(false));
    }

    #[test]
    fn explicit_settings_zero_and_false_override_preserved_values() {
        let values = ClickPipeSettingsValues {
            clickhouse_max_download_threads: Some(0),
            clickhouse_min_insert_block_size_bytes: Some(0),
            clickhouse_parallel_distributed_insert_select: Some(0),
            kafka_read_committed: Some(false),
            ..Default::default()
        };
        let request = build_clickpipe_settings_request(&values, Some(true));
        assert_eq!(request.clickhouse_max_download_threads, Some(0));
        assert_eq!(request.clickhouse_min_insert_block_size_bytes, Some(0));
        assert_eq!(
            request.clickhouse_parallel_distributed_insert_select,
            Some(0)
        );
        assert_eq!(request.kafka_read_committed, Some(false));
    }

    #[test]
    fn settings_added_integer_flags_enforce_api_ranges() {
        for (flag, maximum) in [
            ("--clickhouse-max-download-threads", 32_u64),
            ("--clickhouse-min-insert-block-size-bytes", 10737418240),
            ("--clickhouse-parallel-distributed-insert-select", 2),
        ] {
            for value in [0, maximum] {
                parse_clickpipe(&[
                    "settings",
                    "update",
                    "svc",
                    "pipe",
                    flag,
                    &value.to_string(),
                ]);
            }
            let error = Cli::try_parse_from([
                "clickhousectl",
                "cloud",
                "clickpipe",
                "settings",
                "update",
                "svc",
                "pipe",
                flag,
                &(maximum + 1).to_string(),
            ])
            .err()
            .expect("an out-of-range setting should fail parsing");
            assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
        }
    }

    #[test]
    fn settings_update_requires_at_least_one_setting() {
        let error = Cli::try_parse_from([
            "clickhousectl",
            "cloud",
            "clickpipe",
            "settings",
            "update",
            "svc",
            "pipe",
        ])
        .err()
        .expect("an empty update should fail parsing");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn only_database_sources_have_no_ingestion_settings() {
        for (kind, label) in [
            (ClickPipeSourceKind::Postgres, Some("Postgres CDC")),
            (ClickPipeSourceKind::MySql, Some("MySQL CDC")),
            (ClickPipeSourceKind::MongoDb, Some("MongoDB CDC")),
            (ClickPipeSourceKind::BigQuery, Some("BigQuery")),
            (ClickPipeSourceKind::Kafka, None),
            (ClickPipeSourceKind::Kinesis, None),
            (ClickPipeSourceKind::PubSub, None),
            (ClickPipeSourceKind::ObjectStorage, None),
            (ClickPipeSourceKind::Absent, None),
        ] {
            assert_eq!(kind.database_source_label(), label, "{kind:?}");
        }
    }

    #[test]
    fn settings_refusal_names_the_source_and_points_at_clickpipe_get() {
        use clickhouse_cloud_api::models::{
            ClickPipe, ClickPipeMySQLSource, ClickPipeObjectStorageSource, ClickPipeSource,
        };

        let mysql = ClickPipe {
            source: Some(ClickPipeSource {
                mysql: Some(ClickPipeMySQLSource::default()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let error = ensure_clickpipe_has_ingestion_settings(&mysql, "svc-1", "pipe-1")
            .expect_err("a MySQL CDC pipe has no ingestion settings");
        let suggested = "clickhousectl cloud clickpipe get svc-1 pipe-1";
        assert_names(
            &error.message,
            &[
                "pipe-1",
                ClickPipeSourceKind::MySql.database_source_label().unwrap(),
                "`clickpipe settings get`",
                "`settings update`",
                &format!("`{suggested}`"),
            ],
        );
        // The suggested command is a real one.
        Cli::try_parse_from(suggested.split_whitespace()).expect("suggested command parses");
        assert_eq!(error.kind, crate::cloud::client::CloudErrorKind::Generic);

        // Streaming, object-storage and unclassifiable pipes are not refused.
        let object_storage = ClickPipe {
            source: Some(ClickPipeSource {
                object_storage: Some(ClickPipeObjectStorageSource::default()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(
            ensure_clickpipe_has_ingestion_settings(&object_storage, "svc-1", "pipe-1").is_ok()
        );
        assert!(
            ensure_clickpipe_has_ingestion_settings(&ClickPipe::default(), "svc-1", "pipe-1")
                .is_ok()
        );
    }
}
