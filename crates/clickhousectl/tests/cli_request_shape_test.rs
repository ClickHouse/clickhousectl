//! CLI request-shape regression tests.
//!
//! Each test invokes the real `clickhousectl` binary as a subprocess, pointed
//! at a local `wiremock` server impersonating the ClickHouse Cloud API. The
//! mock records the request body the binary sent, and the test asserts on
//! its JSON shape.
//!
//! This is the cheapest way to structurally guard against Al's `4f6c2ba` bug
//! class — handler regressions like `args.foo.clone().unwrap_or_default()`
//! that serialize `""` on the wire when the user didn't pass `--foo`. The
//! API rejects `""` for `undefinedOr(...)` fields; these tests rejected the
//! same shape locally in ~200ms without touching any cloud infrastructure.
//!
//! Tests run as cargo integration tests:
//!     cargo test -p clickhousectl --test cli_request_shape_test

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use wiremock::matchers::{header, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Locate the `clickhousectl` binary. cargo populates `CARGO_BIN_EXE_<name>`
/// for integration tests in the same package — so this is just the absolute
/// path to the build output, no `cargo build` shellout needed.
fn clickhousectl_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_clickhousectl"))
}

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

/// Assert the binary exited zero, panicking with the captured stderr/stdout
/// so the failure cause is visible in the test output.
fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "clickhousectl exited {}\nstderr:\n{}\nstdout:\n{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}

/// Run the clickhousectl binary against the mock, returning the JSON body
/// the binary POSTed. Panics with the captured stderr if the binary exits
/// non-zero — a failure here is almost always a clap-parsing error, which
/// is itself a bug worth surfacing.
async fn invoke_cli_capture_body(mock: &MockServer, cli_args: &[&str]) -> Value {
    let mut full_args: Vec<&str> = vec!["cloud", "--url"];
    let url = mock.uri();
    full_args.push(&url);
    full_args.push("--json");
    full_args.extend(cli_args);

    let output = Command::new(clickhousectl_binary())
        .args(&full_args)
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .output()
        .expect("failed to spawn clickhousectl");

    assert!(
        output.status.success(),
        "clickhousectl exited {} for args {:?}\nstderr:\n{}\nstdout:\n{}",
        output.status.code().unwrap_or(-1),
        full_args,
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let requests = mock
        .received_requests()
        .await
        .expect("mock requests log unavailable");
    // The CLI's first call when --org-id is passed is just the
    // clickpipe-create POST. If a future change adds a discovery GET it
    // would appear here too; assert on the POST specifically.
    let post = requests
        .iter()
        .find(|r| r.method == wiremock::http::Method::POST)
        .expect("no POST request recorded by mock");
    serde_json::from_slice(&post.body).expect("POST body wasn't valid JSON")
}

// ── Bug 1: Postgres CDC must NOT send publicationName / replicationSlotName ─
//
// `cdc` replication mode creates the slot + publication server-side; the
// pre-`4f6c2ba` handler sent `""` for both via `unwrap_or_default()` and the
// API rejected with `replicationSlotName: ''`. The fix made the model field
// `Option<String>` so absence at the args level = absence in the wire body.
// This test would fail if either field reappeared.

#[tokio::test]
async fn postgres_cdc_omits_publication_name_and_slot_when_not_passed() {
    let mock = start_mock_clickpipes_api().await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "postgres",
            "svc-id",
            "--name",
            "test-pipe",
            "--host",
            "pg.example.com",
            "--port",
            "5432",
            "--pg-database",
            "test",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "public.t:t",
            "--replication-mode",
            "cdc",
            // INTENTIONALLY omit --publication-name + --replication-slot-name.
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let settings = &body["source"]["postgres"]["settings"];
    assert!(
        settings.get("publicationName").is_none(),
        "publicationName leaked into wire body: {settings}",
    );
    assert!(
        settings.get("replicationSlotName").is_none(),
        "replicationSlotName leaked into wire body: {settings}",
    );
}

// ── Bug 2: Database-pipe destination must NOT include table/columns/etc. ────
//
// For Postgres / MySQL / MongoDB / BigQuery, the `destination` body must
// carry only `database` — the per-mapping `targetTable` carries the
// destination table name. The pre-`4f6c2ba` handler defaulted `table: ""`,
// `columns: []`, `managedTable: false`, `tableDefinition: {…default…}` and
// the API rejected with `destination.table: ''` and `columns: minLength`.
// Modeling those four fields as `Option<T>` + `skip_serializing_if` made
// absence in the args translate to absence on the wire.

#[tokio::test]
async fn postgres_destination_omits_table_columns_managed_table_definition() {
    let mock = start_mock_clickpipes_api().await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "postgres",
            "svc-id",
            "--name",
            "test-pipe",
            "--host",
            "pg.example.com",
            "--port",
            "5432",
            "--pg-database",
            "test",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "public.t:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let dest = &body["destination"];
    assert_eq!(
        dest["database"], "default",
        "database should default to 'default' for postgres CDC, got {dest}"
    );
    for field in ["table", "columns", "managedTable", "tableDefinition"] {
        assert!(
            dest.get(field).is_none(),
            "{field} leaked into destination body — Al's Bug 2 regression: {dest}",
        );
    }
}

#[tokio::test]
async fn mysql_destination_omits_table_columns_managed_table_definition() {
    let mock = start_mock_clickpipes_api().await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mysql",
            "svc-id",
            "--name",
            "test-pipe",
            "--host",
            "mysql.example.com",
            "--port",
            "3306",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "mydb.t:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let dest = &body["destination"];
    assert_eq!(dest["database"], "default");
    for field in ["table", "columns", "managedTable", "tableDefinition"] {
        assert!(
            dest.get(field).is_none(),
            "{field} leaked into MySQL destination body: {dest}",
        );
    }
}

#[tokio::test]
async fn mongodb_destination_omits_table_columns_managed_table_definition() {
    let mock = start_mock_clickpipes_api().await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mongodb",
            "svc-id",
            "--name",
            "test-pipe",
            "--uri",
            "mongodb://mongo.example.com:27017",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "mydb.coll:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let dest = &body["destination"];
    assert_eq!(dest["database"], "default");
    for field in ["table", "columns", "managedTable", "tableDefinition"] {
        assert!(
            dest.get(field).is_none(),
            "{field} leaked into Mongo destination body: {dest}",
        );
    }
}

// ── Spot-check: optional flags omitted from non-database (S3) pipe too ──────
//
// S3 isn't a database pipe — it DOES include table/columns/etc. in
// destination. But it has its own optionals that should be absent when the
// flag isn't passed (e.g. --iam-role, --queue-url).

#[tokio::test]
async fn s3_pipe_omits_iam_role_and_queue_url_when_not_passed() {
    let mock = start_mock_clickpipes_api().await;

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "test-pipe",
            "--source-url",
            "https://bucket.s3.us-east-1.amazonaws.com/data/*.json",
            "--format",
            "JSONEachRow",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
            "--access-key-id",
            "AKIA000000000000FAKE",
            "--secret-key",
            "fake/secret/for/tests/0000000000000000",
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let s3 = &body["source"]["objectStorage"];
    for field in [
        "iamRole",
        "queueUrl",
        "connectionString",
        "azureContainerName",
        "path",
        "serviceAccountKey",
        "delimiter",
    ] {
        assert!(
            s3.get(field).is_none(),
            "{field} leaked into S3 body when --{field} not passed: {s3}",
        );
    }
}

// `--service-account-file` for GCS object-storage points at a JSON key on disk;
// the handler must read the file and base64-encode its contents into
// `source.objectStorage.serviceAccountKey`, matching the BigQuery flow.
#[tokio::test]
async fn gcs_service_account_file_is_read_and_base64_encoded() {
    use std::io::Write;
    let mock = start_mock_clickpipes_api().await;

    let dir = tempfile::tempdir().unwrap();
    let sa_path = dir.path().join("service-account.json");
    let sa_contents = br#"{"type":"service_account","project_id":"test"}"#;
    let mut sa_file = std::fs::File::create(&sa_path).unwrap();
    sa_file.write_all(sa_contents).unwrap();

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "object-storage",
            "svc-id",
            "--name",
            "gcs-pipe",
            "--source-url",
            "https://storage.googleapis.com/bucket/data/*.json",
            "--format",
            "JSONEachRow",
            "--storage-type",
            "gcs",
            "--database",
            "default",
            "--table",
            "events",
            "--column",
            "id:Int64",
            "--service-account-file",
            sa_path.to_str().unwrap(),
            "--org-id",
            "11dfa1ec-767d-43cb-bfad-618ce2aaf959",
        ],
    )
    .await;

    let gcs = &body["source"]["objectStorage"];
    assert_eq!(gcs["authentication"], "SERVICE_ACCOUNT");
    let expected = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        sa_contents,
    );
    assert_eq!(
        gcs["serviceAccountKey"].as_str(),
        Some(expected.as_str()),
        "serviceAccountKey on the wire should be base64 of the file contents: {gcs}",
    );
}

// Extra postgres coverage: --tls-host, --iam-role, --ca-certificate (file)
// should all be absent from the wire body when their CLI flags aren't set.

#[tokio::test]
async fn postgres_optional_fields_absent_when_flags_omitted() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "postgres",
            "svc-id",
            "--name",
            "t",
            "--host",
            "pg",
            "--port",
            "5432",
            "--pg-database",
            "test",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "public.t:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "org",
        ],
    )
    .await;

    let pg = &body["source"]["postgres"];
    for field in ["iamRole", "tlsHost", "caCertificate"] {
        assert!(
            pg.get(field).is_none(),
            "{field} leaked into postgres source body: {pg}",
        );
    }
}

// MySQL: same set of absent-when-omitted optional fields.

#[tokio::test]
async fn mysql_optional_fields_absent_when_flags_omitted() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
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
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "mydb.t:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "org",
        ],
    )
    .await;

    let mysql = &body["source"]["mysql"];
    for field in ["iamRole", "tlsHost", "caCertificate"] {
        assert!(
            mysql.get(field).is_none(),
            "{field} leaked into mysql source body: {mysql}",
        );
    }
}

// Mongo: tlsHost should be absent when --tls-host not passed.

#[tokio::test]
async fn mongodb_tls_host_absent_when_not_passed() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "mongodb",
            "svc-id",
            "--name",
            "t",
            "--uri",
            "mongodb://m:27017",
            "--username",
            "u",
            "--password",
            "p",
            "--table-mapping",
            "db.c:t",
            "--replication-mode",
            "cdc",
            "--org-id",
            "org",
        ],
    )
    .await;

    let mongo = &body["source"]["mongodb"];
    assert!(
        mongo.get("tlsHost").is_none(),
        "tlsHost leaked into mongodb source body: {mongo}",
    );
    assert!(
        mongo.get("caCertificate").is_none(),
        "caCertificate leaked into mongodb source body: {mongo}",
    );
}

// ── Kafka ──────────────────────────────────────────────────────────────────
//
// Kafka has the largest optional-flag surface; cover the high-traffic ones
// plus all 4 SASL credential shapes (PLAIN, SCRAM-SHA-256, SCRAM-SHA-512,
// MUTUAL_TLS, IAM_ROLE).

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
        "--auth",
        "PLAIN",
        "--username",
        "u",
        "--password",
        "p",
        "--org-id",
        "org",
    ]
}

#[tokio::test]
async fn kafka_optional_fields_absent_when_flags_omitted() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(&mock, &kafka_args_minimal()).await;

    let kafka = &body["source"]["kafka"];
    for field in ["consumerGroup", "iamRole", "schemaRegistry", "caCertificate"] {
        assert!(
            kafka.get(field).is_none(),
            "{field} leaked into kafka source body: {kafka}",
        );
    }
    assert!(
        kafka["offset"].get("timestamp").is_none(),
        "offset.timestamp leaked when --offset-timestamp not passed: {kafka}",
    );
}

#[tokio::test]
async fn kafka_plain_credentials_shape() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(&mock, &kafka_args_minimal()).await;
    let creds = &body["source"]["kafka"]["credentials"];
    assert_eq!(creds["username"], "u");
    assert_eq!(creds["password"], "p");
}

#[tokio::test]
async fn kafka_scram_sha_512_credentials_shape() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = kafka_args_minimal();
    // replace `PLAIN` with SCRAM-SHA-512
    let auth_idx = args.iter().position(|a| *a == "PLAIN").unwrap();
    args[auth_idx] = "SCRAM-SHA-512";
    let body = invoke_cli_capture_body(&mock, &args).await;
    let creds = &body["source"]["kafka"]["credentials"];
    assert_eq!(creds["username"], "u");
    assert_eq!(creds["password"], "p");
}

#[tokio::test]
async fn kafka_iam_role_serializes_iam_role_field() {
    let mock = start_mock_clickpipes_api().await;
    // IAM_ROLE doesn't use --username/--password; build a custom arg list.
    let body = invoke_cli_capture_body(
        &mock,
        &[
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
            "msk",
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:aws:iam::123:role/x",
            "--org-id",
            "org",
        ],
    )
    .await;

    let kafka = &body["source"]["kafka"];
    assert_eq!(kafka["iamRole"], "arn:aws:iam::123:role/x");
    // credentials for IAM_ROLE is sent as JSON null at the field level.
    assert!(
        kafka["credentials"].is_null(),
        "IAM_ROLE credentials should be null, got: {}",
        kafka["credentials"]
    );
}

#[tokio::test]
async fn kafka_iam_user_credentials_shape() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
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
            "msk",
            "--auth",
            "IAM_USER",
            "--access-key-id",
            "AKIA000000000000FAKE",
            "--secret-key",
            "fake/secret/0000000000000000",
            "--org-id",
            "org",
        ],
    )
    .await;

    let creds = &body["source"]["kafka"]["credentials"];
    assert_eq!(creds["accessKeyId"], "AKIA000000000000FAKE");
    assert_eq!(creds["secretKey"], "fake/secret/0000000000000000");
}

#[tokio::test]
async fn kafka_mutual_tls_credentials_use_cert_file_contents() {
    use std::io::Write;
    let mock = start_mock_clickpipes_api().await;
    let dir = tempfile::tempdir().unwrap();
    let cert_path = dir.path().join("client.crt");
    let key_path = dir.path().join("client.key");
    let mut cert_file = std::fs::File::create(&cert_path).unwrap();
    let mut key_file = std::fs::File::create(&key_path).unwrap();
    cert_file
        .write_all(b"-----BEGIN CERTIFICATE-----\nCERT_PEM\n-----END CERTIFICATE-----\n")
        .unwrap();
    key_file
        .write_all(b"-----BEGIN PRIVATE KEY-----\nKEY_PEM\n-----END PRIVATE KEY-----\n")
        .unwrap();

    let body = invoke_cli_capture_body(
        &mock,
        &[
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
            "--auth",
            "MUTUAL_TLS",
            "--client-certificate",
            cert_path.to_str().unwrap(),
            "--client-key",
            key_path.to_str().unwrap(),
            "--org-id",
            "org",
        ],
    )
    .await;

    let creds = &body["source"]["kafka"]["credentials"];
    assert!(
        creds["certificate"]
            .as_str()
            .map(|s| s.contains("CERT_PEM"))
            .unwrap_or(false),
        "MUTUAL_TLS certificate should contain file contents: {creds}",
    );
    assert!(
        creds["privateKey"]
            .as_str()
            .map(|s| s.contains("KEY_PEM"))
            .unwrap_or(false),
        "MUTUAL_TLS privateKey should contain file contents: {creds}",
    );
}

// ── Kinesis ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn kinesis_iam_role_omits_access_key() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "kinesis",
            "svc-id",
            "--name",
            "t",
            "--stream-name",
            "s",
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
            "--auth",
            "IAM_ROLE",
            "--iam-role",
            "arn:aws:iam::123:role/x",
            "--iterator-type",
            "TRIM_HORIZON",
            "--org-id",
            "org",
        ],
    )
    .await;

    let kinesis = &body["source"]["kinesis"];
    assert_eq!(kinesis["iamRole"], "arn:aws:iam::123:role/x");
    assert!(
        kinesis.get("accessKey").is_none(),
        "accessKey leaked when --auth IAM_ROLE: {kinesis}",
    );
}

#[tokio::test]
async fn kinesis_iam_user_omits_iam_role() {
    let mock = start_mock_clickpipes_api().await;
    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "kinesis",
            "svc-id",
            "--name",
            "t",
            "--stream-name",
            "s",
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
            "--auth",
            "IAM_USER",
            "--access-key-id",
            "AKIA000000000000FAKE",
            "--secret-key",
            "fake/secret/0000000000000000",
            "--iterator-type",
            "TRIM_HORIZON",
            "--org-id",
            "org",
        ],
    )
    .await;

    let kinesis = &body["source"]["kinesis"];
    assert_eq!(kinesis["accessKey"]["accessKeyId"], "AKIA000000000000FAKE");
    assert!(
        kinesis.get("iamRole").is_none(),
        "iamRole leaked when --auth IAM_USER: {kinesis}",
    );
}

// ── BigQuery ───────────────────────────────────────────────────────────────
//
// BigQuery has fewer optional flags than other sources, but still falls into
// the "database pipe" bucket — destination MUST omit table/columns/etc.

#[tokio::test]
async fn bigquery_destination_omits_table_columns_managed_table_definition() {
    use std::io::Write;
    let mock = start_mock_clickpipes_api().await;

    let dir = tempfile::tempdir().unwrap();
    let sa_path = dir.path().join("service-account.json");
    let mut sa_file = std::fs::File::create(&sa_path).unwrap();
    sa_file
        .write_all(
            br#"{
            "type": "service_account",
            "project_id": "test",
            "private_key_id": "fake",
            "private_key": "-----BEGIN PRIVATE KEY-----\nfake\n-----END PRIVATE KEY-----\n",
            "client_email": "fake@test.iam.gserviceaccount.com",
            "client_id": "0",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token"
        }"#,
        )
        .unwrap();

    let body = invoke_cli_capture_body(
        &mock,
        &[
            "clickpipe",
            "create",
            "bigquery",
            "svc-id",
            "--name",
            "t",
            "--service-account-file",
            sa_path.to_str().unwrap(),
            "--staging-path",
            "gs://bucket/staging",
            "--table-mapping",
            "dataset.t:t",
            "--org-id",
            "org",
        ],
    )
    .await;

    let dest = &body["destination"];
    assert_eq!(dest["database"], "default");
    for field in ["table", "columns", "managedTable", "tableDefinition"] {
        assert!(
            dest.get(field).is_none(),
            "{field} leaked into BigQuery destination body: {dest}",
        );
    }
}

// ── Postgres expansion ─────────────────────────────────────────────────────
//
// Beyond the absence cases above, these tests cover:
//   - The inverse: when --publication-name / --tls-host / --iam-role ARE
//     passed, the body must contain them with the exact value.
//   - Each `--postgres-type` enum variant flows through unchanged.
//   - Each `--replication-mode` enum variant.
//   - Multiple --table-mapping flags produce an array of N entries.
//   - --auth IAM_ROLE selects the right auth and serialises iamRole.

fn postgres_args_minimal() -> Vec<String> {
    [
        "clickpipe",
        "create",
        "postgres",
        "svc-id",
        "--name",
        "t",
        "--host",
        "pg",
        "--port",
        "5432",
        "--pg-database",
        "test",
        "--username",
        "u",
        "--password",
        "p",
        "--table-mapping",
        "public.t:t",
        "--replication-mode",
        "cdc",
        "--org-id",
        "org",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[tokio::test]
async fn postgres_publication_name_serializes_when_provided() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    args.push("--publication-name".into());
    args.push("my_pub".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["settings"]["publicationName"], "my_pub",
        "publicationName should round-trip the user-provided value"
    );
}

#[tokio::test]
async fn postgres_replication_slot_name_serializes_when_provided() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    args.push("--replication-slot-name".into());
    args.push("my_slot".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["settings"]["replicationSlotName"], "my_slot",
        "replicationSlotName should round-trip the user-provided value"
    );
}

#[tokio::test]
async fn postgres_tls_host_serializes_when_provided() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    args.push("--tls-host".into());
    args.push("pg.example.com".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["tlsHost"], "pg.example.com",
        "tlsHost should round-trip the user-provided value"
    );
}

#[tokio::test]
async fn postgres_iam_role_serializes_when_provided() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    args.push("--iam-role".into());
    args.push("arn:aws:iam::123:role/x".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["iamRole"], "arn:aws:iam::123:role/x",
        "iamRole should round-trip the user-provided value"
    );
}

#[tokio::test]
async fn postgres_ca_certificate_file_contents_flow_to_body() {
    use std::io::Write;
    let mock = start_mock_clickpipes_api().await;
    let dir = tempfile::tempdir().unwrap();
    let ca_path = dir.path().join("ca.pem");
    let pem = "-----BEGIN CERTIFICATE-----\nCA_PEM_CONTENT\n-----END CERTIFICATE-----\n";
    std::fs::File::create(&ca_path)
        .unwrap()
        .write_all(pem.as_bytes())
        .unwrap();

    let mut args = postgres_args_minimal();
    args.push("--ca-certificate".into());
    args.push(ca_path.to_str().unwrap().to_string());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert!(
        body["source"]["postgres"]["caCertificate"]
            .as_str()
            .map(|s| s.contains("CA_PEM_CONTENT"))
            .unwrap_or(false),
        "caCertificate body should contain the file's PEM content, got {}",
        body["source"]["postgres"]["caCertificate"]
    );
}

#[tokio::test]
async fn postgres_replication_mode_snapshot_serializes() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    let idx = args.iter().position(|a| a == "cdc").unwrap();
    args[idx] = "snapshot".into();
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["settings"]["replicationMode"], "snapshot",
    );
}

#[tokio::test]
async fn postgres_replication_mode_cdc_only_serializes() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    let idx = args.iter().position(|a| a == "cdc").unwrap();
    args[idx] = "cdc_only".into();
    // cdc_only typically requires explicit publication + slot to be useful;
    // assert the wire shape regardless.
    args.push("--publication-name".into());
    args.push("p".into());
    args.push("--replication-slot-name".into());
    args.push("s".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;
    assert_eq!(
        body["source"]["postgres"]["settings"]["replicationMode"], "cdc_only",
    );
}

#[tokio::test]
async fn postgres_multiple_table_mappings_serialize_as_array() {
    let mock = start_mock_clickpipes_api().await;
    let mut args = postgres_args_minimal();
    // Append a second --table-mapping. The first one was set in
    // `postgres_args_minimal` as public.t:t.
    args.push("--table-mapping".into());
    args.push("public.t2:t2_dst".into());
    args.push("--table-mapping".into());
    args.push("other_schema.t3:t3_dst".into());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let body = invoke_cli_capture_body(&mock, &arg_refs).await;

    let mappings = body["source"]["postgres"]["tableMappings"]
        .as_array()
        .unwrap_or_else(|| {
            panic!(
                "tableMappings should be an array, got: {}",
                body["source"]["postgres"]["tableMappings"]
            )
        });
    assert_eq!(
        mappings.len(),
        3,
        "expected 3 table mappings (minimal default + 2 added), got {}: {:?}",
        mappings.len(),
        mappings
    );
    let target_tables: Vec<&str> = mappings
        .iter()
        .filter_map(|m| m["targetTable"].as_str())
        .collect();
    assert!(target_tables.contains(&"t"));
    assert!(target_tables.contains(&"t2_dst"));
    assert!(target_tables.contains(&"t3_dst"));
}

// Each --postgres-type value should serialize to the matching enum string.
// The OpenAPI enum has 11 variants; the CLI accepts them via PossibleValuesParser
// and the handler uses parse_enum to convert. A regression here would
// silently change the source type the server uses to route the connection.

macro_rules! postgres_type_test {
    ($test_name:ident, $cli_value:literal, $wire_value:literal) => {
        #[tokio::test]
        async fn $test_name() {
            let mock = start_mock_clickpipes_api().await;
            let mut args = postgres_args_minimal();
            args.push("--postgres-type".into());
            args.push($cli_value.into());
            let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let body = invoke_cli_capture_body(&mock, &arg_refs).await;
            assert_eq!(
                body["source"]["postgres"]["type"], $wire_value,
                "--postgres-type {} should serialize to wire value {}",
                $cli_value, $wire_value,
            );
        }
    };
}

postgres_type_test!(postgres_type_postgres_serializes, "postgres", "postgres");
postgres_type_test!(postgres_type_supabase_serializes, "supabase", "supabase");
postgres_type_test!(postgres_type_neon_serializes, "neon", "neon");
postgres_type_test!(postgres_type_alloydb_serializes, "alloydb", "alloydb");
postgres_type_test!(
    postgres_type_planetscale_serializes,
    "planetscale",
    "planetscale"
);
postgres_type_test!(
    postgres_type_rdspostgres_serializes,
    "rdspostgres",
    "rdspostgres"
);
postgres_type_test!(
    postgres_type_aurorapostgres_serializes,
    "aurorapostgres",
    "aurorapostgres"
);
postgres_type_test!(
    postgres_type_cloudsqlpostgres_serializes,
    "cloudsqlpostgres",
    "cloudsqlpostgres"
);
postgres_type_test!(
    postgres_type_azurepostgres_serializes,
    "azurepostgres",
    "azurepostgres"
);
postgres_type_test!(
    postgres_type_crunchybridge_serializes,
    "crunchybridge",
    "crunchybridge"
);
postgres_type_test!(postgres_type_tigerdata_serializes, "tigerdata", "tigerdata");

// ── Dotenv ─────────────────────────────────────────────────────────────────
//
// A `.env` file in the current working directory supplying
// `CLICKHOUSE_CLOUD_API_KEY` + `CLICKHOUSE_CLOUD_API_SECRET` should produce
// the exact same `Authorization: Basic <base64>` header as exporting those
// vars in the shell. End-to-end proof that the resolver picks up `.env` and
// hands them to the lib client's basic-auth path.

#[tokio::test]
async fn dotenv_creds_produce_basic_auth_request() {
    use std::io::Write;

    let mock = MockServer::start().await;

    let stub_orgs = serde_json::json!({
        "result": [],
        "status": 200,
        "requestId": "stub-org-list",
    });
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_orgs))
        .mount(&mock)
        .await;

    // Put `.env` in the working directory and run the binary cd'd into it,
    // because the loader reads `cwd/.env` rather than walking ancestor
    // directories. The parent process's env vars are cleared in the child so
    // the .env is the only source of credentials — otherwise the test could
    // silently pass for the wrong reason.
    let dir = tempfile::tempdir().unwrap();
    let mut env_file = std::fs::File::create(dir.path().join(".env")).unwrap();
    env_file
        .write_all(b"CLICKHOUSE_CLOUD_API_KEY=dotenv-key\nCLICKHOUSE_CLOUD_API_SECRET=dotenv-secret\n")
        .unwrap();
    drop(env_file);

    let url = mock.uri();
    let output = Command::new(clickhousectl_binary())
        .args(["cloud", "--url", &url, "--json", "org", "list"])
        .current_dir(dir.path())
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);

    let requests = mock
        .received_requests()
        .await
        .expect("mock requests log unavailable");
    let auth = requests
        .iter()
        .find(|r| r.method == wiremock::http::Method::GET)
        .and_then(|r| r.headers.get("Authorization"))
        .expect("no Authorization header recorded");
    let auth_str = auth.to_str().expect("non-utf8 auth header");
    let expected = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "dotenv-key:dotenv-secret",
        )
    );
    assert_eq!(
        auth_str, expected,
        "Authorization header should match the .env credentials exactly"
    );
}

// ── Service query auth modes (issue #247) ──────────────────────────────────
//
// `cloud service query` has two auth paths:
//   - API key auth: a per-service Query API key (stored locally, else
//     auto-provisioned) is sent as Basic auth to the query host.
//   - OAuth: the user's own bearer token is sent directly to the query host
//     — no key lookup and, crucially, NO provisioning calls (key creation
//     and endpoint upsert need write access an OAuth token doesn't have).
// Both tests run the binary against two mocks: one impersonating the
// control plane (service lookup), one impersonating the query host (wired
// up via CLICKHOUSE_CLOUD_QUERY_HOST, which overrides host derivation).

const QUERY_TEST_SERVICE_ID: &str = "11111111-2222-3333-4444-555555555555";

async fn start_mock_control_plane_with_service() -> MockServer {
    let mock = MockServer::start().await;
    let stub_service = serde_json::json!({
        "result": { "id": QUERY_TEST_SERVICE_ID, "name": "demo" },
        "status": 200,
        "requestId": "stub-service-get",
    });
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/organizations/org-1/services/{QUERY_TEST_SERVICE_ID}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_service))
        .mount(&mock)
        .await;
    mock
}

async fn start_mock_query_host() -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .mount(&mock)
        .await;
    mock
}

#[tokio::test]
async fn service_query_with_oauth_sends_bearer_and_never_provisions() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host().await;

    // OAuth tokens are the lowest-precedence credential tier, so clear the
    // API key env vars. Tokens now live in the global ~/.clickhouse/tokens.json
    // (issue #277), so point HOME at a temp dir and write them there.
    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    let ch_dir = home_dir.join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    let tokens = serde_json::json!({
        "access_token": "test-bearer-token",
        "refresh_token": "unused",
        "expires_at": 4102444800u64, // 2100-01-01: never expires in tests
        "api_url": format!("{}/v1", control.uri()),
    });
    std::fs::write(
        ch_dir.join("tokens.json"),
        serde_json::to_vec(&tokens).unwrap(),
    )
    .unwrap();

    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("HOME", &home_dir)
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);

    // The query request must carry the OAuth bearer token, not Basic auth,
    // and no `auth-provider: custom` marker (that header means "custom
    // Query API key").
    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 1);
    let run = &query_requests[0];
    let auth = run.headers.get("authorization").unwrap().to_str().unwrap();
    assert_eq!(auth, "Bearer test-bearer-token");
    assert!(
        run.headers.get("auth-provider").is_none(),
        "auth-provider header must not accompany a bearer token",
    );

    // No provisioning: key creation and query-endpoint upsert are both
    // POSTs, so the control plane must see only GETs.
    let control_requests = control.received_requests().await.unwrap();
    assert!(
        control_requests
            .iter()
            .all(|r| r.method == wiremock::http::Method::GET),
        "OAuth service query made non-GET control-plane calls: {:?}",
        control_requests
            .iter()
            .map(|r| format!("{} {}", r.method, r.url.path()))
            .collect::<Vec<_>>(),
    );

    // And no Query API key may be written locally for the OAuth path.
    assert!(
        !ch_dir.join("credentials.json").exists(),
        "OAuth service query wrote .clickhouse/credentials.json",
    );

    // The query result streams through to stdout untouched.
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
}

#[tokio::test]
async fn service_query_with_stored_key_sends_basic_auth_with_that_key() {
    let control = start_mock_control_plane_with_service().await;
    let query_host = start_mock_query_host().await;

    // A stored per-service Query API key short-circuits provisioning; the
    // control-plane creds come from the env tier (the credentials file
    // carries only service_query_keys, no api_key/api_secret).
    let dir = tempfile::tempdir().unwrap();
    let ch_dir = dir.path().join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    let creds = serde_json::json!({
        "service_query_keys": {
            QUERY_TEST_SERVICE_ID: {
                "key_id": "stored-key-id",
                "key_secret": "stored-key-secret",
                "endpoint_id": "ep-1",
                "service_name": "demo",
                "created_at": "2026-05-11T12:00:00Z",
            }
        }
    });
    std::fs::write(
        ch_dir.join("credentials.json"),
        serde_json::to_vec(&creds).unwrap(),
    )
    .unwrap();

    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);

    // The query request authenticates with the stored per-service key, not
    // the org-level env creds, and keeps the custom-key marker header.
    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 1);
    let run = &query_requests[0];
    let auth = run.headers.get("authorization").unwrap().to_str().unwrap();
    let expected = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "stored-key-id:stored-key-secret",
        )
    );
    assert_eq!(auth, expected);
    assert_eq!(run.headers.get("auth-provider").unwrap(), "custom");

    // Stored key present → no provisioning POSTs on the control plane.
    let control_requests = control.received_requests().await.unwrap();
    assert!(
        control_requests
            .iter()
            .all(|r| r.method == wiremock::http::Method::GET),
        "stored-key service query made non-GET control-plane calls: {:?}",
        control_requests
            .iter()
            .map(|r| format!("{} {}", r.method, r.url.path()))
            .collect::<Vec<_>>(),
    );
}

// ── Idled / stopped services (query host 206 protocol) ─────────────────────
//
// An idled or stopped service answers the run request with 206 and
// `{"data": "<state>"}` instead of executing the query. For `Confirm wake
// service` the CLI must resend the query once with the `wake-service: true`
// header (waking the service, like the SQL console does after prompting);
// for `Service is stopped` it must fail with a hint to start the service.

/// Write an OAuth tokens.json into `ch_dir` (the caller's `$HOME/.clickhouse`)
/// so the binary authenticates with a bearer token against the given control
/// plane. Callers must also set `HOME` to the parent of `ch_dir`.
fn write_oauth_tokens(ch_dir: &std::path::Path, control_uri: &str) {
    let tokens = serde_json::json!({
        "access_token": "test-bearer-token",
        "refresh_token": "unused",
        "expires_at": 4102444800u64, // 2100-01-01: never expires in tests
        "api_url": format!("{control_uri}/v1"),
    });
    std::fs::write(
        ch_dir.join("tokens.json"),
        serde_json::to_vec(&tokens).unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn service_query_resends_with_wake_header_when_service_is_idle() {
    let control = start_mock_control_plane_with_service().await;

    // Query host that refuses attempts without the wake confirmation: the
    // header-matched mock (higher priority) runs the query, the fallback
    // answers 206 `Confirm wake service`.
    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .and(header("wake-service", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string("1\n"))
        .with_priority(1)
        .mount(&query_host)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(
            ResponseTemplate::new(206).set_body_string(r#"{"data":"Confirm wake service"}"#),
        )
        .with_priority(5)
        .mount(&query_host)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    let ch_dir = home_dir.join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    write_oauth_tokens(&ch_dir, &control.uri());

    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("HOME", &home_dir)
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .output()
        .expect("failed to spawn clickhousectl");
    assert_success(&output);

    // Exactly two attempts: the refused one without the wake header, then
    // the resend carrying the wake confirmation.
    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 2);
    assert!(
        query_requests[0].headers.get("wake-service").is_none(),
        "first attempt must not pre-emptively wake the service",
    );
    assert_eq!(
        query_requests[1].headers.get("wake-service").unwrap(),
        "true"
    );

    // The 206 body must not leak into the query output, and the user is
    // told about the wake on stderr.
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("idle"),
        "stderr should mention the service is idle:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[tokio::test]
async fn service_query_fails_with_start_hint_when_service_is_stopped() {
    let control = start_mock_control_plane_with_service().await;

    let query_host = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/service/{QUERY_TEST_SERVICE_ID}/run")))
        .respond_with(ResponseTemplate::new(206).set_body_string(r#"{"data":"Service is stopped"}"#))
        .mount(&query_host)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let home_dir = dir.path().join("home");
    let ch_dir = home_dir.join(".clickhouse");
    std::fs::create_dir_all(&ch_dir).unwrap();
    write_oauth_tokens(&ch_dir, &control.uri());

    let url = control.uri();
    let output = Command::new(clickhousectl_binary())
        .args([
            "cloud",
            "--url",
            &url,
            "service",
            "query",
            "--id",
            QUERY_TEST_SERVICE_ID,
            "--org-id",
            "org-1",
            "--query",
            "SELECT 1",
        ])
        .current_dir(dir.path())
        .env("HOME", &home_dir)
        .env_remove("CLICKHOUSE_CLOUD_API_KEY")
        .env_remove("CLICKHOUSE_CLOUD_API_SECRET")
        .env("CLICKHOUSE_CLOUD_QUERY_HOST", query_host.uri())
        .output()
        .expect("failed to spawn clickhousectl");

    assert!(
        !output.status.success(),
        "querying a stopped service must fail\nstdout:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stopped") && stderr.contains("service start"),
        "stderr should say the service is stopped and hint at `service start`:\n{stderr}",
    );

    // A stopped service is never woken: no wake-service resend.
    let query_requests = query_host.received_requests().await.unwrap();
    assert_eq!(query_requests.len(), 1);
}

// Shell env vars must win over `.env` — if both are set, the request is
// signed with the shell values, never the file values.

#[tokio::test]
async fn shell_env_overrides_dotenv_creds_in_request() {
    use std::io::Write;

    let mock = MockServer::start().await;

    let stub_orgs = serde_json::json!({
        "result": [],
        "status": 200,
        "requestId": "stub-org-list",
    });
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_orgs))
        .mount(&mock)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let mut env_file = std::fs::File::create(dir.path().join(".env")).unwrap();
    env_file
        .write_all(b"CLICKHOUSE_CLOUD_API_KEY=dotenv-key\nCLICKHOUSE_CLOUD_API_SECRET=dotenv-secret\n")
        .unwrap();
    drop(env_file);

    let url = mock.uri();
    let output = Command::new(clickhousectl_binary())
        .args(["cloud", "--url", &url, "--json", "org", "list"])
        .current_dir(dir.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "shell-key")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "shell-secret")
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);

    let requests = mock.received_requests().await.unwrap();
    let auth = requests
        .iter()
        .find(|r| r.method == wiremock::http::Method::GET)
        .and_then(|r| r.headers.get("Authorization"))
        .expect("no Authorization header recorded");
    let auth_str = auth.to_str().expect("non-utf8 auth header");
    let expected = format!(
        "Basic {}",
        base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            "shell-key:shell-secret",
        )
    );
    assert_eq!(
        auth_str, expected,
        "shell env vars must override .env values on the wire"
    );
}

// ── Issue #267: agent session/trace headers land on outbound requests ────────
//
// When invoked under a detected AI agent that publishes a session id /
// traceparent to its subprocesses (Claude Code uses CLAUDE_CODE_SESSION_ID;
// TRACEPARENT is the W3C standard var), `clickhousectl` forwards them as the
// `agent-session-id` and `traceparent` request headers via the default headers
// on the shared HTTP client (`crate::http::client_builder`). This proves they
// reach the wire through the client the Cloud library actually uses.

#[tokio::test]
async fn agent_session_and_trace_headers_are_forwarded() {
    let mock = MockServer::start().await;

    let stub_orgs = serde_json::json!({
        "result": [],
        "status": 200,
        "requestId": "stub-org-list",
    });
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(stub_orgs))
        .mount(&mock)
        .await;

    let url = mock.uri();
    let traceparent = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
    let output = Command::new(clickhousectl_binary())
        .args(["cloud", "--url", &url, "--json", "org", "list"])
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        // Mark this invocation as Claude Code and expose the session/trace ids.
        .env("AGENT", "claude-code")
        .env("CLAUDE_CODE_SESSION_ID", "sess-test-267")
        .env("TRACEPARENT", traceparent)
        .output()
        .expect("failed to spawn clickhousectl");

    assert_success(&output);

    let requests = mock
        .received_requests()
        .await
        .expect("mock requests log unavailable");
    let req = requests
        .iter()
        .find(|r| r.method == wiremock::http::Method::GET)
        .expect("no GET request recorded");

    assert_eq!(
        req.headers
            .get("agent-session-id")
            .expect("agent-session-id header missing")
            .to_str()
            .unwrap(),
        "sess-test-267",
    );
    assert_eq!(
        req.headers
            .get("traceparent")
            .expect("traceparent header missing")
            .to_str()
            .unwrap(),
        traceparent,
    );
}
