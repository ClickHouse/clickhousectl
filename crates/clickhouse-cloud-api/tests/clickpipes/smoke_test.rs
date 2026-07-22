//! ClickPipe create smoke tests.
//!
//! For each source variant we build a request the way the CLI would build it
//! when invoked with reasonable flags, and assert the create call doesn't get a
//! request-shape 400 from the live API. Credentials are fake-but-syntactically
//! valid — the pipe will fail to actually run, but the create endpoint should
//! still accept the request shape.
//!
//! These tests share a single pre-provisioned ClickHouse Cloud service so they
//! don't pay the multi-minute service-create cost per test. The service ID is
//! read from `CLICKHOUSE_CLOUD_TEST_CLICKPIPE_SERVICE_ID`. Provision it once
//! in the CI workflow (or locally) before running the suite.

#[path = "../common/mod.rs"]
mod common;
mod support;

use std::env;

use clickhouse_cloud_api::models::*;
use clickhouse_cloud_api::{Client, Error};
use support::*;

const SERVICE_ID_ENV: &str = "CLICKHOUSE_CLOUD_TEST_CLICKPIPE_SERVICE_ID";

struct SmokeCtx {
    client: Client,
    org_id: String,
    service_id: String,
    run_id: String,
    delete_timeout: std::time::Duration,
    poll_interval: std::time::Duration,
}

impl SmokeCtx {
    fn from_env() -> TestResult<Self> {
        let ctx = TestContext::from_env()?;
        let service_id = env::var(SERVICE_ID_ENV).map_err(|_| {
            format!("{SERVICE_ID_ENV} must be set to a pre-provisioned ClickHouse Cloud service ID")
        })?;
        Ok(Self {
            client: create_client()?,
            org_id: ctx.org_id,
            service_id,
            run_id: ctx.run_id,
            delete_timeout: ctx.delete_timeout,
            poll_interval: ctx.poll_interval,
        })
    }

    fn pipe_name(&self, source: &str) -> String {
        // Keep names short — API caps ClickPipe name length.
        let suffix: String = self
            .run_id
            .chars()
            .rev()
            .take(8)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("smoke-{source}-{suffix}")
    }
}

/// Phrases the live API uses for runtime/connectivity validation that happens
/// *after* the request shape is accepted. A 400 carrying any of these means
/// the JSON body was structurally valid and the API got far enough to try the
/// source — exactly what these smoke tests want to confirm. Anything else in a
/// 400 likely points to actual shape mangling.
const RUNTIME_ERROR_PHRASES: &[&str] = &[
    "does not exist",
    "no such host",
    "dial tcp",
    "failed to access",
    "Unable to access",
    "is not authorized",
    "not in valid PEM format",
    "does not support",
    "failed permissions validation",
    "failed to establish connection",
    "context deadline",
    "AssumeRole",
];

fn is_runtime_error(message: &str) -> bool {
    RUNTIME_ERROR_PHRASES
        .iter()
        .any(|phrase| message.contains(phrase))
}

/// Send a create request and assert the response isn't a request-shape 400.
/// A successful create gets registered for cleanup; a 400 carrying a known
/// runtime/connectivity phrase counts as a pass (shape was accepted, source
/// just isn't reachable with the fake credentials we sent). Anything else
/// fails the test.
async fn assert_create_shape_accepted(
    ctx: &SmokeCtx,
    request: ClickPipePostRequest,
) -> TestResult<()> {
    let mut cleanup = CleanupRegistry::default();
    eprintln!(
        "create request body: {}",
        serde_json::to_string_pretty(&request)?
    );

    let outcome: TestResult<()> = match ctx
        .client
        .click_pipe_create(&ctx.org_id, &ctx.service_id, &request)
        .await
    {
        Ok(resp) => {
            if let Some(pipe) = resp.result {
                eprintln!("  pipe created: id={} state={}", pipe.id, pipe.state);
                cleanup.register_clickpipe(ctx.service_id.clone(), pipe.id.to_string());
            }
            Ok(())
        }
        Err(Error::Api {
            status: 400,
            message,
        }) if is_runtime_error(&message) => {
            eprintln!("  shape accepted; runtime validation failed as expected: {message}");
            Ok(())
        }
        Err(Error::Api {
            status: 400,
            message,
        }) => Err(format!("shape rejected (HTTP 400): {message}").into()),
        Err(other) => Err(format!("create failed with non-shape error: {other}").into()),
    };

    if let Err(error) = cleanup
        .cleanup(
            &ctx.client,
            &ctx.org_id,
            ctx.delete_timeout,
            ctx.poll_interval,
            None,
        )
        .await
    {
        eprintln!("cleanup error (test outcome unaffected): {error}");
    }
    outcome
}

fn database_destination() -> ClickPipeMutateDestination {
    // Database pipes (Postgres/MySQL/BigQuery/MongoDB) reject the
    // table/columns/managedTable/tableDefinition group entirely.
    ClickPipeMutateDestination {
        database: "default".to_string(),
        ..Default::default()
    }
}

fn managed_destination(table: &str) -> ClickPipeMutateDestination {
    ClickPipeMutateDestination {
        database: "default".to_string(),
        table: Some(table.to_string()),
        managed_table: Some(true),
        columns: vec![
            ClickPipeDestinationColumn {
                name: "id".to_string(),
                r#type: "Int64".to_string(),
            },
            ClickPipeDestinationColumn {
                name: "payload".to_string(),
                r#type: "String".to_string(),
            },
        ],
        table_definition: Some(ClickPipeDestinationTableDefinition {
            engine: ClickPipeDestinationTableEngine {
                r#type: ClickPipeDestinationTableEngineType::MergeTree,
                ..Default::default()
            },
            primary_key: "id".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and a pre-provisioned service"]
async fn cloud_clickpipe_create_kafka_sasl_smoke() -> TestResult<()> {
    let ctx = SmokeCtx::from_env()?;
    let request = ClickPipePostRequest {
        name: ctx.pipe_name("kafka-sasl"),
        source: ClickPipePostSource {
            kafka: Some(ClickPipePostKafkaSource {
                r#type: ClickPipePostKafkaSourceType::default(),
                format: ClickPipePostKafkaSourceFormat::JSONEachRow,
                brokers: "broker.invalid:9092".to_string(),
                topics: "smoke-topic".to_string(),
                consumer_group: Some(format!("smoke-cg-{}", ctx.run_id)),
                authentication: ClickPipePostKafkaSourceAuthentication::PLAIN,
                credentials: serde_json::json!({
                    "username": "smoke-user",
                    "password": "smoke-pass",
                }),
                offset: Some(ClickPipeKafkaOffset {
                    strategy: ClickPipeKafkaOffsetStrategy::From_beginning,
                    timestamp: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        destination: managed_destination("smoke_kafka"),
        ..Default::default()
    };
    assert_create_shape_accepted(&ctx, request).await
}

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and a pre-provisioned service"]
async fn cloud_clickpipe_create_kafka_msk_iam_smoke() -> TestResult<()> {
    let ctx = SmokeCtx::from_env()?;
    let request = ClickPipePostRequest {
        name: ctx.pipe_name("kafka-msk"),
        source: ClickPipePostSource {
            kafka: Some(ClickPipePostKafkaSource {
                r#type: ClickPipePostKafkaSourceType::default(),
                format: ClickPipePostKafkaSourceFormat::JSONEachRow,
                brokers: "msk.invalid:9098".to_string(),
                topics: "smoke-topic".to_string(),
                authentication: ClickPipePostKafkaSourceAuthentication::IAM_ROLE,
                credentials: serde_json::Value::Null,
                iam_role: Some("arn:aws:iam::000000000000:role/smoke-fake-role".to_string()),
                offset: Some(ClickPipeKafkaOffset {
                    strategy: ClickPipeKafkaOffsetStrategy::From_beginning,
                    timestamp: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        destination: managed_destination("smoke_kafka_msk"),
        ..Default::default()
    };
    assert_create_shape_accepted(&ctx, request).await
}

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and a pre-provisioned service"]
async fn cloud_clickpipe_create_kinesis_iam_user_smoke() -> TestResult<()> {
    let ctx = SmokeCtx::from_env()?;
    let request = ClickPipePostRequest {
        name: ctx.pipe_name("kinesis"),
        source: ClickPipePostSource {
            kinesis: Some(ClickPipePostKinesisSource {
                authentication: ClickPipePostKinesisSourceAuthentication::IAM_USER,
                access_key: Some(MskIamUser {
                    access_key_id: "AKIAFAKEKEYFORSMOKE".to_string(),
                    secret_key: "fake/secret/for/smoke/test/0000000000000000".to_string(),
                }),
                format: ClickPipePostKinesisSourceFormat::JSONEachRow,
                region: "us-east-1".to_string(),
                stream_name: "smoke-stream".to_string(),
                iterator_type: ClickPipePostKinesisSourceIteratortype::default(),
                ..Default::default()
            }),
            ..Default::default()
        },
        destination: managed_destination("smoke_kinesis"),
        ..Default::default()
    };
    assert_create_shape_accepted(&ctx, request).await
}

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and a pre-provisioned service"]
async fn cloud_clickpipe_create_s3_iam_user_smoke() -> TestResult<()> {
    let ctx = SmokeCtx::from_env()?;
    let request = ClickPipePostRequest {
        name: ctx.pipe_name("s3-user"),
        source: ClickPipePostSource {
            object_storage: Some(ClickPipePostObjectStorageSource {
                r#type: ClickPipePostObjectStorageSourceType::default(),
                format: ClickPipePostObjectStorageSourceFormat::JSONEachRow,
                url: "https://smoke-fake-bucket.s3.us-east-1.amazonaws.com/data/*.json".to_string(),
                authentication: Some(ClickPipePostObjectStorageSourceAuthentication::IAM_USER),
                access_key: Some(MskIamUser {
                    access_key_id: "AKIAFAKEKEYFORSMOKE".to_string(),
                    secret_key: "fake/secret/for/smoke/test/0000000000000000".to_string(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        destination: managed_destination("smoke_s3_user"),
        ..Default::default()
    };
    assert_create_shape_accepted(&ctx, request).await
}

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and a pre-provisioned service"]
async fn cloud_clickpipe_create_s3_iam_role_smoke() -> TestResult<()> {
    let ctx = SmokeCtx::from_env()?;
    let request = ClickPipePostRequest {
        name: ctx.pipe_name("s3-role"),
        source: ClickPipePostSource {
            object_storage: Some(ClickPipePostObjectStorageSource {
                r#type: ClickPipePostObjectStorageSourceType::default(),
                format: ClickPipePostObjectStorageSourceFormat::JSONEachRow,
                url: "https://smoke-fake-bucket.s3.us-east-1.amazonaws.com/data/*.json".to_string(),
                authentication: Some(ClickPipePostObjectStorageSourceAuthentication::IAM_ROLE),
                iam_role: Some("arn:aws:iam::000000000000:role/smoke-fake-role".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
        destination: managed_destination("smoke_s3_role"),
        ..Default::default()
    };
    assert_create_shape_accepted(&ctx, request).await
}

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and a pre-provisioned service"]
async fn cloud_clickpipe_create_bigquery_snapshot_smoke() -> TestResult<()> {
    let ctx = SmokeCtx::from_env()?;
    // Fake but syntactically valid service account JSON, base64 encoded.
    let fake_sa = serde_json::json!({
        "type": "service_account",
        "project_id": "smoke-fake-project",
        "private_key_id": "smokefakekeyid",
        "private_key": "-----BEGIN PRIVATE KEY-----\nfake\n-----END PRIVATE KEY-----\n",
        "client_email": "smoke@smoke-fake-project.iam.gserviceaccount.com",
        "client_id": "000000000000000000000",
        "auth_uri": "https://accounts.google.com/o/oauth2/auth",
        "token_uri": "https://oauth2.googleapis.com/token",
    })
    .to_string();
    let sa_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        fake_sa.as_bytes(),
    );

    let request = ClickPipePostRequest {
        name: ctx.pipe_name("bq"),
        source: ClickPipePostSource {
            bigquery: Some(ClickPipeMutateBigQuerySource {
                credentials: ServiceAccount {
                    service_account_file: sa_b64,
                },
                snapshot_staging_path: "gs://smoke-fake-bucket/staging".to_string(),
                settings: ClickPipeBigQueryPipeSettings {
                    replication_mode: ClickPipeBigQueryPipeSettingsReplicationmode::Snapshot,
                    ..Default::default()
                },
                table_mappings: vec![ClickPipeBigQueryPipeTableMapping {
                    source_dataset_name: "smoke_dataset".to_string(),
                    source_table: "smoke_source".to_string(),
                    target_table: "smoke_target".to_string(),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        },
        destination: database_destination(),
        ..Default::default()
    };
    assert_create_shape_accepted(&ctx, request).await
}

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and a pre-provisioned service"]
async fn cloud_clickpipe_create_mysql_cdc_smoke() -> TestResult<()> {
    let ctx = SmokeCtx::from_env()?;
    let request = ClickPipePostRequest {
        name: ctx.pipe_name("mysql"),
        source: ClickPipePostSource {
            mysql: Some(ClickPipeMutateMySQLSource {
                r#type: Some(ClickPipeMutateMySQLSourceType::Mysql),
                authentication: Some(ClickPipeMutateMySQLSourceAuthentication::Basic),
                credentials: Some(PLAIN {
                    username: "smoke-user".to_string(),
                    password: "smoke-pass".to_string(),
                }),
                host: "mysql.invalid".to_string(),
                port: 3306,
                settings: ClickPipeMySQLPipeSettings {
                    replication_mode: ClickPipeMySQLPipeSettingsReplicationmode::Cdc,
                    ..Default::default()
                },
                table_mappings: vec![ClickPipeMySQLPipeTableMapping {
                    source_schema_name: "smoke_db".to_string(),
                    source_table: "smoke_source".to_string(),
                    target_table: "smoke_target".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        },
        destination: database_destination(),
        ..Default::default()
    };
    assert_create_shape_accepted(&ctx, request).await
}

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud credentials and a pre-provisioned service"]
async fn cloud_clickpipe_create_mongodb_cdc_smoke() -> TestResult<()> {
    let ctx = SmokeCtx::from_env()?;
    let request = ClickPipePostRequest {
        name: ctx.pipe_name("mongo"),
        source: ClickPipePostSource {
            mongodb: Some(ClickPipeMutateMongoDBSource {
                credentials: Some(PLAIN {
                    username: "smoke-user".to_string(),
                    password: "smoke-pass".to_string(),
                }),
                uri: "mongodb://mongo.invalid:27017".to_string(),
                read_preference: ClickPipeMutateMongoDBSourceReadpreference::Primary,
                settings: ClickPipeMongoDBPipeSettings {
                    replication_mode: ClickPipeMongoDBPipeSettingsReplicationmode::Cdc,
                    ..Default::default()
                },
                table_mappings: vec![ClickPipeMongoDBPipeTableMapping {
                    source_database_name: "smoke_db".to_string(),
                    source_collection: "smoke_collection".to_string(),
                    target_table: "smoke_target".to_string(),
                    table_engine: None,
                }],
                ..Default::default()
            }),
            ..Default::default()
        },
        destination: database_destination(),
        ..Default::default()
    };
    assert_create_shape_accepted(&ctx, request).await
}
