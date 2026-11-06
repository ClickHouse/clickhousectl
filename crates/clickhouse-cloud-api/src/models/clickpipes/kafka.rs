use serde::{Deserialize, Serialize};

/// Inline enum for `ClickPipeKafkaOffset.strategy`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaOffsetStrategy {
    #[serde(rename = "from_beginning")]
    #[default]
    From_beginning,
    #[serde(rename = "from_latest")]
    From_latest,
    #[serde(rename = "from_timestamp")]
    From_timestamp,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl ClickPipeKafkaOffsetStrategy {
    pub const VALUES: &'static [&'static str] =
        &["from_beginning", "from_latest", "from_timestamp"];
}

impl std::fmt::Display for ClickPipeKafkaOffsetStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::From_beginning => write!(f, "from_beginning"),
            Self::From_latest => write!(f, "from_latest"),
            Self::From_timestamp => write!(f, "from_timestamp"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaConfluentSchemaRegistry.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaConfluentSchemaRegistryAuthentication {
    #[default]
    PLAIN,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaConfluentSchemaRegistryAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::SCRAM_SHA_256 => write!(f, "SCRAM-SHA-256"),
            Self::SCRAM_SHA_512 => write!(f, "SCRAM-SHA-512"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::MUTUAL_TLS => write!(f, "MUTUAL_TLS"),
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSourceFormat {
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

/// Inline enum for `ClickPipeKafkaSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceType {
    #[serde(rename = "kafka")]
    #[default]
    Kafka,
    #[serde(rename = "redpanda")]
    Redpanda,
    #[serde(rename = "msk")]
    Msk,
    #[serde(rename = "gcmk")]
    Gcmk,
    #[serde(rename = "confluent")]
    Confluent,
    #[serde(rename = "warpstream")]
    Warpstream,
    #[serde(rename = "azureeventhub")]
    Azureeventhub,
    #[serde(rename = "dokafka")]
    Dokafka,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kafka => write!(f, "kafka"),
            Self::Redpanda => write!(f, "redpanda"),
            Self::Msk => write!(f, "msk"),
            Self::Gcmk => write!(f, "gcmk"),
            Self::Confluent => write!(f, "confluent"),
            Self::Warpstream => write!(f, "warpstream"),
            Self::Azureeventhub => write!(f, "azureeventhub"),
            Self::Dokafka => write!(f, "dokafka"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateKafkaConfluentSchemaRegistry.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateKafkaConfluentSchemaRegistryAuthentication {
    #[default]
    PLAIN,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateKafkaConfluentSchemaRegistryAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchKafkaSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::SCRAM_SHA_256 => write!(f, "SCRAM-SHA-256"),
            Self::SCRAM_SHA_512 => write!(f, "SCRAM-SHA-512"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::MUTUAL_TLS => write!(f, "MUTUAL_TLS"),
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKafkaSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceAuthentication {
    #[default]
    PLAIN,
    #[serde(rename = "SCRAM-SHA-256")]
    SCRAM_SHA_256,
    #[serde(rename = "SCRAM-SHA-512")]
    SCRAM_SHA_512,
    IAM_ROLE,
    IAM_USER,
    MUTUAL_TLS,
    #[serde(rename = "SERVICE_ACCOUNT_WORKLOAD_IDENTITY")]
    ServiceAccountWorkloadIdentity,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl ClickPipePostKafkaSourceAuthentication {
    pub const VALUES: &'static [&'static str] = &[
        "PLAIN",
        "SCRAM-SHA-256",
        "SCRAM-SHA-512",
        "IAM_ROLE",
        "IAM_USER",
        "MUTUAL_TLS",
        "SERVICE_ACCOUNT_WORKLOAD_IDENTITY",
    ];
}

impl std::fmt::Display for ClickPipePostKafkaSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::SCRAM_SHA_256 => write!(f, "SCRAM-SHA-256"),
            Self::SCRAM_SHA_512 => write!(f, "SCRAM-SHA-512"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::MUTUAL_TLS => write!(f, "MUTUAL_TLS"),
            Self::ServiceAccountWorkloadIdentity => write!(f, "SERVICE_ACCOUNT_WORKLOAD_IDENTITY"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostKafkaSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    AvroConfluent,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl ClickPipePostKafkaSourceFormat {
    pub const VALUES: &'static [&'static str] =
        &["JSONEachRow", "Avro", "AvroConfluent", "Protobuf"];
}

impl std::fmt::Display for ClickPipePostKafkaSourceFormat {
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

/// Inline enum for `ClickPipePostKafkaSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceType {
    #[serde(rename = "kafka")]
    #[default]
    Kafka,
    #[serde(rename = "redpanda")]
    Redpanda,
    #[serde(rename = "msk")]
    Msk,
    #[serde(rename = "gcmk")]
    Gcmk,
    #[serde(rename = "confluent")]
    Confluent,
    #[serde(rename = "warpstream")]
    Warpstream,
    #[serde(rename = "azureeventhub")]
    Azureeventhub,
    #[serde(rename = "dokafka")]
    Dokafka,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl ClickPipePostKafkaSourceType {
    pub const VALUES: &'static [&'static str] = &[
        "kafka",
        "redpanda",
        "msk",
        "gcmk",
        "confluent",
        "warpstream",
        "azureeventhub",
        "dokafka",
    ];
}

impl std::fmt::Display for ClickPipePostKafkaSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kafka => write!(f, "kafka"),
            Self::Redpanda => write!(f, "redpanda"),
            Self::Msk => write!(f, "msk"),
            Self::Gcmk => write!(f, "gcmk"),
            Self::Confluent => write!(f, "confluent"),
            Self::Warpstream => write!(f, "warpstream"),
            Self::Azureeventhub => write!(f, "azureeventhub"),
            Self::Dokafka => write!(f, "dokafka"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `AzureEventHub` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureEventHub {
    #[serde(rename = "connectionString")]
    pub connection_string: String,
}

/// `ClickPipeKafkaOffset` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaOffset {
    pub strategy: ClickPipeKafkaOffsetStrategy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// `ClickPipeKafkaOffset` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickPipeKafkaOffset`]: every field is `Option<T>`, so
/// a field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaOffsetResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<ClickPipeKafkaOffsetStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Values of `ClickPipeKafkaConfluentSchemaRegistry.type` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaConfluentSchemaRegistryType {
    #[serde(rename = "confluent")]
    #[default]
    Confluent,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaConfluentSchemaRegistryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Confluent => write!(f, "confluent"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipeKafkaConfluentSchemaRegistry` in a Kafka source response.
///
/// The API omits `type` for registries created before AWS Glue support; an
/// absent `type` means a Confluent-compatible registry.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaConfluentSchemaRegistry {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeKafkaConfluentSchemaRegistryType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeKafkaConfluentSchemaRegistryAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Values of `ClickPipeKafkaGlueSchemaRegistry.type` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaGlueSchemaRegistryType {
    #[serde(rename = "glue")]
    #[default]
    Glue,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaGlueSchemaRegistryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Glue => write!(f, "glue"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipeKafkaGlueSchemaRegistry` for Kafka create requests.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaGlueSchemaRegistry {
    #[serde(rename = "type")]
    pub r#type: ClickPipeKafkaGlueSchemaRegistryType,
    #[serde(rename = "glueRegion")]
    pub glue_region: String,
    #[serde(rename = "glueRegistryName")]
    pub glue_registry_name: String,
    /// Omit to use the Kafka source's IAM role for Glue access; required when
    /// the source does not authenticate with IAM.
    #[serde(rename = "glueRoleArn", skip_serializing_if = "Option::is_none")]
    pub glue_role_arn: Option<String>,
}

/// `ClickPipeKafkaGlueSchemaRegistry` in a Kafka source response.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaGlueSchemaRegistryResponse {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeKafkaGlueSchemaRegistryType>,
    #[serde(rename = "glueRegion", skip_serializing_if = "Option::is_none")]
    pub glue_region: Option<String>,
    #[serde(rename = "glueRegistryName", skip_serializing_if = "Option::is_none")]
    pub glue_registry_name: Option<String>,
    #[serde(rename = "glueRoleArn", skip_serializing_if = "Option::is_none")]
    pub glue_role_arn: Option<String>,
}

/// `ClickPipeKafkaSchemaRegistry` from the ClickHouse Cloud API, selected by `type`.
///
/// An absent `type` selects the Confluent variant, matching the API's default
/// and legacy payloads, unless the payload carries a Glue-only key; such a
/// payload is kept verbatim in `Unknown` rather than misread as Confluent.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickPipeKafkaSchemaRegistry {
    ClickPipeKafkaConfluentSchemaRegistry(ClickPipeKafkaConfluentSchemaRegistry),
    ClickPipeKafkaGlueSchemaRegistryResponse(ClickPipeKafkaGlueSchemaRegistryResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}
discriminated_union! {
    ClickPipeKafkaSchemaRegistry, "type" {
        "confluent" => ClickPipeKafkaConfluentSchemaRegistry,
        "glue" => ClickPipeKafkaGlueSchemaRegistryResponse,
        none unless "glueRegion" | "glueRegistryName" | "glueRoleArn" => ClickPipeKafkaConfluentSchemaRegistry,
    }
}
impl Default for ClickPipeKafkaSchemaRegistry {
    fn default() -> Self {
        // `{}` dispatches to Confluent through the absent-`type` arm, so the
        // default round-trips without fabricating a discriminator.
        Self::ClickPipeKafkaConfluentSchemaRegistry(ClickPipeKafkaConfluentSchemaRegistry::default())
    }
}
impl std::fmt::Display for ClickPipeKafkaSchemaRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickPipeKafkaConfluentSchemaRegistry(_) => {
                write!(f, "ClickPipeKafkaConfluentSchemaRegistry")
            }
            Self::ClickPipeKafkaGlueSchemaRegistryResponse(_) => {
                write!(f, "ClickPipeKafkaGlueSchemaRegistryResponse")
            }
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipeKafkaSchemaRegistryCredentials` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSchemaRegistryCredentials {
    pub password: String,
    pub username: String,
}

/// `ClickPipeKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeKafkaSourceAuthentication>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brokers: Option<String>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(rename = "consumerGroup", skip_serializing_if = "Option::is_none")]
    pub consumer_group: Option<String>,
    #[serde(rename = "exactlyOnce", skip_serializing_if = "Option::is_none")]
    pub exactly_once: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ClickPipeKafkaSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<ClickPipeKafkaOffsetResponse>,
    #[serde(
        rename = "reversePrivateEndpointIds",
        skip_serializing_if = "Option::is_none"
    )]
    pub reverse_private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "schemaRegistry", skip_serializing_if = "Option::is_none")]
    pub schema_registry: Option<ClickPipeKafkaSchemaRegistry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<String>,
    #[serde(rename = "tombstoneMode", skip_serializing_if = "Option::is_none")]
    pub tombstone_mode: Option<ClickPipeKafkaSourceTombstonemode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeKafkaSourceType>,
}

/// Values of `ClickPipeMutateKafkaConfluentSchemaRegistry.type` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateKafkaConfluentSchemaRegistryType {
    #[serde(rename = "confluent")]
    #[default]
    Confluent,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateKafkaConfluentSchemaRegistryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Confluent => write!(f, "confluent"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipeMutateKafkaConfluentSchemaRegistry` for Kafka create requests.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateKafkaConfluentSchemaRegistry {
    /// Omit for the API default, a Confluent-compatible registry.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeMutateKafkaConfluentSchemaRegistryType>,
    pub authentication: ClickPipeMutateKafkaConfluentSchemaRegistryAuthentication,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    pub credentials: ClickPipeKafkaSchemaRegistryCredentials,
    pub url: String,
}

/// `ClickPipeMutateKafkaSchemaRegistry` from the ClickHouse Cloud API, selected by `type`.
///
/// An absent `type` selects the Confluent variant (the API default) unless the
/// payload carries a Glue-only key, which is kept verbatim in `Unknown`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickPipeMutateKafkaSchemaRegistry {
    ClickPipeMutateKafkaConfluentSchemaRegistry(ClickPipeMutateKafkaConfluentSchemaRegistry),
    ClickPipeKafkaGlueSchemaRegistry(ClickPipeKafkaGlueSchemaRegistry),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}
discriminated_union! {
    ClickPipeMutateKafkaSchemaRegistry, "type" {
        "confluent" => ClickPipeMutateKafkaConfluentSchemaRegistry,
        "glue" => ClickPipeKafkaGlueSchemaRegistry,
        none unless "glueRegion" | "glueRegistryName" | "glueRoleArn" => ClickPipeMutateKafkaConfluentSchemaRegistry,
    }
}
impl Default for ClickPipeMutateKafkaSchemaRegistry {
    fn default() -> Self {
        Self::ClickPipeMutateKafkaConfluentSchemaRegistry(
            ClickPipeMutateKafkaConfluentSchemaRegistry::default(),
        )
    }
}
impl std::fmt::Display for ClickPipeMutateKafkaSchemaRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickPipeMutateKafkaConfluentSchemaRegistry(_) => {
                write!(f, "ClickPipeMutateKafkaConfluentSchemaRegistry")
            }
            Self::ClickPipeKafkaGlueSchemaRegistry(_) => {
                write!(f, "ClickPipeKafkaGlueSchemaRegistry")
            }
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `ClickPipePatchKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchKafkaSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePatchKafkaSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<serde_json::Value>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(
        rename = "reversePrivateEndpointIds",
        skip_serializing_if = "Option::is_none"
    )]
    pub reverse_private_endpoint_ids: Option<Vec<String>>,
}

/// `ClickPipePostKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostKafkaSource {
    #[serde(rename = "protobufSchema", skip_serializing_if = "Option::is_none")]
    pub protobuf_schema: Option<String>,
    /// Omitted for a broker that requires no authentication: the spec enum has
    /// no "none" value, and the control plane's own field is `omitempty`, so
    /// absence — not a sentinel value — is how "no auth" is expressed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePostKafkaSourceAuthentication>,
    pub brokers: String,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(rename = "consumerGroup", skip_serializing_if = "Option::is_none")]
    pub consumer_group: Option<String>,
    pub credentials: serde_json::Value,
    #[serde(rename = "exactlyOnce", skip_serializing_if = "Option::is_none")]
    pub exactly_once: Option<bool>,
    pub format: ClickPipePostKafkaSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<ClickPipeKafkaOffset>,
    #[serde(rename = "reversePrivateEndpointIds")]
    pub reverse_private_endpoint_ids: Vec<String>,
    #[serde(rename = "schemaRegistry", skip_serializing_if = "Option::is_none")]
    pub schema_registry: Option<ClickPipeMutateKafkaSchemaRegistry>,
    pub topics: String,
    /// Deleting Kafka tombstones requires exactly-once delivery and can only be set on creation.
    #[serde(rename = "tombstoneMode", skip_serializing_if = "Option::is_none")]
    pub tombstone_mode: Option<ClickPipePostKafkaSourceTombstonemode>,
    pub r#type: ClickPipePostKafkaSourceType,
}

/// Inline enum for `ClickPipePostKafkaSource.tombstoneMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostKafkaSourceTombstonemode {
    #[serde(rename = "delete")]
    #[default]
    Delete,
    #[serde(rename = "soft_delete")]
    Soft_delete,
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostKafkaSourceTombstonemode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Delete => write!(f, "delete"),
            Self::Soft_delete => write!(f, "soft_delete"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// Inline enum for `ClickPipeKafkaSource.tombstoneMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSourceTombstonemode {
    #[serde(rename = "delete")]
    #[default]
    Delete,
    #[serde(rename = "soft_delete")]
    Soft_delete,
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSourceTombstonemode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Delete => write!(f, "delete"),
            Self::Soft_delete => write!(f, "soft_delete"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `MutualTLS` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MutualTLS {
    pub certificate: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

impl From<ClickPipeMutateKafkaConfluentSchemaRegistry> for ClickPipeMutateKafkaSchemaRegistry {
    fn from(value: ClickPipeMutateKafkaConfluentSchemaRegistry) -> Self {
        Self::ClickPipeMutateKafkaConfluentSchemaRegistry(value)
    }
}

impl From<ClickPipeKafkaGlueSchemaRegistry> for ClickPipeMutateKafkaSchemaRegistry {
    fn from(value: ClickPipeKafkaGlueSchemaRegistry) -> Self {
        Self::ClickPipeKafkaGlueSchemaRegistry(value)
    }
}
