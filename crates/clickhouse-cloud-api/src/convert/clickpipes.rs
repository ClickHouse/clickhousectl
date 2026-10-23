use super::MissingRequiredFields;
use crate::models::{
    ClickPipeKafkaGlueSchemaRegistry, ClickPipeKafkaGlueSchemaRegistryResponse,
    ClickPipeKinesisSchemaRegistry, ClickPipeKinesisSchemaRegistryResponse,
    ClickPipeMongoDBTableConfig, ClickPipeMongoDBTableConfigResponse,
};

impl TryFrom<ClickPipeKafkaGlueSchemaRegistryResponse> for ClickPipeKafkaGlueSchemaRegistry {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickPipeKafkaGlueSchemaRegistryResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.r#type.is_none() {
            missing.push("type");
        }
        if value.glue_region.is_none() {
            missing.push("glueRegion");
        }
        if value.glue_registry_name.is_none() {
            missing.push("glueRegistryName");
        }
        if !missing.is_empty() {
            return Err(MissingRequiredFields::new(missing));
        }

        Ok(Self {
            r#type: value.r#type.expect("checked above"),
            glue_region: value.glue_region.expect("checked above"),
            glue_registry_name: value.glue_registry_name.expect("checked above"),
            glue_role_arn: value.glue_role_arn,
        })
    }
}

impl TryFrom<ClickPipeKinesisSchemaRegistryResponse> for ClickPipeKinesisSchemaRegistry {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickPipeKinesisSchemaRegistryResponse) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.r#type.is_none() {
            missing.push("type");
        }
        if value.glue_region.is_none() {
            missing.push("glueRegion");
        }
        if value.glue_registry_name.is_none() {
            missing.push("glueRegistryName");
        }
        if !missing.is_empty() {
            return Err(MissingRequiredFields::new(missing));
        }

        Ok(Self {
            r#type: value.r#type.expect("checked above"),
            glue_region: value.glue_region.expect("checked above"),
            glue_registry_name: value.glue_registry_name.expect("checked above"),
            glue_role_arn: value.glue_role_arn,
        })
    }
}

impl TryFrom<ClickPipeMongoDBTableConfigResponse> for ClickPipeMongoDBTableConfig {
    type Error = MissingRequiredFields;

    fn try_from(value: ClickPipeMongoDBTableConfigResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            persist_deleted_documents: value
                .persist_deleted_documents
                .ok_or_else(|| MissingRequiredFields::new(vec!["persistDeletedDocuments"]))?,
        })
    }
}

impl TryFrom<crate::models::ClickPipeBigQueryPipeSettingsResponse>
    for crate::models::ClickPipeBigQueryPipeSettings
{
    type Error = MissingRequiredFields;

    fn try_from(
        value: crate::models::ClickPipeBigQueryPipeSettingsResponse,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            replication_mode: value
                .replication_mode
                .ok_or_else(|| MissingRequiredFields::new(vec!["replicationMode"]))?,
            allow_nullable_columns: value.allow_nullable_columns,
            initial_load_parallelism: value.initial_load_parallelism,
            snapshot_num_rows_per_partition: value.snapshot_num_rows_per_partition,
            snapshot_number_of_parallel_tables: value.snapshot_number_of_parallel_tables,
            pull_window_size_seconds: value.pull_window_size_seconds,
            query_cdc_pull_sync_parallelism: value.query_cdc_pull_sync_parallelism,
            replication_method: value.replication_method,
            sync_cron: value.sync_cron,
            sync_delay_seconds: value.sync_delay_seconds,
            sync_interval_seconds: value.sync_interval_seconds,
        })
    }
}

impl TryFrom<crate::models::ClickPipeBigQueryPipeTableMappingResponse>
    for crate::models::ClickPipeBigQueryPipeTableMapping
{
    type Error = MissingRequiredFields;

    fn try_from(
        value: crate::models::ClickPipeBigQueryPipeTableMappingResponse,
    ) -> Result<Self, Self::Error> {
        let mut missing = Vec::new();
        if value.source_dataset_name.is_none() {
            missing.push("sourceDatasetName");
        }
        if value.source_table.is_none() {
            missing.push("sourceTable");
        }
        if value.target_table.is_none() {
            missing.push("targetTable");
        }
        if !missing.is_empty() {
            return Err(MissingRequiredFields::new(missing));
        }
        Ok(Self {
            source_dataset_name: value.source_dataset_name.expect("checked above"),
            source_table: value.source_table.expect("checked above"),
            target_table: value.target_table.expect("checked above"),
            excluded_columns: value.excluded_columns,
            sorting_keys: value.sorting_keys,
            table_engine: value.table_engine,
            use_custom_sorting_key: value.use_custom_sorting_key,
            events_function: value.events_function,
            query_cdc_watermark_column: value.query_cdc_watermark_column,
        })
    }
}
