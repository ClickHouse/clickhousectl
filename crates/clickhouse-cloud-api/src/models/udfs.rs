use serde::{Deserialize, Serialize};
/// `Pagination` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Pagination {
    #[serde(rename = "currentCursor", skip_serializing_if = "Option::is_none")]
    pub current_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(rename = "nextCursor", skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(rename = "totalRecords", skip_serializing_if = "Option::is_none")]
    pub total_records: Option<i64>,
}

/// `UdfAttachment` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfAttachment {
    #[serde(rename = "functionName", skip_serializing_if = "Option::is_none")]
    pub function_name: Option<String>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<UdfAttachmentStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
}

/// Inline enum for `UdfAttachment.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UdfAttachmentStatus {
    #[serde(rename = "deployed")]
    #[default]
    Deployed,
    #[serde(rename = "deprovisioning")]
    Deprovisioning,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "provisioning")]
    Provisioning,
    #[serde(rename = "standby")]
    Standby,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UdfAttachmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Deployed => write!(f, "deployed"),
            Self::Deprovisioning => write!(f, "deprovisioning"),
            Self::Error => write!(f, "error"),
            Self::Provisioning => write!(f, "provisioning"),
            Self::Standby => write!(f, "standby"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `UdfAttachmentListResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfAttachmentListResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<UdfAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
}

/// A UDF argument sent in a create or version-create request.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfArgument {
    pub name: String,
    pub r#type: String,
}

/// `Udf` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Udf {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<UdfArgumentResponse>>,
    #[serde(rename = "commandReadTimeout", skip_serializing_if = "Option::is_none")]
    pub command_read_timeout: Option<i64>,
    #[serde(
        rename = "commandWriteTimeout",
        skip_serializing_if = "Option::is_none"
    )]
    pub command_write_timeout: Option<i64>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(rename = "functionName", skip_serializing_if = "Option::is_none")]
    pub function_name: Option<String>,
    #[serde(
        rename = "maxCommandExecutionTime",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_command_execution_time: Option<i64>,
    #[serde(rename = "poolSize", skip_serializing_if = "Option::is_none")]
    pub pool_size: Option<i64>,
    #[serde(rename = "returnName", skip_serializing_if = "Option::is_none")]
    pub return_name: Option<String>,
    #[serde(rename = "returnType", skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<UdfRuntime>,
    #[serde(rename = "sandboxType", skip_serializing_if = "Option::is_none")]
    pub sandbox_type: Option<UdfSandboxType>,
    #[serde(rename = "sandboxVersion", skip_serializing_if = "Option::is_none")]
    pub sandbox_version: Option<UdfSandboxVersion>,
    #[serde(rename = "sendChunkHeader", skip_serializing_if = "Option::is_none")]
    pub send_chunk_header: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<UdfStatus>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<UdfType>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
}

/// An argument returned in a UDF response.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfArgumentResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// Inline enum for `Udf.runtime`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UdfRuntime {
    #[serde(rename = "python3.11")]
    #[default]
    Python3_11,
    #[serde(rename = "native")]
    Native,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UdfRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Python3_11 => write!(f, "python3.11"),
            Self::Native => write!(f, "native"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Udf.sandboxType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UdfSandboxType {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    #[serde(rename = "netenable")]
    Netenable,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UdfSandboxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::Netenable => write!(f, "netenable"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Udf.sandboxVersion`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UdfSandboxVersion {
    #[serde(rename = "v1")]
    #[default]
    V1,
    #[serde(rename = "v2")]
    V2,
    #[serde(rename = "v3")]
    V3,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UdfSandboxVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1 => write!(f, "v1"),
            Self::V2 => write!(f, "v2"),
            Self::V3 => write!(f, "v3"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Udf.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UdfStatus {
    #[serde(rename = "building")]
    #[default]
    Building,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "ready")]
    Ready,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UdfStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Building => write!(f, "building"),
            Self::Error => write!(f, "error"),
            Self::Ready => write!(f, "ready"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Udf.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UdfType {
    #[serde(rename = "executable")]
    #[default]
    Executable,
    #[serde(rename = "executable_pool")]
    ExecutablePool,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UdfType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Executable => write!(f, "executable"),
            Self::ExecutablePool => write!(f, "executable_pool"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `UdfCreateRequest` - one of multiple variants.
///
/// Dispatched on the `type` field; the raw-value dispatch preserves unknown
/// variants and prevents overlapping request shapes from being misrouted.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum UdfCreateRequest {
    UdfCreateRequestV1(UdfCreateRequestV1),
    UdfCreateRequestV2(UdfCreateRequestV2),
    /// Catch-all for unknown or newly-added values.
    Unknown(serde_json::Value),
}

discriminated_union! {
    UdfCreateRequest, "type" {
        "executable" => UdfCreateRequestV1,
        "executable_pool" => UdfCreateRequestV2,
    }
}

impl std::fmt::Display for UdfCreateRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UdfCreateRequestV1(_) => write!(f, "UdfCreateRequestV1"),
            Self::UdfCreateRequestV2(_) => write!(f, "UdfCreateRequestV2"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// The `executable` variant of [`UdfCreateRequest`].
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfCreateRequestV1 {
    pub arguments: Vec<UdfArgument>,
    #[serde(rename = "commandReadTimeout", skip_serializing_if = "Option::is_none")]
    pub command_read_timeout: Option<i64>,
    #[serde(
        rename = "commandWriteTimeout",
        skip_serializing_if = "Option::is_none"
    )]
    pub command_write_timeout: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(rename = "functionName")]
    pub function_name: String,
    #[serde(
        rename = "maxCommandExecutionTime",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_command_execution_time: Option<i64>,
    #[serde(rename = "poolSize", skip_serializing_if = "Option::is_none")]
    pub pool_size: Option<()>,
    #[serde(rename = "returnName", skip_serializing_if = "Option::is_none")]
    pub return_name: Option<String>,
    #[serde(rename = "returnType")]
    pub return_type: String,
    pub runtime: UdfRuntime,
    #[serde(rename = "sandboxType", skip_serializing_if = "Option::is_none")]
    pub sandbox_type: Option<UdfSandboxType>,
    #[serde(rename = "sandboxVersion", skip_serializing_if = "Option::is_none")]
    pub sandbox_version: Option<UdfSandboxVersion>,
    #[serde(rename = "sendChunkHeader", skip_serializing_if = "Option::is_none")]
    pub send_chunk_header: Option<bool>,
    #[serde(rename = "type")]
    pub r#type: UdfCreateRequestV1Type,
    #[serde(rename = "uploadId")]
    pub upload_id: String,
}

/// Inline enum for `UdfCreateRequestV1.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UdfCreateRequestV1Type {
    #[serde(rename = "executable")]
    #[default]
    Executable,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UdfCreateRequestV1Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Executable => write!(f, "executable"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// The `executable_pool` variant of [`UdfCreateRequest`].
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfCreateRequestV2 {
    pub arguments: Vec<UdfArgument>,
    #[serde(rename = "commandReadTimeout", skip_serializing_if = "Option::is_none")]
    pub command_read_timeout: Option<i64>,
    #[serde(
        rename = "commandWriteTimeout",
        skip_serializing_if = "Option::is_none"
    )]
    pub command_write_timeout: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(rename = "functionName")]
    pub function_name: String,
    #[serde(
        rename = "maxCommandExecutionTime",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_command_execution_time: Option<i64>,
    #[serde(rename = "poolSize", skip_serializing_if = "Option::is_none")]
    pub pool_size: Option<i64>,
    #[serde(rename = "returnName", skip_serializing_if = "Option::is_none")]
    pub return_name: Option<String>,
    #[serde(rename = "returnType")]
    pub return_type: String,
    pub runtime: UdfRuntime,
    #[serde(rename = "sandboxType", skip_serializing_if = "Option::is_none")]
    pub sandbox_type: Option<UdfSandboxType>,
    #[serde(rename = "sandboxVersion", skip_serializing_if = "Option::is_none")]
    pub sandbox_version: Option<UdfSandboxVersion>,
    #[serde(rename = "sendChunkHeader", skip_serializing_if = "Option::is_none")]
    pub send_chunk_header: Option<bool>,
    #[serde(rename = "type")]
    pub r#type: UdfCreateRequestV2Type,
    #[serde(rename = "uploadId")]
    pub upload_id: String,
}

/// Inline enum for `UdfCreateRequestV2.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UdfCreateRequestV2Type {
    #[serde(rename = "executable_pool")]
    #[default]
    ExecutablePool,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UdfCreateRequestV2Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecutablePool => write!(f, "executable_pool"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `UdfListResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfListResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<Udf>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
}

/// `UdfUploadSession` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfUploadSession {
    #[serde(rename = "expiresAt", skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "uploadId", skip_serializing_if = "Option::is_none")]
    pub upload_id: Option<String>,
    #[serde(rename = "uploadUrl", skip_serializing_if = "Option::is_none")]
    pub upload_url: Option<String>,
}

/// `UdfVersionCreateRequest` - one of multiple variants.
///
/// Dispatched on the `type` field; the raw-value dispatch preserves unknown
/// variants and prevents overlapping request shapes from being misrouted.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum UdfVersionCreateRequest {
    UdfVersionCreateRequestV1(UdfVersionCreateRequestV1),
    UdfVersionCreateRequestV2(UdfVersionCreateRequestV2),
    /// Catch-all for unknown or newly-added values.
    Unknown(serde_json::Value),
}

discriminated_union! {
    UdfVersionCreateRequest, "type" {
        "executable" => UdfVersionCreateRequestV1,
        "executable_pool" => UdfVersionCreateRequestV2,
    }
}

impl std::fmt::Display for UdfVersionCreateRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UdfVersionCreateRequestV1(_) => write!(f, "UdfVersionCreateRequestV1"),
            Self::UdfVersionCreateRequestV2(_) => write!(f, "UdfVersionCreateRequestV2"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// The `executable` variant of [`UdfVersionCreateRequest`].
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfVersionCreateRequestV1 {
    pub arguments: Vec<UdfArgument>,
    #[serde(rename = "commandReadTimeout", skip_serializing_if = "Option::is_none")]
    pub command_read_timeout: Option<i64>,
    #[serde(
        rename = "commandWriteTimeout",
        skip_serializing_if = "Option::is_none"
    )]
    pub command_write_timeout: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(
        rename = "maxCommandExecutionTime",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_command_execution_time: Option<i64>,
    #[serde(rename = "poolSize", skip_serializing_if = "Option::is_none")]
    pub pool_size: Option<()>,
    #[serde(rename = "returnName", skip_serializing_if = "Option::is_none")]
    pub return_name: Option<String>,
    #[serde(rename = "returnType")]
    pub return_type: String,
    pub runtime: UdfRuntime,
    #[serde(rename = "sandboxType", skip_serializing_if = "Option::is_none")]
    pub sandbox_type: Option<UdfSandboxType>,
    #[serde(rename = "sandboxVersion", skip_serializing_if = "Option::is_none")]
    pub sandbox_version: Option<UdfSandboxVersion>,
    #[serde(rename = "sendChunkHeader", skip_serializing_if = "Option::is_none")]
    pub send_chunk_header: Option<bool>,
    #[serde(rename = "type")]
    pub r#type: UdfVersionCreateRequestV1Type,
    #[serde(rename = "uploadId")]
    pub upload_id: String,
}

/// Inline enum for `UdfVersionCreateRequestV1.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UdfVersionCreateRequestV1Type {
    #[serde(rename = "executable")]
    #[default]
    Executable,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UdfVersionCreateRequestV1Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Executable => write!(f, "executable"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// The `executable_pool` variant of [`UdfVersionCreateRequest`].
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfVersionCreateRequestV2 {
    pub arguments: Vec<UdfArgument>,
    #[serde(rename = "commandReadTimeout", skip_serializing_if = "Option::is_none")]
    pub command_read_timeout: Option<i64>,
    #[serde(
        rename = "commandWriteTimeout",
        skip_serializing_if = "Option::is_none"
    )]
    pub command_write_timeout: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(
        rename = "maxCommandExecutionTime",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_command_execution_time: Option<i64>,
    #[serde(rename = "poolSize", skip_serializing_if = "Option::is_none")]
    pub pool_size: Option<i64>,
    #[serde(rename = "returnName", skip_serializing_if = "Option::is_none")]
    pub return_name: Option<String>,
    #[serde(rename = "returnType")]
    pub return_type: String,
    pub runtime: UdfRuntime,
    #[serde(rename = "sandboxType", skip_serializing_if = "Option::is_none")]
    pub sandbox_type: Option<UdfSandboxType>,
    #[serde(rename = "sandboxVersion", skip_serializing_if = "Option::is_none")]
    pub sandbox_version: Option<UdfSandboxVersion>,
    #[serde(rename = "sendChunkHeader", skip_serializing_if = "Option::is_none")]
    pub send_chunk_header: Option<bool>,
    #[serde(rename = "type")]
    pub r#type: UdfVersionCreateRequestV2Type,
    #[serde(rename = "uploadId")]
    pub upload_id: String,
}

/// Inline enum for `UdfVersionCreateRequestV2.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UdfVersionCreateRequestV2Type {
    #[serde(rename = "executable_pool")]
    #[default]
    ExecutablePool,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UdfVersionCreateRequestV2Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecutablePool => write!(f, "executable_pool"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `UdfVersionListResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UdfVersionListResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<Udf>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
}
