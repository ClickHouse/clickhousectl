use super::cli::ClickPipeSchemaDiscoverCommands;
use super::sources::kafka::build_kafka_source;
use super::sources::kinesis::build_kinesis_source;
use super::sources::object_storage::build_object_storage_schema_discovery_request;
use super::sources::pubsub::build_pubsub_schema_discovery_request;
use crate::cloud::client::{CloudClient, Result as CloudResult};
use crate::cloud::output::or_absent;
use crate::cloud::shared::resolve_org_id;
use tabled::{Table, Tabled, settings::Style};

/// Discover the inferred schema for a Kafka, Kinesis, object-storage or
/// Pub/Sub source without creating a ClickPipe (Beta). Side-effect-free, but
/// the API gateway rejects OAuth/Bearer on this POST endpoint, so it is
/// classified as a write command and requires API key auth.
pub(super) async fn clickpipe_schema_discover(
    client: &CloudClient,
    command: &ClickPipeSchemaDiscoverCommands,
    json: bool,
) -> CloudResult<()> {
    use clickhouse_cloud_api::models::{
        ClickPipeSchemaDiscoveryRequest, ClickPipeSchemaDiscoverySource,
    };

    let (service_id, request) = match command {
        ClickPipeSchemaDiscoverCommands::Kafka(args) => (
            &args.service_id,
            ClickPipeSchemaDiscoveryRequest {
                source: ClickPipeSchemaDiscoverySource {
                    kafka: Some(build_kafka_source(&args.source)?),
                    kinesis: None,
                    object_storage: None,
                    pubsub: None,
                },
            },
        ),
        ClickPipeSchemaDiscoverCommands::Kinesis(args) => (
            &args.service_id,
            ClickPipeSchemaDiscoveryRequest {
                source: ClickPipeSchemaDiscoverySource {
                    kafka: None,
                    kinesis: Some(build_kinesis_source(&args.source)?),
                    object_storage: None,
                    pubsub: None,
                },
            },
        ),
        ClickPipeSchemaDiscoverCommands::ObjectStorage(args) => (
            &args.service_id,
            build_object_storage_schema_discovery_request(&args.source)?,
        ),
        ClickPipeSchemaDiscoverCommands::PubSub(args) => (
            &args.service_id,
            build_pubsub_schema_discovery_request(&args.source)?,
        ),
    };
    let org_id = resolve_org_id(client).await?;
    let response = client
        .click_pipe_schema_discovery(&org_id, service_id, &request)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        let rows = schema_discovery_rows(response.fields.unwrap_or_default());
        if rows.is_empty() {
            println!("No fields discovered");
        } else {
            println!("{}", Table::new(rows).with(Style::markdown()));
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Tabled)]
struct SchemaDiscoveryRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Type")]
    r#type: String,
    #[tabled(rename = "Optional")]
    optional: String,
    #[tabled(rename = "Virtual")]
    is_virtual: String,
    #[tabled(rename = "Disabled By Default")]
    is_virtual_disabled_default: String,
}

/// One table row per discovered field; an absent flag renders as an empty cell.
fn schema_discovery_rows(
    fields: Vec<clickhouse_cloud_api::models::ClickPipeSchemaDiscoveryField>,
) -> Vec<SchemaDiscoveryRow> {
    let flag = |value: Option<bool>| value.map(|value| value.to_string()).unwrap_or_default();
    fields
        .into_iter()
        .map(|field| SchemaDiscoveryRow {
            name: or_absent(field.name),
            r#type: or_absent(field.r#type),
            optional: flag(field.optional),
            is_virtual: flag(field.is_virtual),
            is_virtual_disabled_default: flag(field.is_virtual_disabled_default),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_discovery_rows_show_virtual_flags_and_leave_absent_flags_empty() {
        use clickhouse_cloud_api::models::ClickPipeSchemaDiscoveryField;
        let rows = schema_discovery_rows(vec![
            ClickPipeSchemaDiscoveryField {
                name: Some("_topic".into()),
                r#type: Some("String".into()),
                optional: Some(false),
                is_virtual: Some(true),
                is_virtual_disabled_default: Some(true),
            },
            ClickPipeSchemaDiscoveryField {
                name: Some("user_id".into()),
                r#type: Some("Int64".into()),
                optional: Some(true),
                is_virtual: Some(false),
                is_virtual_disabled_default: Some(false),
            },
            ClickPipeSchemaDiscoveryField::default(),
        ]);
        let cells = |row: &SchemaDiscoveryRow| {
            [
                row.name.clone(),
                row.r#type.clone(),
                row.optional.clone(),
                row.is_virtual.clone(),
                row.is_virtual_disabled_default.clone(),
            ]
        };
        assert_eq!(
            cells(&rows[0]),
            ["_topic", "String", "false", "true", "true"]
        );
        assert_eq!(
            cells(&rows[1]),
            ["user_id", "Int64", "true", "false", "false"]
        );
        assert_eq!(cells(&rows[2]), ["-", "-", "", "", ""]);
        assert_eq!(rows.len(), 3);
    }
}
