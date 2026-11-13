//! `cloud clickpipe` (src/cloud/clickpipes/, src/cloud/clickpipe_endpoints.rs).

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use wiremock::matchers::{body_json, header, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

/// Start a wiremock server that accepts a clickpipe-create POST, returns a
/// stub ClickPipe JSON, and records the request body for later assertion.
async fn start_mock_clickpipes_api() -> MockServer {
    let mock = MockServer::start().await;

    // Stub response: minimum fields the CLI's `--json` output needs to
    // deserialize into a `ClickPipe`. The CLI prints whatever comes back;
    // we only care about the request body, which wiremock records.
    let stub_pipe = serde_json::json!({
        "result": {
            "id": "00000000-0000-0000-0000-000000000000",
            "name": "stub",
            "state": "Stopped",
            "scaling": { "replicas": 1 },
            "source": {},
            "destination": { "database": "default" },
            "metrics": {},
        },
        "status": 200,
        "requestId": "stub-request-id"
    });

    Mock::given(method("POST"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/clickpipes$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_pipe))
        .mount(&mock)
        .await;

    mock
}

// ── ClickPipe create ───────────────────────────────────────────────────────
//
// Field-by-field request shape belongs to the `build_*_request` unit tests in
// `src/cloud/clickpipes.rs`. These tests cover what only the real handler
// reaches: the assembled body on the wire, stdin reads, and error output.

fn kafka_args_minimal() -> Vec<&'static str> {
    vec![
        "clickpipe",
        "create",
        "kafka",
        "svc-id",
        "--name",
        "t",
        "--brokers",
        "broker:9092",
        "--topics",
        "topic",
        "--format",
        "JSONEachRow",
        "--database",
        "default",
        "--table",
        "events",
        "--column",
        "id:Int64",
        "--kafka-type",
        "kafka",
        "--org-id",
        "org",
        "--auth",
        "PLAIN",
        "--username",
        "u",
        "--password",
        "p",
    ]
}

fn append_initial_scaling_and_validation(args: &mut Vec<String>) {
    args.extend([
        "--replicas".into(),
        "2".into(),
        "--cpu-millicores".into(),
        "500".into(),
        "--memory-gb".into(),
        "2".into(),
    ]);
    append_sample_validation(args);
}

fn append_sample_validation(args: &mut Vec<String>) {
    args.extend(["--validate-samples".into(), "true".into()]);
}

/// The controls every streaming create shares. The destination flags go
/// through each streaming handler, which assembles its own request rather than
/// calling a `build_*_request` builder, so only this test sees them on the wire.
fn append_streaming_create_controls(args: &mut Vec<String>, table_definition_file: &str) {
    append_initial_scaling_and_validation(args);
    args.extend([
        "--start-paused".into(),
        "--field-mapping".into(),
        r#"{"sourceField":"source:a=b","destinationField":"destination:x=y"}"#.into(),
        "--clickhouse-max-threads".into(),
        "0".into(),
        "--clickhouse-parallel-view-processing".into(),
        "false".into(),
        "--role".into(),
        "analytics_reader".into(),
        "--managed-table".into(),
        "false".into(),
        "--table-definition-file".into(),
        table_definition_file.into(),
    ]);
}

#[tokio::test]
async fn cross_source_create_controls_reach_all_eight_request_shapes() {
    let directory = tempfile::tempdir().unwrap();
    let service_account = directory.path().join("service-account.json");
    std::fs::write(&service_account, "{}").unwrap();
    let service_account = service_account.to_str().unwrap();
    let table_definition = directory.path().join("table-definition.json");
    std::fs::write(
        &table_definition,
        serde_json::json!({
            "engine": {
                "columnIds": ["amount"],
                "type": "SummingMergeTree",
                "versionColumnId": null
            },
            "partitionBy": "toYYYYMM(created_at)",
            "primaryKey": "event_id",
            "sortingKey": ["event_id", "created_at"]
        })
        .to_string(),
    )
    .unwrap();
    let table_definition = table_definition.to_str().unwrap();

    let mut object_storage = [
        "clickpipe",
        "create",
        "object-storage",
        "svc-id",
        "--name",
        "object-pipe",
        "--source-url",
        "https://example.test/events.json",
        "--format",
        "JSONEachRow",
        "--database",
        "default",
        "--table",
        "events",
        "--column",
        "id:Int64",
        "--org-id",
        "org",
    ]
    .map(str::to_string)
    .to_vec();
    append_streaming_create_controls(&mut object_storage, table_definition);
    object_storage.extend([
        "--object-storage-concurrency".into(),
        "1".into(),
        "--object-storage-polling-interval-ms".into(),
        "100".into(),
        "--object-storage-max-insert-bytes".into(),
        "524288000".into(),
        "--object-storage-max-file-count".into(),
        "1".into(),
        "--object-storage-use-cluster-function".into(),
        "false".into(),
    ]);

    let mut kafka = kafka_args_minimal()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    append_streaming_create_controls(&mut kafka, table_definition);
    kafka.extend([
        "--streaming-max-insert-wait-ms".into(),
        "500".into(),
        "--kafka-read-committed".into(),
        "false".into(),
        "--exactly-once".into(),
        "true".into(),
    ]);

    let mut kinesis = [
        "clickpipe",
        "create",
        "kinesis",
        "svc-id",
        "--name",
        "kinesis-pipe",
        "--stream-name",
        "events",
        "--region",
        "us-east-1",
        "--format",
        "JSONEachRow",
        "--iam-role",
        "arn:aws:iam::123456789012:role/clickpipe",
        "--database",
        "default",
        "--table",
        "events",
        "--column",
        "id:Int64",
        "--org-id",
        "org",
    ]
    .map(str::to_string)
    .to_vec();
    append_streaming_create_controls(&mut kinesis, table_definition);
    kinesis.extend(["--streaming-max-insert-wait-ms".into(), "500".into()]);

    let mut postgres = postgres_args_minimal();
    append_sample_validation(&mut postgres);
    let mut mysql = mysql_args_minimal();
    append_sample_validation(&mut mysql);

    let mut mongodb = [
        "clickpipe",
        "create",
        "mongodb",
        "svc-id",
        "--name",
        "mongo-pipe",
        "--uri",
        "mongodb://mongo.example/source",
        "--username",
        "u",
        "--password",
        "p",
        "--table-mapping",
        "source.events:events",
        "--destination-database",
        "default",
        "--org-id",
        "org",
    ]
    .map(str::to_string)
    .to_vec();
    append_sample_validation(&mut mongodb);

    let mut bigquery = [
        "clickpipe",
        "create",
        "bigquery",
        "svc-id",
        "--name",
        "bigquery-pipe",
        "--service-account-file",
        service_account,
        "--staging-path",
        "gs://bucket/staging",
        "--table-mapping",
        "source.events:events",
        "--destination-database",
        "default",
        "--org-id",
        "org",
    ]
    .map(str::to_string)
    .to_vec();
    append_sample_validation(&mut bigquery);

    let mut pubsub = pubsub_create_args(service_account);
    append_streaming_create_controls(&mut pubsub, table_definition);
    pubsub.extend(["--streaming-max-insert-wait-ms".into(), "500".into()]);

    for (source, args, has_streaming_controls) in [
        ("objectStorage", object_storage, true),
        ("kafka", kafka, true),
        ("kinesis", kinesis, true),
        ("postgres", postgres, false),
        ("mysql", mysql, false),
        ("mongodb", mongodb, false),
        ("bigquery", bigquery, false),
        ("pubsub", pubsub, true),
    ] {
        let mock = start_mock_clickpipes_api().await;
        let body = invoke_cli_capture_body(&mock, &as_str_args(&args)).await;
        assert_eq!(body["source"]["validateSamples"], true, "{source}");
        assert!(body["source"][source].is_object(), "{source}: {body}");
        if has_streaming_controls {
            assert_eq!(body["startPaused"], true, "{source}");
            assert_eq!(
                body["scaling"],
                serde_json::json!({
                    "replicas": 2,
                    "replicaCpuMillicores": 500,
                    "replicaMemoryGb": 2.0,
                }),
                "{source}"
            );
            assert_eq!(
                body["fieldMappings"],
                serde_json::json!([{
                    "sourceField": "source:a=b",
                    "destinationField": "destination:x=y",
                }]),
                "{source}"
            );
            assert_eq!(body["settings"]["clickhouse_max_threads"], 0, "{source}");
            assert_eq!(
                body["settings"]["clickhouse_parallel_view_processing"], false,
                "{source}"
            );
            assert_eq!(
                body["destination"]["roles"],
                serde_json::json!(["analytics_reader"]),
                "{source}"
            );
            assert_eq!(body["destination"]["managedTable"], false, "{source}");
            assert_eq!(
                body["destination"]["tableDefinition"],
                serde_json::json!({
                    "engine": {"columnIds": ["amount"], "type": "SummingMergeTree"},
                    "partitionBy": "toYYYYMM(created_at)",
                    "primaryKey": "event_id",
                    "sortingKey": ["event_id", "created_at"]
                }),
                "{source}"
            );
            if source == "kafka" {
                assert_eq!(body["settings"]["kafka_read_committed"], false, "{source}");
                assert_eq!(body["source"]["kafka"]["exactlyOnce"], true);
            } else {
                assert!(
                    body["settings"].get("kafka_read_committed").is_none(),
                    "{source}: {body}"
                );
            }
            if source == "objectStorage" {
                assert_eq!(body["settings"]["object_storage_concurrency"], 1);
                assert_eq!(
                    body["settings"]["object_storage_use_cluster_function"],
                    false
                );
                assert!(
                    body["settings"]
                        .get("streaming_max_insert_wait_ms")
                        .is_none(),
                    "{body}"
                );
            } else {
                assert_eq!(body["settings"]["streaming_max_insert_wait_ms"], 500);
            }
        } else {
            assert!(body.get("startPaused").is_none(), "{source}");
            assert!(body.get("scaling").is_none(), "{source}: {body}");
            assert!(body.get("fieldMappings").is_none(), "{source}: {body}");
            assert!(body.get("settings").is_none(), "{source}: {body}");
        }
    }
}

/// The exact body each create subcommand sends for a minimal invocation.
///
/// The builder unit tests assert the library request structs; this is the one
/// wire-level check that an omitted flag stays off the body after the handler
/// assembles and serializes the request. A key that appears here without a
/// flag is a default the CLI chose, so a stray `""`, `[]` or zero (the
/// `4f6c2ba` bug class: `unwrap_or_default()` on an optional field) shows up
/// as a diff against this table.
#[tokio::test]
async fn create_minimal_invocations_send_exact_request_bodies() {
    let directory = tempfile::tempdir().unwrap();
    let bigquery_key = directory.path().join("bigquery-key.json");
    std::fs::write(&bigquery_key, "{}").unwrap();
    let pubsub_key = write_service_account_key(directory.path());
    let owned = |args: &[&str]| args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
    let streaming_destination = |column: &str| {
        serde_json::json!({
            "columns": [{"name": column, "type": "Int64"}],
            "database": "default",
            "managedTable": true,
            "table": "events",
            "tableDefinition": {"engine": {"type": "MergeTree"}},
        })
    };
    let cases: Vec<(Vec<String>, Value)> = vec![
        (
            owned(&[
                "clickpipe",
                "create",
                "object-storage",
                "svc-id",
                "--name",
                "object-pipe",
                "--source-url",
                "https://bucket.example/events/*.json",
                "--format",
                "JSONEachRow",
                "--database",
                "default",
                "--table",
                "events",
                "--column",
                "id:Int64",
                "--org-id",
                "org",
            ]),
            serde_json::json!({
                "name": "object-pipe",
                "destination": streaming_destination("id"),
                "source": {"objectStorage": {
                    "compression": "auto",
                    "format": "JSONEachRow",
                    "type": "s3",
                    "url": "https://bucket.example/events/*.json",
                }},
            }),
        ),
        (
            owned(&kafka_args_minimal()),
            serde_json::json!({
                "name": "t",
                "destination": streaming_destination("id"),
                "source": {"kafka": {
                    "authentication": "PLAIN",
                    "brokers": "broker:9092",
                    "credentials": {"password": "p", "username": "u"},
                    "format": "JSONEachRow",
                    "offset": {"strategy": "from_beginning"},
                    "reversePrivateEndpointIds": [],
                    "topics": "topic",
                    "type": "kafka",
                }},
            }),
        ),
        (
            owned(&[
                "clickpipe",
                "create",
                "kinesis",
                "svc-id",
                "--name",
                "kinesis-pipe",
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
                "id:Int64",
                "--org-id",
                "org",
            ]),
            serde_json::json!({
                "name": "kinesis-pipe",
                "destination": streaming_destination("id"),
                "source": {"kinesis": {
                    "authentication": "IAM_ROLE",
                    "format": "JSONEachRow",
                    "iteratorType": "TRIM_HORIZON",
                    "region": "us-east-1",
                    "streamName": "events",
                }},
            }),
        ),
        (
            postgres_args_minimal(),
            serde_json::json!({
                "name": "t",
                // Database pipes carry only the destination database.
                "destination": {"database": "default"},
                "source": {"postgres": {
                    "authentication": "basic",
                    "credentials": {"password": "p", "username": "u"},
                    "database": "test",
                    "disableTls": false,
                    "host": "pg",
                    "port": 5432,
                    // `cdc` creates the publication and slot server-side, so
                    // neither name is sent.
                    "settings": {
                        "allowNullableColumns": false,
                        "deleteOnMerge": false,
                        "enableFailoverSlots": false,
                        "replicationMode": "cdc",
                    },
                    "skipCertVerification": false,
                    "tableMappings": [{
                        "excludedColumns": [],
                        "partitionByExpr": "",
                        "partitionKey": "",
                        "sortingKeys": [],
                        "sourceSchemaName": "public",
                        "sourceTable": "t",
                        "tableEngine": "ReplacingMergeTree",
                        "targetTable": "t",
                        "useCustomSortingKey": false,
                    }],
                    "type": "postgres",
                }},
            }),
        ),
        (
            mysql_args_minimal(),
            serde_json::json!({
                "name": "t",
                "destination": {"database": "default"},
                "source": {"mysql": {
                    "authentication": "basic",
                    "credentials": {"password": "p", "username": "u"},
                    "host": "mysql",
                    "port": 3306,
                    "settings": {"replicationMechanism": "GTID", "replicationMode": "cdc"},
                    "tableMappings": [{
                        "sourceSchemaName": "mydb",
                        "sourceTable": "t",
                        "targetTable": "t",
                    }],
                    "type": "mysql",
                }},
            }),
        ),
        (
            owned(&[
                "clickpipe",
                "create",
                "mongodb",
                "svc-id",
                "--name",
                "mongo-pipe",
                "--uri",
                "mongodb://mongo.example:27017",
                "--username",
                "u",
                "--password",
                "p",
                "--table-mapping",
                "source.events:events",
                "--org-id",
                "org",
            ]),
            serde_json::json!({
                "name": "mongo-pipe",
                "destination": {"database": "default"},
                "source": {"mongodb": {
                    "credentials": {"password": "p", "username": "u"},
                    "readPreference": "secondaryPreferred",
                    "settings": {"replicationMode": "cdc"},
                    "tableMappings": [{
                        "sourceCollection": "events",
                        "sourceDatabaseName": "source",
                        "targetTable": "events",
                    }],
                    "uri": "mongodb://mongo.example:27017",
                }},
            }),
        ),
        (
            owned(&[
                "clickpipe",
                "create",
                "bigquery",
                "svc-id",
                "--name",
                "bigquery-pipe",
                "--service-account-file",
                bigquery_key.to_str().unwrap(),
                "--staging-path",
                "gs://bucket/staging",
                "--table-mapping",
                "dataset.events:events",
                "--org-id",
                "org",
            ]),
            serde_json::json!({
                "name": "bigquery-pipe",
                "destination": {"database": "default"},
                "source": {"bigquery": {
                    // The key file's contents, base64-encoded; never its path.
                    "credentials": {"serviceAccountFile": "e30="},
                    "settings": {"replicationMode": "snapshot"},
                    "snapshotStagingPath": "gs://bucket/staging",
                    "tableMappings": [{
                        "sourceDatasetName": "dataset",
                        "sourceTable": "events",
                        "targetTable": "events",
                    }],
                }},
            }),
        ),
        (
            pubsub_create_args(pubsub_key.to_str().unwrap()),
            serde_json::json!({
                "name": "pubsub-pipe",
                "destination": streaming_destination("event_id"),
                "source": {"pubsub": {
                    "authentication": "SERVICE_ACCOUNT",
                    "format": "JSONEachRow",
                    "projectId": "my-gcp-project",
                    "seekType": "earliest",
                    "serviceAccountKey": {"serviceAccountFile": encoded_service_account_key()},
                    "topic": "events",
                }},
            }),
        ),
    ];

    for (args, expected) in cases {
        let mock = start_mock_clickpipes_api().await;
        let body = invoke_cli_capture_body(&mock, &as_str_args(&args)).await;
        assert_eq!(body, expected, "clickpipe create {}", args[2]);
    }
}

fn kinesis_auth_args<'a>(operation: &'a str, flags: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "clickpipe",
        operation,
        "kinesis",
        "svc-id",
        "--org-id",
        "org",
        "--stream-name",
        "stream",
        "--region",
        "us-east-1",
        "--format",
        "JSONEachRow",
    ];
    if operation == "create" {
        args.extend([
            "--name",
            "pipe",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
        ]);
    }
    args.extend(flags);
    args
}

#[tokio::test]
async fn kinesis_auth_conflicts_are_secret_free_usage_errors_before_credentials_or_http() {
    // The conflict matrix is unit-tested in `main.rs`; this run checks the
    // process: the usage error wins over missing credentials (exit 2, not 4),
    // nothing reaches HTTP, and neither stream echoes a credential.
    let mock = MockServer::start().await;
    let home = tempfile::tempdir().unwrap();
    let flags = [
        "--iam-role",
        "private-role-arn",
        "--access-key-id",
        "private-access-id",
        "--secret-key",
        "private-secret-key",
    ];
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("HOME", home.path())
        .env("DO_NOT_TRACK", "1")
        .current_dir(home.path())
        .args(["cloud", "--url", &mock.uri(), "--json"])
        .args(kinesis_auth_args("create", &flags))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--iam-role"), "{stderr}");
    for value in [
        "private-role-arn",
        "private-access-id",
        "private-secret-key",
    ] {
        assert!(!stderr.contains(value), "credential leaked: {stderr}");
        assert!(!String::from_utf8_lossy(&output.stdout).contains(value));
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn kinesis_destination_table_definition_can_be_read_from_stdin() {
    let mock = start_mock_clickpipes_api().await;
    let args = [
        "clickpipe",
        "create",
        "kinesis",
        "svc-id",
        "--name",
        "kinesis-pipe",
        "--stream-name",
        "events",
        "--region",
        "eu-west-1",
        "--format",
        "JSONEachRow",
        "--database",
        "analytics",
        "--table",
        "events",
        "--column",
        "event_id:Int64",
        "--table-definition-file",
        "-",
        "--org-id",
        "org",
    ];
    let definition = serde_json::json!({
        "engine": {
            "columnIds": [],
            "type": "ReplacingMergeTree",
            "versionColumnId": "version"
        },
        "partitionBy": "toYYYYMM(created_at)",
        "primaryKey": "event_id",
        "sortingKey": ["event_id"]
    });

    let body =
        invoke_cli_capture_body_with_stdin(&mock, &args, definition.to_string().as_bytes()).await;

    assert_eq!(body["destination"]["managedTable"], true);
    assert_eq!(
        body["destination"]["tableDefinition"],
        serde_json::json!({
            "engine": {
                "type": "ReplacingMergeTree",
                "versionColumnId": "version"
            },
            "partitionBy": "toYYYYMM(created_at)",
            "primaryKey": "event_id",
            "sortingKey": ["event_id"]
        })
    );
}

#[tokio::test]
async fn object_storage_create_reads_columns_from_schema_discovery_on_stdin() {
    let mock = start_mock_clickpipes_api().await;
    let args = [
        "clickpipe",
        "create",
        "object-storage",
        "svc-id",
        "--name",
        "s3-pipe",
        "--source-url",
        "https://bucket.example/events/*.json",
        "--format",
        "JSONEachRow",
        "--database",
        "analytics",
        "--table",
        "events",
        "--columns-from",
        "-",
        "--column",
        "ingested_at:DateTime",
        "--org-id",
        "org",
    ];
    // The shape `clickpipe schema-discover object-storage --json` prints.
    let discovery = serde_json::json!({
        "fields": [
            {"name": "id", "type": "Int64", "optional": false,
             "isVirtual": false, "isVirtualDisabledDefault": false},
            {"name": "_path", "type": "LowCardinality(String)", "optional": false,
             "isVirtual": true, "isVirtualDisabledDefault": false},
            {"name": "_raw", "type": "String", "optional": false,
             "isVirtual": true, "isVirtualDisabledDefault": true}
        ],
        "meta": {}
    });

    let body =
        invoke_cli_capture_body_with_stdin(&mock, &args, discovery.to_string().as_bytes()).await;

    assert_eq!(
        body["destination"]["columns"],
        serde_json::json!([
            {"name": "id", "type": "Int64"},
            {"name": "_path", "type": "LowCardinality(String)"},
            {"name": "ingested_at", "type": "DateTime"}
        ])
    );
}

/// The minimal `clickpipe create mysql` argument set for basic auth.
fn mysql_args_minimal() -> Vec<String> {
    [
        "clickpipe",
        "create",
        "mysql",
        "svc-id",
        "--name",
        "t",
        "--host",
        "mysql",
        "--port",
        "3306",
        "--table-mapping",
        "mydb.t:t",
        "--replication-mode",
        "cdc",
        "--org-id",
        "org",
        "--username",
        "u",
        "--password",
        "p",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

// ── ClickPipe lifecycle acknowledgments (#842) ──────────────────────────────

#[tokio::test]
async fn clickpipe_lifecycle_human_output_acknowledges_the_immediate_state() {
    for (command, clickpipe_id, name, state, expected) in [
        (
            "start",
            "pipe-start",
            Some("batch-pipe"),
            Some("Stopped"),
            "ClickPipe batch-pipe start request accepted (returned state: Stopped)\n",
        ),
        (
            "stop",
            "pipe-stop",
            Some("cdc-pipe"),
            Some("Pausing"),
            "ClickPipe cdc-pipe stop request accepted (returned state: Pausing)\n",
        ),
        (
            "resync",
            "pipe-resync",
            None,
            None,
            "ClickPipe pipe-resync resync request accepted (returned state: -)\n",
        ),
    ] {
        let mock = MockServer::start().await;
        let mut result = serde_json::Map::new();
        if let Some(name) = name {
            result.insert("name".into(), serde_json::json!(name));
        }
        if let Some(state) = state {
            result.insert("state".into(), serde_json::json!(state));
        }
        Mock::given(method("PATCH"))
            .and(path(format!(
                "/v1/organizations/org/services/svc-id/clickpipes/{clickpipe_id}/state"
            )))
            .and(body_json(serde_json::json!({"command": command})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": result,
                "status": 200,
                "requestId": "stub-clickpipe-state-change",
            })))
            .expect(1)
            .mount(&mock)
            .await;

        let output = invoke_cli_with_cloud_credentials_human(
            &mock,
            &[
                "clickpipe",
                command,
                "svc-id",
                clickpipe_id,
                "--org-id",
                "org",
            ],
        );

        assert_success(&output);
        assert!(output.stderr.is_empty());
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
    }
}

#[tokio::test]
async fn clickpipe_lifecycle_preserves_the_json_response_and_names_the_poll() {
    let mock = MockServer::start().await;
    let result = serde_json::json!({
        "id": "00000000-0000-0000-0000-000000000842",
        "name": "batch-pipe",
        "state": "Stopped",
    });
    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org/services/svc-id/clickpipes/pipe-start/state",
        ))
        .and(body_json(serde_json::json!({"command": "start"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-clickpipe-state-change-json",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "start",
            "svc-id",
            "pipe-start",
            "--org-id",
            "org",
        ],
    );

    assert_success(&output);
    assert!(output.stderr.is_empty());
    let (response, next) = take_next(&output.stdout);
    assert_eq!(response, result);
    let next = next.expect("an accepted state change names its poll");
    assert_eq!(
        next["command"],
        "clickhousectl cloud clickpipe get svc-id pipe-start --org-id org --json"
    );
    assert_eq!(next["field"], "state");
    assert_eq!(next["until"], serde_json::json!(["Running", "Completed"]));
    assert_eq!(
        next["fail_on"],
        serde_json::json!(["Failed", "InternalError"])
    );
}

/// A start answered with `Pausing` is dropped by the API (#1228), so it fails
/// with a code and the read to repeat, not an exit-0 acknowledgment.
#[tokio::test]
async fn clickpipe_start_on_a_pausing_pipe_fails_with_the_poll_command() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org/services/svc-id/clickpipes/pipe-cdc/state",
        ))
        .and(body_json(serde_json::json!({"command": "start"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"name": "cdc-pipe", "state": "Pausing"},
            "status": 200,
            "requestId": "stub-clickpipe-start-pausing",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "start",
            "svc-id",
            "pipe-cdc",
            "--org-id",
            "org",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error envelope");
    assert_eq!(error["error"]["code"], "clickpipe_still_pausing");
    assert_eq!(
        error["error"]["command"],
        "clickhousectl cloud clickpipe get svc-id pipe-cdc --org-id org"
    );
}

/// A state change answered with `Paused`, for `pipe-cdc`.
async fn mount_clickpipe_state_returning_paused(mock: &MockServer, command: &str) {
    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org/services/svc-id/clickpipes/pipe-cdc/state",
        ))
        .and(body_json(serde_json::json!({"command": command})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"name": "cdc-pipe", "state": "Paused"},
            "status": 200,
            "requestId": "stub-clickpipe-state-paused",
        })))
        .expect(1)
        .mount(mock)
        .await;
}

/// A start answered with `Paused`, just after a stop, is dropped too (#1228),
/// so it fails with a code and the start to rerun; no `next` poll on stdout.
#[tokio::test]
async fn clickpipe_start_answered_with_paused_fails_with_the_start_to_rerun() {
    let mock = MockServer::start().await;
    mount_clickpipe_state_returning_paused(&mock, "start").await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "start",
            "svc-id",
            "pipe-cdc",
            "--org-id",
            "org",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error envelope");
    assert_eq!(error["error"]["code"], "clickpipe_start_not_applied");
    assert_eq!(
        error["error"]["command"],
        "clickhousectl cloud clickpipe start svc-id pipe-cdc --org-id org"
    );
}

/// A start answered with `Completed` has no effect (#926): it fails with its
/// own code, no command to retry, and no `next` poll on stdout.
#[tokio::test]
async fn clickpipe_start_answered_with_completed_fails_with_nothing_to_retry() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(
            "/v1/organizations/org/services/svc-id/clickpipes/pipe-s3/state",
        ))
        .and(body_json(serde_json::json!({"command": "start"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"name": "s3-pipe", "state": "Completed"},
            "status": 200,
            "requestId": "stub-clickpipe-start-completed",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["clickpipe", "start", "svc-id", "pipe-s3", "--org-id", "org"],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error envelope");
    assert_eq!(error["error"]["code"], "clickpipe_start_no_effect");
    assert!(error["error"].get("command").is_none());
}

/// A resync answered with `Paused` is applied, so it still succeeds and
/// names the poll; the not-applied check is specific to start.
#[tokio::test]
async fn clickpipe_resync_answered_with_paused_still_succeeds() {
    let mock = MockServer::start().await;
    mount_clickpipe_state_returning_paused(&mock, "resync").await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "resync",
            "svc-id",
            "pipe-cdc",
            "--org-id",
            "org",
        ],
    );

    assert_success(&output);
    assert!(output.stderr.is_empty());
    let (response, next) = take_next(&output.stdout);
    assert_eq!(response["state"], "Paused");
    let next = next.expect("an applied resync names its poll");
    assert_eq!(next["until"], serde_json::json!(["Running", "Completed"]));
}

// ── ClickPipe settings updates are source-aware (#602) ─────────────────────
//
// `kafka_read_committed` is only supported for Kafka pipes: the API fails the
// whole PUT with "Setting 'kafka_read_committed' is only supported for Kafka
// ClickPipes" for any other source. The handler therefore fetches the pipe to
// classify its source, and only reads back and re-sends the Kafka-only setting
// when the source is Kafka.

const CLICKPIPE_PATH: &str = "/v1/organizations/org/services/svc-id/clickpipes/pipe-id";
const CLICKPIPE_SETTINGS_PATH: &str =
    "/v1/organizations/org/services/svc-id/clickpipes/pipe-id/settings";

/// Stub the pipe GET with a source of the given shape, e.g.
/// `json!({ "objectStorage": { "type": "s3" } })`.
async fn mount_clickpipe_get(mock: &MockServer, source: Value) {
    let stub_pipe = serde_json::json!({
        "result": {
            "id": "00000000-0000-0000-0000-0000000000aa",
            "name": "test-pipe",
            "state": "Running",
            "source": source,
        },
        "status": 200,
        "requestId": "stub-clickpipe-get",
    });
    Mock::given(method("GET"))
        .and(path(CLICKPIPE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_pipe))
        .mount(mock)
        .await;
}

#[tokio::test]
async fn clickpipe_get_renders_a_glue_schema_registry_and_keeps_json_verbatim() {
    let source = serde_json::json!({
        "kafka": {
            "type": "msk",
            "format": "Avro",
            "brokers": "broker:9098",
            "topics": "events",
            "authentication": "IAM_ROLE",
            "iamRole": "arn:aws:iam::123456789012:role/Source",
            "schemaRegistry": {
                "type": "glue",
                "glueRegion": "us-east-1",
                "glueRegistryName": "my-registry",
                "glueRoleArn": "arn:aws:iam::123456789012:role/Glue"
            }
        }
    });
    let mock = MockServer::start().await;
    mount_clickpipe_get(&mock, source.clone()).await;
    let args = ["clickpipe", "get", "svc-id", "pipe-id", "--org-id", "org"];

    let output = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&output);
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["source"], source);

    let output = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in [
        "type: glue",
        "glueRegion: us-east-1",
        "glueRegistryName: my-registry",
        "glueRoleArn: arn:aws:iam::123456789012:role/Glue",
    ] {
        assert!(stdout.contains(line), "{line}\n{stdout}");
    }
}

async fn mount_clickpipe_settings_put(mock: &MockServer, result: Value) {
    let updated_settings = serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": "stub-settings-update",
    });
    Mock::given(method("PUT"))
        .and(path(CLICKPIPE_SETTINGS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(updated_settings))
        .mount(mock)
        .await;
}

/// The (method, path) pairs the mock saw, in order.
async fn recorded_request_shape(mock: &MockServer) -> Vec<(String, String)> {
    mock.received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| {
            (
                request.method.as_str().to_string(),
                request.url.path().to_string(),
            )
        })
        .collect()
}

async fn recorded_put_body(mock: &MockServer) -> Value {
    let requests = mock.received_requests().await.unwrap();
    let put = requests
        .iter()
        .find(|request| request.method == wiremock::http::Method::PUT)
        .expect("no settings PUT request recorded by mock");
    serde_json::from_slice::<Value>(&put.body).unwrap()
}

#[tokio::test]
async fn clickpipe_settings_update_omits_kafka_only_settings_for_non_kafka_pipes() {
    for source in [
        serde_json::json!({ "objectStorage": { "type": "s3", "format": "JSONEachRow" } }),
        serde_json::json!({ "kinesis": { "stream": "events" } }),
        // A response that drops `source` entirely is treated as non-Kafka.
        serde_json::json!({}),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, source.clone()).await;
        mount_clickpipe_settings_put(
            &mock,
            serde_json::json!({ "object_storage_max_file_count": 200 }),
        )
        .await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--object-storage-max-file-count",
                "200",
                "--org-id",
                "org",
            ],
        );
        assert_success(&output);
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            serde_json::json!({ "object_storage_max_file_count": 200 })
        );

        // No settings GET: nothing needs reading back when no Kafka-only
        // setting is being re-sent.
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".to_string(), CLICKPIPE_PATH.to_string()),
                ("PUT".to_string(), CLICKPIPE_SETTINGS_PATH.to_string()),
            ],
            "unexpected requests for source {source}"
        );
        assert_eq!(
            recorded_put_body(&mock).await,
            serde_json::json!({ "object_storage_max_file_count": 200 }),
            "kafka-only settings leaked for source {source}"
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_preserves_kafka_read_committed() {
    for (current_settings, expected) in [
        (serde_json::json!({ "kafka_read_committed": true }), true),
        (serde_json::json!({ "kafka_read_committed": false }), false),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(
            &mock,
            serde_json::json!({ "kafka": { "type": "kafka", "brokers": "b:9092" } }),
        )
        .await;
        let current_settings = serde_json::json!({
            "result": current_settings,
            "status": 200,
            "requestId": "stub-settings-get",
        });
        Mock::given(method("GET"))
            .and(path(CLICKPIPE_SETTINGS_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(current_settings))
            .mount(&mock)
            .await;
        mount_clickpipe_settings_put(
            &mock,
            serde_json::json!({
                "streaming_max_insert_wait_ms": 1000,
                "kafka_read_committed": expected,
            }),
        )
        .await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--streaming-max-insert-wait-ms",
                "1000",
                "--org-id",
                "org",
            ],
        );
        assert_success(&output);

        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".to_string(), CLICKPIPE_PATH.to_string()),
                ("GET".to_string(), CLICKPIPE_SETTINGS_PATH.to_string()),
                ("PUT".to_string(), CLICKPIPE_SETTINGS_PATH.to_string()),
            ]
        );
        assert_eq!(
            recorded_put_body(&mock).await,
            serde_json::json!({
                "streaming_max_insert_wait_ms": 1000,
                "kafka_read_committed": expected,
            })
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_can_explicitly_set_kafka_read_committed() {
    for requested in [true, false] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, serde_json::json!({ "kafka": { "type": "kafka" } })).await;
        let expected = serde_json::json!({ "kafka_read_committed": requested });
        mount_clickpipe_settings_put(&mock, expected.clone()).await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--org-id",
                "org",
                "--kafka-read-committed",
                &requested.to_string(),
            ],
        );
        assert_success(&output);
        assert_eq!(recorded_put_body(&mock).await, expected);
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".into(), CLICKPIPE_PATH.into()),
                ("PUT".into(), CLICKPIPE_SETTINGS_PATH.into()),
            ]
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_refuses_kafka_setting_for_other_or_unknown_sources() {
    for source in [
        serde_json::json!({ "objectStorage": { "type": "s3" } }),
        serde_json::json!({ "kinesis": {} }),
        serde_json::json!({ "pubsub": {} }),
        serde_json::json!({}),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, source).await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--org-id",
                "org",
                "--kafka-read-committed",
                "false",
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--kafka-read-committed"), "{stderr}");
        assert!(!stderr.contains("kafka_read_committed"), "{stderr}");
        assert!(output.stdout.is_empty());
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![("GET".into(), CLICKPIPE_PATH.into())]
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_never_guesses_an_absent_kafka_setting() {
    for current in [
        serde_json::json!({}),
        serde_json::json!({"kafka_read_committed": null}),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, serde_json::json!({ "kafka": {} })).await;
        Mock::given(method("GET"))
            .and(path(CLICKPIPE_SETTINGS_PATH))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": current})),
            )
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "update",
                "svc-id",
                "pipe-id",
                "--org-id",
                "org",
                "--streaming-max-insert-wait-ms",
                "1000",
            ],
        );
        // The omitted settings field, and the flag that supplies it.
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("kafka_read_committed"), "{stderr}");
        assert!(stderr.contains("--kafka-read-committed"), "{stderr}");
        assert!(output.stdout.is_empty());
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".into(), CLICKPIPE_PATH.into()),
                ("GET".into(), CLICKPIPE_SETTINGS_PATH.into()),
            ]
        );
    }
}

#[tokio::test]
async fn clickpipe_settings_update_stops_when_the_preservation_read_fails() {
    let mock = MockServer::start().await;
    mount_clickpipe_get(&mock, serde_json::json!({ "kafka": {} })).await;
    Mock::given(method("GET"))
        .and(path(CLICKPIPE_SETTINGS_PATH))
        .respond_with(
            ResponseTemplate::new(503).set_body_json(serde_json::json!({"error": "unavailable"})),
        )
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "settings",
            "update",
            "svc-id",
            "pipe-id",
            "--org-id",
            "org",
            "--streaming-max-insert-wait-ms",
            "1000",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        recorded_request_shape(&mock).await,
        vec![
            ("GET".into(), CLICKPIPE_PATH.into()),
            ("GET".into(), CLICKPIPE_SETTINGS_PATH.into()),
        ]
    );
}

// ── ClickPipe ingestion settings apply to some pipe types only (#643) ──────
//
// The ingestion settings endpoints exist for streaming and object-storage pipes
// only. For a database CDC pipe the API answers `NOT_FOUND: ingestion for pipe
// "<id>" not found`, which reads as "the pipe is gone". Both settings commands
// therefore classify the pipe's source first and refuse before touching the
// settings endpoint.

/// The settings GET the CLI must not reach for a database CDC pipe. Mounted so
/// the assertion is "the CLI never called it", not "the mock had no route".
async fn mount_clickpipe_settings_get(mock: &MockServer) {
    let settings = serde_json::json!({
        "result": { "streaming_max_insert_wait_ms": 1000 },
        "status": 200,
        "requestId": "stub-settings-get",
    });
    Mock::given(method("GET"))
        .and(path(CLICKPIPE_SETTINGS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(settings))
        .mount(mock)
        .await;
}

#[tokio::test]
async fn clickpipe_settings_get_refuses_database_pipes_before_calling_the_endpoint() {
    // The label for each database source is unit-tested; one source is enough.
    let mock = MockServer::start().await;
    mount_clickpipe_get(
        &mock,
        serde_json::json!({ "postgres": { "host": "db.example.com" } }),
    )
    .await;
    mount_clickpipe_settings_get(&mock).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "settings",
            "get",
            "svc-id",
            "pipe-id",
            "--org-id",
            "org",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    let error = cloud_runtime_error(&output);
    assert_eq!(error["code"], "other");
    // Names the pipe and its source, and points at the command that shows
    // a CDC pipe's settings.
    let message = error["message"].as_str().unwrap();
    for identifier in [
        "pipe-id",
        "Postgres",
        "`clickhousectl cloud clickpipe get svc-id pipe-id`",
    ] {
        assert!(message.contains(identifier), "{identifier}: {message}");
    }
    assert!(output.stdout.is_empty(), "a refusal must print no settings");
    // Only the pipe read: the settings endpoint is never called.
    assert_eq!(
        recorded_request_shape(&mock).await,
        vec![("GET".to_string(), CLICKPIPE_PATH.to_string())],
    );
}

#[tokio::test]
async fn clickpipe_settings_update_refuses_database_pipes_before_calling_the_endpoint() {
    let mock = MockServer::start().await;
    mount_clickpipe_get(
        &mock,
        serde_json::json!({ "postgres": { "host": "db.example.com" } }),
    )
    .await;
    mount_clickpipe_settings_get(&mock).await;
    mount_clickpipe_settings_put(&mock, serde_json::json!({})).await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "settings",
            "update",
            "svc-id",
            "pipe-id",
            "--streaming-max-insert-wait-ms",
            "1000",
            "--org-id",
            "org",
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("pipe-id"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        recorded_request_shape(&mock).await,
        vec![("GET".to_string(), CLICKPIPE_PATH.to_string())],
    );
}

#[tokio::test]
async fn clickpipe_settings_get_reads_settings_for_pipes_that_have_them() {
    for source in [
        serde_json::json!({ "kafka": { "type": "kafka", "brokers": "b:9092" } }),
        serde_json::json!({ "kinesis": { "stream": "events" } }),
        serde_json::json!({ "objectStorage": { "type": "s3", "format": "JSONEachRow" } }),
        // An unclassifiable pipe proceeds: the API stays the authority.
        serde_json::json!({}),
    ] {
        let mock = MockServer::start().await;
        mount_clickpipe_get(&mock, source.clone()).await;
        mount_clickpipe_settings_get(&mock).await;

        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickpipe",
                "settings",
                "get",
                "svc-id",
                "pipe-id",
                "--org-id",
                "org",
            ],
        );
        assert_success(&output);

        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout).unwrap(),
            serde_json::json!({ "streaming_max_insert_wait_ms": 1000 }),
            "unexpected output for source {source}"
        );
        assert_eq!(
            recorded_request_shape(&mock).await,
            vec![
                ("GET".to_string(), CLICKPIPE_PATH.to_string()),
                ("GET".to_string(), CLICKPIPE_SETTINGS_PATH.to_string()),
            ],
            "unexpected requests for source {source}"
        );
    }
}

// ── ClickPipe schema discovery (#289, beta) ────────────────────────────────
//
// `clickpipe schema-discover` POSTs to .../clickpipes/schemaDiscovery with a
// `source` built by the same source builders as `clickpipe create`, which the
// unit tests cover; this wiring test asserts the organization/service path.

/// Start a wiremock server that accepts a schema-discovery POST and records
/// the request body. Returns inferred fields the CLI renders.
async fn start_mock_schema_discovery_api() -> MockServer {
    let mock = MockServer::start().await;
    let stub_response = serde_json::json!({
        "result": {
            "fields": [
                { "name": "id", "type": "Int64", "optional": false },
                { "name": "event", "type": "String", "optional": true },
            ],
        },
        "status": 200,
        "requestId": "stub-schema-discovery",
    });
    Mock::given(method("POST"))
        .and(path(
            "/v1/organizations/org/services/svc-id/clickpipes/schemaDiscovery",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_response))
        .expect(1)
        .mount(&mock)
        .await;
    mock
}

#[tokio::test]
async fn schema_discover_kinesis_posts_source_body() {
    let mock = start_mock_schema_discovery_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "schema-discover",
            "--org-id",
            "org",
            "kinesis",
            "svc-id",
            "--stream-name",
            "mystream",
            "--region",
            "us-east-1",
            "--format",
            "JSONEachRow",
        ],
    )
    .await;
    let kinesis = &body["source"]["kinesis"];
    assert_eq!(kinesis["streamName"], "mystream");
    assert_eq!(kinesis["region"], "us-east-1");
    assert_eq!(kinesis["format"], "JSONEachRow");
    // Kafka is absent for a Kinesis discovery request.
    assert!(
        body["source"].get("kafka").is_none(),
        "kafka leaked into kinesis schema-discovery body: {}",
        body["source"],
    );
}

/// The Kafka arm assembles its discovery request in the handler, so this is
/// the only check that it sends the shared source without create's
/// `exactlyOnce`.
#[tokio::test]
async fn schema_discover_kafka_posts_the_source_without_create_only_fields() {
    let mock = start_mock_schema_discovery_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "schema-discover",
            "--org-id",
            "org",
            "kafka",
            "svc-id",
            "--brokers",
            "broker:9092",
            "--topics",
            "topic",
            "--format",
            "JSONEachRow",
            "--iam-role",
            "arn:role",
        ],
    )
    .await;
    assert_eq!(
        body,
        serde_json::json!({"source": {"kafka": {
            "authentication": "IAM_ROLE",
            "brokers": "broker:9092",
            "credentials": null,
            "iamRole": "arn:role",
            "format": "JSONEachRow",
            "offset": {"strategy": "from_beginning"},
            "reversePrivateEndpointIds": [],
            "topics": "topic",
            "type": "kafka",
        }}})
    );
}

// ── Google Cloud Pub/Sub source (issue #587) ───────────────────────────────
//
// `clickpipe create pubsub` and `clickpipe schema-discover pubsub <SERVICE_ID>`
// build the same `source.pubsub` object. The service account key is read from a
// file (or from stdin for `-`) and sent base64-encoded under
// `serviceAccountKey.serviceAccountFile`, so the path never goes on the wire —
// and neither the key nor its encoding may reach output or an error message.

const PUBSUB_SERVICE_ACCOUNT_KEY: &str =
    r#"{"type":"service_account","private_key":"FAKE_PRIVATE_KEY"}"#;

fn encoded_service_account_key() -> String {
    base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        PUBSUB_SERVICE_ACCOUNT_KEY.as_bytes(),
    )
}

fn write_service_account_key(dir: &Path) -> PathBuf {
    let path = dir.join("sa-key.json");
    std::fs::File::create(&path)
        .expect("create service account key file")
        .write_all(PUBSUB_SERVICE_ACCOUNT_KEY.as_bytes())
        .expect("write service account key file");
    path
}

/// Minimal `clickpipe create pubsub` invocation reading the key from
/// `service_account_file`.
fn pubsub_create_args(service_account_file: &str) -> Vec<String> {
    [
        "clickpipe",
        "create",
        "pubsub",
        "svc-id",
        "--name",
        "pubsub-pipe",
        "--topic",
        "events",
        "--project-id",
        "my-gcp-project",
        "--format",
        "JSONEachRow",
        "--seek-type",
        "earliest",
        "--service-account-file",
        service_account_file,
        "--database",
        "default",
        "--table",
        "events",
        "--column",
        "event_id:Int64",
        "--org-id",
        "org",
    ]
    .iter()
    .map(|arg| arg.to_string())
    .collect()
}

fn as_str_args(args: &[String]) -> Vec<&str> {
    args.iter().map(|arg| arg.as_str()).collect()
}

/// Spawn the binary with a piped stdin, write `stdin_data`, and return the
/// finished output. Used for `--service-account-file -`.
fn invoke_cli_with_piped_stdin(
    mock: &MockServer,
    cli_args: &[String],
    stdin_data: &str,
) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir(&home).unwrap();
    let mut args = vec![
        "cloud".to_string(),
        "--url".to_string(),
        mock.uri(),
        "--json".to_string(),
    ];
    args.extend_from_slice(cli_args);
    let mut child = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", &home)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(dir.path())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clickhousectl");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(stdin_data.as_bytes())
        .expect("write service account key to stdin");
    child
        .wait_with_output()
        .expect("failed to wait for clickhousectl")
}

/// The JSON body of the first POST the mock recorded.
async fn recorded_post_body(mock: &MockServer) -> Value {
    let requests = mock
        .received_requests()
        .await
        .expect("mock requests log unavailable");
    let post = requests
        .iter()
        .find(|request| request.method == wiremock::http::Method::POST)
        .expect("no POST request recorded by mock");
    serde_json::from_slice(&post.body).expect("POST body wasn't valid JSON")
}

#[tokio::test]
async fn pubsub_service_account_key_can_be_read_from_stdin() {
    let mock = start_mock_clickpipes_api().await;
    let args = pubsub_create_args("-");

    let output = invoke_cli_with_piped_stdin(&mock, &args, PUBSUB_SERVICE_ACCOUNT_KEY);
    assert_success(&output);

    let body = recorded_post_body(&mock).await;
    assert_eq!(
        body["source"]["pubsub"]["serviceAccountKey"]["serviceAccountFile"],
        Value::String(encoded_service_account_key()),
    );
}

#[tokio::test]
async fn pubsub_service_account_key_never_reaches_output_on_an_api_error() {
    let mock = MockServer::start().await;
    // A rejected create: the API error body is surfaced to the user, so this is
    // where an echoed credential would show up.
    Mock::given(method("POST"))
        .and(path_regex(
            r"^/v1/organizations/[^/]+/services/[^/]+/clickpipes$",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "pubsub source is not enabled for this organization",
            "status": 400,
            "requestId": "stub-request-id",
        })))
        .mount(&mock)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let key_path = write_service_account_key(dir.path());
    let args = pubsub_create_args(key_path.to_str().expect("utf-8 temp path"));

    let output = invoke_cli_with_cloud_credentials(&mock, &as_str_args(&args));

    assert_eq!(
        output.status.code(),
        Some(1),
        "a rejected create must fail: {}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains("not enabled for this organization"),
        "the API error should still be reported: {stderr}",
    );
    for secret in [
        PUBSUB_SERVICE_ACCOUNT_KEY,
        "FAKE_PRIVATE_KEY",
        &encoded_service_account_key(),
    ] {
        assert!(
            !stderr.contains(secret),
            "the service account key leaked into stderr: {stderr}",
        );
        assert!(
            !stdout.contains(secret),
            "the service account key leaked into stdout: {stdout}",
        );
    }
}

// ── ClickPipes GCP workload identity (issue #690) ──────────────────────────

async fn mount_clickpipe_context(mock: &MockServer, result: Value) {
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org/services/svc-id/clickpipes/context",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-clickpipes-context",
        })))
        .expect(1)
        .mount(mock)
        .await;
}

#[tokio::test]
async fn clickpipe_context_get_renders_json_and_human_output() {
    let mock = MockServer::start().await;
    mount_clickpipe_context(
        &mock,
        serde_json::json!({
            "gcpWorkloadIdentity": {
                "supported": true,
                "ready": true,
                "principal": "clickpipes@project.iam.gserviceaccount.com",
            }
        }),
    )
    .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &["clickpipe", "context", "get", "svc-id", "--org-id", "org"],
    );
    assert_success(&output);
    let output_json: Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    assert_eq!(
        output_json["gcpWorkloadIdentity"]["principal"],
        "clickpipes@project.iam.gserviceaccount.com"
    );

    let human = MockServer::start().await;
    mount_clickpipe_context(
        &human,
        serde_json::json!({
            "gcpWorkloadIdentity": {
                "supported": true,
                "ready": true,
                "principal": "clickpipes@project.iam.gserviceaccount.com",
            }
        }),
    )
    .await;
    let output = invoke_cli_human(
        &human,
        &["clickpipe", "context", "get", "svc-id", "--org-id", "org"],
    );
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "gcpWorkloadIdentity:\n  supported: true\n  ready: true\n  principal: clickpipes@project.iam.gserviceaccount.com\n"
    );
}

// ── `clickpipe scale` (issue #605) ─────────────────────────────────────────
//
// The at-least-one-target rule and the value ranges are clap rules with their
// own unit tests. The handler builds the PATCH body itself, so these tests pin
// that body on the wire.

#[tokio::test]
async fn clickpipe_scale_sends_exactly_the_requested_targets() {
    for (flags, body) in [
        // An omitted target stays off the wire.
        (vec!["--replicas", "4"], serde_json::json!({"replicas": 4})),
        (
            vec![
                "--replicas",
                "1",
                "--cpu-millicores",
                "125",
                "--memory-gb",
                "0.5",
            ],
            serde_json::json!({
                "replicas": 1, "replicaCpuMillicores": 125, "replicaMemoryGb": 0.5
            }),
        ),
        (
            vec![
                "--replicas",
                "40",
                "--cpu-millicores",
                "2000",
                "--memory-gb",
                "8",
            ],
            serde_json::json!({
                "replicas": 40, "replicaCpuMillicores": 2000, "replicaMemoryGb": 8.0
            }),
        ),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(
                "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1/scaling",
            ))
            .and(body_json(&body))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": {"id": "11111111-2222-3333-4444-555555555555", "scaling": body},
                "status": 200,
                "requestId": "stub-clickpipe-scale",
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let mut args = vec!["clickpipe", "scale", "svc-1", "pipe-1", "--org-id", "org-1"];
        args.extend(flags);
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_success(&output);
    }
}

// ── service-wide ClickPipes CDC scaling (issue #586) ───────────────────────

#[tokio::test]
async fn clickpipe_cdc_scaling_get_supports_oauth_and_preserves_sparse_json() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(CDC_SCALING_PATH))
        .and(header("authorization", "Bearer test-bearer-token"))
        .respond_with(cdc_scaling_envelope(serde_json::json!({
            "replicaCpuMillicores": 2000,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let project = tempfile::tempdir().unwrap();
    let home = project.path().join("home");
    let cloud_dir = home.join(".clickhouse");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    write_oauth_tokens(&cloud_dir, &mock.uri());
    let mut command = Command::new(clickhousectl_binary());
    command.isolated_env();
    let output = command
        .env("DO_NOT_TRACK", "1")
        .env("HOME", home)
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "clickpipe",
            "cdc-scaling",
            "get",
            "svc-1",
            "--org-id",
            "org-1",
        ])
        .output()
        .expect("failed to run CDC scaling get with OAuth");

    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({ "replicaCpuMillicores": 2000 })
    );
}

#[tokio::test]
async fn clickpipe_cdc_scaling_update_patches_the_built_body() {
    let mock = MockServer::start().await;
    let scaling = serde_json::json!({ "replicaCpuMillicores": 1000, "replicaMemoryGb": 4.0 });
    Mock::given(method("PATCH"))
        .and(path(CDC_SCALING_PATH))
        .and(body_json(scaling.clone()))
        .respond_with(cdc_scaling_envelope(scaling.clone()))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "cdc-scaling",
            "update",
            "svc-1",
            "--cpu-millicores",
            "1000",
            "--memory-gb",
            "4",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let (response, next) = take_next(&output.stdout);
    assert_eq!(response, scaling);
    let next = next.expect("a scaling change names its poll");
    assert_eq!(
        next["command"],
        "clickhousectl cloud clickpipe cdc-scaling get svc-1 --org-id org-1 --json"
    );
    assert_eq!(next["field"], "replicaCpuMillicores");
    assert_eq!(next["until"], serde_json::json!([1000]));
}

// ── `clickpipe reverse-private-endpoint` CRUD (issue #567) ─────────────────
//
// PrivateLink connectivity for ClickPipes is a reverse private endpoint the
// user creates on the service and then references from a pipe: by ID for
// Kafka, or by DNS name as `--host` for Postgres/MySQL CDC. These tests pin
// the five requests and what each handler prints; the per-type bodies and
// the type/flag validation are unit tests in `src/cloud/clickpipe_endpoints.rs`
// and `main.rs`.

const RPE_COLLECTION_PATH: &str =
    "/v1/organizations/org-1/services/svc-1/clickpipesReversePrivateEndpoints";
const RPE_ID: &str = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

fn reverse_private_endpoint_path() -> String {
    format!("{RPE_COLLECTION_PATH}/{RPE_ID}")
}

/// One endpoint as the API returns it, with the fields the CLI renders.
fn reverse_private_endpoint_json() -> serde_json::Value {
    serde_json::json!({
        "id": RPE_ID,
        "serviceId": "11111111-2222-3333-4444-555555555555",
        "type": "VPC_ENDPOINT_SERVICE",
        "description": "warehouse",
        "status": "PendingAcceptance",
        "endpointId": "vpce-12345678901234567",
        "dnsNames": ["vpce-1-abc.vpce.amazonaws.com"],
        "vpcEndpointServiceName": "com.amazonaws.vpce.us-east-1.vpce-svc-1",
    })
}

fn reverse_private_endpoint_envelope(result: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": result,
        "status": 200,
        "requestId": "stub-reverse-private-endpoint",
    }))
}

#[tokio::test]
async fn reverse_private_endpoint_list_returns_the_resource_array() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(RPE_COLLECTION_PATH))
        .respond_with(reverse_private_endpoint_envelope(serde_json::json!([
            reverse_private_endpoint_json()
        ])))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "list",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
        .expect("stdout should be the resource array as JSON");
    // The resource array itself, not the API envelope around it.
    assert_eq!(stdout, serde_json::json!([reverse_private_endpoint_json()]));
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("GET".to_string(), RPE_COLLECTION_PATH.to_string())]
    );
}

/// Every response field is `Option`, so a row with nothing but an ID must
/// render placeholders rather than fabricate values.
#[tokio::test]
async fn reverse_private_endpoint_list_table_renders_absent_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(RPE_COLLECTION_PATH))
        .respond_with(reverse_private_endpoint_envelope(serde_json::json!([
            { "id": RPE_ID }
        ])))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_human(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "list",
            "svc-1",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("| ID"), "{stdout}");
    assert!(stdout.contains("DNS Names"), "{stdout}");
    assert!(stdout.contains(RPE_ID), "{stdout}");
    // Type, Description, Status, DNS Names and DNS Targets are all absent here.
    assert_eq!(stdout.matches(" - ").count(), 5, "{stdout}");
}

#[tokio::test]
async fn reverse_private_endpoint_get_reads_the_single_endpoint() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(reverse_private_endpoint_path()))
        .respond_with(reverse_private_endpoint_envelope(
            reverse_private_endpoint_json(),
        ))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "get",
            "svc-1",
            RPE_ID,
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
        .expect("stdout should be the resource object as JSON");
    assert_eq!(stdout["endpointId"], "vpce-12345678901234567");
    assert_eq!(stdout["dnsNames"][0], "vpce-1-abc.vpce.amazonaws.com");
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("GET".to_string(), reverse_private_endpoint_path())]
    );
}

/// The create handler posts the builder's body to the service's collection.
/// Each type's field mapping is a `build_create_reverse_private_endpoint_request`
/// unit test.
#[tokio::test]
async fn reverse_private_endpoint_create_posts_the_built_body() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(RPE_COLLECTION_PATH))
        .respond_with(reverse_private_endpoint_envelope(
            reverse_private_endpoint_json(),
        ))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "create",
            "svc-1",
            "--description",
            "warehouse",
            "--type",
            "VPC_ENDPOINT_SERVICE",
            "--vpc-endpoint-service-name",
            "com.amazonaws.vpce.us-east-1.vpce-svc-1",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("POST".to_string(), RPE_COLLECTION_PATH.to_string())],
    );
    let (_, next) = take_next(&output.stdout);
    let next = next.expect("a created endpoint names its poll");
    assert_eq!(
        next["command"],
        format!(
            "clickhousectl cloud clickpipe reverse-private-endpoint get svc-1 {RPE_ID} --org-id \
             org-1 --json"
        )
    );
    assert_eq!(next["until"], serde_json::json!(["Ready"]));
    let requests = mock.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "description": "warehouse",
            "type": "VPC_ENDPOINT_SERVICE",
            "vpcEndpointServiceName": "com.amazonaws.vpce.us-east-1.vpce-svc-1",
        })
    );
}

#[tokio::test]
async fn reverse_private_endpoint_create_prints_the_status_to_wait_for() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(RPE_COLLECTION_PATH))
        .respond_with(reverse_private_endpoint_envelope(
            reverse_private_endpoint_json(),
        ))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_human(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "create",
            "svc-1",
            "--description",
            "warehouse",
            "--type",
            "VPC_ENDPOINT_SERVICE",
            "--vpc-endpoint-service-name",
            "com.amazonaws.vpce.us-east-1.vpce-svc-1",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(RPE_ID), "{stdout}");
    assert!(stdout.contains("Status: PendingAcceptance"), "{stdout}");
    assert!(
        stdout.contains("DNS names: vpce-1-abc.vpce.amazonaws.com"),
        "{stdout}"
    );
}

#[tokio::test]
async fn reverse_private_endpoint_update_patches_the_full_mapping_list() {
    let mock = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(reverse_private_endpoint_path()))
        .respond_with(reverse_private_endpoint_envelope(
            reverse_private_endpoint_json(),
        ))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "reverse-private-endpoint",
            "update",
            "svc-1",
            RPE_ID,
            "--custom-private-dns-mapping",
            "db.example.com",
            "--custom-private-dns-mapping",
            "*.example.com=rcfg-2",
            "--org-id",
            "org-1",
        ],
    );

    assert_success(&output);
    assert_eq!(
        received_request_shape(&mock).await,
        vec![("PATCH".to_string(), reverse_private_endpoint_path())]
    );
    let requests = mock.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    // A mapping without `=TARGET_ID` leaves `targetId` off the wire.
    assert_eq!(
        body,
        serde_json::json!({
            "customPrivateDnsMappings": [
                { "privateDnsName": "db.example.com" },
                { "privateDnsName": "*.example.com", "targetId": "rcfg-2" },
            ],
        })
    );
}

#[tokio::test]
async fn reverse_private_endpoint_delete_confirms_the_removal() {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(reverse_private_endpoint_path()))
        .respond_with(successful_delete_response("stub-rpe-delete"))
        .expect(2)
        .mount(&mock)
        .await;

    let args = [
        "clickpipe",
        "reverse-private-endpoint",
        "delete",
        "svc-1",
        RPE_ID,
        "--org-id",
        "org-1",
    ];

    let human = invoke_cli_human(&mock, &args);
    assert_success(&human);
    let stdout = String::from_utf8_lossy(&human.stdout);
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(stdout.contains(RPE_ID), "{stdout}");

    let json = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&json);
    let stdout: Value = serde_json::from_str(String::from_utf8_lossy(&json.stdout).trim())
        .expect("stdout should be JSON");
    assert_eq!(stdout, serde_json::json!({ "deleted": RPE_ID }));

    assert_eq!(
        received_request_shape(&mock).await,
        vec![
            ("DELETE".to_string(), reverse_private_endpoint_path()),
            ("DELETE".to_string(), reverse_private_endpoint_path()),
        ]
    );
}

// ── Complete ClickPipe PATCH input (#569) ──────────────────────────────────

const CLICKPIPE_UPDATE_PATH: &str = "/v1/organizations/org-1/services/svc-1/clickpipes/pipe-1";

async fn mount_clickpipe_update(mock: &MockServer, patch: Value) {
    Mock::given(method("PATCH"))
        .and(path(CLICKPIPE_UPDATE_PATH))
        .and(body_json(patch))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "id": "00000000-0000-0000-0000-000000000001",
                "name": "updated"
            },
            "status": 200,
            "requestId": "stub-clickpipe-update"
        })))
        .expect(1)
        .mount(mock)
        .await;
}

#[tokio::test]
async fn clickpipe_update_reads_stdin_and_preserves_explicit_empty_values() {
    let mock = MockServer::start().await;
    let patch = serde_json::json!({"fieldMappings": []});
    mount_clickpipe_update(&mock, patch.clone()).await;
    let output = invoke_cli_with_cloud_credentials_and_stdin(
        &mock,
        &[
            "clickpipe",
            "update",
            "svc-1",
            "pipe-1",
            "--file",
            "-",
            "--org-id",
            "org-1",
        ],
        &patch.to_string(),
    );
    assert_success(&output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "name": "updated"
        })
    );
}

#[tokio::test]
async fn clickpipe_list_renders_source_destination_table_and_preserves_json() {
    let mock = MockServer::start().await;
    let sources = [
        ("objectStorage", "Object storage"),
        ("kafka", "Kafka"),
        ("kinesis", "Kinesis"),
        ("pubsub", "Pub/Sub"),
        ("postgres", "Postgres"),
        ("mysql", "MySQL"),
        ("mongodb", "MongoDB"),
        ("bigquery", "BigQuery"),
    ];
    let mut pipes: Vec<Value> = sources
        .iter()
        .enumerate()
        .map(|(index, (source, _))| {
            serde_json::json!({
                "id": format!("00000000-0000-0000-0000-{index:012}"),
                "name": format!("pipe-{index}"),
                "source": { *source: {} },
                "destination": { "database": "analytics", "table": format!("events_{index}") },
                "state": "Running"
            })
        })
        .collect();
    pipes.extend([
        serde_json::json!({}),
        serde_json::json!({"source": {}, "destination": {}}),
        serde_json::json!({"destination": {"database": "database_only"}, "state": "FutureState"}),
        serde_json::json!({"destination": {"table": "table_only"}}),
    ]);
    let result = serde_json::json!(pipes);
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/services/svc-1/clickpipes"))
        .and(header(
            "authorization",
            "Basic ZmFrZS1rZXktZm9yLXRlc3RzOmZha2Utc2VjcmV0LWZvci10ZXN0cw==",
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": result})),
        )
        .expect(2)
        .mount(&mock)
        .await;
    let args = ["clickpipe", "list", "svc-1", "--org-id", "org-1"];
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    let stdout = String::from_utf8(human.stdout).unwrap();
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
        .collect();
    assert_eq!(rows.len(), pipes.len() + 2, "{stdout}");
    assert_eq!(rows[0], ["Name", "ID", "Source", "Destination", "State"]);
    for (index, (_, label)) in sources.iter().enumerate() {
        assert_eq!(
            rows[index + 2],
            [
                format!("pipe-{index}"),
                format!("00000000-0000-0000-0000-{index:012}"),
                label.to_string(),
                format!("analytics.events_{index}"),
                "Running".to_string(),
            ],
            "{stdout}"
        );
    }
    assert_eq!(rows[10], ["-", "-", "-", "-", "-"]);
    assert_eq!(rows[11], ["-", "-", "-", "-", "-"]);
    assert_eq!(rows[12], ["-", "-", "-", "database_only", "FutureState"]);
    assert_eq!(rows[13], ["-", "-", "-", "table_only", "-"]);

    let explicit = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&explicit);
    assert_eq!(
        serde_json::from_slice::<Value>(&explicit.stdout).unwrap(),
        result
    );
    for request in mock.received_requests().await.unwrap() {
        assert_eq!(request.url.query(), None);
        assert!(request.body.is_empty());
    }
}

#[tokio::test]
async fn clickpipe_list_and_name_lookup_read_every_page() {
    // The first request carries no `limit` or `cursor`; a `nextCursor` in any
    // response is followed until absent. Used by `list` and by `--name`.
    const CLICKPIPES_PATH: &str = "/v1/organizations/org-1/services/svc-1/clickpipes";
    const SECOND_ID: &str = "00000000-0000-0000-0000-0000000000c2";
    let mock = MockServer::start().await;
    let first = serde_json::json!({"id": "00000000-0000-0000-0000-0000000000c1", "name": "first"});
    let second = serde_json::json!({"id": SECOND_ID, "name": "target"});
    mount_cursor_page(
        &mock,
        CLICKPIPES_PATH,
        None,
        serde_json::json!({"result": [first], "nextCursor": "pipe page /+?", "limit": 1}),
        3,
    )
    .await;
    mount_cursor_page(
        &mock,
        CLICKPIPES_PATH,
        Some("pipe page /+?"),
        serde_json::json!({"result": [second], "nextCursor": null}),
        3,
    )
    .await;
    Mock::given(method("GET"))
        .and(path(format!("{CLICKPIPES_PATH}/{SECOND_ID}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": second})),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let args = ["clickpipe", "list", "svc-1", "--org-id", "org-1"];
    let json = invoke_cli_with_cloud_credentials(&mock, &args);
    assert_success(&json);
    assert_eq!(
        serde_json::from_slice::<Value>(&json.stdout).unwrap(),
        serde_json::json!([first, second])
    );
    let human = invoke_cli_with_cloud_credentials_human(&mock, &args);
    assert_success(&human);
    let stdout = String::from_utf8(human.stdout).unwrap();
    for id in ["00000000-0000-0000-0000-0000000000c1", SECOND_ID] {
        assert!(stdout.contains(id), "{stdout}");
    }

    let lookup = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickpipe",
            "get",
            "svc-1",
            "--name",
            "target",
            "--org-id",
            "org-1",
        ],
    );
    assert_success(&lookup);
    assert_eq!(
        serde_json::from_slice::<Value>(&lookup.stdout).unwrap(),
        second
    );
}

async fn invoke_clickpipe_delete(result: Value, json: bool) -> std::process::Output {
    let mock = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(CLICKPIPE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": result,
            "status": 200,
            "requestId": "stub-clickpipe-delete",
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let args = [
        "clickpipe",
        "delete",
        "svc-id",
        "pipe-id",
        "--org-id",
        "org",
    ];
    let output = if json {
        invoke_cli_with_cloud_credentials(&mock, &args)
    } else {
        invoke_cli_human(&mock, &args)
    };
    assert_success(&output);
    assert_eq!(
        recorded_request_shape(&mock).await,
        [("DELETE".to_string(), CLICKPIPE_PATH.to_string())],
    );
    output
}

#[tokio::test]
async fn clickpipe_delete_prints_the_returned_name_or_the_requested_id() {
    for (result, json, expected) in [
        (
            serde_json::json!({"name": "events-pipe"}),
            false,
            "ClickPipe events-pipe deleted\n",
        ),
        (serde_json::json!({}), false, "ClickPipe pipe-id deleted\n"),
        (
            serde_json::json!({"name": ""}),
            false,
            "ClickPipe pipe-id deleted\n",
        ),
        (
            serde_json::json!({"name": "events-pipe"}),
            true,
            "{\"deleted\":\"pipe-id\"}\n",
        ),
    ] {
        let output = invoke_clickpipe_delete(result.clone(), json).await;
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            expected,
            "{result} json={json}"
        );
    }
}

fn kinesis_protobuf_args<'a>(
    operation: &'a str,
    format: &'a str,
    schema: Option<&'a str>,
) -> Vec<&'a str> {
    let mut args = kinesis_auth_args(operation, &[]);
    let format_index = args.iter().position(|arg| *arg == "--format").unwrap() + 1;
    args[format_index] = format;
    if let Some(schema) = schema {
        args.extend(["--protobuf-schema-file", schema]);
    }
    args
}

#[tokio::test]
async fn kinesis_protobuf_empty_stdin_is_a_usage_error_and_missing_file_stays_io() {
    // Create and schema discovery share the source builder, so one operation
    // covers both. Empty stdin is a usage error that keeps clap's text even in
    // JSON mode; a missing file is an I/O failure with the JSON `io` code.
    let mock = MockServer::start().await;
    let args = kinesis_protobuf_args("create", "Protobuf", Some("-"));
    for json in [false, true] {
        let project = tempfile::tempdir().unwrap();
        let mut command = Command::new(clickhousectl_binary());
        command
            .isolated_env()
            .env("DO_NOT_TRACK", "1")
            .env("HOME", project.path())
            .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
            .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
            .current_dir(project.path())
            .args(["cloud", "--url", &mock.uri()])
            .args(&args)
            .stdin(Stdio::null());
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(2), "json={json}: {output:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Protobuf") && stderr.contains("stdin"),
            "{stderr}"
        );
        assert!(serde_json::from_slice::<Value>(&output.stderr).is_err());
    }
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &kinesis_protobuf_args("create", "Protobuf", Some("/missing/schema.proto")),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(cloud_runtime_error(&output)["code"], "io");
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn kinesis_protobuf_schema_can_be_read_from_stdin() {
    let raw = "syntax = \"proto3\"; message Event {}";
    let mock = start_mock_clickpipes_api().await;
    let args = [
        "clickpipe",
        "create",
        "kinesis",
        "svc-id",
        "--org-id",
        "org",
        "--stream-name",
        "events",
        "--region",
        "us-east-1",
        "--format",
        "Protobuf",
        "--protobuf-schema-file",
        "-",
        "--name",
        "pipe",
        "--database",
        "default",
        "--table",
        "events",
        "--column",
        "id:Int64",
    ];
    let body = invoke_cli_capture_body_with_stdin(&mock, &args, raw.as_bytes()).await;
    assert_eq!(
        body["source"]["kinesis"]["protobufSchema"],
        "c3ludGF4ID0gInByb3RvMyI7IG1lc3NhZ2UgRXZlbnQge30="
    );
}
