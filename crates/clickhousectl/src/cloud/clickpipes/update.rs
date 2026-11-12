use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::config::{config_source_label, deserialize_strict_config, read_config_value};
use crate::cloud::output::print_human;
use crate::cloud::shared::resolve_org_id;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KafkaPatchCredentialKind {
    Plain,
    IamUser,
    AzureEventHub,
    MutualTls,
}

fn parse_kafka_patch_credentials(
    credentials: &serde_json::Value,
    source: &str,
) -> CloudResult<KafkaPatchCredentialKind> {
    use clickhouse_cloud_api::models::{AzureEventHub, MskIamUser, MutualTLS, PLAIN};

    let variants = [
        (
            KafkaPatchCredentialKind::Plain,
            deserialize_strict_config::<PLAIN>(credentials.clone(), source).is_ok(),
        ),
        (
            KafkaPatchCredentialKind::IamUser,
            deserialize_strict_config::<MskIamUser>(credentials.clone(), source).is_ok(),
        ),
        (
            KafkaPatchCredentialKind::AzureEventHub,
            deserialize_strict_config::<AzureEventHub>(credentials.clone(), source).is_ok(),
        ),
        (
            KafkaPatchCredentialKind::MutualTls,
            deserialize_strict_config::<MutualTLS>(credentials.clone(), source).is_ok(),
        ),
    ];
    let mut matches = variants
        .into_iter()
        .filter_map(|(kind, matched)| matched.then_some(kind));
    match (matches.next(), matches.next()) {
        (Some(kind), None) => Ok(kind),
        _ => Err(CloudError::usage(format!(
            "invalid request body in config {source}: `source.kafka.credentials` must be exactly one supported credential object"
        ))),
    }
}

fn invalid_kafka_patch_auth(config_source: &str, message: &str) -> CloudError {
    CloudError::usage(format!(
        "invalid request body in config {config_source}: {message}"
    ))
}

fn require_patch_object_fields(
    value: &serde_json::Value,
    path: &str,
    fields: &[&str],
    config_source: &str,
) -> CloudResult<()> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    let missing = fields
        .iter()
        .filter(|field| !object.contains_key(**field))
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(CloudError::usage(format!(
            "invalid request body in config {config_source}: `{path}` is missing required field{} {}",
            if missing.len() == 1 { "" } else { "s" },
            missing
                .iter()
                .map(|field| format!("`{field}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

fn validate_clickpipe_patch_required_fields(
    value: &serde_json::Value,
    config_source: &str,
) -> CloudResult<()> {
    let Some(source) = value.get("source").and_then(serde_json::Value::as_object) else {
        return Ok(());
    };

    if source.contains_key("bigquery") {
        return Err(CloudError::usage(format!(
            "invalid request body in config {config_source}: `source.bigquery` cannot be updated by the ClickPipes API"
        )));
    }

    if let Some(mysql) = source.get("mysql") {
        require_patch_object_fields(mysql, "source.mysql", &["host", "port"], config_source)?;
        if let Some(mappings) = mysql
            .get("tableMappingsToRemove")
            .and_then(serde_json::Value::as_array)
        {
            for mapping in mappings {
                require_patch_object_fields(
                    mapping,
                    "source.mysql.tableMappingsToRemove[]",
                    &["sourceSchemaName", "sourceTable", "targetTable"],
                    config_source,
                )?;
            }
        }
    }

    if let Some(mongodb) = source.get("mongodb") {
        require_patch_object_fields(
            mongodb,
            "source.mongodb",
            &["uri", "readPreference"],
            config_source,
        )?;
        if let Some(mappings) = mongodb
            .get("tableMappingsToRemove")
            .and_then(serde_json::Value::as_array)
        {
            for mapping in mappings {
                require_patch_object_fields(
                    mapping,
                    "source.mongodb.tableMappingsToRemove[]",
                    &["sourceDatabaseName", "sourceCollection", "targetTable"],
                    config_source,
                )?;
            }
        }
    }

    if let Some(pubsub) = source.get("pubsub") {
        require_patch_object_fields(pubsub, "source.pubsub", &["authentication"], config_source)?;
    }

    Ok(())
}

fn validate_clickpipe_patch_source(
    patch: &clickhouse_cloud_api::models::ClickPipePatchSource,
    config_source: &str,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickPipeMongoDBPipeTableMappingTableengine as MongoAddEngine,
        ClickPipeMySQLPipeTableMappingTableengine as MySqlAddEngine,
        ClickPipePatchKafkaSourceAuthentication as KafkaAuth,
        ClickPipePatchKinesisSourceAuthentication as KinesisAuth,
        ClickPipePatchMongoDBPipeRemoveTableMappingTableengine as MongoRemoveEngine,
        ClickPipePatchMongoDBSourceReadpreference as MongoReadPreference,
        ClickPipePatchMySQLPipeRemoveTableMappingTableengine as MySqlRemoveEngine,
        ClickPipePatchMySQLSourceAuthentication as MySqlAuth,
        ClickPipePatchObjectStorageSourceAuthentication as ObjectStorageAuth,
        ClickPipePatchPostgresPipeRemoveTableMappingTableengine as PostgresRemoveEngine,
        ClickPipePatchPubSubSourceAuthentication as PubSubAuth,
        ClickPipePostgresPipeTableMappingTableengine as PostgresAddEngine,
    };

    let selected_sources = [
        patch.kafka.is_some(),
        patch.kinesis.is_some(),
        patch.object_storage.is_some(),
        patch.pubsub.is_some(),
        patch.postgres.is_some(),
        patch.mysql.is_some(),
        patch.mongodb.is_some(),
    ]
    .into_iter()
    .filter(|selected| *selected)
    .count();
    if selected_sources > 1 {
        return Err(CloudError::usage(format!(
            "invalid request body in config {config_source}: `source` can update at most one provider"
        )));
    }

    if let Some(source) = &patch.kafka {
        if let Some(KafkaAuth::Unknown(value)) = &source.authentication {
            return Err(CloudError::usage(format!(
                "invalid request body in config {config_source}: unknown `source.kafka.authentication` value `{value}`"
            )));
        }
        let credentials = source
            .credentials
            .as_ref()
            .map(|credentials| parse_kafka_patch_credentials(credentials, config_source))
            .transpose()?;
        match source.authentication.as_ref() {
            None => {}
            Some(KafkaAuth::PLAIN) => {
                if credentials.is_some_and(|kind| {
                    !matches!(
                        kind,
                        KafkaPatchCredentialKind::Plain | KafkaPatchCredentialKind::AzureEventHub
                    )
                }) || source.iam_role.is_some()
                {
                    return Err(invalid_kafka_patch_auth(
                        config_source,
                        "PLAIN authentication accepts only username/password or Event Hubs credentials",
                    ));
                }
            }
            Some(KafkaAuth::SCRAM_SHA_256 | KafkaAuth::SCRAM_SHA_512) => {
                if credentials.is_some_and(|kind| kind != KafkaPatchCredentialKind::Plain)
                    || source.iam_role.is_some()
                {
                    return Err(invalid_kafka_patch_auth(
                        config_source,
                        "SCRAM authentication accepts only username/password credentials",
                    ));
                }
            }
            Some(KafkaAuth::IAM_USER) => {
                if credentials.is_some_and(|kind| kind != KafkaPatchCredentialKind::IamUser)
                    || source.iam_role.is_some()
                {
                    return Err(invalid_kafka_patch_auth(
                        config_source,
                        "IAM_USER authentication accepts only access-key credentials",
                    ));
                }
            }
            Some(KafkaAuth::IAM_ROLE) => {
                if source.iam_role.is_none() || credentials.is_some() {
                    return Err(invalid_kafka_patch_auth(
                        config_source,
                        "IAM_ROLE authentication requires `iamRole` and no `credentials`",
                    ));
                }
            }
            Some(KafkaAuth::MUTUAL_TLS) => {
                if credentials.is_some_and(|kind| kind != KafkaPatchCredentialKind::MutualTls)
                    || source.iam_role.is_some()
                {
                    return Err(invalid_kafka_patch_auth(
                        config_source,
                        "MUTUAL_TLS authentication accepts only certificate/private-key credentials",
                    ));
                }
            }
            Some(KafkaAuth::ServiceAccountWorkloadIdentity) => {
                if source.iam_role.is_some() || credentials.is_some() {
                    return Err(invalid_kafka_patch_auth(
                        config_source,
                        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY accepts neither `iamRole` nor `credentials`",
                    ));
                }
            }
            Some(KafkaAuth::Unknown(_)) => unreachable!("unknown authentication rejected above"),
        }
    }

    if let Some(source) = &patch.kinesis
        && let Some(KinesisAuth::Unknown(value)) = &source.authentication
    {
        return Err(CloudError::usage(format!(
            "invalid request body in config {config_source}: unknown `source.kinesis.authentication` value `{value}`"
        )));
    }

    if let Some(source) = &patch.object_storage
        && let Some(ObjectStorageAuth::Unknown(value)) = &source.authentication
    {
        return Err(CloudError::usage(format!(
            "invalid request body in config {config_source}: unknown `source.objectStorage.authentication` value `{value}`"
        )));
    }

    if let Some(source) = &patch.pubsub
        && let Some(PubSubAuth::Unknown(value)) = &source.authentication
    {
        return Err(CloudError::usage(format!(
            "invalid request body in config {config_source}: unknown `source.pubsub.authentication` value `{value}`"
        )));
    }

    if let Some(source) = &patch.postgres {
        validate_clickpipe_patch_port(source.port, "source.postgres.port", config_source)?;
        if let Some(mappings) = &source.table_mappings_to_add {
            for mapping in mappings {
                if let PostgresAddEngine::Unknown(value) = &mapping.table_engine {
                    return Err(CloudError::usage(format!(
                        "invalid request body in config {config_source}: unknown `source.postgres.tableMappingsToAdd[].tableEngine` value `{value}`"
                    )));
                }
            }
        }
        if let Some(mappings) = &source.table_mappings_to_remove {
            for mapping in mappings {
                if let Some(PostgresRemoveEngine::Unknown(value)) = &mapping.table_engine {
                    return Err(CloudError::usage(format!(
                        "invalid request body in config {config_source}: unknown `source.postgres.tableMappingsToRemove[].tableEngine` value `{value}`"
                    )));
                }
            }
        }
    }

    if let Some(source) = &patch.mysql {
        validate_clickpipe_patch_port(source.port, "source.mysql.port", config_source)?;
        if let Some(MySqlAuth::Unknown(value)) = &source.authentication {
            return Err(CloudError::usage(format!(
                "invalid request body in config {config_source}: unknown `source.mysql.authentication` value `{value}`"
            )));
        }
        if let Some(mappings) = &source.table_mappings_to_add {
            for mapping in mappings {
                if let Some(MySqlAddEngine::Unknown(value)) = &mapping.table_engine {
                    return Err(CloudError::usage(format!(
                        "invalid request body in config {config_source}: unknown `source.mysql.tableMappingsToAdd[].tableEngine` value `{value}`"
                    )));
                }
            }
        }
        if let Some(mappings) = &source.table_mappings_to_remove {
            for mapping in mappings {
                if let Some(MySqlRemoveEngine::Unknown(value)) = &mapping.table_engine {
                    return Err(CloudError::usage(format!(
                        "invalid request body in config {config_source}: unknown `source.mysql.tableMappingsToRemove[].tableEngine` value `{value}`"
                    )));
                }
            }
        }
    }

    if let Some(source) = &patch.mongodb {
        if let Some(MongoReadPreference::Unknown(value)) = &source.read_preference {
            return Err(CloudError::usage(format!(
                "invalid request body in config {config_source}: unknown `source.mongodb.readPreference` value `{value}`"
            )));
        }
        if let Some(mappings) = &source.table_mappings_to_add {
            for mapping in mappings {
                if let Some(MongoAddEngine::Unknown(value)) = &mapping.table_engine {
                    return Err(CloudError::usage(format!(
                        "invalid request body in config {config_source}: unknown `source.mongodb.tableMappingsToAdd[].tableEngine` value `{value}`"
                    )));
                }
            }
        }
        if let Some(mappings) = &source.table_mappings_to_remove {
            for mapping in mappings {
                if let Some(MongoRemoveEngine::Unknown(value)) = &mapping.table_engine {
                    return Err(CloudError::usage(format!(
                        "invalid request body in config {config_source}: unknown `source.mongodb.tableMappingsToRemove[].tableEngine` value `{value}`"
                    )));
                }
            }
        }
    }

    Ok(())
}

fn validate_clickpipe_patch_port(
    port: Option<i64>,
    path: &str,
    config_source: &str,
) -> CloudResult<()> {
    if port.is_some_and(|port| !(1..=u16::MAX.into()).contains(&port)) {
        return Err(CloudError::usage(format!(
            "invalid request body in config {config_source}: `{path}` must be in the range 1..=65535"
        )));
    }
    Ok(())
}

fn build_clickpipe_update_request(
    value: serde_json::Value,
    config_source: &str,
) -> CloudResult<clickhouse_cloud_api::models::ClickPipePatchRequest> {
    validate_clickpipe_patch_required_fields(&value, config_source)?;
    let request: clickhouse_cloud_api::models::ClickPipePatchRequest =
        deserialize_strict_config(value, config_source)?;
    if let Some(source) = &request.source {
        validate_clickpipe_patch_source(source, config_source)?;
    }

    // The service accepts root `settings` on this PATCH but does not apply
    // them: a settings-only body fails and a mixed body silently drops them
    // (#824). Refuse them until ClickHouse/control-plane#44988 lands, then
    // remove this check and restore `settings` to the no-op check below.
    // Source-level `settings` (Postgres, MySQL, MongoDB) are applied and stay
    // accepted.
    if request.settings.is_some() {
        return Err(CloudError::usage(format!(
            "invalid request body in config {config_source}: root `settings` cannot be \
             changed by `clickpipe update`; use `clickpipe settings update` instead"
        )));
    }

    if request.name.is_none()
        && request.source.is_none()
        && request.destination.is_none()
        && request.field_mappings.is_none()
    {
        return Err(CloudError::usage(format!(
            "invalid request body in config {config_source}: at least one PATCH field is required"
        )));
    }

    Ok(request)
}

pub(super) async fn clickpipe_update(
    client: &CloudClient,
    service_id: &str,
    clickpipe_id: &str,
    config_file: &str,
    json: bool,
) -> CloudResult<()> {
    let config_source = config_source_label(config_file);
    let request = build_clickpipe_update_request(read_config_value(config_file)?, config_source)?;
    let org_id = resolve_org_id(client).await?;
    let clickpipe = client
        .update_clickpipe(&org_id, service_id, clickpipe_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&clickpipe)?);
    } else {
        print_human(&clickpipe)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::clickpipes::cli::ClickPipeCommands;
    use crate::cloud::clickpipes::test_support::{assert_names, assert_rejected, parse_clickpipe};

    #[test]
    fn primary_json_file_argument_contract() {
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "clickpipe", "update", "svc-1", "pipe-1"],
            "config_file",
            &["config-file"],
        );
    }

    #[test]
    fn clickpipe_update_parses_file_and_stdin_paths() {
        for config_file in ["patch.json", "-"] {
            let ClickPipeCommands::Update {
                service_id,
                clickpipe_id,
                config_file: parsed_file,
            } = parse_clickpipe(&[
                "update",
                "svc-1",
                "pipe-1",
                "--file",
                config_file,
                "--org-id",
                "org-1",
            ])
            else {
                panic!("expected clickpipe update");
            };
            assert_eq!(service_id, "svc-1");
            assert_eq!(clickpipe_id.id.as_deref(), Some("pipe-1"));
            assert_eq!(parsed_file, config_file);
        }
        assert_rejected(&["update", "svc-1", "pipe-1"]);
    }

    #[test]
    fn clickpipe_update_builder_preserves_minimal_and_full_patch_shapes() {
        let minimal = serde_json::json!({"name": ""});
        let request = build_clickpipe_update_request(minimal.clone(), "test").unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), minimal);

        let full = serde_json::json!({
            "name": "renamed",
            "destination": {
                "columns": [
                    {"name": "id", "type": "UInt64"},
                    {"name": "payload", "type": "String"}
                ]
            },
            "fieldMappings": [
                {"sourceField": "event_id", "destinationField": "id"}
            ],
            "source": {
                "mysql": {
                    "authentication": "basic",
                    "credentials": {"username": "rotated", "password": "secret"},
                    "host": "mysql.example.com",
                    "port": 3306,
                    "tlsHost": "mysql-tls.example.com",
                    "caCertificate": "certificate",
                    "disableTls": false,
                    "skipCertVerification": false,
                    "serverId": 0,
                    "settings": {
                        "syncIntervalSeconds": 0,
                        "pullBatchSize": 0,
                        "useCompression": false
                    },
                    "tableMappingsToAdd": [{
                        "sourceSchemaName": "sales",
                        "sourceTable": "orders",
                        "targetTable": "orders_v2",
                        "excludedColumns": [],
                        "useCustomSortingKey": false,
                        "sortingKeys": [],
                        "tableEngine": "ReplacingMergeTree",
                        "partitionKey": "id",
                        "partitionByExpr": "toYYYYMM(created_at)"
                    }],
                    "tableMappingsToRemove": [{
                        "sourceSchemaName": "sales",
                        "sourceTable": "old_orders",
                        "targetTable": "old_orders",
                        "tableEngine": "MergeTree",
                        "partitionKey": "id",
                        "partitionByExpr": "toYYYYMM(created_at)"
                    }]
                },
                "validateSamples": false
            }
        });
        let request = build_clickpipe_update_request(full.clone(), "test").unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), full);

        for partial in [
            serde_json::json!({"source": {"kafka": {"caCertificate": "new-ca"}}}),
            serde_json::json!({"source": {"kafka": {
                "authentication": "IAM_ROLE",
                "iamRole": "arn:aws:iam::123456789012:role/clickpipe"
            }}}),
            serde_json::json!({"source": {"kafka": {
                "authentication": "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"
            }}}),
            serde_json::json!({"source": {"postgres": {
                "host": "postgres.example.com"
            }}}),
        ] {
            let request = build_clickpipe_update_request(partial.clone(), "test").unwrap();
            assert_eq!(
                serde_json::to_value(request).unwrap(),
                partial,
                "omitted PATCH fields must remain absent"
            );
        }
    }

    /// Each PATCH source arm survives the strict parse and serializes back
    /// exactly, including explicit zeros, `false` and empty lists.
    #[test]
    fn clickpipe_update_builder_round_trips_every_source_variant() {
        for patch in [
            serde_json::json!({
                "source": {
                    "kafka": {
                        "authentication": "MUTUAL_TLS",
                        "caCertificate": "ca",
                        "credentials": {"certificate": "cert", "privateKey": "key"},
                        "reversePrivateEndpointIds": []
                    },
                    "validateSamples": false
                }
            }),
            serde_json::json!({"source": {
                "kinesis": {
                    "authentication": "IAM_USER",
                    "accessKey": {"accessKeyId": "key-id", "secretKey": "secret"}
                },
                "validateSamples": true
            }}),
            serde_json::json!({"source": {
                "objectStorage": {
                    "authentication": "SERVICE_ACCOUNT",
                    "serviceAccountKey": "base64-key",
                    "skipInitialLoad": false,
                    "startAfter": "events/2026-06-01/",
                    "path": "events/*.json"
                },
                "validateSamples": false
            }}),
            serde_json::json!({"source": {
                "pubsub": {
                    "authentication": "SERVICE_ACCOUNT",
                    "ackDeadline": 10,
                    "serviceAccountKey": {"serviceAccountFile": "base64-key"}
                },
                "validateSamples": true
            }}),
            serde_json::json!({"source": {
                "postgres": {
                    "credentials": {"username": "rotated", "password": "secret"},
                    "host": "postgres.example.com",
                    "port": 5432,
                    "database": "source_db",
                    "disableTls": false,
                    "skipCertVerification": false,
                    "settings": {"syncIntervalSeconds": 0, "pullBatchSize": 0},
                    "tableMappingsToAdd": [{
                        "sourceSchemaName": "public",
                        "sourceTable": "events",
                        "targetTable": "events",
                        "excludedColumns": [],
                        "useCustomSortingKey": false,
                        "sortingKeys": [],
                        "tableEngine": "ReplacingMergeTree",
                        "partitionKey": "id",
                        "partitionByExpr": "toYYYYMM(ts)"
                    }],
                    "tableMappingsToRemove": [{
                        "sourceSchemaName": "public",
                        "sourceTable": "old_events",
                        "targetTable": "old_events"
                    }]
                },
                "validateSamples": false
            }}),
            serde_json::json!({"source": {
                "mongodb": {
                    "credentials": {"username": "rotated", "password": "secret"},
                    "uri": "mongodb+srv://mongo.example/source",
                    "readPreference": "secondaryPreferred",
                    "disableTls": false,
                    "skipCertVerification": false,
                    "settings": {"syncIntervalSeconds": 0, "pullBatchSize": 0},
                    "tableMappingsToAdd": [],
                    "tableMappingsToRemove": [{
                        "sourceDatabaseName": "source",
                        "sourceCollection": "old_events",
                        "targetTable": "old_events",
                        "tableEngine": "Null"
                    }]
                },
                "validateSamples": false
            }}),
        ] {
            let request = build_clickpipe_update_request(patch.clone(), "test").unwrap();
            assert_eq!(serde_json::to_value(request).unwrap(), patch);
        }
    }

    #[test]
    fn clickpipe_update_builder_validates_typed_database_source_ports() {
        for provider in ["mysql", "postgres"] {
            for port in [1, 65535] {
                let patch = serde_json::json!({"source": {
                    (provider): {"host": "db.example.com", "port": port}
                }});
                let request = build_clickpipe_update_request(patch.clone(), "test").unwrap();
                assert_eq!(serde_json::to_value(request).unwrap(), patch);
            }

            for port in [-1, 0, 65536] {
                let patch = serde_json::json!({"source": {
                    (provider): {"host": "db.example.com", "port": port}
                }});
                let error = build_clickpipe_update_request(patch, "patch.json").unwrap_err();
                assert_names(
                    &error.message,
                    &[
                        "patch.json",
                        &format!("`source.{provider}.port`"),
                        "1..=65535",
                    ],
                );
            }
        }

        let omitted = serde_json::json!({"source": {
            "postgres": {"host": "db.example.com"}
        }});
        let request = build_clickpipe_update_request(omitted.clone(), "test").unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), omitted);
    }

    #[test]
    fn clickpipe_update_builder_reports_bigquery_as_an_unsupported_source() {
        let error = build_clickpipe_update_request(
            serde_json::json!({"source": {
                "bigquery": {},
                "validateSamples": false
            }}),
            "test",
        )
        .unwrap_err();
        assert_names(&error.message, &["`source.bigquery`"]);
        assert!(!error.message.contains(".?."), "{error}");
        // The dedicated rejection, not the generic unknown-field one that the
        // typed body would also produce: the pre-deserialization check refuses
        // BigQuery while it lets another unknown source key through to serde.
        assert!(
            validate_clickpipe_patch_required_fields(
                &serde_json::json!({"source": {"bigquery": {}}}),
                "test"
            )
            .is_err()
        );
        assert!(
            validate_clickpipe_patch_required_fields(
                &serde_json::json!({"source": {"warehouse": {}}}),
                "test"
            )
            .is_ok()
        );

        // The error names the config source and the full field path, with no
        // serde path placeholders.
        let unknown = build_clickpipe_update_request(
            serde_json::json!({"source": {"kafka": {"unknownOption": true}}}),
            crate::cloud::config::config_source_label("-"),
        )
        .unwrap_err();
        assert_names(
            &unknown.message,
            &[
                crate::cloud::config::config_source_label("-"),
                "`source.kafka.unknownOption`",
            ],
        );
        assert!(!unknown.message.contains(".?"), "{unknown}");
    }

    #[test]
    fn clickpipe_update_builder_rejects_noop_unknowns_and_invalid_sources() {
        for invalid in [
            serde_json::json!({}),
            serde_json::json!({"bigquery": {}}),
            serde_json::json!({"destination": {}}),
            serde_json::json!({"fieldMappings": [{"sourceFiled": "id", "destinationField": "id"}]}),
            serde_json::json!({"source": {
                "kinesis": {"authentication": "FUTURE_AUTH"},
                "validateSamples": false
            }}),
            serde_json::json!({"source": {
                "mongodb": {
                    "uri": "mongodb://example/db",
                    "readPreference": "futurePreference"
                },
                "validateSamples": false
            }}),
            serde_json::json!({"source": {
                "kafka": {
                    "credentials": {"username": "user", "password": "pw", "extra": true},
                    "reversePrivateEndpointIds": []
                },
                "validateSamples": false
            }}),
            serde_json::json!({"source": {"kafka": {
                "authentication": "PLAIN",
                "credentials": {"accessKeyId": "key", "secretKey": "secret"}
            }}}),
            serde_json::json!({"source": {"kafka": {
                "authentication": "IAM_ROLE",
                "iamRole": "arn:aws:iam::123456789012:role/clickpipe",
                "credentials": {"username": "user", "password": "secret"}
            }}}),
            serde_json::json!({"source": {"kafka": {
                "authentication": "IAM_ROLE"
            }}}),
            serde_json::json!({"source": {"kafka": {
                "authentication": "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
                "credentials": {"username": "user", "password": "secret"}
            }}}),
            serde_json::json!({"source": {"mysql": {
                "settings": {"syncIntervalSeconds": 5}
            }}}),
            serde_json::json!({"source": {"mongodb": {
                "settings": {"syncIntervalSeconds": 5}
            }}}),
            serde_json::json!({"source": {"pubsub": {
                "ackDeadline": 10
            }}}),
            serde_json::json!({"source": {
                "kinesis": {},
                "objectStorage": {},
                "validateSamples": false
            }}),
            serde_json::json!({"destination": {"colums": []}}),
            serde_json::json!({"source": {
                "kafka": {
                    "credentials": {"username": "user", "password": "pw", "token": "bad"},
                    "reversePrivateEndpointIds": []
                },
                "validateSamples": false
            }}),
            serde_json::json!({"source": {
                "mysql": {"host": "db", "port": 3306, "authentication": "future"},
                "validateSamples": false
            }}),
        ] {
            let error = build_clickpipe_update_request(invalid.clone(), "patch.json")
                .expect_err(&format!("accepted invalid patch: {invalid}"));
            // Every rejection is attributed to the config source it came from.
            assert!(error.message.contains("patch.json"), "{invalid}: {error}");
        }
    }

    /// Root `settings` are refused (#824) whether alone or mixed with another
    /// field, and the error points to the command that applies them. Null root
    /// settings and source-level settings are unaffected.
    #[test]
    fn clickpipe_update_builder_rejects_root_settings() {
        for patch in [
            serde_json::json!({"settings": {"streaming_max_insert_wait_ms": 0}}),
            serde_json::json!({
                "name": "renamed",
                "settings": {"object_storage_max_insert_bytes": 10_737_418_240_i64}
            }),
            serde_json::json!({"settings": {}}),
        ] {
            let error = build_clickpipe_update_request(patch.clone(), "patch.json")
                .expect_err(&format!("accepted root settings: {patch}"));
            assert_eq!(
                error.kind,
                crate::cloud::client::CloudErrorKind::Usage,
                "{patch}"
            );
            assert_names(
                &error.message,
                &["patch.json", "`settings`", "clickpipe settings update"],
            );
        }

        let request = build_clickpipe_update_request(
            serde_json::json!({"name": "renamed", "settings": null}),
            "test",
        )
        .unwrap();
        assert!(request.settings.is_none());

        let source_settings = serde_json::json!({"source": {"postgres": {
            "host": "postgres.example.com",
            "settings": {"syncIntervalSeconds": 30}
        }}});
        let request = build_clickpipe_update_request(source_settings.clone(), "test").unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), source_settings);
    }
}
