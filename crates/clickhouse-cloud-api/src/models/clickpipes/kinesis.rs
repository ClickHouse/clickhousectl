use super::MskIamUser;
use serde::{Deserialize, Serialize};

/// Inline enum for `ClickPipeKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKinesisSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKinesisSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKinesisSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::AvroConfluent => write!(f, "AvroConfluent"),
            Self::Protobuf => write!(f, "Protobuf"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKinesisSource.iteratorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSourceIteratortype {
    #[default]
    TRIM_HORIZON,
    LATEST,
    AT_TIMESTAMP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKinesisSourceIteratortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TRIM_HORIZON => write!(f, "TRIM_HORIZON"),
            Self::LATEST => write!(f, "LATEST"),
            Self::AT_TIMESTAMP => write!(f, "AT_TIMESTAMP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchKinesisSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKinesisSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKinesisSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKinesisSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKinesisSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::AvroConfluent => write!(f, "AvroConfluent"),
            Self::Protobuf => write!(f, "Protobuf"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKinesisSource.iteratorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKinesisSourceIteratortype {
    #[default]
    TRIM_HORIZON,
    LATEST,
    AT_TIMESTAMP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKinesisSourceIteratortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TRIM_HORIZON => write!(f, "TRIM_HORIZON"),
            Self::LATEST => write!(f, "LATEST"),
            Self::AT_TIMESTAMP => write!(f, "AT_TIMESTAMP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickPipeKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKinesisSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeKinesisSourceAuthentication>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ClickPipeKinesisSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(rename = "iteratorType", skip_serializing_if = "Option::is_none")]
    pub iterator_type: Option<ClickPipeKinesisSourceIteratortype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(rename = "schemaRegistry", skip_serializing_if = "Option::is_none")]
    pub schema_registry: Option<ClickPipeKinesisSchemaRegistryResponse>,
    #[serde(rename = "streamName", skip_serializing_if = "Option::is_none")]
    pub stream_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(rename = "useEnhancedFanOut", skip_serializing_if = "Option::is_none")]
    pub use_enhanced_fan_out: Option<bool>,
}

/// Values of `ClickPipeKinesisSchemaRegistry.type` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKinesisSchemaRegistryType {
    #[serde(rename = "glue")]
    #[default]
    Glue,
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKinesisSchemaRegistryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Glue => write!(f, "glue"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipeKinesisSchemaRegistry` for Kinesis create requests.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKinesisSchemaRegistry {
    #[serde(rename = "type")]
    pub r#type: ClickPipeKinesisSchemaRegistryType,
    #[serde(rename = "glueRegion")]
    pub glue_region: String,
    #[serde(rename = "glueRegistryName")]
    pub glue_registry_name: String,
    /// Omit to use the Kinesis source's IAM identity for Glue access.
    #[serde(rename = "glueRoleArn", skip_serializing_if = "Option::is_none")]
    pub glue_role_arn: Option<String>,
}

/// `ClickPipeKinesisSchemaRegistry` in a Kinesis source response.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKinesisSchemaRegistryResponse {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeKinesisSchemaRegistryType>,
    #[serde(rename = "glueRegion", skip_serializing_if = "Option::is_none")]
    pub glue_region: Option<String>,
    #[serde(rename = "glueRegistryName", skip_serializing_if = "Option::is_none")]
    pub glue_registry_name: Option<String>,
    #[serde(rename = "glueRoleArn", skip_serializing_if = "Option::is_none")]
    pub glue_role_arn: Option<String>,
}

/// `ClickPipePatchKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchKinesisSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none")]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePatchKinesisSourceAuthentication>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
}

/// `ClickPipePostKinesisSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostKinesisSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none")]
    pub access_key: Option<MskIamUser>,
    pub authentication: ClickPipePostKinesisSourceAuthentication,
    pub format: ClickPipePostKinesisSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(rename = "iteratorType")]
    pub iterator_type: ClickPipePostKinesisSourceIteratortype,
    pub region: String,
    #[serde(rename = "schemaRegistry", skip_serializing_if = "Option::is_none")]
    pub schema_registry: Option<ClickPipeKinesisSchemaRegistry>,
    #[serde(rename = "streamName")]
    pub stream_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(rename = "useEnhancedFanOut", skip_serializing_if = "Option::is_none")]
    pub use_enhanced_fan_out: Option<bool>,
    /// Base64-encoded .proto or FileDescriptorSet; required only for Protobuf.
    #[serde(rename = "protobufSchema", skip_serializing_if = "Option::is_none")]
    pub protobuf_schema: Option<String>,
}
