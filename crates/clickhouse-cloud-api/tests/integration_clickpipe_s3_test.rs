mod integration;

use std::time::Duration;

use clickhouse_cloud_api::models::*;
use integration::support::*;

const TARGET_TABLE: &str = "s3_users";
const SEED_ROW_COUNT: i64 = 3;
const FIXTURE_KEY: &str = "fixtures/users.json";
const FIXTURE_BODY: &str = concat!(
    "{\"id\": 1, \"name\": \"Ada Lovelace\",     \"email\": \"ada@example.com\"}\n",
    "{\"id\": 2, \"name\": \"Grace Hopper\",      \"email\": \"grace@example.com\"}\n",
    "{\"id\": 3, \"name\": \"Margaret Hamilton\", \"email\": \"margaret@example.com\"}\n",
);

const AWS_REGION: &str = "us-east-2";

const DEFAULT_CLICKPIPE_READY_TIMEOUT_SECS: u64 = 600;
const DEFAULT_INGEST_TIMEOUT_SECS: u64 = 300;

#[tokio::test]
#[ignore = "requires live ClickHouse Cloud + AWS credentials and provisions real resources"]
async fn cloud_clickpipe_s3_iam_role() -> TestResult<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let ctx = TestContext::from_env()?;
    let clickpipe_ready_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_CLICKPIPE_READY_SECS",
        DEFAULT_CLICKPIPE_READY_TIMEOUT_SECS,
    )?;
    let ingest_timeout = duration_from_env_or(
        "CLICKHOUSE_CLOUD_TEST_TIMEOUT_INGEST_SECS",
        DEFAULT_INGEST_TIMEOUT_SECS,
    )?;

    let client = create_client()?;

    // AWS clients: loaded from the standard credential chain. Locally that's
    // the user's shell (we expect AWS_PROFILE=Integrations_Tester); in CI it's
    // an OIDC role assumption.
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(AWS_REGION))
        .load()
        .await;
    let s3 = aws_sdk_s3::Client::new(&aws_config);
    let iam = aws_sdk_iam::Client::new(&aws_config);

    let mut cleanup = CleanupRegistry::default();
    let mut aws_cleanup = AwsCleanupRegistry::default();

    let test_result = async {
        log_run_header("cloud_clickpipe_s3_iam_role", &ctx);

        // ── Provision ClickHouse service ─────────────────────────────
        log_phase("Provision ClickHouse");

        let ch_create_body = ServicePostRequest {
            name: ctx.clickpipe_s3_service_name(),
            provider: ServicePostRequestProvider::Unknown(ctx.provider.clone()),
            region: ServicePostRequestRegion::Unknown(ctx.region.clone()),
            min_replica_memory_gb: Some(8.0),
            max_replica_memory_gb: Some(8.0),
            num_replicas: Some(1.0),
            idle_scaling: Some(true),
            idle_timeout_minutes: Some(5.0),
            ip_access_list: vec![IpAccessListEntry {
                source: "0.0.0.0/0".to_string(),
                description: Some("clickpipe-s3 integration test".to_string()),
            }],
            tags: Some(ctx.clickpipe_s3_run_tags()),
            ..Default::default()
        };

        let ch_created = client
            .instance_create(&ctx.org_id, &ch_create_body)
            .await?
            .result
            .ok_or("service create returned no result")?;
        let clickhouse_id = ch_created.service.id.to_string();
        let clickhouse_password = ch_created.password.clone();
        cleanup.register_service(clickhouse_id.clone());
        eprintln!("  provisioned clickhouse id <redacted>");

        log_phase("Wait for ClickHouse steady state");

        let ch_ready = poll_until(
            "clickhouse steady state",
            ctx.steady_state_timeout,
            ctx.poll_interval,
            || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let clickhouse_id = clickhouse_id.clone();
                async move {
                    let resp = client.instance_get(&org_id, &clickhouse_id).await?;
                    let svc = resp.result.ok_or("service get returned no result")?;
                    let state = svc.state.to_string();
                    if matches!(state.as_str(), "running" | "idle") {
                        Ok(Some(svc))
                    } else {
                        Ok(None)
                    }
                }
            },
        )
        .await?;

        // The service's `iamRole` is the principal that any S3 IAM role we
        // create needs to trust. Empty on services that haven't finished
        // initialising the ClickPipes side of the world.
        assert!(
            !ch_ready.iam_role.is_empty(),
            "service has no iamRole populated — cannot establish ClickPipes S3 trust"
        );
        let service_principal_arn = ch_ready.iam_role.clone();

        let ch_endpoint = ch_ready
            .endpoints
            .iter()
            .find(|e| matches!(e.protocol, ServiceEndpointProtocol::Https))
            .ok_or("ClickHouse service has no https endpoint")?
            .clone();
        let ch_username = ch_endpoint
            .username
            .clone()
            .unwrap_or_else(|| "default".to_string());

        // ── Provision AWS resources ──────────────────────────────────
        log_phase("Provision AWS bucket + role");

        let bucket = ctx.aws_s3_bucket_name();
        let role_name = ctx.aws_iam_role_name();
        let aws_tags = vec![
            ("managed_by".to_string(), "clickhousectl_e2e".to_string()),
            ("run_id".to_string(), ctx.run_id.clone()),
            ("suite".to_string(), "clickpipe_s3".to_string()),
        ];

        create_private_bucket(&s3, AWS_REGION, &bucket, &aws_tags).await?;
        aws_cleanup
            .register_s3_bucket(aws_sdk_s3::config::Region::new(AWS_REGION), bucket.clone());
        eprintln!("  created bucket {bucket}");

        put_object_bytes(
            &s3,
            &bucket,
            FIXTURE_KEY,
            FIXTURE_BODY.as_bytes().to_vec(),
            "application/x-ndjson",
        )
        .await?;
        eprintln!("  uploaded fixture {FIXTURE_KEY}");

        let read_policy = serde_json::json!({
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Sid": "ReadFixtures",
                    "Effect": "Allow",
                    "Action": ["s3:GetObject"],
                    "Resource": format!("arn:aws:s3:::{bucket}/*"),
                },
                {
                    "Sid": "ListBucket",
                    "Effect": "Allow",
                    "Action": ["s3:ListBucket", "s3:GetBucketLocation"],
                    "Resource": format!("arn:aws:s3:::{bucket}"),
                }
            ]
        })
        .to_string();

        let role_arn = create_clickpipes_iam_role(
            &iam,
            &role_name,
            &service_principal_arn,
            &read_policy,
            &aws_tags,
        )
        .await?;
        aws_cleanup.register_iam_role(role_name.clone());
        eprintln!("  created iam role {role_name}");

        // IAM has eventual consistency: a freshly-created role can take up to
        // ~10s to be assumable. Sleep briefly so ClickPipes' first connect
        // doesn't trip on a transient AccessDenied.
        tokio::time::sleep(Duration::from_secs(10)).await;

        // ── Create ClickPipe ─────────────────────────────────────────
        log_phase("Create ClickPipe");

        let object_url = format!(
            "https://{bucket}.s3.{AWS_REGION}.amazonaws.com/{FIXTURE_KEY}"
        );

        let pipe_request = ClickPipePostRequest {
            name: format!("s3-{}", ctx.run_id),
            destination: ClickPipeMutateDestination {
                database: "default".to_string(),
                table: Some(TARGET_TABLE.to_string()),
                managed_table: Some(true),
                columns: vec![
                    ClickPipeDestinationColumn {
                        name: "id".to_string(),
                        r#type: "Int64".to_string(),
                    },
                    ClickPipeDestinationColumn {
                        name: "name".to_string(),
                        r#type: "String".to_string(),
                    },
                    ClickPipeDestinationColumn {
                        name: "email".to_string(),
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
            },
            source: ClickPipePostSource {
                object_storage: Some(ClickPipePostObjectStorageSource {
                    r#type: ClickPipePostObjectStorageSourceType::default(),
                    format: ClickPipePostObjectStorageSourceFormat::JSONEachRow,
                    url: object_url,
                    authentication: Some(
                        ClickPipePostObjectStorageSourceAuthentication::IAM_ROLE,
                    ),
                    iam_role: Some(role_arn.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let pipe = client
            .click_pipe_create(&ctx.org_id, &clickhouse_id, &pipe_request)
            .await?
            .result
            .ok_or("clickpipe create returned no result")?;
        let clickpipe_id = pipe.id.to_string();
        cleanup.register_clickpipe(clickhouse_id.clone(), clickpipe_id.clone());
        eprintln!("  provisioned clickpipe id <redacted>");

        // ── Wait for Running ─────────────────────────────────────────
        log_phase("Wait for ClickPipe Running");

        let _running_pipe = poll_until(
            "clickpipe Running state",
            clickpipe_ready_timeout,
            ctx.poll_interval,
            || {
                let client = client.clone();
                let org_id = ctx.org_id.clone();
                let clickhouse_id = clickhouse_id.clone();
                let clickpipe_id = clickpipe_id.clone();
                async move {
                    let resp = client
                        .click_pipe_get(&org_id, &clickhouse_id, &clickpipe_id)
                        .await?;
                    let pipe = resp.result.ok_or("clickpipe get returned no result")?;
                    match pipe.state {
                        ClickPipeState::Running | ClickPipeState::Completed => Ok(Some(pipe)),
                        ClickPipeState::Failed | ClickPipeState::InternalError => Err(format!(
                            "clickpipe entered terminal failure state {}",
                            pipe.state
                        )
                        .into()),
                        _ => Ok(None),
                    }
                }
            },
        )
        .await?;

        // ── Verify rows arrived ──────────────────────────────────────
        log_phase("Verify rows in ClickHouse");

        let ch_query = ClickHouseQuery::new(
            &ch_endpoint.host,
            ch_endpoint.port as u16,
            &ch_username,
            &clickhouse_password,
        );

        poll_until(
            "fixture row count in ClickHouse",
            ingest_timeout,
            ctx.poll_interval,
            || {
                let ch_query = ch_query.clone();
                async move {
                    match ch_query.count_rows(TARGET_TABLE).await {
                        Ok(count) if count >= SEED_ROW_COUNT => Ok(Some(count)),
                        Ok(_) => Ok(None),
                        Err(e) => Err(e),
                    }
                }
            },
        )
        .await?;

        let ada = ch_query
            .scalar_string(&format!(
                "SELECT name FROM default.{TARGET_TABLE} WHERE id = 1 LIMIT 1"
            ))
            .await?;
        assert_eq!(ada.as_deref(), Some("Ada Lovelace"), "id=1 spot-check failed");

        log_phase("Teardown");
        client
            .click_pipe_delete(&ctx.org_id, &clickhouse_id, &clickpipe_id)
            .await?;
        cleanup.unregister_clickpipe(&clickhouse_id, &clickpipe_id);

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    let cleanup_result = cleanup
        .cleanup(&client, &ctx.org_id, ctx.delete_timeout, ctx.poll_interval)
        .await;
    let aws_cleanup_result = aws_cleanup.cleanup(&aws_config, &iam).await;

    match (test_result, cleanup_result, aws_cleanup_result) {
        (Ok(()), Ok(()), Ok(())) => Ok(()),
        (Err(error), _, _) => Err(error),
        (Ok(()), Err(cleanup_error), Ok(())) => Err(cleanup_error.into()),
        (Ok(()), Ok(()), Err(aws_error)) => Err(aws_error.into()),
        (Ok(()), Err(cleanup_error), Err(aws_error)) => {
            Err(format!("{cleanup_error}\naws cleanup failed:\n{aws_error}").into())
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn duration_from_env_or(name: &str, default_secs: u64) -> TestResult<Duration> {
    match std::env::var(name) {
        Ok(value) => Ok(Duration::from_secs(value.parse()?)),
        Err(std::env::VarError::NotPresent) => Ok(Duration::from_secs(default_secs)),
        Err(error) => Err(Box::new(error)),
    }
}

#[derive(Clone)]
struct ClickHouseQuery {
    base_url: String,
    username: String,
    password: String,
    http: reqwest::Client,
}

impl ClickHouseQuery {
    fn new(host: &str, port: u16, username: &str, password: &str) -> Self {
        Self {
            base_url: format!("https://{host}:{port}"),
            username: username.to_string(),
            password: password.to_string(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }

    async fn run_query(&self, query: &str) -> TestResult<String> {
        let resp = self
            .http
            .post(&self.base_url)
            .basic_auth(&self.username, Some(&self.password))
            .body(query.to_string())
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(format!("ClickHouse query failed ({status}): {body}").into());
        }
        Ok(body)
    }

    async fn count_rows(&self, table: &str) -> TestResult<i64> {
        let body = self
            .run_query(&format!(
                "SELECT count() FROM default.{table} FORMAT TabSeparated"
            ))
            .await?;
        Ok(body.trim().parse::<i64>()?)
    }

    async fn scalar_string(&self, query: &str) -> TestResult<Option<String>> {
        let body = self
            .run_query(&format!("{query} FORMAT TabSeparated"))
            .await?;
        let value = body.trim();
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value.to_string()))
        }
    }
}
