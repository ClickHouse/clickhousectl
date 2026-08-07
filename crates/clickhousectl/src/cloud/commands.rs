use crate::cloud::client::CloudClient;
use crate::cloud::output::{or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use tabled::{Table, Tabled, settings::Style};

pub async fn clickpipe_list(
    client: &CloudClient,
    service_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let clickpipes = client.list_clickpipes(&org_id, service_id).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipes)?);
    } else if clickpipes.is_empty() {
        println!("No ClickPipes found");
    } else {
        println!("ClickPipes:");
        for cp in &clickpipes {
            println!(
                "  {} ({}) - {}",
                or_absent(cp.name.as_deref()),
                or_absent(cp.id.as_ref()),
                or_absent(cp.state.as_ref())
            );
        }
    }
    Ok(())
}

pub async fn clickpipe_create_s3(
    client: &CloudClient,
    args: &crate::cloud::cli::ObjectStorageCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipePostObjectStorageSource, ClickPipePostObjectStorageSourceAuthentication,
        ClickPipePostRequest, ClickPipePostSource, MskIamUser,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let parsed_columns = parse_columns(&args.columns)?;

    let (authentication, iam_role_val, access_key) = match (
        args.iam_role.as_deref(),
        args.access_key_id.as_deref(),
        args.secret_key.as_deref(),
    ) {
        (Some(role), _, _) => (
            Some(ClickPipePostObjectStorageSourceAuthentication::IAM_ROLE),
            Some(role.to_string()),
            None,
        ),
        (_, Some(key_id), Some(secret)) => (
            Some(ClickPipePostObjectStorageSourceAuthentication::IAM_USER),
            None,
            Some(MskIamUser {
                access_key_id: key_id.to_string(),
                secret_key: secret.to_string(),
            }),
        ),
        _ => (None, None, None),
    };
    let authentication = authentication
        .or_else(|| {
            args.connection_string
                .as_ref()
                .map(|_| ClickPipePostObjectStorageSourceAuthentication::CONNECTION_STRING)
        })
        .or_else(|| {
            args.service_account_file
                .as_ref()
                .map(|_| ClickPipePostObjectStorageSourceAuthentication::SERVICE_ACCOUNT)
        });

    let service_account_key = match args.service_account_file.as_deref() {
        Some(path) => Some(read_gcp_service_account_file(path)?),
        None => None,
    };

    let source = ClickPipePostObjectStorageSource {
        r#type: parse_enum(&args.storage_type)?,
        format: parse_enum(&args.format)?,
        url: args.source_url.clone(),
        compression: Some(parse_enum(&args.compression)?),
        is_continuous: if args.continuous { Some(true) } else { None },
        queue_url: args.queue_url.clone(),
        delimiter: args.delimiter.clone(),
        authentication,
        iam_role: iam_role_val,
        access_key,
        connection_string: args.connection_string.clone(),
        azure_container_name: args.azure_container_name.clone(),
        path: args.path.clone(),
        service_account_key,
        skip_initial_load: if args.skip_initial_load {
            Some(true)
        } else {
            None
        },
        start_after: args.start_after.clone(),
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            object_storage: Some(source),
            ..Default::default()
        },
        destination: build_destination(&args.database, &args.table, parsed_columns),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

/// Build the Kafka `credentials` JSON body, whose shape is a `oneOf` determined
/// by the auth mode (see the `ClickPipePostKafkaSource.credentials` schema).
/// IAM_ROLE sends a null body — the role ARN flows through the separate
/// top-level `iamRole` field on the source, not through credentials.
///
/// `mtls_contents` is the pre-read (certificate, privateKey) PEM bundle used
/// only for MUTUAL_TLS; the caller reads these from disk so this function
/// stays pure and testable.
fn build_kafka_credentials(
    authentication: &clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication,
    args: &crate::cloud::cli::KafkaSourceFields,
    mtls_contents: Option<(String, String)>,
) -> Result<serde_json::Value, String> {
    use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
    match authentication {
        Auth::PLAIN | Auth::SCRAM_SHA_256 | Auth::SCRAM_SHA_512 => {
            match (args.username.as_deref(), args.password.as_deref()) {
                (Some(u), Some(p)) => Ok(serde_json::json!({ "username": u, "password": p })),
                _ => Err(format!(
                    "{} requires --username and --password",
                    args.auth.as_deref().unwrap_or("PLAIN")
                )),
            }
        }
        Auth::IAM_USER => match (args.access_key_id.as_deref(), args.secret_key.as_deref()) {
            (Some(k), Some(s)) => Ok(serde_json::json!({ "accessKeyId": k, "secretKey": s })),
            _ => Err("IAM_USER requires --access-key-id and --secret-key".into()),
        },
        Auth::IAM_ROLE => {
            if args.iam_role.is_none() {
                Err("IAM_ROLE requires --iam-role".into())
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        Auth::MUTUAL_TLS => match mtls_contents {
            Some((cert, key)) => Ok(serde_json::json!({ "certificate": cert, "privateKey": key })),
            None => Err("MUTUAL_TLS requires --client-certificate and --client-key".into()),
        },
        Auth::Unknown(_) => Ok(serde_json::Value::Null),
    }
}

/// Build a `ClickPipePostKafkaSource` from the CLI args, performing all
/// authentication/credential/schema-registry/CA validation up front so bad
/// invocations fail fast before any network call. Shared by the
/// `clickpipe create kafka` and `clickpipe schema-discover <SERVICE_ID> kafka`
/// handlers.
fn build_kafka_source(
    args: &crate::cloud::cli::KafkaSourceFields,
) -> Result<clickhouse_cloud_api::models::ClickPipePostKafkaSource, Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeKafkaOffset, ClickPipeKafkaSchemaRegistryCredentials,
        ClickPipeMutateKafkaSchemaRegistry, ClickPipePostKafkaSource,
        ClickPipePostKafkaSourceAuthentication,
    };

    let authentication: ClickPipePostKafkaSourceAuthentication = match args.auth.as_deref() {
        Some(a) => parse_enum(a)?,
        None => ClickPipePostKafkaSourceAuthentication::default(),
    };

    let mtls_cert_contents = match (
        &authentication,
        args.client_certificate.as_deref(),
        args.client_key.as_deref(),
    ) {
        (ClickPipePostKafkaSourceAuthentication::MUTUAL_TLS, Some(cert_path), Some(key_path)) => {
            Some((
                std::fs::read_to_string(cert_path)?,
                std::fs::read_to_string(key_path)?,
            ))
        }
        _ => None,
    };
    let credentials = build_kafka_credentials(&authentication, args, mtls_cert_contents)?;

    let schema_registry = args
        .schema_registry_url
        .as_ref()
        .map(|url| -> Result<_, Box<dyn std::error::Error>> {
            let creds = match (
                args.schema_registry_username.as_deref(),
                args.schema_registry_password.as_deref(),
            ) {
                (Some(u), Some(p)) => ClickPipeKafkaSchemaRegistryCredentials {
                    username: u.to_string(),
                    password: p.to_string(),
                },
                _ => ClickPipeKafkaSchemaRegistryCredentials::default(),
            };
            let ca_cert = match args.schema_registry_ca_certificate.as_deref() {
                Some(path) => Some(std::fs::read_to_string(path)?),
                None => None,
            };
            Ok(ClickPipeMutateKafkaSchemaRegistry {
                url: url.clone(),
                authentication: Default::default(),
                credentials: creds,
                ca_certificate: ca_cert,
            })
        })
        .transpose()?;

    let ca_cert_contents = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    Ok(ClickPipePostKafkaSource {
        r#type: parse_enum(&args.kafka_type)?,
        format: parse_enum(&args.format)?,
        brokers: args.brokers.clone(),
        topics: args.topics.clone(),
        consumer_group: args.consumer_group.clone(),
        exactly_once: None,
        authentication,
        credentials,
        iam_role: args.iam_role.clone(),
        offset: Some(ClickPipeKafkaOffset {
            strategy: parse_enum(&args.offset)?,
            timestamp: args.offset_timestamp.clone(),
        }),
        schema_registry,
        ca_certificate: ca_cert_contents,
        reverse_private_endpoint_ids: args.reverse_private_endpoint_ids.clone(),
    })
}

/// Build a `ClickPipePostKinesisSource` from the CLI args. Shared by the
/// `clickpipe create kinesis` and `clickpipe schema-discover <SERVICE_ID> kinesis`
/// handlers.
fn build_kinesis_source(
    args: &crate::cloud::cli::KinesisSourceFields,
) -> Result<clickhouse_cloud_api::models::ClickPipePostKinesisSource, Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{ClickPipePostKinesisSource, MskIamUser};

    let access_key = match (args.access_key_id.as_deref(), args.secret_key.as_deref()) {
        (Some(k), Some(s)) => Some(MskIamUser {
            access_key_id: k.to_string(),
            secret_key: s.to_string(),
        }),
        _ => None,
    };

    Ok(ClickPipePostKinesisSource {
        format: parse_enum(&args.format)?,
        stream_name: args.stream_name.clone(),
        region: args.region.clone(),
        authentication: parse_enum(&args.auth)?,
        iam_role: args.iam_role.clone(),
        access_key,
        use_enhanced_fan_out: if args.enhanced_fan_out {
            Some(true)
        } else {
            None
        },
        iterator_type: parse_enum(&args.iterator_type)?,
        timestamp: args
            .iterator_timestamp
            .map(|t| {
                i64::try_from(t).map_err(|_| format!("--iterator-timestamp {t} is out of range"))
            })
            .transpose()?,
    })
}

pub async fn clickpipe_create_kafka(
    client: &CloudClient,
    args: &crate::cloud::cli::KafkaCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    // Validate args and build the source before any network call so bad
    // invocations fail fast.
    let parsed_columns = parse_columns(&args.columns)?;
    let source = build_kafka_source(&args.source)?;

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            kafka: Some(source),
            ..Default::default()
        },
        destination: build_destination(&args.database, &args.table, parsed_columns),
        ..Default::default()
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

pub async fn clickpipe_create_kinesis(
    client: &CloudClient,
    args: &crate::cloud::cli::KinesisCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{ClickPipePostRequest, ClickPipePostSource};

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let parsed_columns = parse_columns(&args.columns)?;
    let source = build_kinesis_source(&args.source)?;

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            kinesis: Some(source),
            ..Default::default()
        },
        destination: build_destination(&args.database, &args.table, parsed_columns),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

/// Discover the inferred schema for a Kafka or Kinesis source without creating
/// a ClickPipe (Beta). Side-effect-free, but the API gateway rejects
/// OAuth/Bearer on this POST endpoint, so it is classified as a write command
/// and requires API key auth.
pub async fn clickpipe_schema_discover(
    client: &CloudClient,
    service_id: &str,
    command: &crate::cloud::cli::ClickPipeSchemaDiscoverCommands,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeSchemaDiscoveryRequest, ClickPipeSchemaDiscoverySource,
    };

    let source = match command {
        crate::cloud::cli::ClickPipeSchemaDiscoverCommands::Kafka(args) => {
            ClickPipeSchemaDiscoverySource {
                kafka: Some(build_kafka_source(args)?),
                kinesis: None,
            }
        }
        crate::cloud::cli::ClickPipeSchemaDiscoverCommands::Kinesis(args) => {
            ClickPipeSchemaDiscoverySource {
                kafka: None,
                kinesis: Some(build_kinesis_source(args)?),
            }
        }
    };

    let request = ClickPipeSchemaDiscoveryRequest { source };
    let org_id = resolve_org_id(client, org_id).await?;
    let response = client
        .click_pipe_schema_discovery(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        #[derive(Tabled)]
        struct Row {
            #[tabled(rename = "Name")]
            name: String,
            #[tabled(rename = "Type")]
            r#type: String,
            #[tabled(rename = "Optional")]
            optional: String,
        }
        let rows: Vec<Row> = response
            .fields
            .unwrap_or_default()
            .into_iter()
            .map(|f| Row {
                name: or_absent(f.name),
                r#type: or_absent(f.r#type),
                optional: match f.optional {
                    Some(true) => "true".to_string(),
                    Some(false) => "false".to_string(),
                    None => "".to_string(),
                },
            })
            .collect();
        if rows.is_empty() {
            println!("No fields discovered");
        } else {
            println!("{}", Table::new(rows).with(Style::markdown()));
        }
    }
    Ok(())
}

pub async fn clickpipe_get(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
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

pub async fn clickpipe_delete(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    client
        .delete_clickpipe(&org_id, service_id, clickpipe_id)
        .await?;

    if json {
        println!("{}", serde_json::json!({ "deleted": clickpipe_id }));
    } else {
        println!("ClickPipe {} deleted", clickpipe_id);
    }
    Ok(())
}

pub async fn clickpipe_state(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    command: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::ClickPipeStatePatchRequestCommand;
    let cmd = match command {
        "start" => ClickPipeStatePatchRequestCommand::Start,
        "stop" => ClickPipeStatePatchRequestCommand::Stop,
        "resync" => ClickPipeStatePatchRequestCommand::Resync,
        other => return Err(format!("Unknown state command: {}", other).into()),
    };
    let org_id = resolve_org_id(client, org_id).await?;
    let clickpipe = client
        .change_clickpipe_state(&org_id, service_id, clickpipe_id, cmd)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        println!(
            "ClickPipe {} {} (state: {})",
            or_absent(clickpipe.name.as_deref()),
            command,
            or_absent(clickpipe.state.as_ref())
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn clickpipe_scale(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    replicas: Option<u32>,
    cpu_millicores: Option<u32>,
    memory_gb: Option<f64>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = clickhouse_cloud_api::models::ClickPipeScalingPatchRequest {
        replicas: replicas.map(i64::from),
        replica_cpu_millicores: cpu_millicores.map(i64::from),
        replica_memory_gb: memory_gb,
        #[cfg(feature = "deprecated-fields")]
        concurrency: None,
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

pub async fn clickpipe_settings_get(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
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

#[allow(clippy::too_many_arguments)]
pub async fn clickpipe_settings_update(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    streaming_max_insert_wait_ms: Option<u32>,
    object_storage_concurrency: Option<u32>,
    object_storage_polling_interval_ms: Option<u32>,
    object_storage_max_insert_bytes: Option<u64>,
    object_storage_max_file_count: Option<u32>,
    clickhouse_max_threads: Option<u32>,
    clickhouse_max_insert_threads: Option<u32>,
    object_storage_use_cluster_function: Option<bool>,
    clickhouse_parallel_view_processing: Option<bool>,
    org_id: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let org_id = resolve_org_id(client, org_id).await?;
    let request = clickhouse_cloud_api::models::ClickPipeSettingsPutRequest {
        streaming_max_insert_wait_ms: streaming_max_insert_wait_ms.map(i64::from),
        object_storage_concurrency: object_storage_concurrency.map(i64::from),
        object_storage_polling_interval_ms: object_storage_polling_interval_ms.map(i64::from),
        object_storage_max_insert_bytes: object_storage_max_insert_bytes.map(|v| v as i64),
        object_storage_max_file_count: object_storage_max_file_count.map(i64::from),
        clickhouse_max_threads: clickhouse_max_threads.map(i64::from),
        clickhouse_max_insert_threads: clickhouse_max_insert_threads.map(i64::from),
        object_storage_use_cluster_function,
        clickhouse_parallel_view_processing,
        clickhouse_max_download_threads: None,
        clickhouse_min_insert_block_size_bytes: None,
        clickhouse_parallel_distributed_insert_select: None,
    };
    let settings = client
        .update_clickpipe_settings(&org_id, service_id, clickpipe_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&settings)?);
    } else {
        println!("ClickPipe settings updated");
        let value = serde_json::to_value(&settings)?;
        if let Some(obj) = value.as_object() {
            for (key, val) in obj {
                if !val.is_null() {
                    println!("  {}: {}", key, val);
                }
            }
        }
    }
    Ok(())
}

/// Parse a CLI string into a library enum. Library enums have a
/// `#[serde(untagged)] Unknown(String)` variant so unknown inputs are
/// forwarded to the API (which returns the canonical validation error).
fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, String> {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .map_err(|e| format!("invalid value '{}': {}", s, e))
}

/// Parse `name:type` column specifications into library destination columns.
fn parse_columns(
    columns: &[String],
) -> Result<Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>, String> {
    columns
        .iter()
        .map(|col| {
            let (name, col_type) = col
                .split_once(':')
                .ok_or_else(|| format!("Invalid column format '{}': expected name:type", col))?;
            Ok(clickhouse_cloud_api::models::ClickPipeDestinationColumn {
                name: name.to_string(),
                r#type: col_type.to_string(),
            })
        })
        .collect()
}

/// Build a managed-table destination with the default MergeTree engine.
fn build_destination(
    database: &str,
    table: &str,
    columns: Vec<clickhouse_cloud_api::models::ClickPipeDestinationColumn>,
) -> clickhouse_cloud_api::models::ClickPipeMutateDestination {
    // Database pipes (Postgres/MySQL/BigQuery) carry the destination table on
    // the per-mapping `targetTable` and reject any of {table, managedTable,
    // tableDefinition, columns} at the top level. Detect that case via empty
    // `table` and emit a destination with only `database` populated.
    if table.is_empty() {
        return clickhouse_cloud_api::models::ClickPipeMutateDestination {
            database: database.to_string(),
            ..Default::default()
        };
    }
    clickhouse_cloud_api::models::ClickPipeMutateDestination {
        database: database.to_string(),
        table: Some(table.to_string()),
        columns,
        managed_table: Some(true),
        roles: None,
        table_definition: Some(
            clickhouse_cloud_api::models::ClickPipeDestinationTableDefinition::default(),
        ),
    }
}

/// Read a GCP service-account JSON key file from disk and return the
/// base64-encoded contents. Used by both the object-storage and BigQuery
/// `create` handlers — the upstream API wants the encoded blob regardless
/// of which source it ends up on.
fn read_gcp_service_account_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        contents.as_bytes(),
    ))
}

/// Print the standard "created" confirmation for any create_* handler.
fn print_created(
    clickpipe: &clickhouse_cloud_api::models::ClickPipe,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
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

/// Parse `schema.table:target_table` mappings into (schema, table, target) tuples.
/// Source-specific handlers map these into their own TableMapping struct.
fn parse_db_table_mappings(mappings: &[String]) -> Result<Vec<(String, String, String)>, String> {
    mappings
        .iter()
        .map(|m| {
            let (source, target) = m.split_once(':').ok_or_else(|| {
                format!(
                    "Invalid table mapping '{}': expected schema.table:target_table",
                    m
                )
            })?;
            let (schema, table) = source
                .split_once('.')
                .ok_or_else(|| format!("Invalid source '{}': expected schema.table", source))?;
            Ok((schema.to_string(), table.to_string(), target.to_string()))
        })
        .collect()
}

pub async fn clickpipe_create_postgres(
    client: &CloudClient,
    args: &crate::cloud::cli::PostgresCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeMutatePostgresSource, ClickPipePostRequest, ClickPipePostSource,
        ClickPipePostgresPipeSettings, ClickPipePostgresPipeTableMapping, PLAIN,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let mappings = parse_db_table_mappings(&args.table_mappings)?;

    let ca_cert_contents = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let pg_mappings = mappings
        .into_iter()
        .map(|(schema, t, target)| ClickPipePostgresPipeTableMapping {
            source_schema_name: schema,
            source_table: t,
            target_table: target,
            ..Default::default()
        })
        .collect();

    let source = ClickPipeMutatePostgresSource {
        r#type: Some(parse_enum(&args.postgres_type)?),
        credentials: PLAIN {
            username: args.username.clone(),
            password: args.password.clone(),
        },
        host: args.host.clone(),
        port: i64::from(args.port),
        database: args.pg_database.clone(),
        disable_tls: false,
        skip_cert_verification: false,
        authentication: parse_enum(&args.auth)?,
        iam_role: args.iam_role.clone(),
        tls_host: args.tls_host.clone(),
        ca_certificate: ca_cert_contents,
        settings: ClickPipePostgresPipeSettings {
            replication_mode: parse_enum(&args.replication_mode)?,
            publication_name: args.publication_name.clone(),
            replication_slot_name: args.replication_slot_name.clone(),
            ..Default::default()
        },
        table_mappings: pg_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            postgres: Some(source),
            ..Default::default()
        },
        destination: build_destination("default", "", vec![]),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

pub async fn clickpipe_create_mysql(
    client: &CloudClient,
    args: &crate::cloud::cli::MySqlCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeMutateMySQLSource, ClickPipeMySQLPipeSettings, ClickPipeMySQLPipeTableMapping,
        ClickPipePostRequest, ClickPipePostSource, PLAIN,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let mappings = parse_db_table_mappings(&args.table_mappings)?;

    let ca_cert_contents = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let mysql_mappings = mappings
        .into_iter()
        .map(|(schema, t, target)| ClickPipeMySQLPipeTableMapping {
            source_schema_name: schema,
            source_table: t,
            target_table: target,
            ..Default::default()
        })
        .collect();

    let source = ClickPipeMutateMySQLSource {
        r#type: Some(parse_enum(&args.mysql_type)?),
        credentials: Some(PLAIN {
            username: args.username.clone(),
            password: args.password.clone(),
        }),
        host: args.host.clone(),
        port: i64::from(args.port),
        authentication: Some(parse_enum(&args.auth)?),
        iam_role: args.iam_role.clone(),
        tls_host: args.tls_host.clone(),
        ca_certificate: ca_cert_contents,
        disable_tls: if args.disable_tls { Some(true) } else { None },
        skip_cert_verification: if args.skip_cert_verification {
            Some(true)
        } else {
            None
        },
        server_id: args.server_id.map(|v| v as i64),
        settings: ClickPipeMySQLPipeSettings {
            replication_mode: parse_enum(&args.replication_mode)?,
            replication_mechanism: Some(parse_enum(&args.replication_mechanism)?),
            ..Default::default()
        },
        table_mappings: mysql_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            mysql: Some(source),
            ..Default::default()
        },
        destination: build_destination("default", "", vec![]),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

pub async fn clickpipe_create_mongodb(
    client: &CloudClient,
    args: &crate::cloud::cli::MongoDbCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeMongoDBPipeSettings, ClickPipeMongoDBPipeTableMapping,
        ClickPipeMutateMongoDBSource, ClickPipePostRequest, ClickPipePostSource, PLAIN,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;

    // MongoDB uses `database.collection:target_table` format.
    let mongo_mappings: Vec<ClickPipeMongoDBPipeTableMapping> = args
        .table_mappings
        .iter()
        .map(|m| {
            let (source, target) = m.split_once(':').ok_or_else(|| {
                format!(
                    "Invalid table mapping '{}': expected database.collection:target_table",
                    m
                )
            })?;
            let (db, collection) = source.split_once('.').ok_or_else(|| {
                format!("Invalid source '{}': expected database.collection", source)
            })?;
            Ok(ClickPipeMongoDBPipeTableMapping {
                source_database_name: db.to_string(),
                source_collection: collection.to_string(),
                target_table: target.to_string(),
                table_engine: None,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let ca_cert_contents = match args.ca_certificate.as_deref() {
        Some(path) => Some(std::fs::read_to_string(path)?),
        None => None,
    };

    let source = ClickPipeMutateMongoDBSource {
        credentials: Some(PLAIN {
            username: args.username.clone(),
            password: args.password.clone(),
        }),
        uri: args.uri.clone(),
        read_preference: parse_enum(&args.read_preference)?,
        tls_host: args.tls_host.clone(),
        ca_certificate: ca_cert_contents,
        disable_tls: if args.disable_tls { Some(true) } else { None },
        skip_cert_verification: None,
        settings: ClickPipeMongoDBPipeSettings {
            replication_mode: parse_enum(&args.replication_mode)?,
            ..Default::default()
        },
        table_mappings: mongo_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            mongodb: Some(source),
            ..Default::default()
        },
        destination: build_destination("default", "", vec![]),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

pub async fn clickpipe_create_bigquery(
    client: &CloudClient,
    args: &crate::cloud::cli::BigQueryCreateArgs,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use clickhouse_cloud_api::models::{
        ClickPipeBigQueryPipeSettings, ClickPipeBigQueryPipeTableMapping,
        ClickPipeMutateBigQuerySource, ClickPipePostRequest, ClickPipePostSource, ServiceAccount,
    };

    let org_id = resolve_org_id(client, args.org_id.as_deref()).await?;
    let sa_b64 = read_gcp_service_account_file(&args.service_account_file)?;

    // BigQuery uses `dataset.table:target_table` format.
    let bq_mappings: Vec<ClickPipeBigQueryPipeTableMapping> = args
        .table_mappings
        .iter()
        .map(|m| {
            let (source, target) = m.split_once(':').ok_or_else(|| {
                format!(
                    "Invalid table mapping '{}': expected dataset.table:target_table",
                    m
                )
            })?;
            let (dataset, t) = source
                .split_once('.')
                .ok_or_else(|| format!("Invalid source '{}': expected dataset.table", source))?;
            Ok(ClickPipeBigQueryPipeTableMapping {
                source_dataset_name: dataset.to_string(),
                source_table: t.to_string(),
                target_table: target.to_string(),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let source = ClickPipeMutateBigQuerySource {
        credentials: ServiceAccount {
            service_account_file: sa_b64,
        },
        snapshot_staging_path: args.staging_path.clone(),
        settings: ClickPipeBigQueryPipeSettings {
            replication_mode: parse_enum("snapshot")?,
            ..Default::default()
        },
        table_mappings: bq_mappings,
    };

    let request = ClickPipePostRequest {
        name: args.name.clone(),
        source: ClickPipePostSource {
            bigquery: Some(source),
            ..Default::default()
        },
        destination: build_destination("default", "", vec![]),
        ..Default::default()
    };

    let clickpipe = client
        .create_clickpipe(&org_id, &args.service_id, &request)
        .await?;
    print_created(&clickpipe, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_kinesis_source_rejects_out_of_range_iterator_timestamp() {
        let args = crate::cloud::cli::KinesisSourceFields {
            stream_name: "stream".to_string(),
            region: "us-east-1".to_string(),
            format: "JSONEachRow".to_string(),
            auth: "IAM_ROLE".to_string(),
            iam_role: None,
            access_key_id: None,
            secret_key: None,
            iterator_type: "AT_TIMESTAMP".to_string(),
            iterator_timestamp: Some(u64::MAX),
            enhanced_fan_out: false,
        };
        let err = build_kinesis_source(&args).unwrap_err();
        assert!(
            err.to_string().contains("out of range"),
            "error should mention the range: {}",
            err
        );

        let args = crate::cloud::cli::KinesisSourceFields {
            iterator_timestamp: Some(1_750_000_000),
            ..args
        };
        let source = build_kinesis_source(&args).unwrap();
        assert_eq!(source.timestamp, Some(1_750_000_000));
    }

    #[test]
    fn parse_db_table_mappings_valid() {
        let mappings = vec![
            "public.users:public_users".to_string(),
            "schema1.orders:schema1_orders".to_string(),
        ];
        let result = super::parse_db_table_mappings(&mappings).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            ("public".into(), "users".into(), "public_users".into())
        );
        assert_eq!(
            result[1],
            ("schema1".into(), "orders".into(), "schema1_orders".into())
        );
    }

    #[test]
    fn parse_db_table_mappings_missing_colon() {
        let mappings = vec!["public.users".to_string()];
        let result = super::parse_db_table_mappings(&mappings);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("expected schema.table:target_table")
        );
    }

    #[test]
    fn parse_db_table_mappings_missing_dot() {
        let mappings = vec!["users:target".to_string()];
        let result = super::parse_db_table_mappings(&mappings);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected schema.table"));
    }

    #[test]
    fn parse_db_table_mappings_empty() {
        let mappings: Vec<String> = vec![];
        let result = super::parse_db_table_mappings(&mappings).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_enum_known_variant() {
        use clickhouse_cloud_api::models::ClickPipePostObjectStorageSourceFormat;
        let format: ClickPipePostObjectStorageSourceFormat =
            super::parse_enum("JSONEachRow").unwrap();
        assert_eq!(format, ClickPipePostObjectStorageSourceFormat::JSONEachRow);
    }

    #[test]
    fn parse_enum_unknown_falls_through() {
        // Unknown values map to the catch-all Unknown(String) variant —
        // forwarded to the API which returns the canonical validation error.
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceType;
        let kafka_type: ClickPipePostKafkaSourceType =
            super::parse_enum("not-a-real-type").unwrap();
        assert_eq!(
            kafka_type,
            ClickPipePostKafkaSourceType::Unknown("not-a-real-type".to_string())
        );
    }

    #[test]
    fn parse_enum_preserves_rename_spellings() {
        // Enums use `#[serde(rename = "s3")]` etc. — wire format is authoritative.
        use clickhouse_cloud_api::models::{
            ClickPipePostKafkaSourceAuthentication, ClickPipePostObjectStorageSourceType,
        };
        let ty: ClickPipePostObjectStorageSourceType = super::parse_enum("s3").unwrap();
        assert_eq!(ty, ClickPipePostObjectStorageSourceType::S3);
        let auth: ClickPipePostKafkaSourceAuthentication =
            super::parse_enum("SCRAM-SHA-256").unwrap();
        assert_eq!(auth, ClickPipePostKafkaSourceAuthentication::SCRAM_SHA_256);
    }

    #[test]
    fn parse_columns_valid() {
        let cols = vec!["id:Int64".to_string(), "name:String".to_string()];
        let parsed = super::parse_columns(&cols).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "id");
        assert_eq!(parsed[0].r#type, "Int64");
        assert_eq!(parsed[1].name, "name");
        assert_eq!(parsed[1].r#type, "String");
    }

    #[test]
    fn parse_columns_missing_colon_errors() {
        let cols = vec!["id_without_type".to_string()];
        let err = super::parse_columns(&cols).unwrap_err();
        assert!(err.contains("expected name:type"));
    }

    #[test]
    fn build_destination_uses_defaults_for_table_definition() {
        let dest = super::build_destination("mydb", "events", vec![]);
        assert_eq!(dest.database, "mydb");
        assert_eq!(dest.table.as_deref(), Some("events"));
        assert_eq!(dest.managed_table, Some(true));
        // Default table engine is MergeTree, not something else.
        assert_eq!(
            dest.table_definition
                .as_ref()
                .expect("non-database pipe gets a tableDefinition")
                .engine
                .r#type,
            clickhouse_cloud_api::models::ClickPipeDestinationTableEngineType::MergeTree
        );
    }

    // `build_kafka_credentials` tests — lock the wire shape for each auth mode.
    // Authoritative source: `ClickPipePostKafkaSource.credentials` in
    // `crates/clickhouse-cloud-api/clickhouse_cloud_openapi.json`.

    fn kafka_args() -> crate::cloud::cli::KafkaCreateArgs {
        crate::cloud::cli::KafkaCreateArgs {
            service_id: "svc".into(),
            name: "pipe".into(),
            source: crate::cloud::cli::KafkaSourceFields {
                brokers: "b:9092".into(),
                topics: "t".into(),
                format: "JSONEachRow".into(),
                kafka_type: "kafka".into(),
                consumer_group: None,
                auth: None,
                username: None,
                password: None,
                iam_role: None,
                access_key_id: None,
                secret_key: None,
                offset: "from_beginning".into(),
                offset_timestamp: None,
                schema_registry_url: None,
                schema_registry_username: None,
                schema_registry_password: None,
                ca_certificate: None,
                client_certificate: None,
                client_key: None,
                schema_registry_ca_certificate: None,
                reverse_private_endpoint_ids: vec![],
            },
            database: "d".into(),
            table: "t".into(),
            columns: vec![],
            org_id: None,
        }
    }

    #[test]
    fn kafka_credentials_plain_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("PLAIN".into());
        args.source.username = Some("u".into());
        args.source.password = Some("p".into());
        let creds = super::build_kafka_credentials(&Auth::PLAIN, &args.source, None).unwrap();
        assert_eq!(creds["username"], "u");
        assert_eq!(creds["password"], "p");
    }

    #[test]
    fn kafka_credentials_iam_user_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("IAM_USER".into());
        args.source.access_key_id = Some("AKIA".into());
        args.source.secret_key = Some("secret".into());
        let creds = super::build_kafka_credentials(&Auth::IAM_USER, &args.source, None).unwrap();
        // MskIamUser wire shape is {accessKeyId, secretKey} — NOT snake_case.
        assert_eq!(creds["accessKeyId"], "AKIA");
        assert_eq!(creds["secretKey"], "secret");
        assert!(creds.get("access_key_id").is_none());
    }

    #[test]
    fn kafka_credentials_iam_role_is_null() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("IAM_ROLE".into());
        args.source.iam_role = Some("arn:aws:iam::123:role/Foo".into());
        // IAM_ROLE sends credentials=null; the role ARN flows through the
        // top-level `iamRole` field on the Kafka source, not credentials.
        let creds = super::build_kafka_credentials(&Auth::IAM_ROLE, &args.source, None).unwrap();
        assert!(creds.is_null());
    }

    #[test]
    fn kafka_credentials_mutual_tls_shape() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let args = kafka_args();
        let contents = Some(("CERT_PEM".into(), "KEY_PEM".into()));
        let creds =
            super::build_kafka_credentials(&Auth::MUTUAL_TLS, &args.source, contents).unwrap();
        assert_eq!(creds["certificate"], "CERT_PEM");
        assert_eq!(creds["privateKey"], "KEY_PEM");
    }

    #[test]
    fn kafka_credentials_iam_user_missing_args_errors() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let args = kafka_args();
        let err = super::build_kafka_credentials(&Auth::IAM_USER, &args.source, None).unwrap_err();
        assert!(err.contains("--access-key-id"));
    }

    #[test]
    fn kafka_credentials_iam_role_missing_arn_errors() {
        use clickhouse_cloud_api::models::ClickPipePostKafkaSourceAuthentication as Auth;
        let mut args = kafka_args();
        args.source.auth = Some("IAM_ROLE".into());
        let err = super::build_kafka_credentials(&Auth::IAM_ROLE, &args.source, None).unwrap_err();
        assert!(err.contains("--iam-role"));
    }
}
