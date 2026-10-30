use super::{
    ClickPipePostKafkaSource, ClickPipePostKinesisSource, ClickPipePostObjectStorageSource,
    ClickPipePostPubSubSource,
};
use serde::{Deserialize, Serialize};

/// `ClickPipeSchemaDiscoveryField` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSchemaDiscoveryField {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

/// `ClickPipeSchemaDiscoveryRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSchemaDiscoveryRequest {
    pub source: ClickPipeSchemaDiscoverySource,
}

/// `ClickPipeSchemaDiscoveryResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSchemaDiscoveryResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<ClickPipeSchemaDiscoveryField>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ClickPipeSchemaDiscoveryMeta>,
}

/// `ClickPipeSchemaDiscoveryMeta` from the ClickHouse Cloud API.
pub type ClickPipeSchemaDiscoveryMeta = std::collections::BTreeMap<String, String>;

/// `ClickPipeSchemaDiscoverySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSchemaDiscoverySource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<ClickPipePostKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinesis: Option<ClickPipePostKinesisSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none")]
    pub object_storage: Option<ClickPipePostObjectStorageSource>,
    #[serde(rename = "pubsub", skip_serializing_if = "Option::is_none")]
    pub pubsub: Option<ClickPipePostPubSubSource>,
}
