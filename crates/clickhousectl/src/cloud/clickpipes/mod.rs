mod cli;
mod client;
mod create;
mod destination;
mod manage;
mod schema_discover;
mod settings;
mod sources;
mod table_mapping;
#[cfg(test)]
mod test_support;
mod update;

use super::permissions::{Conditional, Declaration as Permission};
use clickhouse_cloud_api::meta::operations as op;

// Declare every API call made by these workflows, including optional lookups.
pub(super) const PERMISSIONS: &[Permission] = &[
    Permission::api("clickpipe list", &[&op::CLICK_PIPE_GET_LIST]),
    Permission::api("clickpipe get", &[&op::CLICK_PIPE_GET])
        .when(&[Conditional::flag("name", &[&op::CLICK_PIPE_GET_LIST])]),
    Permission::api("clickpipe update", &[&op::CLICK_PIPE_UPDATE])
        .when(&[Conditional::flag("name", &[&op::CLICK_PIPE_GET_LIST])]),
    Permission::api("clickpipe delete", &[&op::CLICK_PIPE_DELETE])
        .when(&[Conditional::flag("name", &[&op::CLICK_PIPE_GET_LIST])]),
    Permission::api("clickpipe start", &[&op::CLICK_PIPE_STATE_UPDATE])
        .when(&[Conditional::flag("name", &[&op::CLICK_PIPE_GET_LIST])]),
    Permission::api("clickpipe stop", &[&op::CLICK_PIPE_STATE_UPDATE])
        .when(&[Conditional::flag("name", &[&op::CLICK_PIPE_GET_LIST])]),
    Permission::api("clickpipe resync", &[&op::CLICK_PIPE_STATE_UPDATE])
        .when(&[Conditional::flag("name", &[&op::CLICK_PIPE_GET_LIST])]),
    Permission::api("clickpipe scale", &[&op::CLICK_PIPE_SCALING_UPDATE])
        .when(&[Conditional::flag("name", &[&op::CLICK_PIPE_GET_LIST])]),
    Permission::api(
        "clickpipe cdc-scaling get",
        &[&op::CLICK_PIPE_CDC_SCALING_GET],
    ),
    Permission::api(
        "clickpipe cdc-scaling update",
        &[&op::CLICK_PIPE_CDC_SCALING_UPDATE],
    ),
    Permission::api(
        "clickpipe settings get",
        &[&op::CLICK_PIPE_GET, &op::CLICK_PIPE_SETTINGS_GET],
    )
    .when(&[Conditional::flag("name", &[&op::CLICK_PIPE_GET_LIST])]),
    Permission::api(
        "clickpipe context get",
        &[&op::CLICK_PIPES_SERVICE_CONTEXT_GET],
    ),
    Permission::api(
        "clickpipe settings update",
        &[&op::CLICK_PIPE_GET, &op::CLICK_PIPE_SETTINGS_UPDATE],
    )
    .when(&[
        Conditional::flag("name", &[&op::CLICK_PIPE_GET_LIST]),
        Conditional::new(
            "Kafka: keep read-committed",
            &[&op::CLICK_PIPE_SETTINGS_GET],
        ),
    ]),
    Permission::api(
        "clickpipe schema-discover kafka",
        &[&op::CLICK_PIPE_SCHEMA_DISCOVERY],
    ),
    Permission::api(
        "clickpipe schema-discover kinesis",
        &[&op::CLICK_PIPE_SCHEMA_DISCOVERY],
    ),
    Permission::api(
        "clickpipe schema-discover object-storage",
        &[&op::CLICK_PIPE_SCHEMA_DISCOVERY],
    ),
    Permission::api(
        "clickpipe schema-discover pubsub",
        &[&op::CLICK_PIPE_SCHEMA_DISCOVERY],
    ),
    Permission::api("clickpipe create object-storage", &[&op::CLICK_PIPE_CREATE]),
    Permission::api("clickpipe create kafka", &[&op::CLICK_PIPE_CREATE]),
    Permission::api("clickpipe create kinesis", &[&op::CLICK_PIPE_CREATE]),
    Permission::api("clickpipe create postgres", &[&op::CLICK_PIPE_CREATE]),
    Permission::api("clickpipe create mysql", &[&op::CLICK_PIPE_CREATE]),
    Permission::api("clickpipe create mongodb", &[&op::CLICK_PIPE_CREATE]),
    Permission::api("clickpipe create bigquery", &[&op::CLICK_PIPE_CREATE]),
    Permission::api("clickpipe create pubsub", &[&op::CLICK_PIPE_CREATE]),
];

use crate::cloud::client::{CloudClient, CloudError, ResourceKind, Result as CloudResult, Scope};
use crate::cloud::shared::{NameSelector, resolve_org_id, select_named_id};
pub use cli::ClickPipeCommands;
use cli::{
    ClickPipeCdcScalingCommands, ClickPipeContextCommands, ClickPipeCreateCommands,
    ClickPipeSettingsCommands,
};
use manage::{
    CdcScalingValues, ClickPipeLifecycleCommand, clickpipe_cdc_scaling_get,
    clickpipe_cdc_scaling_update, clickpipe_context_get, clickpipe_delete, clickpipe_get,
    clickpipe_list, clickpipe_scale, clickpipe_state,
};
use schema_discover::clickpipe_schema_discover;
use settings::{clickpipe_settings_get, clickpipe_settings_update};
use sources::bigquery::clickpipe_create_bigquery;
use sources::kafka::clickpipe_create_kafka;
use sources::kinesis::clickpipe_create_kinesis;
use sources::mongodb::clickpipe_create_mongodb;
use sources::mysql::clickpipe_create_mysql;
use sources::object_storage::clickpipe_create_object_storage;
use sources::postgres::clickpipe_create_postgres;
use sources::pubsub::clickpipe_create_pubsub;
use update::clickpipe_update;

pub async fn run(client: &CloudClient, command: ClickPipeCommands, json: bool) -> CloudResult<()> {
    match command {
        ClickPipeCommands::List { service_id } => clickpipe_list(client, &service_id, json).await,
        ClickPipeCommands::Get {
            service_id,
            clickpipe_id,
        } => {
            clickpipe_get(
                client,
                &service_id,
                &resolve_clickpipe_id(client, &service_id, &clickpipe_id).await?,
                json,
            )
            .await
        }
        ClickPipeCommands::Update {
            service_id,
            clickpipe_id,
            config_file,
        } => {
            clickpipe_update(
                client,
                &service_id,
                &resolve_clickpipe_id(client, &service_id, &clickpipe_id).await?,
                &config_file,
                json,
            )
            .await
        }
        ClickPipeCommands::Delete {
            service_id,
            clickpipe_id,
        } => {
            clickpipe_delete(
                client,
                &service_id,
                &resolve_clickpipe_id(client, &service_id, &clickpipe_id).await?,
                json,
            )
            .await
        }
        ClickPipeCommands::Start {
            service_id,
            clickpipe_id,
        } => {
            clickpipe_state(
                client,
                &service_id,
                &resolve_clickpipe_id(client, &service_id, &clickpipe_id).await?,
                ClickPipeLifecycleCommand::Start,
                json,
            )
            .await
        }
        ClickPipeCommands::Stop {
            service_id,
            clickpipe_id,
        } => {
            clickpipe_state(
                client,
                &service_id,
                &resolve_clickpipe_id(client, &service_id, &clickpipe_id).await?,
                ClickPipeLifecycleCommand::Stop,
                json,
            )
            .await
        }
        ClickPipeCommands::Resync {
            service_id,
            clickpipe_id,
        } => {
            clickpipe_state(
                client,
                &service_id,
                &resolve_clickpipe_id(client, &service_id, &clickpipe_id).await?,
                ClickPipeLifecycleCommand::Resync,
                json,
            )
            .await
        }
        ClickPipeCommands::Scale {
            service_id,
            clickpipe_id,
            replicas,
            cpu_millicores,
            memory_gb,
        } => {
            clickpipe_scale(
                client,
                &service_id,
                &resolve_clickpipe_id(client, &service_id, &clickpipe_id).await?,
                replicas,
                cpu_millicores,
                memory_gb,
                json,
            )
            .await
        }
        ClickPipeCommands::CdcScaling { command } => match command {
            ClickPipeCdcScalingCommands::Get { service_id } => {
                clickpipe_cdc_scaling_get(client, &service_id, json).await
            }
            ClickPipeCdcScalingCommands::Update {
                service_id,
                cpu_millicores,
                memory_gb,
            } => {
                let values = CdcScalingValues {
                    cpu_millicores,
                    memory_gb,
                };
                clickpipe_cdc_scaling_update(client, &service_id, &values, json).await
            }
        },
        ClickPipeCommands::Settings { command } => match command {
            ClickPipeSettingsCommands::Get {
                service_id,
                clickpipe_id,
            } => {
                clickpipe_settings_get(
                    client,
                    &service_id,
                    &resolve_clickpipe_id(client, &service_id, &clickpipe_id).await?,
                    json,
                )
                .await
            }
            ClickPipeSettingsCommands::Update {
                service_id,
                clickpipe_id,
                settings,
            } => {
                clickpipe_settings_update(
                    client,
                    &service_id,
                    &resolve_clickpipe_id(client, &service_id, &clickpipe_id).await?,
                    &settings,
                    json,
                )
                .await
            }
        },
        ClickPipeCommands::Context { command } => match command {
            ClickPipeContextCommands::Get { service_id } => {
                clickpipe_context_get(client, &service_id, json).await
            }
        },
        ClickPipeCommands::SchemaDiscover { command } => {
            clickpipe_schema_discover(client, &command, json).await
        }
        ClickPipeCommands::ReversePrivateEndpoint { command } => {
            crate::cloud::clickpipe_endpoints::run(client, command, json).await
        }
        ClickPipeCommands::Create { command } => match command {
            ClickPipeCreateCommands::ObjectStorage(args) => {
                clickpipe_create_object_storage(client, &args, json).await
            }
            ClickPipeCreateCommands::Kafka(args) => {
                clickpipe_create_kafka(client, &args, json).await
            }
            ClickPipeCreateCommands::Kinesis(args) => {
                clickpipe_create_kinesis(client, &args, json).await
            }
            ClickPipeCreateCommands::Postgres(args) => {
                clickpipe_create_postgres(client, &args, json).await
            }
            ClickPipeCreateCommands::MySQL(args) => {
                clickpipe_create_mysql(client, &args, json).await
            }
            ClickPipeCreateCommands::MongoDB(args) => {
                clickpipe_create_mongodb(client, &args, json).await
            }
            ClickPipeCreateCommands::BigQuery(args) => {
                clickpipe_create_bigquery(client, &args, json).await
            }
            ClickPipeCreateCommands::PubSub(args) => {
                clickpipe_create_pubsub(client, &args, json).await
            }
        },
    }
}

async fn resolve_clickpipe_id(
    client: &CloudClient,
    service_id: &str,
    selector: &NameSelector,
) -> CloudResult<String> {
    let name = match (&selector.id, &selector.name) {
        (Some(id), None) => return Ok(id.clone()),
        (None, Some(name)) => name,
        _ => {
            return Err(CloudError::usage(
                "supply exactly one positional ClickPipe ID or --name",
            ));
        }
    };
    let org = resolve_org_id(client).await?;
    let rows = client.list_clickpipes(&org, service_id).await?;
    select_named_id(
        ResourceKind::ClickPipe,
        Scope::Service {
            org_id: &org,
            service_id,
        },
        name,
        rows.iter().map(|r| (r.name.as_deref(), r.id.as_ref())),
    )
}

/// Which source a fetched pipe reads from, as a closed vocabulary.
///
/// Two decisions hang off this: whether the Kafka-only `kafka_read_committed`
/// setting may appear in a settings PUT at all, and whether the ingestion
/// settings endpoints exist for the pipe in the first place. The endpoints are
/// only implemented for streaming and object-storage pipes, so a database CDC
/// pipe gets `NOT_FOUND: ingestion for pipe "<id>" not found`, which reads like
/// the pipe is gone (#643).
///
/// [`ClickPipeSourceKind::Absent`] covers a response that carries no `source`
/// (or an unrecognized source arm). Settings commands proceed in that case
/// unless the caller explicitly requests a Kafka-only setting: the API remains
/// the authority for the other settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClickPipeSourceKind {
    Kafka,
    Kinesis,
    PubSub,
    ObjectStorage,
    Postgres,
    MySql,
    MongoDb,
    BigQuery,
    Absent,
}

impl ClickPipeSourceKind {
    /// Kafka is the only source whose settings include `kafka_read_committed`.
    fn is_kafka(self) -> bool {
        self == Self::Kafka
    }

    /// The human label for a database CDC source, or `None` for a source that
    /// does have ingestion settings.
    ///
    /// The label is a literal owned by this enum, never anything read out of an
    /// API response body, so no response text can reach the error message.
    fn database_source_label(self) -> Option<&'static str> {
        match self {
            Self::Postgres => Some("Postgres CDC"),
            Self::MySql => Some("MySQL CDC"),
            Self::MongoDb => Some("MongoDB CDC"),
            Self::BigQuery => Some("BigQuery"),
            Self::Kafka | Self::Kinesis | Self::PubSub | Self::ObjectStorage | Self::Absent => None,
        }
    }
}

/// Classify the source of a fetched pipe.
///
/// Exactly one arm of `source` is populated in practice. The streaming and
/// object-storage arms are matched first anyway, so a response that somehow
/// sets several arms resolves to the kind that keeps the settings commands
/// working rather than to a refusal.
fn classify_clickpipe_source(
    clickpipe: &clickhouse_cloud_api::models::ClickPipe,
) -> ClickPipeSourceKind {
    let Some(source) = clickpipe.source.as_ref() else {
        return ClickPipeSourceKind::Absent;
    };
    if source.kafka.is_some() {
        ClickPipeSourceKind::Kafka
    } else if source.kinesis.is_some() {
        ClickPipeSourceKind::Kinesis
    } else if source.pubsub.is_some() {
        ClickPipeSourceKind::PubSub
    } else if source.object_storage.is_some() {
        ClickPipeSourceKind::ObjectStorage
    } else if source.postgres.is_some() {
        ClickPipeSourceKind::Postgres
    } else if source.mysql.is_some() {
        ClickPipeSourceKind::MySql
    } else if source.mongodb.is_some() {
        ClickPipeSourceKind::MongoDb
    } else if source.bigquery.is_some() {
        ClickPipeSourceKind::BigQuery
    } else {
        ClickPipeSourceKind::Absent
    }
}

/// Parse a CLI string into a library enum. Library enums have a
/// `#[serde(untagged)] Unknown(String)` variant so unknown inputs are
/// forwarded to the API (which returns the canonical validation error).
fn parse_enum<T: serde::de::DeserializeOwned>(value: &str) -> CloudResult<T> {
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|error| CloudError::usage(format!("invalid value '{}': {}", value, error)))
}

/// `None` for an empty list, so an optional array field is omitted from the
/// request rather than sent as `[]`.
fn non_empty<T>(items: Vec<T>) -> Option<Vec<T>> {
    (!items.is_empty()).then_some(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_every_clickpipe_source_and_an_absent_one() {
        use clickhouse_cloud_api::models::{
            ClickPipe, ClickPipeBigQuerySource, ClickPipeKafkaSource, ClickPipeKinesisSource,
            ClickPipeMongoDBSource, ClickPipeMySQLSource, ClickPipeObjectStorageSource,
            ClickPipePostgresSource, ClickPipePubSubSource, ClickPipeSource,
        };

        fn pipe(source: ClickPipeSource) -> ClickPipe {
            ClickPipe {
                source: Some(source),
                ..Default::default()
            }
        }

        let cases: Vec<(ClickPipe, ClickPipeSourceKind)> = vec![
            (
                pipe(ClickPipeSource {
                    kafka: Some(ClickPipeKafkaSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::Kafka,
            ),
            (
                pipe(ClickPipeSource {
                    kinesis: Some(ClickPipeKinesisSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::Kinesis,
            ),
            (
                pipe(ClickPipeSource {
                    pubsub: Some(ClickPipePubSubSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::PubSub,
            ),
            (
                pipe(ClickPipeSource {
                    object_storage: Some(ClickPipeObjectStorageSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::ObjectStorage,
            ),
            (
                pipe(ClickPipeSource {
                    postgres: Some(ClickPipePostgresSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::Postgres,
            ),
            (
                pipe(ClickPipeSource {
                    mysql: Some(ClickPipeMySQLSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::MySql,
            ),
            (
                pipe(ClickPipeSource {
                    mongodb: Some(ClickPipeMongoDBSource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::MongoDb,
            ),
            (
                pipe(ClickPipeSource {
                    bigquery: Some(ClickPipeBigQuerySource::default()),
                    ..Default::default()
                }),
                ClickPipeSourceKind::BigQuery,
            ),
            // An absent source, and a source object with no arm set, are both
            // unclassifiable: the API stays the authority for those.
            (ClickPipe::default(), ClickPipeSourceKind::Absent),
            (
                pipe(ClickPipeSource::default()),
                ClickPipeSourceKind::Absent,
            ),
        ];

        for (clickpipe, expected) in &cases {
            assert_eq!(classify_clickpipe_source(clickpipe), *expected);
        }

        // Only Kafka carries the Kafka-only settings key.
        for (clickpipe, expected) in &cases {
            assert_eq!(
                classify_clickpipe_source(clickpipe).is_kafka(),
                *expected == ClickPipeSourceKind::Kafka,
            );
        }
    }
}
