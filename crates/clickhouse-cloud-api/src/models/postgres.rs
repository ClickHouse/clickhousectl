use super::{ResourceTagsV1, ResourceTagsV1Response};
use serde::{Deserialize, Serialize};

/// `pgHaType` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgHaType {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "async")]
    Async,
    #[serde(rename = "sync")]
    Sync,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgHaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Async => write!(f, "async"),
            Self::Sync => write!(f, "sync"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl PgHaType {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["none", "async", "sync"];
}

/// `pgProvider` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl PgProvider {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["aws"];
}

/// `pgSize` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgSize {
    #[serde(rename = "c6gd.large")]
    #[default]
    C6gd_large,
    #[serde(rename = "c6gd.xlarge")]
    C6gd_xlarge,
    #[serde(rename = "c6gd.2xlarge")]
    C6gd_2xlarge,
    #[serde(rename = "c6gd.4xlarge")]
    C6gd_4xlarge,
    #[serde(rename = "c6gd.8xlarge")]
    C6gd_8xlarge,
    #[serde(rename = "c6gd.16xlarge")]
    C6gd_16xlarge,
    #[serde(rename = "i7i.large")]
    I7i_large,
    #[serde(rename = "i7i.xlarge")]
    I7i_xlarge,
    #[serde(rename = "i7i.2xlarge")]
    I7i_2xlarge,
    #[serde(rename = "i7i.4xlarge")]
    I7i_4xlarge,
    #[serde(rename = "i7i.8xlarge")]
    I7i_8xlarge,
    #[serde(rename = "i7i.12xlarge")]
    I7i_12xlarge,
    #[serde(rename = "i7i.16xlarge")]
    I7i_16xlarge,
    #[serde(rename = "i7i.24xlarge")]
    I7i_24xlarge,
    #[serde(rename = "i7ie.large")]
    I7ie_large,
    #[serde(rename = "i7ie.xlarge")]
    I7ie_xlarge,
    #[serde(rename = "i7ie.2xlarge")]
    I7ie_2xlarge,
    #[serde(rename = "i7ie.3xlarge")]
    I7ie_3xlarge,
    #[serde(rename = "i7ie.6xlarge")]
    I7ie_6xlarge,
    #[serde(rename = "i7ie.12xlarge")]
    I7ie_12xlarge,
    #[serde(rename = "i7ie.18xlarge")]
    I7ie_18xlarge,
    #[serde(rename = "i7ie.24xlarge")]
    I7ie_24xlarge,
    #[serde(rename = "i8g.large")]
    I8g_large,
    #[serde(rename = "i8g.xlarge")]
    I8g_xlarge,
    #[serde(rename = "i8g.2xlarge")]
    I8g_2xlarge,
    #[serde(rename = "i8g.4xlarge")]
    I8g_4xlarge,
    #[serde(rename = "i8g.8xlarge")]
    I8g_8xlarge,
    #[serde(rename = "i8g.16xlarge")]
    I8g_16xlarge,
    #[serde(rename = "i8g.24xlarge")]
    I8g_24xlarge,
    #[serde(rename = "i8ge.large")]
    I8ge_large,
    #[serde(rename = "i8ge.xlarge")]
    I8ge_xlarge,
    #[serde(rename = "i8ge.2xlarge")]
    I8ge_2xlarge,
    #[serde(rename = "i8ge.3xlarge")]
    I8ge_3xlarge,
    #[serde(rename = "i8ge.6xlarge")]
    I8ge_6xlarge,
    #[serde(rename = "i8ge.12xlarge")]
    I8ge_12xlarge,
    #[serde(rename = "i8ge.18xlarge")]
    I8ge_18xlarge,
    #[serde(rename = "i8ge.24xlarge")]
    I8ge_24xlarge,
    #[serde(rename = "m6gd.large")]
    M6gd_large,
    #[serde(rename = "m6gd.xlarge")]
    M6gd_xlarge,
    #[serde(rename = "m6gd.2xlarge")]
    M6gd_2xlarge,
    #[serde(rename = "m6gd.4xlarge")]
    M6gd_4xlarge,
    #[serde(rename = "m6gd.8xlarge")]
    M6gd_8xlarge,
    #[serde(rename = "m6gd.16xlarge")]
    M6gd_16xlarge,
    #[serde(rename = "m6id.large")]
    M6id_large,
    #[serde(rename = "m6id.xlarge")]
    M6id_xlarge,
    #[serde(rename = "m6id.2xlarge")]
    M6id_2xlarge,
    #[serde(rename = "m6id.4xlarge")]
    M6id_4xlarge,
    #[serde(rename = "m6id.8xlarge")]
    M6id_8xlarge,
    #[serde(rename = "m6id.16xlarge")]
    M6id_16xlarge,
    #[serde(rename = "m8gd.large")]
    M8gd_large,
    #[serde(rename = "m8gd.xlarge")]
    M8gd_xlarge,
    #[serde(rename = "m8gd.2xlarge")]
    M8gd_2xlarge,
    #[serde(rename = "m8gd.4xlarge")]
    M8gd_4xlarge,
    #[serde(rename = "m8gd.8xlarge")]
    M8gd_8xlarge,
    #[serde(rename = "m8gd.16xlarge")]
    M8gd_16xlarge,
    #[serde(rename = "r6gd.medium")]
    R6gd_medium,
    #[serde(rename = "r6gd.large")]
    R6gd_large,
    #[serde(rename = "r6gd.xlarge")]
    R6gd_xlarge,
    #[serde(rename = "r6gd.2xlarge")]
    R6gd_2xlarge,
    #[serde(rename = "r6gd.4xlarge")]
    R6gd_4xlarge,
    #[serde(rename = "r6gd.8xlarge")]
    R6gd_8xlarge,
    #[serde(rename = "r6gd.12xlarge")]
    R6gd_12xlarge,
    #[serde(rename = "r6gd.16xlarge")]
    R6gd_16xlarge,
    #[serde(rename = "r6id.large")]
    R6id_large,
    #[serde(rename = "r6id.xlarge")]
    R6id_xlarge,
    #[serde(rename = "r6id.2xlarge")]
    R6id_2xlarge,
    #[serde(rename = "r6id.4xlarge")]
    R6id_4xlarge,
    #[serde(rename = "r6id.8xlarge")]
    R6id_8xlarge,
    #[serde(rename = "r6id.12xlarge")]
    R6id_12xlarge,
    #[serde(rename = "r6id.16xlarge")]
    R6id_16xlarge,
    #[serde(rename = "r6id.24xlarge")]
    R6id_24xlarge,
    #[serde(rename = "r6id.32xlarge")]
    R6id_32xlarge,
    #[serde(rename = "r8gd.medium")]
    R8gd_medium,
    #[serde(rename = "r8gd.large")]
    R8gd_large,
    #[serde(rename = "r8gd.xlarge")]
    R8gd_xlarge,
    #[serde(rename = "r8gd.2xlarge")]
    R8gd_2xlarge,
    #[serde(rename = "r8gd.4xlarge")]
    R8gd_4xlarge,
    #[serde(rename = "r8gd.8xlarge")]
    R8gd_8xlarge,
    #[serde(rename = "r8gd.12xlarge")]
    R8gd_12xlarge,
    #[serde(rename = "r8gd.16xlarge")]
    R8gd_16xlarge,
    #[serde(rename = "r8gd.24xlarge")]
    R8gd_24xlarge,
    #[serde(rename = "r8gd.48xlarge")]
    R8gd_48xlarge,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::C6gd_large => write!(f, "c6gd.large"),
            Self::C6gd_xlarge => write!(f, "c6gd.xlarge"),
            Self::C6gd_2xlarge => write!(f, "c6gd.2xlarge"),
            Self::C6gd_4xlarge => write!(f, "c6gd.4xlarge"),
            Self::C6gd_8xlarge => write!(f, "c6gd.8xlarge"),
            Self::C6gd_16xlarge => write!(f, "c6gd.16xlarge"),
            Self::I7i_large => write!(f, "i7i.large"),
            Self::I7i_xlarge => write!(f, "i7i.xlarge"),
            Self::I7i_2xlarge => write!(f, "i7i.2xlarge"),
            Self::I7i_4xlarge => write!(f, "i7i.4xlarge"),
            Self::I7i_8xlarge => write!(f, "i7i.8xlarge"),
            Self::I7i_12xlarge => write!(f, "i7i.12xlarge"),
            Self::I7i_16xlarge => write!(f, "i7i.16xlarge"),
            Self::I7i_24xlarge => write!(f, "i7i.24xlarge"),
            Self::I7ie_large => write!(f, "i7ie.large"),
            Self::I7ie_xlarge => write!(f, "i7ie.xlarge"),
            Self::I7ie_2xlarge => write!(f, "i7ie.2xlarge"),
            Self::I7ie_3xlarge => write!(f, "i7ie.3xlarge"),
            Self::I7ie_6xlarge => write!(f, "i7ie.6xlarge"),
            Self::I7ie_12xlarge => write!(f, "i7ie.12xlarge"),
            Self::I7ie_18xlarge => write!(f, "i7ie.18xlarge"),
            Self::I7ie_24xlarge => write!(f, "i7ie.24xlarge"),
            Self::I8g_large => write!(f, "i8g.large"),
            Self::I8g_xlarge => write!(f, "i8g.xlarge"),
            Self::I8g_2xlarge => write!(f, "i8g.2xlarge"),
            Self::I8g_4xlarge => write!(f, "i8g.4xlarge"),
            Self::I8g_8xlarge => write!(f, "i8g.8xlarge"),
            Self::I8g_16xlarge => write!(f, "i8g.16xlarge"),
            Self::I8g_24xlarge => write!(f, "i8g.24xlarge"),
            Self::I8ge_large => write!(f, "i8ge.large"),
            Self::I8ge_xlarge => write!(f, "i8ge.xlarge"),
            Self::I8ge_2xlarge => write!(f, "i8ge.2xlarge"),
            Self::I8ge_3xlarge => write!(f, "i8ge.3xlarge"),
            Self::I8ge_6xlarge => write!(f, "i8ge.6xlarge"),
            Self::I8ge_12xlarge => write!(f, "i8ge.12xlarge"),
            Self::I8ge_18xlarge => write!(f, "i8ge.18xlarge"),
            Self::I8ge_24xlarge => write!(f, "i8ge.24xlarge"),
            Self::M6gd_large => write!(f, "m6gd.large"),
            Self::M6gd_xlarge => write!(f, "m6gd.xlarge"),
            Self::M6gd_2xlarge => write!(f, "m6gd.2xlarge"),
            Self::M6gd_4xlarge => write!(f, "m6gd.4xlarge"),
            Self::M6gd_8xlarge => write!(f, "m6gd.8xlarge"),
            Self::M6gd_16xlarge => write!(f, "m6gd.16xlarge"),
            Self::M6id_large => write!(f, "m6id.large"),
            Self::M6id_xlarge => write!(f, "m6id.xlarge"),
            Self::M6id_2xlarge => write!(f, "m6id.2xlarge"),
            Self::M6id_4xlarge => write!(f, "m6id.4xlarge"),
            Self::M6id_8xlarge => write!(f, "m6id.8xlarge"),
            Self::M6id_16xlarge => write!(f, "m6id.16xlarge"),
            Self::M8gd_large => write!(f, "m8gd.large"),
            Self::M8gd_xlarge => write!(f, "m8gd.xlarge"),
            Self::M8gd_2xlarge => write!(f, "m8gd.2xlarge"),
            Self::M8gd_4xlarge => write!(f, "m8gd.4xlarge"),
            Self::M8gd_8xlarge => write!(f, "m8gd.8xlarge"),
            Self::M8gd_16xlarge => write!(f, "m8gd.16xlarge"),
            Self::R6gd_medium => write!(f, "r6gd.medium"),
            Self::R6gd_large => write!(f, "r6gd.large"),
            Self::R6gd_xlarge => write!(f, "r6gd.xlarge"),
            Self::R6gd_2xlarge => write!(f, "r6gd.2xlarge"),
            Self::R6gd_4xlarge => write!(f, "r6gd.4xlarge"),
            Self::R6gd_8xlarge => write!(f, "r6gd.8xlarge"),
            Self::R6gd_12xlarge => write!(f, "r6gd.12xlarge"),
            Self::R6gd_16xlarge => write!(f, "r6gd.16xlarge"),
            Self::R6id_large => write!(f, "r6id.large"),
            Self::R6id_xlarge => write!(f, "r6id.xlarge"),
            Self::R6id_2xlarge => write!(f, "r6id.2xlarge"),
            Self::R6id_4xlarge => write!(f, "r6id.4xlarge"),
            Self::R6id_8xlarge => write!(f, "r6id.8xlarge"),
            Self::R6id_12xlarge => write!(f, "r6id.12xlarge"),
            Self::R6id_16xlarge => write!(f, "r6id.16xlarge"),
            Self::R6id_24xlarge => write!(f, "r6id.24xlarge"),
            Self::R6id_32xlarge => write!(f, "r6id.32xlarge"),
            Self::R8gd_medium => write!(f, "r8gd.medium"),
            Self::R8gd_large => write!(f, "r8gd.large"),
            Self::R8gd_xlarge => write!(f, "r8gd.xlarge"),
            Self::R8gd_2xlarge => write!(f, "r8gd.2xlarge"),
            Self::R8gd_4xlarge => write!(f, "r8gd.4xlarge"),
            Self::R8gd_8xlarge => write!(f, "r8gd.8xlarge"),
            Self::R8gd_12xlarge => write!(f, "r8gd.12xlarge"),
            Self::R8gd_16xlarge => write!(f, "r8gd.16xlarge"),
            Self::R8gd_24xlarge => write!(f, "r8gd.24xlarge"),
            Self::R8gd_48xlarge => write!(f, "r8gd.48xlarge"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `pgStateProperty` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgStateProperty {
    #[serde(rename = "creating")]
    #[default]
    Creating,
    #[serde(rename = "restarting")]
    Restarting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "replaying_wal")]
    Replaying_wal,
    #[serde(rename = "restoring_backup")]
    Restoring_backup,
    #[serde(rename = "finalizing_restore")]
    Finalizing_restore,
    #[serde(rename = "unavailable")]
    Unavailable,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "deleting")]
    Deleting,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgStateProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Restarting => write!(f, "restarting"),
            Self::Running => write!(f, "running"),
            Self::Replaying_wal => write!(f, "replaying_wal"),
            Self::Restoring_backup => write!(f, "restoring_backup"),
            Self::Finalizing_restore => write!(f, "finalizing_restore"),
            Self::Unavailable => write!(f, "unavailable"),
            Self::Stopped => write!(f, "stopped"),
            Self::Deleting => write!(f, "deleting"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `pgVersion` enum from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgVersion {
    #[serde(rename = "18")]
    #[default]
    _18,
    #[serde(rename = "17")]
    _17,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_18 => write!(f, "18"),
            Self::_17 => write!(f, "17"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl PgVersion {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["18", "17"];
}

/// Inline enum for `PostgresServiceSetState.command`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PostgresServiceSetStateCommand {
    #[serde(rename = "restart")]
    #[default]
    Restart,
    #[serde(rename = "promote")]
    Promote,
    #[serde(rename = "switchover")]
    Switchover,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PostgresServiceSetStateCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Restart => write!(f, "restart"),
            Self::Promote => write!(f, "promote"),
            Self::Switchover => write!(f, "switchover"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `pgConfig.default_transaction_isolation`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgConfigDefaultTransactionIsolation {
    #[serde(rename = "read committed")]
    #[default]
    Read_committed,
    #[serde(rename = "repeatable read")]
    Repeatable_read,
    #[serde(rename = "serializable")]
    Serializable,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgConfigDefaultTransactionIsolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read_committed => write!(f, "read committed"),
            Self::Repeatable_read => write!(f, "repeatable read"),
            Self::Serializable => write!(f, "serializable"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `pgConfig.ssl_min_protocol_version`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgConfigSslMinProtocolVersion {
    #[serde(rename = "TLSv1")]
    #[default]
    TlsV1,
    #[serde(rename = "TLSv1.1")]
    TlsV1_1,
    #[serde(rename = "TLSv1.2")]
    TlsV1_2,
    #[serde(rename = "TLSv1.3")]
    TlsV1_3,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgConfigSslMinProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TlsV1 => write!(f, "TLSv1"),
            Self::TlsV1_1 => write!(f, "TLSv1.1"),
            Self::TlsV1_2 => write!(f, "TLSv1.2"),
            Self::TlsV1_3 => write!(f, "TLSv1.3"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `pgConfig.wal_compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PgConfigWalCompression {
    #[serde(rename = "off")]
    #[default]
    Off,
    #[serde(rename = "on")]
    On,
    #[serde(rename = "lz4")]
    Lz4,
    #[serde(rename = "zstd")]
    Zstd,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PgConfigWalCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Off => write!(f, "off"),
            Self::On => write!(f, "on"),
            Self::Lz4 => write!(f, "lz4"),
            Self::Zstd => write!(f, "zstd"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Type alias for `pgCreatedAtProperty`.
pub type PgCreatedAtProperty = chrono::DateTime<chrono::Utc>;

/// Type alias for `pgIdProperty`.
pub type PgIdProperty = uuid::Uuid;

/// Type alias for `pgIsPrimaryProperty`.
pub type PgIsPrimaryProperty = bool;

/// Type alias for `pgNameProperty`.
pub type PgNameProperty = String;

/// Type alias for `pgPassword`.
pub type PgPassword = String;

/// Type alias for `pgPitrRestoreTargetProperty`.
pub type PgPitrRestoreTargetProperty = chrono::DateTime<chrono::Utc>;

/// Type alias for `pgRegion`.
pub type PgRegion = String;

/// Type alias for `pgStorageSize`.
pub type PgStorageSize = i64;

/// Type alias for `pgTags`.
pub type PgTags = Vec<ResourceTagsV1>;

/// Type alias for `pgTags` in response position, over
/// [`ResourceTagsV1Response`].
pub type PgTagsResponse = Vec<ResourceTagsV1Response>;

/// `BasePostgresService` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BasePostgresService {
    #[serde(rename = "haType")]
    pub ha_type: PgHaType,
    pub name: PgNameProperty,
    #[serde(rename = "postgresVersion")]
    pub postgres_version: PgVersion,
    pub provider: PgProvider,
    pub region: PgRegion,
    pub size: PgSize,
    pub tags: PgTags,
}

/// `PostgresService` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresService {
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<PgCreatedAtProperty>,
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none")]
    pub ha_type: Option<PgHaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<PgIdProperty>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<PgIsPrimaryProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<PgNameProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename = "postgresVersion", skip_serializing_if = "Option::is_none")]
    pub postgres_version: Option<PgVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<PgProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<PgRegion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<PgSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<PgStateProperty>,
    #[serde(rename = "storageSize", skip_serializing_if = "Option::is_none")]
    pub storage_size: Option<PgStorageSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<PgTagsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// `PostgresServiceListItem` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceListItem {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<PgCreatedAtProperty>,
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none")]
    pub ha_type: Option<PgHaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<PgIdProperty>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<PgIsPrimaryProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<PgNameProperty>,
    #[serde(rename = "postgresVersion", skip_serializing_if = "Option::is_none")]
    pub postgres_version: Option<PgVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<PgProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<PgRegion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<PgSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<PgStateProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<PgTagsResponse>,
}

/// `PostgresServicePasswordResource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServicePasswordResource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// `PostgresServicePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServicePatchRequest {
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none")]
    pub ha_type: Option<PgHaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<PgNameProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<PgSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<PgTags>,
}

/// `PostgresServicePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServicePostRequest {
    #[serde(rename = "haType", skip_serializing_if = "Option::is_none")]
    pub ha_type: Option<PgHaType>,
    pub name: PgNameProperty,
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none")]
    pub pg_bouncer_config: Option<PgBouncerConfig>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none")]
    pub pg_config: Option<PgConfig>,
    #[serde(rename = "postgresVersion", skip_serializing_if = "Option::is_none")]
    pub postgres_version: Option<PgVersion>,
    pub provider: PgProvider,
    pub region: PgRegion,
    pub size: PgSize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<PgTags>,
}

/// `PostgresServiceReadReplicaRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceReadReplicaRequest {
    pub name: PgNameProperty,
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none")]
    pub pg_bouncer_config: Option<PgBouncerConfig>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none")]
    pub pg_config: Option<PgConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<PgTags>,
}

/// `PostgresServiceRestoreRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceRestoreRequest {
    pub name: PgNameProperty,
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none")]
    pub pg_bouncer_config: Option<PgBouncerConfig>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none")]
    pub pg_config: Option<PgConfig>,
    #[serde(rename = "restoreTarget")]
    pub restore_target: PgPitrRestoreTargetProperty,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<PgTags>,
}

/// `PostgresServiceSetPassword` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceSetPassword {
    pub password: PgPassword,
}

/// `PostgresServiceSetState` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresServiceSetState {
    pub command: PostgresServiceSetStateCommand,
}

/// `PostgresMetricDataPoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresMetricDataPoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

/// `PostgresMetricSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresMetricSeries {
    #[serde(rename = "dataPoints", skip_serializing_if = "Option::is_none")]
    pub data_points: Option<Vec<PostgresMetricDataPoint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// `PostgresMetric` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresMetric {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<Vec<PostgresMetricSeries>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// `PostgresMetrics` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<PostgresMetric>>,
}

/// `PostgresQueryExecution` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresQueryExecution {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(rename = "cpuSysTimeUs", skip_serializing_if = "Option::is_none")]
    pub cpu_sys_time_us: Option<i64>,
    #[serde(rename = "cpuUserTimeUs", skip_serializing_if = "Option::is_none")]
    pub cpu_user_time_us: Option<i64>,
    #[serde(rename = "dbName", skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,
    #[serde(rename = "dbOperation", skip_serializing_if = "Option::is_none")]
    pub db_operation: Option<String>,
    #[serde(rename = "dbUser", skip_serializing_if = "Option::is_none")]
    pub db_user: Option<String>,
    #[serde(rename = "durationUs", skip_serializing_if = "Option::is_none")]
    pub duration_us: Option<i64>,
    #[serde(rename = "errElevel", skip_serializing_if = "Option::is_none")]
    pub err_elevel: Option<i64>,
    #[serde(rename = "errMessage", skip_serializing_if = "Option::is_none")]
    pub err_message: Option<String>,
    #[serde(rename = "errSqlstate", skip_serializing_if = "Option::is_none")]
    pub err_sqlstate: Option<String>,
    #[serde(rename = "jitDeformTimeUs", skip_serializing_if = "Option::is_none")]
    pub jit_deform_time_us: Option<i64>,
    #[serde(rename = "jitEmissionTimeUs", skip_serializing_if = "Option::is_none")]
    pub jit_emission_time_us: Option<i64>,
    #[serde(rename = "jitFunctions", skip_serializing_if = "Option::is_none")]
    pub jit_functions: Option<i64>,
    #[serde(
        rename = "jitGenerationTimeUs",
        skip_serializing_if = "Option::is_none"
    )]
    pub jit_generation_time_us: Option<i64>,
    #[serde(rename = "jitInliningTimeUs", skip_serializing_if = "Option::is_none")]
    pub jit_inlining_time_us: Option<i64>,
    #[serde(
        rename = "jitOptimizationTimeUs",
        skip_serializing_if = "Option::is_none"
    )]
    pub jit_optimization_time_us: Option<i64>,
    #[serde(rename = "localBlksDirtied", skip_serializing_if = "Option::is_none")]
    pub local_blks_dirtied: Option<i64>,
    #[serde(rename = "localBlksHit", skip_serializing_if = "Option::is_none")]
    pub local_blks_hit: Option<i64>,
    #[serde(rename = "localBlksRead", skip_serializing_if = "Option::is_none")]
    pub local_blks_read: Option<i64>,
    #[serde(rename = "localBlksWritten", skip_serializing_if = "Option::is_none")]
    pub local_blks_written: Option<i64>,
    #[serde(
        rename = "parallelWorkersLaunched",
        skip_serializing_if = "Option::is_none"
    )]
    pub parallel_workers_launched: Option<i64>,
    #[serde(
        rename = "parallelWorkersPlanned",
        skip_serializing_if = "Option::is_none"
    )]
    pub parallel_workers_planned: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<String>,
    #[serde(rename = "queryId", skip_serializing_if = "Option::is_none")]
    pub query_id: Option<String>,
    #[serde(rename = "queryText", skip_serializing_if = "Option::is_none")]
    pub query_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<i64>,
    #[serde(rename = "serverRole", skip_serializing_if = "Option::is_none")]
    pub server_role: Option<String>,
    #[serde(
        rename = "sharedBlkReadTimeUs",
        skip_serializing_if = "Option::is_none"
    )]
    pub shared_blk_read_time_us: Option<i64>,
    #[serde(
        rename = "sharedBlkWriteTimeUs",
        skip_serializing_if = "Option::is_none"
    )]
    pub shared_blk_write_time_us: Option<i64>,
    #[serde(rename = "sharedBlksDirtied", skip_serializing_if = "Option::is_none")]
    pub shared_blks_dirtied: Option<i64>,
    #[serde(rename = "sharedBlksHit", skip_serializing_if = "Option::is_none")]
    pub shared_blks_hit: Option<i64>,
    #[serde(rename = "sharedBlksRead", skip_serializing_if = "Option::is_none")]
    pub shared_blks_read: Option<i64>,
    #[serde(rename = "sharedBlksWritten", skip_serializing_if = "Option::is_none")]
    pub shared_blks_written: Option<i64>,
    #[serde(rename = "spanId", skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    #[serde(rename = "tempBlkReadTimeUs", skip_serializing_if = "Option::is_none")]
    pub temp_blk_read_time_us: Option<i64>,
    #[serde(rename = "tempBlkWriteTimeUs", skip_serializing_if = "Option::is_none")]
    pub temp_blk_write_time_us: Option<i64>,
    #[serde(rename = "tempBlksRead", skip_serializing_if = "Option::is_none")]
    pub temp_blks_read: Option<i64>,
    #[serde(rename = "tempBlksWritten", skip_serializing_if = "Option::is_none")]
    pub temp_blks_written: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(rename = "traceId", skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(rename = "walBytes", skip_serializing_if = "Option::is_none")]
    pub wal_bytes: Option<i64>,
    #[serde(rename = "walFpi", skip_serializing_if = "Option::is_none")]
    pub wal_fpi: Option<i64>,
    #[serde(rename = "walRecords", skip_serializing_if = "Option::is_none")]
    pub wal_records: Option<i64>,
}

/// `PostgresSlowQueryPattern` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresSlowQueryPattern {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(rename = "avgDurationUs", skip_serializing_if = "Option::is_none")]
    pub avg_duration_us: Option<i64>,
    #[serde(rename = "callCount", skip_serializing_if = "Option::is_none")]
    pub call_count: Option<i64>,
    #[serde(rename = "dbName", skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,
    #[serde(rename = "dbOperation", skip_serializing_if = "Option::is_none")]
    pub db_operation: Option<String>,
    #[serde(rename = "dbUser", skip_serializing_if = "Option::is_none")]
    pub db_user: Option<String>,
    #[serde(rename = "errorCount", skip_serializing_if = "Option::is_none")]
    pub error_count: Option<i64>,
    #[serde(rename = "maxDurationUs", skip_serializing_if = "Option::is_none")]
    pub max_duration_us: Option<i64>,
    #[serde(rename = "p50DurationUs", skip_serializing_if = "Option::is_none")]
    pub p50_duration_us: Option<i64>,
    #[serde(rename = "p95DurationUs", skip_serializing_if = "Option::is_none")]
    pub p95_duration_us: Option<i64>,
    #[serde(rename = "p99DurationUs", skip_serializing_if = "Option::is_none")]
    pub p99_duration_us: Option<i64>,
    #[serde(rename = "queryId", skip_serializing_if = "Option::is_none")]
    pub query_id: Option<String>,
    #[serde(rename = "queryText", skip_serializing_if = "Option::is_none")]
    pub query_text: Option<String>,
    #[serde(rename = "totalCpuTimeUs", skip_serializing_if = "Option::is_none")]
    pub total_cpu_time_us: Option<i64>,
    #[serde(rename = "totalDurationUs", skip_serializing_if = "Option::is_none")]
    pub total_duration_us: Option<i64>,
    #[serde(rename = "totalRows", skip_serializing_if = "Option::is_none")]
    pub total_rows: Option<i64>,
    #[serde(rename = "totalSharedBlksHit", skip_serializing_if = "Option::is_none")]
    pub total_shared_blks_hit: Option<i64>,
    #[serde(
        rename = "totalSharedBlksRead",
        skip_serializing_if = "Option::is_none"
    )]
    pub total_shared_blks_read: Option<i64>,
    #[serde(rename = "totalWalBytes", skip_serializing_if = "Option::is_none")]
    pub total_wal_bytes: Option<i64>,
}

/// `PostgresSlowQueryPatternDetail` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresSlowQueryPatternDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<PostgresSlowQueryPattern>,
    #[serde(rename = "recentExecutions", skip_serializing_if = "Option::is_none")]
    pub recent_executions: Option<Vec<PostgresQueryExecution>>,
}

/// `pgBouncerConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PgBouncerConfig {}

/// `pgBouncerConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`PgBouncerConfig`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing. The schema currently declares no properties.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PgBouncerConfigResponse {}

/// `pgConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PgConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_analyze_scale_factor: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_max_workers: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_naptime: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_vacuum_cost_delay: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_vacuum_cost_limit: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_vacuum_insert_scale_factor: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_vacuum_scale_factor: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_work_mem: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_transaction_isolation: Option<PgConfigDefaultTransactionIsolation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_cache_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_io_concurrency: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_in_transaction_session_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_session_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance_work_mem: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_parallel_maintenance_workers: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_parallel_workers: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_parallel_workers_per_gather: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_slot_wal_keep_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_wal_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_worker_processes: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_wal_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_page_cost: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_min_protocol_version: Option<PgConfigSslMinProtocolVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_compression: Option<PgConfigWalCompression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_keep_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_sender_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_mem: Option<serde_json::Value>,
}

/// `pgConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`PgConfig`]: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PgConfigResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_analyze_scale_factor: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_max_workers: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_naptime: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_vacuum_cost_delay: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_vacuum_cost_limit: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_vacuum_insert_scale_factor: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_vacuum_scale_factor: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_work_mem: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_transaction_isolation: Option<PgConfigDefaultTransactionIsolation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_cache_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_io_concurrency: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_in_transaction_session_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_session_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maintenance_work_mem: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_parallel_maintenance_workers: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_parallel_workers: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_parallel_workers_per_gather: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_slot_wal_keep_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_wal_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_worker_processes: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_wal_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub random_page_cost: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_min_protocol_version: Option<PgConfigSslMinProtocolVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_compression: Option<PgConfigWalCompression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_keep_size: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wal_sender_timeout: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_mem: Option<serde_json::Value>,
}

/// `postgresInstanceConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresInstanceConfig {
    #[serde(rename = "pgBouncerConfig")]
    pub pg_bouncer_config: PgBouncerConfig,
    #[serde(rename = "pgConfig")]
    pub pg_config: PgConfig,
}

/// `postgresInstanceConfig` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`PostgresInstanceConfig`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing. Writing a fetched configuration back to the API goes through
/// `TryFrom<PostgresInstanceConfigResponse>` (see [`crate::convert`]), which
/// forces every absent required field to be resolved explicitly.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresInstanceConfigResponse {
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none")]
    pub pg_bouncer_config: Option<PgBouncerConfigResponse>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none")]
    pub pg_config: Option<PgConfigResponse>,
}

/// `postgresInstanceUpdateConfigResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PostgresInstanceUpdateConfigResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(rename = "pgBouncerConfig", skip_serializing_if = "Option::is_none")]
    pub pg_bouncer_config: Option<PgBouncerConfigResponse>,
    #[serde(rename = "pgConfig", skip_serializing_if = "Option::is_none")]
    pub pg_config: Option<PgConfigResponse>,
}
