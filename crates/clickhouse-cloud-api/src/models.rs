//! Typed models for ClickHouse Cloud API schemas.
//!
//! Derived from the OpenAPI specification and kept in step with it by the drift
//! analyzer, which parses this file — every model struct, enum and type alias
//! must be declared here as literal source.
//!
//! Request models are strict and response models have every field `Option<T>`; a
//! schema used in both directions appears twice, as `{Name}` and
//! `{Name}Response`. `#[serde(default)]` is banned. See the crate-level docs for
//! the policy and the reasoning behind it.

use serde::{Deserialize, Serialize};

/// Generates the `Deserialize` impl for an externally-discriminated
/// `#[serde(untagged)]` enum.
///
/// Every ClickHouse Cloud "one of multiple variants" model whose JSON carries a
/// string discriminator field (e.g. `bucketProvider`, `type`, `kind`,
/// `displayType`, `service`, `operator`) shares the same deserialization shape:
/// buffer the payload as a [`serde_json::Value`], read the discriminator key,
/// and route each known wire value to the matching variant via
/// [`serde_json::from_value`]. This explicit dispatch avoids the greedy
/// first-match misrouting that `#[serde(untagged)]` derives suffer when variants
/// share a discriminator.
///
/// Once the payload buffers into a `Value`, deserialization cannot fail. Two
/// routes reach the enum's `Unknown(serde_json::Value)` catch-all, which holds
/// the payload verbatim so it round-trips losslessly:
///
/// * an unrecognized discriminator value, through the final catch-all arm;
/// * a recognized discriminator whose payload does not fit the selected variant
///   — e.g. the API changes a field from an array to a string — through
///   [`crate::serde_helpers::deserialize_or_raw`]. Field-level tolerance covers
///   a field the API stops sending; this covers a field whose shape it changes.
///
/// The macro emits **only** the `Deserialize` impl. The enum declaration, its
/// derives/serde attributes, and its `Display` impl must remain literal source
/// so the syn-based OpenAPI drift analyzer can inventory them structurally (it
/// cannot expand macros).
///
/// Each arm lists one or more discriminator wire values mapping to a single
/// variant, so several values can share a variant:
///
/// ```ignore
/// discriminated_union! {
///     ClickStackNumberTileColorCondition, "operator" {
///         "gt" | "gte" | "lt" | "lte" => ClickStackNumericColorCondition,
///         "between" => ClickStackBetweenColorCondition,
///         "eq" | "neq" => ClickStackEqualityColorCondition,
///     }
/// }
/// ```
///
/// Some unions discriminate one variant by the *absence* of the key rather than
/// by a wire value of it (e.g. a ClickStack chart config carries
/// `configType: "sql"` when it is a raw-SQL config and carries no `configType`
/// at all when it is a builder config). Such a union adds a trailing `none` arm
/// naming the variant the key's absence selects, plus the keys whose presence
/// disqualifies that variant:
///
/// ```ignore
/// discriminated_union! {
///     ClickStackLineChartConfig, "configType" {
///         "sql" => ClickStackLineRawSqlChartConfig,
///         none unless "connectionId" | "sqlTemplate" => ClickStackLineBuilderChartConfig,
///     }
/// }
/// ```
///
/// The `none` arm pins two semantics:
///
/// * It deliberately conflates "key absent" and "key present but not a string":
///   both produce a `None` scrutinee, so both take the arm.
/// * The `unless` keys guard against a *dropped* discriminator. A total absence
///   variant — one that cannot fail to deserialize, because none of its fields
///   is required — would otherwise absorb any keyless payload, silently
///   retyping a raw-SQL config as an empty builder config and discarding its
///   `connectionId`/`sqlTemplate`. Listing keys that only the other variants
///   carry routes such a payload to `Unknown` instead, where it survives
///   intact. Unknown *added* keys are not listed and stay ignored. If the spec
///   ever gives the absence variant one of the guard keys, drop that key from
///   the list.
///
/// Without a `none` arm, an absent or non-string discriminator falls to
/// `Unknown` through the final catch-all.
///
/// New discriminated unions in this module should use this macro rather than
/// hand-writing the impl. Enums whose variants need multi-level or nested
/// dispatch do not fit this single-key shape and must stay hand-written.
macro_rules! discriminated_union {
    (
        $enum:ident, $key:literal {
            $( $( $wire:literal )|+ => $variant:ident, )+
            $( none unless $( $guard:literal )|+ => $absent:ident, )?
        }
    ) => {
        impl<'de> Deserialize<'de> for $enum {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = serde_json::Value::deserialize(deserializer)?;
                match value.get($key).and_then(|v| v.as_str()) {
                    $(
                        $( Some($wire) )|+ => Ok(
                            crate::serde_helpers::deserialize_or_raw(value)
                                .map($enum::$variant)
                                .unwrap_or_else($enum::Unknown),
                        ),
                    )+
                    $(
                        None => Ok(
                            if [$($guard),+].iter().any(|key| value.get(key).is_some()) {
                                $enum::Unknown(value)
                            } else {
                                crate::serde_helpers::deserialize_or_raw(value)
                                    .map($enum::$absent)
                                    .unwrap_or_else($enum::Unknown)
                            },
                        ),
                    )?
                    _ => Ok($enum::Unknown(value)),
                }
            }
        }
    };
}

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

/// Inline enum for `Activity.actorType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ActivityActortype {
    #[serde(rename = "user")]
    #[default]
    User,
    #[serde(rename = "support")]
    Support,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "api")]
    Api,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ActivityActortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Support => write!(f, "support"),
            Self::System => write!(f, "system"),
            Self::Api => write!(f, "api"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Activity.keyUpdateType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ActivityKeyupdatetype {
    #[serde(rename = "created")]
    #[default]
    Created,
    #[serde(rename = "deleted")]
    Deleted,
    #[serde(rename = "name-changed")]
    Name_changed,
    #[serde(rename = "role-changed")]
    Role_changed,
    #[serde(rename = "state-changed")]
    State_changed,
    #[serde(rename = "date-changed")]
    Date_changed,
    #[serde(rename = "ip-access-list-changed")]
    Ip_access_list_changed,
    #[serde(rename = "org-role-changed")]
    Org_role_changed,
    #[serde(rename = "default-service-role-changed")]
    Default_service_role_changed,
    #[serde(rename = "service-role-changed")]
    Service_role_changed,
    #[serde(rename = "roles-v2-changed")]
    Roles_v2_changed,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ActivityKeyupdatetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Deleted => write!(f, "deleted"),
            Self::Name_changed => write!(f, "name-changed"),
            Self::Role_changed => write!(f, "role-changed"),
            Self::State_changed => write!(f, "state-changed"),
            Self::Date_changed => write!(f, "date-changed"),
            Self::Ip_access_list_changed => write!(f, "ip-access-list-changed"),
            Self::Org_role_changed => write!(f, "org-role-changed"),
            Self::Default_service_role_changed => write!(f, "default-service-role-changed"),
            Self::Service_role_changed => write!(f, "service-role-changed"),
            Self::Roles_v2_changed => write!(f, "roles-v2-changed"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Activity.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ActivityType {
    #[serde(rename = "create_organization")]
    #[default]
    Create_organization,
    #[serde(rename = "organization_update_name")]
    Organization_update_name,
    #[serde(rename = "transfer_service_in")]
    Transfer_service_in,
    #[serde(rename = "transfer_service_out")]
    Transfer_service_out,
    #[serde(rename = "save_payment_method")]
    Save_payment_method,
    #[serde(rename = "marketplace_subscription")]
    Marketplace_subscription,
    #[serde(rename = "migrate_marketplace_billing_details_in")]
    Migrate_marketplace_billing_details_in,
    #[serde(rename = "migrate_marketplace_billing_details_out")]
    Migrate_marketplace_billing_details_out,
    #[serde(rename = "organization_update_tier")]
    Organization_update_tier,
    #[serde(rename = "organization_invite_create")]
    Organization_invite_create,
    #[serde(rename = "organization_invite_delete")]
    Organization_invite_delete,
    #[serde(rename = "organization_member_join")]
    Organization_member_join,
    #[serde(rename = "organization_member_add")]
    Organization_member_add,
    #[serde(rename = "organization_member_leave")]
    Organization_member_leave,
    #[serde(rename = "organization_member_delete")]
    Organization_member_delete,
    #[serde(rename = "organization_member_update_role")]
    Organization_member_update_role,
    #[serde(rename = "organization_member_update_roles")]
    Organization_member_update_roles,
    #[serde(rename = "organization_member_update_mfa_method")]
    Organization_member_update_mfa_method,
    #[serde(rename = "organization_saml_connection_create")]
    Organization_saml_connection_create,
    #[serde(rename = "organization_saml_connection_update")]
    Organization_saml_connection_update,
    #[serde(rename = "user_login")]
    User_login,
    #[serde(rename = "user_login_failed")]
    User_login_failed,
    #[serde(rename = "user_logout")]
    User_logout,
    #[serde(rename = "key_create")]
    Key_create,
    #[serde(rename = "key_delete")]
    Key_delete,
    #[serde(rename = "openapi_key_update")]
    Openapi_key_update,
    #[serde(rename = "service_create")]
    Service_create,
    #[serde(rename = "service_start")]
    Service_start,
    #[serde(rename = "service_stop")]
    Service_stop,
    #[serde(rename = "service_awaken")]
    Service_awaken,
    #[serde(rename = "service_idle")]
    Service_idle,
    #[serde(rename = "service_running")]
    Service_running,
    #[serde(rename = "service_partially_running")]
    Service_partially_running,
    #[serde(rename = "service_delete")]
    Service_delete,
    #[serde(rename = "service_update_name")]
    Service_update_name,
    #[serde(rename = "service_update_ip_access_list")]
    Service_update_ip_access_list,
    #[serde(rename = "service_update_autoscaling_memory")]
    Service_update_autoscaling_memory,
    #[serde(rename = "service_update_autoscaling_idling")]
    Service_update_autoscaling_idling,
    #[serde(rename = "service_update_password")]
    Service_update_password,
    #[serde(rename = "service_update_autoscaling_replicas")]
    Service_update_autoscaling_replicas,
    #[serde(rename = "service_update_max_allowable_replicas")]
    Service_update_max_allowable_replicas,
    #[serde(rename = "service_update_backup_configuration")]
    Service_update_backup_configuration,
    #[serde(rename = "service_update_snapshot_configuration")]
    Service_update_snapshot_configuration,
    #[serde(rename = "service_restore_backup")]
    Service_restore_backup,
    #[serde(rename = "service_update_release_channel")]
    Service_update_release_channel,
    #[serde(rename = "service_update_gpt_usage_consent")]
    Service_update_gpt_usage_consent,
    #[serde(rename = "service_update_private_endpoints")]
    Service_update_private_endpoints,
    #[serde(rename = "service_import_to_organization")]
    Service_import_to_organization,
    #[serde(rename = "service_export_from_organization")]
    Service_export_from_organization,
    #[serde(rename = "service_maintenance_start")]
    Service_maintenance_start,
    #[serde(rename = "service_maintenance_end")]
    Service_maintenance_end,
    #[serde(rename = "service_update_core_dump")]
    Service_update_core_dump,
    #[serde(rename = "backup_delete")]
    Backup_delete,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create_organization => write!(f, "create_organization"),
            Self::Organization_update_name => write!(f, "organization_update_name"),
            Self::Transfer_service_in => write!(f, "transfer_service_in"),
            Self::Transfer_service_out => write!(f, "transfer_service_out"),
            Self::Save_payment_method => write!(f, "save_payment_method"),
            Self::Marketplace_subscription => write!(f, "marketplace_subscription"),
            Self::Migrate_marketplace_billing_details_in => {
                write!(f, "migrate_marketplace_billing_details_in")
            }
            Self::Migrate_marketplace_billing_details_out => {
                write!(f, "migrate_marketplace_billing_details_out")
            }
            Self::Organization_update_tier => write!(f, "organization_update_tier"),
            Self::Organization_invite_create => write!(f, "organization_invite_create"),
            Self::Organization_invite_delete => write!(f, "organization_invite_delete"),
            Self::Organization_member_join => write!(f, "organization_member_join"),
            Self::Organization_member_add => write!(f, "organization_member_add"),
            Self::Organization_member_leave => write!(f, "organization_member_leave"),
            Self::Organization_member_delete => write!(f, "organization_member_delete"),
            Self::Organization_member_update_role => write!(f, "organization_member_update_role"),
            Self::Organization_member_update_roles => {
                write!(f, "organization_member_update_roles")
            }
            Self::Organization_member_update_mfa_method => {
                write!(f, "organization_member_update_mfa_method")
            }
            Self::Organization_saml_connection_create => {
                write!(f, "organization_saml_connection_create")
            }
            Self::Organization_saml_connection_update => {
                write!(f, "organization_saml_connection_update")
            }
            Self::User_login => write!(f, "user_login"),
            Self::User_login_failed => write!(f, "user_login_failed"),
            Self::User_logout => write!(f, "user_logout"),
            Self::Key_create => write!(f, "key_create"),
            Self::Key_delete => write!(f, "key_delete"),
            Self::Openapi_key_update => write!(f, "openapi_key_update"),
            Self::Service_create => write!(f, "service_create"),
            Self::Service_start => write!(f, "service_start"),
            Self::Service_stop => write!(f, "service_stop"),
            Self::Service_awaken => write!(f, "service_awaken"),
            Self::Service_idle => write!(f, "service_idle"),
            Self::Service_running => write!(f, "service_running"),
            Self::Service_partially_running => write!(f, "service_partially_running"),
            Self::Service_delete => write!(f, "service_delete"),
            Self::Service_update_name => write!(f, "service_update_name"),
            Self::Service_update_ip_access_list => write!(f, "service_update_ip_access_list"),
            Self::Service_update_autoscaling_memory => {
                write!(f, "service_update_autoscaling_memory")
            }
            Self::Service_update_autoscaling_idling => {
                write!(f, "service_update_autoscaling_idling")
            }
            Self::Service_update_password => write!(f, "service_update_password"),
            Self::Service_update_autoscaling_replicas => {
                write!(f, "service_update_autoscaling_replicas")
            }
            Self::Service_update_max_allowable_replicas => {
                write!(f, "service_update_max_allowable_replicas")
            }
            Self::Service_update_backup_configuration => {
                write!(f, "service_update_backup_configuration")
            }
            Self::Service_update_snapshot_configuration => {
                write!(f, "service_update_snapshot_configuration")
            }
            Self::Service_restore_backup => write!(f, "service_restore_backup"),
            Self::Service_update_release_channel => write!(f, "service_update_release_channel"),
            Self::Service_update_gpt_usage_consent => write!(f, "service_update_gpt_usage_consent"),
            Self::Service_update_private_endpoints => write!(f, "service_update_private_endpoints"),
            Self::Service_import_to_organization => write!(f, "service_import_to_organization"),
            Self::Service_export_from_organization => write!(f, "service_export_from_organization"),
            Self::Service_maintenance_start => write!(f, "service_maintenance_start"),
            Self::Service_maintenance_end => write!(f, "service_maintenance_end"),
            Self::Service_update_core_dump => write!(f, "service_update_core_dump"),
            Self::Backup_delete => write!(f, "backup_delete"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ApiKey.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ApiKeyState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ApiKeyPatchRequest.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyPatchRequestState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ApiKeyPatchRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ApiKeyPostRequest.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ApiKeyPostRequestState {
    #[serde(rename = "enabled")]
    #[default]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ApiKeyPostRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AssignedRole.roleType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AssignedRoleRoletype {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "custom")]
    Custom,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AssignedRoleRoletype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::Custom => write!(f, "custom"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketBucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketPatchRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPostRequestV1Bucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketPostRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AwsBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AwsBackupBucketPropertiesBucketprovider {
    #[default]
    AWS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AwsBackupBucketPropertiesBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AWS => write!(f, "AWS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketBucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketPatchRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPostRequestV1Bucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketPostRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `AzureBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AzureBackupBucketPropertiesBucketprovider {
    #[default]
    AZURE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AzureBackupBucketPropertiesBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AZURE => write!(f, "AZURE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Backup.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum BackupStatus {
    #[serde(rename = "done")]
    #[default]
    Done,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "in_progress")]
    In_progress,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for BackupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Done => write!(f, "done"),
            Self::Error => write!(f, "error"),
            Self::In_progress => write!(f, "in_progress"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Backup.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum BackupType {
    #[serde(rename = "full")]
    #[default]
    Full,
    #[serde(rename = "incremental")]
    Incremental,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for BackupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "full"),
            Self::Incremental => write!(f, "incremental"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocConfig.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocConfigCloudprovider {
    #[serde(rename = "gcp")]
    #[default]
    Gcp,
    #[serde(rename = "aws")]
    Aws,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocConfigCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocConfig.regionId`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocConfigRegionid {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "ca-central-1")]
    Ca_central_1,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west2")]
    Europe_west2,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocConfigRegionid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Ca_central_1 => write!(f, "ca-central-1"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west2 => write!(f, "europe-west2"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocConfig.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocConfigState {
    #[serde(rename = "infra-ready")]
    #[default]
    Infra_ready,
    #[serde(rename = "infra-provisioning")]
    Infra_provisioning,
    #[serde(rename = "infra-terminated")]
    Infra_terminated,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocConfigState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Infra_ready => write!(f, "infra-ready"),
            Self::Infra_provisioning => write!(f, "infra-provisioning"),
            Self::Infra_terminated => write!(f, "infra-terminated"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ByocInfrastructurePostRequest.regionId`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ByocInfrastructurePostRequestRegionid {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "ca-central-1")]
    Ca_central_1,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west2")]
    Europe_west2,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ByocInfrastructurePostRequestRegionid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Ca_central_1 => write!(f, "ca-central-1"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west2 => write!(f, "europe-west2"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipe.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeState {
    #[default]
    Unknown,
    Provisioning,
    Running,
    Stopping,
    Stopped,
    Failed,
    Completed,
    InternalError,
    Setup,
    Snapshot,
    Paused,
    Pausing,
    Modifying,
    Resync,
    Degraded,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Other(String),
}

impl std::fmt::Display for ClickPipeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Provisioning => write!(f, "Provisioning"),
            Self::Running => write!(f, "Running"),
            Self::Stopping => write!(f, "Stopping"),
            Self::Stopped => write!(f, "Stopped"),
            Self::Failed => write!(f, "Failed"),
            Self::Completed => write!(f, "Completed"),
            Self::InternalError => write!(f, "InternalError"),
            Self::Setup => write!(f, "Setup"),
            Self::Snapshot => write!(f, "Snapshot"),
            Self::Paused => write!(f, "Paused"),
            Self::Pausing => write!(f, "Pausing"),
            Self::Modifying => write!(f, "Modifying"),
            Self::Resync => write!(f, "Resync"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeBigQueryPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryPipeSettingsReplicationmode {
    #[serde(rename = "snapshot")]
    #[default]
    Snapshot,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeBigQueryPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot => write!(f, "snapshot"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeBigQueryPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeBigQueryPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeBigQueryPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeDestinationTableEngine.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeDestinationTableEngineType {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    SummingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeDestinationTableEngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::SummingMergeTree => write!(f, "SummingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

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

/// Inline enum for `ClickPipeKafkaSchemaRegistry.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeKafkaSchemaRegistryAuthentication {
    #[default]
    PLAIN,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeKafkaSchemaRegistryAuthentication {
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

/// Inline enum for `ClickPipeMongoDBPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMongoDBPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cdc => write!(f, "cdc"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Cdc_only => write!(f, "cdc_only"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMongoDBPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMongoDBPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMongoDBSourceReadpreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::PrimaryPreferred => write!(f, "primaryPreferred"),
            Self::Secondary => write!(f, "secondary"),
            Self::SecondaryPreferred => write!(f, "secondaryPreferred"),
            Self::Nearest => write!(f, "nearest"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateKafkaSchemaRegistry.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateKafkaSchemaRegistryAuthentication {
    #[default]
    PLAIN,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateKafkaSchemaRegistryAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PLAIN => write!(f, "PLAIN"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateMongoDBSourceReadpreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::PrimaryPreferred => write!(f, "primaryPreferred"),
            Self::Secondary => write!(f, "secondary"),
            Self::SecondaryPreferred => write!(f, "secondaryPreferred"),
            Self::Nearest => write!(f, "nearest"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutateMySQLSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutateMySQLSourceType {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    #[serde(rename = "rdsmysql")]
    Rdsmysql,
    #[serde(rename = "auroramysql")]
    Auroramysql,
    #[serde(rename = "mariadb")]
    Mariadb,
    #[serde(rename = "rdsmariadb")]
    Rdsmariadb,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutateMySQLSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Rdsmysql => write!(f, "rdsmysql"),
            Self::Auroramysql => write!(f, "auroramysql"),
            Self::Mariadb => write!(f, "mariadb"),
            Self::Rdsmariadb => write!(f, "rdsmariadb"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutatePostgresSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutatePostgresSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutatePostgresSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMutatePostgresSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMutatePostgresSourceType {
    #[serde(rename = "postgres")]
    #[default]
    Postgres,
    #[serde(rename = "supabase")]
    Supabase,
    #[serde(rename = "neon")]
    Neon,
    #[serde(rename = "alloydb")]
    Alloydb,
    #[serde(rename = "planetscale")]
    Planetscale,
    #[serde(rename = "rdspostgres")]
    Rdspostgres,
    #[serde(rename = "aurorapostgres")]
    Aurorapostgres,
    #[serde(rename = "cloudsqlpostgres")]
    Cloudsqlpostgres,
    #[serde(rename = "azurepostgres")]
    Azurepostgres,
    #[serde(rename = "crunchybridge")]
    Crunchybridge,
    #[serde(rename = "tigerdata")]
    Tigerdata,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMutatePostgresSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "postgres"),
            Self::Supabase => write!(f, "supabase"),
            Self::Neon => write!(f, "neon"),
            Self::Alloydb => write!(f, "alloydb"),
            Self::Planetscale => write!(f, "planetscale"),
            Self::Rdspostgres => write!(f, "rdspostgres"),
            Self::Aurorapostgres => write!(f, "aurorapostgres"),
            Self::Cloudsqlpostgres => write!(f, "cloudsqlpostgres"),
            Self::Azurepostgres => write!(f, "azurepostgres"),
            Self::Crunchybridge => write!(f, "crunchybridge"),
            Self::Tigerdata => write!(f, "tigerdata"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLPipeSettings.replicationMechanism`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeSettingsReplicationmechanism {
    #[default]
    GTID,
    FILE_POS,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLPipeSettingsReplicationmechanism {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GTID => write!(f, "GTID"),
            Self::FILE_POS => write!(f, "FILE_POS"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cdc => write!(f, "cdc"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Cdc_only => write!(f, "cdc_only"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeMySQLSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeMySQLSourceType {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    #[serde(rename = "rdsmysql")]
    Rdsmysql,
    #[serde(rename = "auroramysql")]
    Auroramysql,
    #[serde(rename = "mariadb")]
    Mariadb,
    #[serde(rename = "rdsmariadb")]
    Rdsmariadb,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeMySQLSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Rdsmysql => write!(f, "rdsmysql"),
            Self::Auroramysql => write!(f, "auroramysql"),
            Self::Mariadb => write!(f, "mariadb"),
            Self::Rdsmariadb => write!(f, "rdsmariadb"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::CONNECTION_STRING => write!(f, "CONNECTION_STRING"),
            Self::SERVICE_ACCOUNT => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceCompression {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "gz")]
    Gz,
    #[serde(rename = "brotli")]
    Brotli,
    #[serde(rename = "br")]
    Br,
    #[serde(rename = "xz")]
    Xz,
    LZMA,
    #[serde(rename = "zstd")]
    Zstd,
    #[serde(rename = "auto")]
    Auto,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Gzip => write!(f, "gzip"),
            Self::Gz => write!(f, "gz"),
            Self::Brotli => write!(f, "brotli"),
            Self::Br => write!(f, "br"),
            Self::Xz => write!(f, "xz"),
            Self::LZMA => write!(f, "LZMA"),
            Self::Zstd => write!(f, "zstd"),
            Self::Auto => write!(f, "auto"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceFormat {
    #[default]
    JSONEachRow,
    JSONAsObject,
    CSV,
    CSVWithNames,
    TabSeparated,
    TabSeparatedWithNames,
    Parquet,
    Avro,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::JSONAsObject => write!(f, "JSONAsObject"),
            Self::CSV => write!(f, "CSV"),
            Self::CSVWithNames => write!(f, "CSVWithNames"),
            Self::TabSeparated => write!(f, "TabSeparated"),
            Self::TabSeparatedWithNames => write!(f, "TabSeparatedWithNames"),
            Self::Parquet => write!(f, "Parquet"),
            Self::Avro => write!(f, "Avro"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeObjectStorageSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeObjectStorageSourceType {
    #[serde(rename = "s3")]
    #[default]
    S3,
    #[serde(rename = "gcs")]
    Gcs,
    #[serde(rename = "dospaces")]
    Dospaces,
    #[serde(rename = "azureblobstorage")]
    Azureblobstorage,
    #[serde(rename = "cloudflarer2")]
    Cloudflarer2,
    #[serde(rename = "ovhobjectstorage")]
    Ovhobjectstorage,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeObjectStorageSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::S3 => write!(f, "s3"),
            Self::Gcs => write!(f, "gcs"),
            Self::Dospaces => write!(f, "dospaces"),
            Self::Azureblobstorage => write!(f, "azureblobstorage"),
            Self::Cloudflarer2 => write!(f, "cloudflarer2"),
            Self::Ovhobjectstorage => write!(f, "ovhobjectstorage"),
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

/// Inline enum for `ClickPipePatchMongoDBPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMongoDBPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMongoDBPipeRemoveTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMongoDBSource.readPreference`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMongoDBSourceReadpreference {
    #[serde(rename = "primary")]
    #[default]
    Primary,
    #[serde(rename = "primaryPreferred")]
    PrimaryPreferred,
    #[serde(rename = "secondary")]
    Secondary,
    #[serde(rename = "secondaryPreferred")]
    SecondaryPreferred,
    #[serde(rename = "nearest")]
    Nearest,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMongoDBSourceReadpreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::PrimaryPreferred => write!(f, "primaryPreferred"),
            Self::Secondary => write!(f, "secondary"),
            Self::SecondaryPreferred => write!(f, "secondaryPreferred"),
            Self::Nearest => write!(f, "nearest"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMySQLPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMySQLPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMySQLPipeRemoveTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchMySQLSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchMySQLSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchMySQLSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchObjectStorageSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::CONNECTION_STRING => write!(f, "CONNECTION_STRING"),
            Self::SERVICE_ACCOUNT => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchPostgresPipeRemoveTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchPostgresPipeRemoveTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchPostgresPipeRemoveTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePatchPubSubSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePatchPubSubSourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT")]
    #[default]
    ServiceAccount,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePatchPubSubSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccount => write!(f, "SERVICE_ACCOUNT"),
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
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
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

/// Inline enum for `ClickPipePostObjectStorageSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceAuthentication {
    #[default]
    IAM_ROLE,
    IAM_USER,
    CONNECTION_STRING,
    SERVICE_ACCOUNT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::IAM_USER => write!(f, "IAM_USER"),
            Self::CONNECTION_STRING => write!(f, "CONNECTION_STRING"),
            Self::SERVICE_ACCOUNT => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.compression`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceCompression {
    #[serde(rename = "none")]
    #[default]
    None,
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "gz")]
    Gz,
    #[serde(rename = "brotli")]
    Brotli,
    #[serde(rename = "br")]
    Br,
    #[serde(rename = "xz")]
    Xz,
    LZMA,
    #[serde(rename = "zstd")]
    Zstd,
    #[serde(rename = "auto")]
    Auto,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceCompression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Gzip => write!(f, "gzip"),
            Self::Gz => write!(f, "gz"),
            Self::Brotli => write!(f, "brotli"),
            Self::Br => write!(f, "br"),
            Self::Xz => write!(f, "xz"),
            Self::LZMA => write!(f, "LZMA"),
            Self::Zstd => write!(f, "zstd"),
            Self::Auto => write!(f, "auto"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceFormat {
    #[default]
    JSONEachRow,
    JSONAsObject,
    CSV,
    CSVWithNames,
    TabSeparated,
    TabSeparatedWithNames,
    Parquet,
    Avro,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::JSONAsObject => write!(f, "JSONAsObject"),
            Self::CSV => write!(f, "CSV"),
            Self::CSVWithNames => write!(f, "CSVWithNames"),
            Self::TabSeparated => write!(f, "TabSeparated"),
            Self::TabSeparatedWithNames => write!(f, "TabSeparatedWithNames"),
            Self::Parquet => write!(f, "Parquet"),
            Self::Avro => write!(f, "Avro"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostObjectStorageSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostObjectStorageSourceType {
    #[serde(rename = "s3")]
    #[default]
    S3,
    #[serde(rename = "gcs")]
    Gcs,
    #[serde(rename = "dospaces")]
    Dospaces,
    #[serde(rename = "azureblobstorage")]
    Azureblobstorage,
    #[serde(rename = "cloudflarer2")]
    Cloudflarer2,
    #[serde(rename = "ovhobjectstorage")]
    Ovhobjectstorage,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostObjectStorageSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::S3 => write!(f, "s3"),
            Self::Gcs => write!(f, "gcs"),
            Self::Dospaces => write!(f, "dospaces"),
            Self::Azureblobstorage => write!(f, "azureblobstorage"),
            Self::Cloudflarer2 => write!(f, "cloudflarer2"),
            Self::Ovhobjectstorage => write!(f, "ovhobjectstorage"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostPubSubSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostPubSubSourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT")]
    #[default]
    ServiceAccount,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostPubSubSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccount => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostPubSubSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostPubSubSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostPubSubSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::Protobuf => write!(f, "Protobuf"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostPubSubSource.seekType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostPubSubSourceSeektype {
    #[serde(rename = "latest")]
    #[default]
    Latest,
    #[serde(rename = "earliest")]
    Earliest,
    #[serde(rename = "timestamp")]
    Timestamp,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostPubSubSourceSeektype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Latest => write!(f, "latest"),
            Self::Earliest => write!(f, "earliest"),
            Self::Timestamp => write!(f, "timestamp"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresPipeSettings.replicationMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresPipeSettingsReplicationmode {
    #[serde(rename = "cdc")]
    #[default]
    Cdc,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "cdc_only")]
    Cdc_only,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresPipeSettingsReplicationmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cdc => write!(f, "cdc"),
            Self::Snapshot => write!(f, "snapshot"),
            Self::Cdc_only => write!(f, "cdc_only"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresPipeTableMapping.tableEngine`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresPipeTableMappingTableengine {
    #[default]
    MergeTree,
    ReplacingMergeTree,
    Null,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresPipeTableMappingTableengine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MergeTree => write!(f, "MergeTree"),
            Self::ReplacingMergeTree => write!(f, "ReplacingMergeTree"),
            Self::Null => write!(f, "Null"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresSourceAuthentication {
    #[serde(rename = "basic")]
    #[default]
    Basic,
    IAM_ROLE,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic => write!(f, "basic"),
            Self::IAM_ROLE => write!(f, "IAM_ROLE"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePostgresSource.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePostgresSourceType {
    #[serde(rename = "postgres")]
    #[default]
    Postgres,
    #[serde(rename = "supabase")]
    Supabase,
    #[serde(rename = "neon")]
    Neon,
    #[serde(rename = "alloydb")]
    Alloydb,
    #[serde(rename = "planetscale")]
    Planetscale,
    #[serde(rename = "rdspostgres")]
    Rdspostgres,
    #[serde(rename = "aurorapostgres")]
    Aurorapostgres,
    #[serde(rename = "cloudsqlpostgres")]
    Cloudsqlpostgres,
    #[serde(rename = "azurepostgres")]
    Azurepostgres,
    #[serde(rename = "crunchybridge")]
    Crunchybridge,
    #[serde(rename = "tigerdata")]
    Tigerdata,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePostgresSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "postgres"),
            Self::Supabase => write!(f, "supabase"),
            Self::Neon => write!(f, "neon"),
            Self::Alloydb => write!(f, "alloydb"),
            Self::Planetscale => write!(f, "planetscale"),
            Self::Rdspostgres => write!(f, "rdspostgres"),
            Self::Aurorapostgres => write!(f, "aurorapostgres"),
            Self::Cloudsqlpostgres => write!(f, "cloudsqlpostgres"),
            Self::Azurepostgres => write!(f, "azurepostgres"),
            Self::Crunchybridge => write!(f, "crunchybridge"),
            Self::Tigerdata => write!(f, "tigerdata"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePubSubSource.authentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePubSubSourceAuthentication {
    #[serde(rename = "SERVICE_ACCOUNT")]
    #[default]
    ServiceAccount,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePubSubSourceAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServiceAccount => write!(f, "SERVICE_ACCOUNT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePubSubSource.format`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePubSubSourceFormat {
    #[default]
    JSONEachRow,
    Avro,
    Protobuf,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePubSubSourceFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JSONEachRow => write!(f, "JSONEachRow"),
            Self::Avro => write!(f, "Avro"),
            Self::Protobuf => write!(f, "Protobuf"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipePubSubSource.seekType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipePubSubSourceSeektype {
    #[serde(rename = "latest")]
    #[default]
    Latest,
    #[serde(rename = "earliest")]
    Earliest,
    #[serde(rename = "timestamp")]
    Timestamp,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipePubSubSourceSeektype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Latest => write!(f, "latest"),
            Self::Earliest => write!(f, "earliest"),
            Self::Timestamp => write!(f, "timestamp"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickPipeStatePatchRequest.command`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickPipeStatePatchRequestCommand {
    #[serde(rename = "start")]
    #[default]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "resync")]
    Resync,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickPipeStatePatchRequestCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::Stop => write!(f, "stop"),
            Self::Resync => write!(f, "resync"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertChannelEmail.type`.
///
/// The spec gives both alert-channel variants the same `enum: ["webhook",
/// "email"]`, so `#[default]` sits on `Email` rather than on the first value:
/// this field discriminates the `ClickStackAlertChannel` union, and defaulting
/// it to `webhook` would make `ClickStackAlertChannelEmail::default()`
/// deserialize back as the webhook variant.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelEmailType {
    #[serde(rename = "webhook")]
    Webhook,
    #[serde(rename = "email")]
    #[default]
    Email,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertChannelEmailType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Webhook => write!(f, "webhook"),
            Self::Email => write!(f, "email"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertChannelWebhook.severity`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelWebhookSeverity {
    #[serde(rename = "critical")]
    #[default]
    Critical,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "info")]
    Info,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertChannelWebhookSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertChannelWebhook.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertChannelWebhookType {
    #[serde(rename = "webhook")]
    #[default]
    Webhook,
    #[serde(rename = "email")]
    Email,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertChannelWebhookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Webhook => write!(f, "webhook"),
            Self::Email => write!(f, "email"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertExecutionError.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertExecutionErrorType {
    #[default]
    QUERY_ERROR,
    WEBHOOK_ERROR,
    INVALID_ALERT,
    UNKNOWN,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertExecutionErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QUERY_ERROR => write!(f, "QUERY_ERROR"),
            Self::WEBHOOK_ERROR => write!(f, "WEBHOOK_ERROR"),
            Self::INVALID_ALERT => write!(f, "INVALID_ALERT"),
            Self::UNKNOWN => write!(f, "UNKNOWN"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saved_search => write!(f, "saved_search"),
            Self::Tile => write!(f, "tile"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseState {
    #[default]
    ALERT,
    OK,
    INSUFFICIENT_DATA,
    DISABLED,
    PENDING,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ALERT => write!(f, "ALERT"),
            Self::OK => write!(f, "OK"),
            Self::INSUFFICIENT_DATA => write!(f, "INSUFFICIENT_DATA"),
            Self::DISABLED => write!(f, "DISABLED"),
            Self::PENDING => write!(f, "PENDING"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackAlertResponse.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackAlertResponseThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
    #[serde(rename = "above_exclusive")]
    Above_exclusive,
    #[serde(rename = "below_or_equal")]
    Below_or_equal,
    #[serde(rename = "equal")]
    Equal,
    #[serde(rename = "not_equal")]
    Not_equal,
    #[serde(rename = "between")]
    Between,
    #[serde(rename = "not_between")]
    Not_between,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackAlertResponseThresholdtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Above => write!(f, "above"),
            Self::Below => write!(f, "below"),
            Self::Above_exclusive => write!(f, "above_exclusive"),
            Self::Below_or_equal => write!(f, "below_or_equal"),
            Self::Equal => write!(f, "equal"),
            Self::Not_equal => write!(f, "not_equal"),
            Self::Between => write!(f, "between"),
            Self::Not_between => write!(f, "not_between"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBackgroundChart.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBackgroundChartType {
    #[serde(rename = "line")]
    #[default]
    Line,
    #[serde(rename = "area")]
    Area,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBackgroundChartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line => write!(f, "line"),
            Self::Area => write!(f, "area"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBarBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarBuilderChartConfigDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBarBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stacked_bar => write!(f, "stacked_bar"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBarRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBarRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBarRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBarRawSqlChartConfigDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBarRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stacked_bar => write!(f, "stacked_bar"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackBetweenColorCondition.operator`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackBetweenColorConditionOperator {
    #[serde(rename = "between")]
    #[default]
    Between,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackBetweenColorConditionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Between => write!(f, "between"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCategoricalBarBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCategoricalBarBuilderChartConfigDisplaytype {
    #[serde(rename = "bar")]
    #[default]
    Bar,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCategoricalBarBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bar => write!(f, "bar"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCategoricalBarRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCategoricalBarRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCategoricalBarRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCategoricalBarRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCategoricalBarRawSqlChartConfigDisplaytype {
    #[serde(rename = "bar")]
    #[default]
    Bar,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCategoricalBarRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bar => write!(f, "bar"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Palette-token colors shared by ClickStack chart tiles.
///
/// Used by `ClickStackBackgroundChart`, `ClickStackNumericColorCondition`,
/// `ClickStackBetweenColorCondition`, `ClickStackEqualityColorCondition`,
/// `ClickStackNumberBuilderChartConfig`, and `ClickStackNumberRawSqlChartConfig`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackChartColor {
    #[serde(rename = "chart-blue")]
    #[default]
    Chart_blue,
    #[serde(rename = "chart-orange")]
    Chart_orange,
    #[serde(rename = "chart-red")]
    Chart_red,
    #[serde(rename = "chart-cyan")]
    Chart_cyan,
    #[serde(rename = "chart-green")]
    Chart_green,
    #[serde(rename = "chart-pink")]
    Chart_pink,
    #[serde(rename = "chart-purple")]
    Chart_purple,
    #[serde(rename = "chart-light-blue")]
    Chart_light_blue,
    #[serde(rename = "chart-brown")]
    Chart_brown,
    #[serde(rename = "chart-gray")]
    Chart_gray,
    #[serde(rename = "chart-success")]
    Chart_success,
    #[serde(rename = "chart-warning")]
    Chart_warning,
    #[serde(rename = "chart-error")]
    Chart_error,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackChartColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Chart_blue => write!(f, "chart-blue"),
            Self::Chart_orange => write!(f, "chart-orange"),
            Self::Chart_red => write!(f, "chart-red"),
            Self::Chart_cyan => write!(f, "chart-cyan"),
            Self::Chart_green => write!(f, "chart-green"),
            Self::Chart_pink => write!(f, "chart-pink"),
            Self::Chart_purple => write!(f, "chart-purple"),
            Self::Chart_light_blue => write!(f, "chart-light-blue"),
            Self::Chart_brown => write!(f, "chart-brown"),
            Self::Chart_gray => write!(f, "chart-gray"),
            Self::Chart_success => write!(f, "chart-success"),
            Self::Chart_warning => write!(f, "chart-warning"),
            Self::Chart_error => write!(f, "chart-error"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateAlertRequest.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateAlertRequestInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateAlertRequest.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateAlertRequestSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saved_search => write!(f, "saved_search"),
            Self::Tile => write!(f, "tile"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateAlertRequest.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateAlertRequestThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
    #[serde(rename = "above_exclusive")]
    Above_exclusive,
    #[serde(rename = "below_or_equal")]
    Below_or_equal,
    #[serde(rename = "equal")]
    Equal,
    #[serde(rename = "not_equal")]
    Not_equal,
    #[serde(rename = "between")]
    Between,
    #[serde(rename = "not_between")]
    Not_between,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateAlertRequestThresholdtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Above => write!(f, "above"),
            Self::Below => write!(f, "below"),
            Self::Above_exclusive => write!(f, "above_exclusive"),
            Self::Below_or_equal => write!(f, "below_or_equal"),
            Self::Equal => write!(f, "equal"),
            Self::Not_equal => write!(f, "not_equal"),
            Self::Between => write!(f, "between"),
            Self::Not_between => write!(f, "not_between"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackCreateDashboardRequest.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackCreateDashboardRequestSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackCreateDashboardRequestSavedquerylanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackDashboardResponse.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackDashboardResponseSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackDashboardResponseSavedquerylanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackEqualityColorCondition.operator`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackEqualityColorConditionOperator {
    #[serde(rename = "eq")]
    #[default]
    Eq,
    #[serde(rename = "neq")]
    Neq,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackEqualityColorConditionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eq => write!(f, "eq"),
            Self::Neq => write!(f, "neq"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackEventPatternsChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackEventPatternsChartConfigDisplaytype {
    #[serde(rename = "event_patterns")]
    #[default]
    Event_patterns,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackEventPatternsChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Event_patterns => write!(f, "event_patterns"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackEventPatternsChartConfig.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackEventPatternsChartConfigWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackEventPatternsChartConfigWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilter.sourceMetricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterSourcemetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterSourcemetrictype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilter.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterType {
    #[default]
    QUERY_EXPRESSION,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QUERY_EXPRESSION => write!(f, "QUERY_EXPRESSION"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilter.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilterInput.sourceMetricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterInputSourcemetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterInputSourcemetrictype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilterInput.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterInputType {
    #[default]
    QUERY_EXPRESSION,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterInputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QUERY_EXPRESSION => write!(f, "QUERY_EXPRESSION"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackFilterInput.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackFilterInputWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackFilterInputWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackGenericWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackGenericWebhookService {
    #[serde(rename = "generic")]
    #[default]
    Generic,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackGenericWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generic => write!(f, "generic"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackHeatmapChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackHeatmapChartConfigDisplaytype {
    #[serde(rename = "heatmap")]
    #[default]
    Heatmap,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackHeatmapChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Heatmap => write!(f, "heatmap"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackHeatmapChartConfig.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackHeatmapChartConfigWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackHeatmapChartConfigWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackHeatmapSelectItem.heatmapScaleType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackHeatmapSelectItemHeatmapscaletype {
    #[serde(rename = "log")]
    #[default]
    Log,
    #[serde(rename = "linear")]
    Linear,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackHeatmapSelectItemHeatmapscaletype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Log => write!(f, "log"),
            Self::Linear => write!(f, "linear"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackIncidentIOWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackIncidentIOWebhookService {
    #[serde(rename = "incidentio")]
    #[default]
    Incidentio,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackIncidentIOWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incidentio => write!(f, "incidentio"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLineBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineBuilderChartConfigDisplaytype {
    #[serde(rename = "line")]
    #[default]
    Line,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLineBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line => write!(f, "line"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLineRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLineRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLineRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLineRawSqlChartConfigDisplaytype {
    #[serde(rename = "line")]
    #[default]
    Line,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLineRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line => write!(f, "line"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLogSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLogSourceKind {
    #[serde(rename = "log")]
    #[default]
    Log,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLogSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Log => write!(f, "log"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackLogSource.useTextIndexForImplicitColumn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackLogSourceUsetextindexforimplicitcolumn {
    #[serde(rename = "auto")]
    #[default]
    Auto,
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackLogSourceUsetextindexforimplicitcolumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMarkdownChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMarkdownChartConfigDisplaytype {
    #[serde(rename = "markdown")]
    #[default]
    Markdown,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMarkdownChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Markdown => write!(f, "markdown"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMarkdownChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMarkdownChartSeriesType {
    #[serde(rename = "markdown")]
    #[default]
    Markdown,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMarkdownChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Markdown => write!(f, "markdown"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMaterializedView.minGranularity`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMaterializedViewMingranularity {
    #[serde(rename = "1s")]
    #[default]
    _1s,
    #[serde(rename = "15s")]
    _15s,
    #[serde(rename = "30s")]
    _30s,
    #[serde(rename = "1m")]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "2h")]
    _2h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    #[serde(rename = "2d")]
    _2d,
    #[serde(rename = "7d")]
    _7d,
    #[serde(rename = "30d")]
    _30d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMaterializedViewMingranularity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1s => write!(f, "1s"),
            Self::_15s => write!(f, "15s"),
            Self::_30s => write!(f, "30s"),
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_2h => write!(f, "2h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::_2d => write!(f, "2d"),
            Self::_7d => write!(f, "7d"),
            Self::_30d => write!(f, "30d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackMetricSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackMetricSourceKind {
    #[serde(rename = "metric")]
    #[default]
    Metric,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackMetricSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Metric => write!(f, "metric"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberBuilderChartConfigDisplaytype {
    #[serde(rename = "number")]
    #[default]
    Number,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number => write!(f, "number"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesMetricdatatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesType {
    #[serde(rename = "number")]
    #[default]
    Number,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number => write!(f, "number"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberFormat.numericUnit`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberFormatNumericunit {
    #[serde(rename = "bytes_iec")]
    #[default]
    Bytes_iec,
    #[serde(rename = "bytes_si")]
    Bytes_si,
    #[serde(rename = "bits_iec")]
    Bits_iec,
    #[serde(rename = "bits_si")]
    Bits_si,
    #[serde(rename = "kibibytes")]
    Kibibytes,
    #[serde(rename = "kilobytes")]
    Kilobytes,
    #[serde(rename = "mebibytes")]
    Mebibytes,
    #[serde(rename = "megabytes")]
    Megabytes,
    #[serde(rename = "gibibytes")]
    Gibibytes,
    #[serde(rename = "gigabytes")]
    Gigabytes,
    #[serde(rename = "tebibytes")]
    Tebibytes,
    #[serde(rename = "terabytes")]
    Terabytes,
    #[serde(rename = "pebibytes")]
    Pebibytes,
    #[serde(rename = "petabytes")]
    Petabytes,
    #[serde(rename = "packets_sec")]
    Packets_sec,
    #[serde(rename = "bytes_sec_iec")]
    Bytes_sec_iec,
    #[serde(rename = "bytes_sec_si")]
    Bytes_sec_si,
    #[serde(rename = "bits_sec_iec")]
    Bits_sec_iec,
    #[serde(rename = "bits_sec_si")]
    Bits_sec_si,
    #[serde(rename = "kibibytes_sec")]
    Kibibytes_sec,
    #[serde(rename = "kibibits_sec")]
    Kibibits_sec,
    #[serde(rename = "kilobytes_sec")]
    Kilobytes_sec,
    #[serde(rename = "kilobits_sec")]
    Kilobits_sec,
    #[serde(rename = "mebibytes_sec")]
    Mebibytes_sec,
    #[serde(rename = "mebibits_sec")]
    Mebibits_sec,
    #[serde(rename = "megabytes_sec")]
    Megabytes_sec,
    #[serde(rename = "megabits_sec")]
    Megabits_sec,
    #[serde(rename = "gibibytes_sec")]
    Gibibytes_sec,
    #[serde(rename = "gibibits_sec")]
    Gibibits_sec,
    #[serde(rename = "gigabytes_sec")]
    Gigabytes_sec,
    #[serde(rename = "gigabits_sec")]
    Gigabits_sec,
    #[serde(rename = "tebibytes_sec")]
    Tebibytes_sec,
    #[serde(rename = "tebibits_sec")]
    Tebibits_sec,
    #[serde(rename = "terabytes_sec")]
    Terabytes_sec,
    #[serde(rename = "terabits_sec")]
    Terabits_sec,
    #[serde(rename = "pebibytes_sec")]
    Pebibytes_sec,
    #[serde(rename = "pebibits_sec")]
    Pebibits_sec,
    #[serde(rename = "petabytes_sec")]
    Petabytes_sec,
    #[serde(rename = "petabits_sec")]
    Petabits_sec,
    #[serde(rename = "cps")]
    Cps,
    #[serde(rename = "ops")]
    Ops,
    #[serde(rename = "rps")]
    Rps,
    #[serde(rename = "reads_sec")]
    Reads_sec,
    #[serde(rename = "wps")]
    Wps,
    #[serde(rename = "iops")]
    Iops,
    #[serde(rename = "cpm")]
    Cpm,
    #[serde(rename = "opm")]
    Opm,
    #[serde(rename = "rpm_reads")]
    Rpm_reads,
    #[serde(rename = "wpm")]
    Wpm,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberFormatNumericunit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bytes_iec => write!(f, "bytes_iec"),
            Self::Bytes_si => write!(f, "bytes_si"),
            Self::Bits_iec => write!(f, "bits_iec"),
            Self::Bits_si => write!(f, "bits_si"),
            Self::Kibibytes => write!(f, "kibibytes"),
            Self::Kilobytes => write!(f, "kilobytes"),
            Self::Mebibytes => write!(f, "mebibytes"),
            Self::Megabytes => write!(f, "megabytes"),
            Self::Gibibytes => write!(f, "gibibytes"),
            Self::Gigabytes => write!(f, "gigabytes"),
            Self::Tebibytes => write!(f, "tebibytes"),
            Self::Terabytes => write!(f, "terabytes"),
            Self::Pebibytes => write!(f, "pebibytes"),
            Self::Petabytes => write!(f, "petabytes"),
            Self::Packets_sec => write!(f, "packets_sec"),
            Self::Bytes_sec_iec => write!(f, "bytes_sec_iec"),
            Self::Bytes_sec_si => write!(f, "bytes_sec_si"),
            Self::Bits_sec_iec => write!(f, "bits_sec_iec"),
            Self::Bits_sec_si => write!(f, "bits_sec_si"),
            Self::Kibibytes_sec => write!(f, "kibibytes_sec"),
            Self::Kibibits_sec => write!(f, "kibibits_sec"),
            Self::Kilobytes_sec => write!(f, "kilobytes_sec"),
            Self::Kilobits_sec => write!(f, "kilobits_sec"),
            Self::Mebibytes_sec => write!(f, "mebibytes_sec"),
            Self::Mebibits_sec => write!(f, "mebibits_sec"),
            Self::Megabytes_sec => write!(f, "megabytes_sec"),
            Self::Megabits_sec => write!(f, "megabits_sec"),
            Self::Gibibytes_sec => write!(f, "gibibytes_sec"),
            Self::Gibibits_sec => write!(f, "gibibits_sec"),
            Self::Gigabytes_sec => write!(f, "gigabytes_sec"),
            Self::Gigabits_sec => write!(f, "gigabits_sec"),
            Self::Tebibytes_sec => write!(f, "tebibytes_sec"),
            Self::Tebibits_sec => write!(f, "tebibits_sec"),
            Self::Terabytes_sec => write!(f, "terabytes_sec"),
            Self::Terabits_sec => write!(f, "terabits_sec"),
            Self::Pebibytes_sec => write!(f, "pebibytes_sec"),
            Self::Pebibits_sec => write!(f, "pebibits_sec"),
            Self::Petabytes_sec => write!(f, "petabytes_sec"),
            Self::Petabits_sec => write!(f, "petabits_sec"),
            Self::Cps => write!(f, "cps"),
            Self::Ops => write!(f, "ops"),
            Self::Rps => write!(f, "rps"),
            Self::Reads_sec => write!(f, "reads_sec"),
            Self::Wps => write!(f, "wps"),
            Self::Iops => write!(f, "iops"),
            Self::Cpm => write!(f, "cpm"),
            Self::Opm => write!(f, "opm"),
            Self::Rpm_reads => write!(f, "rpm_reads"),
            Self::Wpm => write!(f, "wpm"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberFormat.output`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberFormatOutput {
    #[serde(rename = "currency")]
    #[default]
    Currency,
    #[serde(rename = "percent")]
    Percent,
    #[serde(rename = "byte")]
    Byte,
    #[serde(rename = "time")]
    Time,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "data_rate")]
    Data_rate,
    #[serde(rename = "throughput")]
    Throughput,
    #[serde(rename = "duration")]
    Duration,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberFormatOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Currency => write!(f, "currency"),
            Self::Percent => write!(f, "percent"),
            Self::Byte => write!(f, "byte"),
            Self::Time => write!(f, "time"),
            Self::Number => write!(f, "number"),
            Self::Data_rate => write!(f, "data_rate"),
            Self::Throughput => write!(f, "throughput"),
            Self::Duration => write!(f, "duration"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumberRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumberRawSqlChartConfigDisplaytype {
    #[serde(rename = "number")]
    #[default]
    Number,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumberRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number => write!(f, "number"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackNumericColorCondition.operator`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackNumericColorConditionOperator {
    #[serde(rename = "gt")]
    #[default]
    Gt,
    #[serde(rename = "gte")]
    Gte,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "lte")]
    Lte,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackNumericColorConditionOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gt => write!(f, "gt"),
            Self::Gte => write!(f, "gte"),
            Self::Lt => write!(f, "lt"),
            Self::Lte => write!(f, "lte"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickDashboard.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickDashboardType {
    #[serde(rename = "dashboard")]
    #[default]
    Dashboard,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickDashboardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dashboard => write!(f, "dashboard"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickDashboard.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickDashboardWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickDashboardWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickExternal.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickExternalType {
    #[serde(rename = "external")]
    #[default]
    External,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickExternalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::External => write!(f, "external"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickFilterTemplate.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickFilterTemplateKind {
    #[serde(rename = "expressionTemplate")]
    #[default]
    ExpressionTemplate,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickFilterTemplateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExpressionTemplate => write!(f, "expressionTemplate"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickSearch.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickSearchType {
    #[serde(rename = "search")]
    #[default]
    Search,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickSearchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Search => write!(f, "search"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickSearch.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickSearchWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickSearchWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickTargetIdVariant.mode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickTargetIdVariantMode {
    #[serde(rename = "id")]
    #[default]
    Id,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickTargetIdVariantMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id => write!(f, "id"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackOnClickTargetTemplateVariant.mode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackOnClickTargetTemplateVariantMode {
    #[serde(rename = "template")]
    #[default]
    Template,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackOnClickTargetTemplateVariantMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Template => write!(f, "template"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPagerDutyAPIWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPagerDutyAPIWebhookService {
    #[serde(rename = "pagerduty_api")]
    #[default]
    Pagerduty_api,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPagerDutyAPIWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pagerduty_api => write!(f, "pagerduty_api"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPieBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieBuilderChartConfigDisplaytype {
    #[serde(rename = "pie")]
    #[default]
    Pie,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPieBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pie => write!(f, "pie"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPieRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPieRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPieRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPieRawSqlChartConfigDisplaytype {
    #[serde(rename = "pie")]
    #[default]
    Pie,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPieRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pie => write!(f, "pie"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackPromqlSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackPromqlSourceKind {
    #[serde(rename = "promql")]
    #[default]
    Promql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackPromqlSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Promql => write!(f, "promql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSavedFilterValue.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSavedFilterValueType {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSavedFilterValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSavedSearchFilter.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSavedSearchFilterType {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSavedSearchFilterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSavedSearchInput.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSavedSearchInputWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSavedSearchInputWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSavedSearch.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSavedSearchWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSavedSearchWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartConfigDisplaytype {
    #[serde(rename = "search")]
    #[default]
    Search,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Search => write!(f, "search"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartConfig.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartConfigWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartConfigWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartSeriesType {
    #[serde(rename = "search")]
    #[default]
    Search,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Search => write!(f, "search"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSearchChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSearchChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSearchChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.level`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemLevel {
    #[serde(rename = "0.5")]
    #[default]
    _0_5,
    #[serde(rename = "0.9")]
    _0_9,
    #[serde(rename = "0.95")]
    _0_95,
    #[serde(rename = "0.99")]
    _0_99,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_0_5 => write!(f, "0.5"),
            Self::_0_9 => write!(f, "0.9"),
            Self::_0_95 => write!(f, "0.95"),
            Self::_0_99 => write!(f, "0.99"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.metricType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemMetrictype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemMetrictype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.periodAggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemPeriodaggfn {
    #[serde(rename = "delta")]
    #[default]
    Delta,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemPeriodaggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Delta => write!(f, "delta"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSelectItem.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSelectItemWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSelectItemWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSessionSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSessionSourceKind {
    #[serde(rename = "session")]
    #[default]
    Session,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSessionSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session => write!(f, "session"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSlackAPIWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSlackAPIWebhookService {
    #[serde(rename = "slack_api")]
    #[default]
    Slack_api,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSlackAPIWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack_api => write!(f, "slack_api"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackSlackWebhook.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackSlackWebhookService {
    #[serde(rename = "slack")]
    #[default]
    Slack,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackSlackWebhookService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack => write!(f, "slack"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableBuilderChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableBuilderChartConfigDisplaytype {
    #[serde(rename = "table")]
    #[default]
    Table,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableBuilderChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesMetricdatatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.sortOrder`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesSortorder {
    #[serde(rename = "desc")]
    #[default]
    Desc,
    #[serde(rename = "asc")]
    Asc,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesSortorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Desc => write!(f, "desc"),
            Self::Asc => write!(f, "asc"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesType {
    #[serde(rename = "table")]
    #[default]
    Table,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableRawSqlChartConfig.configType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableRawSqlChartConfigConfigtype {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableRawSqlChartConfigConfigtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTableRawSqlChartConfig.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTableRawSqlChartConfigDisplaytype {
    #[serde(rename = "table")]
    #[default]
    Table,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTableRawSqlChartConfigDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.aggFn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesAggfn {
    #[serde(rename = "avg")]
    #[default]
    Avg,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "count_distinct")]
    Count_distinct,
    #[serde(rename = "last_value")]
    Last_value,
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "min")]
    Min,
    #[serde(rename = "quantile")]
    Quantile,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "none")]
    None,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesAggfn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Avg => write!(f, "avg"),
            Self::Count => write!(f, "count"),
            Self::Count_distinct => write!(f, "count_distinct"),
            Self::Last_value => write!(f, "last_value"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Quantile => write!(f, "quantile"),
            Self::Sum => write!(f, "sum"),
            Self::Any => write!(f, "any"),
            Self::None => write!(f, "none"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.displayType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesDisplaytype {
    #[serde(rename = "stacked_bar")]
    #[default]
    Stacked_bar,
    #[serde(rename = "line")]
    Line,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesDisplaytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stacked_bar => write!(f, "stacked_bar"),
            Self::Line => write!(f, "line"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.metricDataType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesMetricdatatype {
    #[serde(rename = "sum")]
    #[default]
    Sum,
    #[serde(rename = "gauge")]
    Gauge,
    #[serde(rename = "histogram")]
    Histogram,
    #[serde(rename = "summary")]
    Summary,
    #[serde(rename = "exponential histogram")]
    Exponential_histogram,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesMetricdatatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sum => write!(f, "sum"),
            Self::Gauge => write!(f, "gauge"),
            Self::Histogram => write!(f, "histogram"),
            Self::Summary => write!(f, "summary"),
            Self::Exponential_histogram => write!(f, "exponential histogram"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesType {
    #[serde(rename = "time")]
    #[default]
    Time,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Time => write!(f, "time"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTimeChartSeries.whereLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTimeChartSeriesWherelanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTimeChartSeriesWherelanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTraceSource.kind`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTraceSourceKind {
    #[serde(rename = "trace")]
    #[default]
    Trace,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTraceSourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => write!(f, "trace"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackTraceSource.useTextIndexForImplicitColumn`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackTraceSourceUsetextindexforimplicitcolumn {
    #[serde(rename = "auto")]
    #[default]
    Auto,
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "disabled")]
    Disabled,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackTraceSourceUsetextindexforimplicitcolumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Enabled => write!(f, "enabled"),
            Self::Disabled => write!(f, "disabled"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateAlertRequest.interval`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestInterval {
    #[serde(rename = "1m")]
    #[default]
    _1m,
    #[serde(rename = "5m")]
    _5m,
    #[serde(rename = "15m")]
    _15m,
    #[serde(rename = "30m")]
    _30m,
    #[serde(rename = "1h")]
    _1h,
    #[serde(rename = "6h")]
    _6h,
    #[serde(rename = "12h")]
    _12h,
    #[serde(rename = "1d")]
    _1d,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateAlertRequestInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::_1m => write!(f, "1m"),
            Self::_5m => write!(f, "5m"),
            Self::_15m => write!(f, "15m"),
            Self::_30m => write!(f, "30m"),
            Self::_1h => write!(f, "1h"),
            Self::_6h => write!(f, "6h"),
            Self::_12h => write!(f, "12h"),
            Self::_1d => write!(f, "1d"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateAlertRequest.source`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestSource {
    #[serde(rename = "saved_search")]
    #[default]
    Saved_search,
    #[serde(rename = "tile")]
    Tile,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateAlertRequestSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saved_search => write!(f, "saved_search"),
            Self::Tile => write!(f, "tile"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateAlertRequest.thresholdType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateAlertRequestThresholdtype {
    #[serde(rename = "above")]
    #[default]
    Above,
    #[serde(rename = "below")]
    Below,
    #[serde(rename = "above_exclusive")]
    Above_exclusive,
    #[serde(rename = "below_or_equal")]
    Below_or_equal,
    #[serde(rename = "equal")]
    Equal,
    #[serde(rename = "not_equal")]
    Not_equal,
    #[serde(rename = "between")]
    Between,
    #[serde(rename = "not_between")]
    Not_between,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateAlertRequestThresholdtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Above => write!(f, "above"),
            Self::Below => write!(f, "below"),
            Self::Above_exclusive => write!(f, "above_exclusive"),
            Self::Below_or_equal => write!(f, "below_or_equal"),
            Self::Equal => write!(f, "equal"),
            Self::Not_equal => write!(f, "not_equal"),
            Self::Between => write!(f, "between"),
            Self::Not_between => write!(f, "not_between"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackUpdateDashboardRequest.savedQueryLanguage`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackUpdateDashboardRequestSavedquerylanguage {
    #[serde(rename = "sql")]
    #[default]
    Sql,
    #[serde(rename = "lucene")]
    Lucene,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackUpdateDashboardRequestSavedquerylanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql => write!(f, "sql"),
            Self::Lucene => write!(f, "lucene"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ClickStackWebhookInput.service`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ClickStackWebhookInputService {
    #[serde(rename = "slack")]
    #[default]
    Slack,
    #[serde(rename = "incidentio")]
    Incidentio,
    #[serde(rename = "generic")]
    Generic,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ClickStackWebhookInputService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack => write!(f, "slack"),
            Self::Incidentio => write!(f, "incidentio"),
            Self::Generic => write!(f, "generic"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `CreateReversePrivateEndpoint.mskAuthentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreateReversePrivateEndpointMskauthentication {
    #[default]
    SASL_IAM,
    SASL_SCRAM,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for CreateReversePrivateEndpointMskauthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SASL_IAM => write!(f, "SASL_IAM"),
            Self::SASL_SCRAM => write!(f, "SASL_SCRAM"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `CreateReversePrivateEndpoint.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CreateReversePrivateEndpointType {
    #[default]
    VPC_ENDPOINT_SERVICE,
    VPC_RESOURCE,
    MSK_MULTI_VPC,
    GCP_PSC_SERVICE_ATTACHMENT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for CreateReversePrivateEndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VPC_ENDPOINT_SERVICE => write!(f, "VPC_ENDPOINT_SERVICE"),
            Self::VPC_RESOURCE => write!(f, "VPC_RESOURCE"),
            Self::MSK_MULTI_VPC => write!(f, "MSK_MULTI_VPC"),
            Self::GCP_PSC_SERVICE_ATTACHMENT => write!(f, "GCP_PSC_SERVICE_ATTACHMENT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `autoscalingMode` enum from the ClickHouse Cloud API.
///
/// Used by `Service`, `ServicePostRequest`, `ServiceReplicaScalingPatchRequest`,
/// `ServiceScalingPatchResponse`, `ScalingScheduleBaseConfig`,
/// `ScalingScheduleEntry`, and `ScalingScheduleEntryRequest`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum AutoscalingMode {
    #[serde(rename = "vertical")]
    #[default]
    Vertical,
    #[serde(rename = "horizontal")]
    Horizontal,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for AutoscalingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vertical => write!(f, "vertical"),
            Self::Horizontal => write!(f, "horizontal"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl AutoscalingMode {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["vertical", "horizontal"];
}

/// Inline enum for `CurrentScaling.effectiveAutoscalingMode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum CurrentScalingEffectiveautoscalingmode {
    #[serde(rename = "vertical")]
    #[default]
    Vertical,
    #[serde(rename = "horizontal")]
    Horizontal,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for CurrentScalingEffectiveautoscalingmode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Vertical => write!(f, "vertical"),
            Self::Horizontal => write!(f, "horizontal"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucket.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketBucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucketPatchRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPatchRequestV1Bucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketPatchRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucketPostRequestV1.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPostRequestV1Bucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketPostRequestV1Bucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `GcpBackupBucketProperties.bucketProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum GcpBackupBucketPropertiesBucketprovider {
    #[default]
    GCP,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for GcpBackupBucketPropertiesBucketprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GCP => write!(f, "GCP"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `InstancePrivateEndpoint.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InstancePrivateEndpointCloudprovider {
    #[serde(rename = "gcp")]
    #[default]
    Gcp,
    #[serde(rename = "aws")]
    Aws,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for InstancePrivateEndpointCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `InstancePrivateEndpoint.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InstancePrivateEndpointRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "ca-central-1")]
    Ca_central_1,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west2")]
    Europe_west2,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for InstancePrivateEndpointRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Ca_central_1 => write!(f, "ca-central-1"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west2 => write!(f, "europe-west2"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Invitation.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InvitationRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for InvitationRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `InvitationPostRequest.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum InvitationPostRequestRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for InvitationPostRequestRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Member.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum MemberRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for MemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `MemberPatchRequest.role`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum MemberPatchRequestRole {
    #[serde(rename = "admin")]
    #[default]
    Admin,
    #[serde(rename = "developer")]
    Developer,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for MemberPatchRequestRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
            Self::Developer => write!(f, "developer"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPatchPrivateEndpoint.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPatchPrivateEndpointCloudprovider {
    #[serde(rename = "gcp")]
    #[default]
    Gcp,
    #[serde(rename = "aws")]
    Aws,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPatchPrivateEndpointCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPatchPrivateEndpoint.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPatchPrivateEndpointRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "ca-central-1")]
    Ca_central_1,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west2")]
    Europe_west2,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPatchPrivateEndpointRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Ca_central_1 => write!(f, "ca-central-1"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west2 => write!(f, "europe-west2"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPrivateEndpoint.cloudProvider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPrivateEndpointCloudprovider {
    #[serde(rename = "gcp")]
    #[default]
    Gcp,
    #[serde(rename = "aws")]
    Aws,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPrivateEndpointCloudprovider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gcp => write!(f, "gcp"),
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationPrivateEndpoint.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationPrivateEndpointRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "ca-central-1")]
    Ca_central_1,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west2")]
    Europe_west2,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationPrivateEndpointRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Ca_central_1 => write!(f, "ca-central-1"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west2 => write!(f, "europe-west2"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationQuota.quotaCode`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationQuotaQuotacode {
    #[serde(rename = "services-per-organization")]
    #[default]
    Services_per_organization,
    #[serde(rename = "postgres-services-per-organization")]
    Postgres_services_per_organization,
    #[serde(rename = "replicas-per-warehouse")]
    Replicas_per_warehouse,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationQuotaQuotacode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Services_per_organization => write!(f, "services-per-organization"),
            Self::Postgres_services_per_organization => {
                write!(f, "postgres-services-per-organization")
            }
            Self::Replicas_per_warehouse => write!(f, "replicas-per-warehouse"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `OrganizationQuota.scope`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum OrganizationQuotaScope {
    #[serde(rename = "organization")]
    #[default]
    Organization,
    #[serde(rename = "warehouse")]
    Warehouse,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for OrganizationQuotaScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Organization => write!(f, "organization"),
            Self::Warehouse => write!(f, "warehouse"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
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

/// Inline enum for `RBACPolicy.allowDeny`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyAllowdeny {
    #[default]
    ALLOW,
    DENY,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACPolicyAllowdeny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ALLOW => write!(f, "ALLOW"),
            Self::DENY => write!(f, "DENY"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `RBACPolicyCreateRequest.allowDeny`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyCreateRequestAllowdeny {
    #[default]
    ALLOW,
    DENY,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACPolicyCreateRequestAllowdeny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ALLOW => write!(f, "ALLOW"),
            Self::DENY => write!(f, "DENY"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `RBACPolicyTags.roleV2`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACPolicyTagsRolev2 {
    #[serde(rename = "sql-console-readonly")]
    #[default]
    Sql_console_readonly,
    #[serde(rename = "sql-console-admin")]
    Sql_console_admin,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACPolicyTagsRolev2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql_console_readonly => write!(f, "sql-console-readonly"),
            Self::Sql_console_admin => write!(f, "sql-console-admin"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `RBACRole.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum RBACRoleType {
    #[serde(rename = "system")]
    #[default]
    System,
    #[serde(rename = "custom")]
    Custom,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for RBACRoleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::Custom => write!(f, "custom"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.mskAuthentication`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointMskauthentication {
    #[default]
    SASL_IAM,
    SASL_SCRAM,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ReversePrivateEndpointMskauthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SASL_IAM => write!(f, "SASL_IAM"),
            Self::SASL_SCRAM => write!(f, "SASL_SCRAM"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.status`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointStatus {
    #[default]
    Unknown,
    Provisioning,
    Deleting,
    Ready,
    Failed,
    PendingAcceptance,
    Rejected,
    Expired,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Other(String),
}

impl std::fmt::Display for ReversePrivateEndpointStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Provisioning => write!(f, "Provisioning"),
            Self::Deleting => write!(f, "Deleting"),
            Self::Ready => write!(f, "Ready"),
            Self::Failed => write!(f, "Failed"),
            Self::PendingAcceptance => write!(f, "PendingAcceptance"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Expired => write!(f, "Expired"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ReversePrivateEndpoint.type`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ReversePrivateEndpointType {
    #[default]
    VPC_ENDPOINT_SERVICE,
    VPC_RESOURCE,
    MSK_MULTI_VPC,
    GCP_PSC_SERVICE_ATTACHMENT,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ReversePrivateEndpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VPC_ENDPOINT_SERVICE => write!(f, "VPC_ENDPOINT_SERVICE"),
            Self::VPC_RESOURCE => write!(f, "VPC_RESOURCE"),
            Self::MSK_MULTI_VPC => write!(f, "MSK_MULTI_VPC"),
            Self::GCP_PSC_SERVICE_ATTACHMENT => write!(f, "GCP_PSC_SERVICE_ATTACHMENT"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ScimPatchOperation.op`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ScimPatchOperationOp {
    #[serde(rename = "add")]
    #[default]
    Add,
    #[serde(rename = "replace")]
    Replace,
    #[serde(rename = "remove")]
    Remove,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ScimPatchOperationOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add => write!(f, "add"),
            Self::Replace => write!(f, "replace"),
            Self::Remove => write!(f, "remove"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceCompliancetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hipaa => write!(f, "hipaa"),
            Self::Pci => write!(f, "pci"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.profile`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceProfile {
    #[serde(rename = "v1-default")]
    #[default]
    V1_default,
    #[serde(rename = "v1-highmem-xs")]
    V1_highmem_xs,
    #[serde(rename = "v1-highmem-s")]
    V1_highmem_s,
    #[serde(rename = "v1-highmem-m")]
    V1_highmem_m,
    #[serde(rename = "v1-highmem-l")]
    V1_highmem_l,
    #[serde(rename = "v1-highmem-xl")]
    V1_highmem_xl,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1_default => write!(f, "v1-default"),
            Self::V1_highmem_xs => write!(f, "v1-highmem-xs"),
            Self::V1_highmem_s => write!(f, "v1-highmem-s"),
            Self::V1_highmem_m => write!(f, "v1-highmem-m"),
            Self::V1_highmem_l => write!(f, "v1-highmem-l"),
            Self::V1_highmem_xl => write!(f, "v1-highmem-xl"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.provider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    #[serde(rename = "gcp")]
    Gcp,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Gcp => write!(f, "gcp"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "ca-central-1")]
    Ca_central_1,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west2")]
    Europe_west2,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Ca_central_1 => write!(f, "ca-central-1"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west2 => write!(f, "europe-west2"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceState {
    #[serde(rename = "starting")]
    #[default]
    Starting,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "terminating")]
    Terminating,
    #[serde(rename = "softdeleting")]
    Softdeleting,
    #[serde(rename = "awaking")]
    Awaking,
    #[serde(rename = "partially_running")]
    Partially_running,
    #[serde(rename = "provisioning")]
    Provisioning,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "terminated")]
    Terminated,
    #[serde(rename = "softdeleted")]
    Softdeleted,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "idle")]
    Idle,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Stopping => write!(f, "stopping"),
            Self::Terminating => write!(f, "terminating"),
            Self::Softdeleting => write!(f, "softdeleting"),
            Self::Awaking => write!(f, "awaking"),
            Self::Partially_running => write!(f, "partially_running"),
            Self::Provisioning => write!(f, "provisioning"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Terminated => write!(f, "terminated"),
            Self::Softdeleted => write!(f, "softdeleted"),
            Self::Degraded => write!(f, "degraded"),
            Self::Failed => write!(f, "failed"),
            Self::Idle => write!(f, "idle"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `Service.tier`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceTier {
    #[serde(rename = "development")]
    #[default]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "dedicated_high_mem")]
    Dedicated_high_mem,
    #[serde(rename = "dedicated_high_cpu")]
    Dedicated_high_cpu,
    #[serde(rename = "dedicated_standard")]
    Dedicated_standard,
    #[serde(rename = "dedicated_standard_n2d_standard_4")]
    Dedicated_standard_n2d_standard_4,
    #[serde(rename = "dedicated_standard_n2d_standard_8")]
    Dedicated_standard_n2d_standard_8,
    #[serde(rename = "dedicated_standard_n2d_standard_32")]
    Dedicated_standard_n2d_standard_32,
    #[serde(rename = "dedicated_standard_n2d_standard_128")]
    Dedicated_standard_n2d_standard_128,
    #[serde(rename = "dedicated_standard_n2d_standard_32_16SSD")]
    Dedicated_standard_n2d_standard_32_16SSD,
    #[serde(rename = "dedicated_standard_n2d_standard_64_24SSD")]
    Dedicated_standard_n2d_standard_64_24SSD,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Dedicated_high_mem => write!(f, "dedicated_high_mem"),
            Self::Dedicated_high_cpu => write!(f, "dedicated_high_cpu"),
            Self::Dedicated_standard => write!(f, "dedicated_standard"),
            Self::Dedicated_standard_n2d_standard_4 => {
                write!(f, "dedicated_standard_n2d_standard_4")
            }
            Self::Dedicated_standard_n2d_standard_8 => {
                write!(f, "dedicated_standard_n2d_standard_8")
            }
            Self::Dedicated_standard_n2d_standard_32 => {
                write!(f, "dedicated_standard_n2d_standard_32")
            }
            Self::Dedicated_standard_n2d_standard_128 => {
                write!(f, "dedicated_standard_n2d_standard_128")
            }
            Self::Dedicated_standard_n2d_standard_32_16SSD => {
                write!(f, "dedicated_standard_n2d_standard_32_16SSD")
            }
            Self::Dedicated_standard_n2d_standard_64_24SSD => {
                write!(f, "dedicated_standard_n2d_standard_64_24SSD")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceEndpoint.protocol`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceEndpointProtocol {
    #[serde(rename = "https")]
    #[default]
    Https,
    #[serde(rename = "nativesecure")]
    Nativesecure,
    #[serde(rename = "mysql")]
    Mysql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceEndpointProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Https => write!(f, "https"),
            Self::Nativesecure => write!(f, "nativesecure"),
            Self::Mysql => write!(f, "mysql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceEndpointChange.protocol`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceEndpointChangeProtocol {
    #[serde(rename = "mysql")]
    #[default]
    Mysql,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceEndpointChangeProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mysql => write!(f, "mysql"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServicePatchRequest.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePatchRequestReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePatchRequestReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePatchRequestReleasechannel {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["slow", "default", "fast"];
}

/// Inline enum for `ServicePostRequest.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestCompliancetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hipaa => write!(f, "hipaa"),
            Self::Pci => write!(f, "pci"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePostRequestCompliancetype {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["hipaa", "pci"];
}

/// Inline enum for `ServicePostRequest.profile`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestProfile {
    #[serde(rename = "v1-default")]
    #[default]
    V1_default,
    #[serde(rename = "v1-highmem-xs")]
    V1_highmem_xs,
    #[serde(rename = "v1-highmem-s")]
    V1_highmem_s,
    #[serde(rename = "v1-highmem-m")]
    V1_highmem_m,
    #[serde(rename = "v1-highmem-l")]
    V1_highmem_l,
    #[serde(rename = "v1-highmem-xl")]
    V1_highmem_xl,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1_default => write!(f, "v1-default"),
            Self::V1_highmem_xs => write!(f, "v1-highmem-xs"),
            Self::V1_highmem_s => write!(f, "v1-highmem-s"),
            Self::V1_highmem_m => write!(f, "v1-highmem-m"),
            Self::V1_highmem_l => write!(f, "v1-highmem-l"),
            Self::V1_highmem_xl => write!(f, "v1-highmem-xl"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePostRequestProfile {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &[
        "v1-default",
        "v1-highmem-xs",
        "v1-highmem-s",
        "v1-highmem-m",
        "v1-highmem-l",
        "v1-highmem-xl",
    ];
}

/// Inline enum for `ServicePostRequest.provider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    #[serde(rename = "gcp")]
    Gcp,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Gcp => write!(f, "gcp"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePostRequestProvider {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["aws", "gcp", "azure"];
}

/// Inline enum for `ServicePostRequest.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "ca-central-1")]
    Ca_central_1,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west2")]
    Europe_west2,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Ca_central_1 => write!(f, "ca-central-1"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west2 => write!(f, "europe-west2"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePostRequestRegion {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &[
        "ap-northeast-1",
        "ap-northeast-2",
        "ap-south-1",
        "ap-southeast-1",
        "ap-southeast-2",
        "ca-central-1",
        "eu-central-1",
        "eu-west-1",
        "eu-west-2",
        "il-central-1",
        "us-east-1",
        "us-east-2",
        "us-west-2",
        "us-east1",
        "us-central1",
        "europe-west2",
        "europe-west4",
        "asia-southeast1",
        "asia-northeast1",
        "eastus",
        "eastus2",
        "westus3",
        "germanywestcentral",
        "centralus",
    ];
}

/// Inline enum for `ServicePostRequest.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

impl ServicePostRequestReleasechannel {
    /// Wire values accepted by the API, excluding the catch-all.
    pub const VALUES: &'static [&'static str] = &["slow", "default", "fast"];
}

/// Inline enum for `ServicePostRequest.tier`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServicePostRequestTier {
    #[serde(rename = "development")]
    #[default]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "dedicated_high_mem")]
    Dedicated_high_mem,
    #[serde(rename = "dedicated_high_cpu")]
    Dedicated_high_cpu,
    #[serde(rename = "dedicated_standard")]
    Dedicated_standard,
    #[serde(rename = "dedicated_standard_n2d_standard_4")]
    Dedicated_standard_n2d_standard_4,
    #[serde(rename = "dedicated_standard_n2d_standard_8")]
    Dedicated_standard_n2d_standard_8,
    #[serde(rename = "dedicated_standard_n2d_standard_32")]
    Dedicated_standard_n2d_standard_32,
    #[serde(rename = "dedicated_standard_n2d_standard_128")]
    Dedicated_standard_n2d_standard_128,
    #[serde(rename = "dedicated_standard_n2d_standard_32_16SSD")]
    Dedicated_standard_n2d_standard_32_16SSD,
    #[serde(rename = "dedicated_standard_n2d_standard_64_24SSD")]
    Dedicated_standard_n2d_standard_64_24SSD,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServicePostRequestTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Dedicated_high_mem => write!(f, "dedicated_high_mem"),
            Self::Dedicated_high_cpu => write!(f, "dedicated_high_cpu"),
            Self::Dedicated_standard => write!(f, "dedicated_standard"),
            Self::Dedicated_standard_n2d_standard_4 => {
                write!(f, "dedicated_standard_n2d_standard_4")
            }
            Self::Dedicated_standard_n2d_standard_8 => {
                write!(f, "dedicated_standard_n2d_standard_8")
            }
            Self::Dedicated_standard_n2d_standard_32 => {
                write!(f, "dedicated_standard_n2d_standard_32")
            }
            Self::Dedicated_standard_n2d_standard_128 => {
                write!(f, "dedicated_standard_n2d_standard_128")
            }
            Self::Dedicated_standard_n2d_standard_32_16SSD => {
                write!(f, "dedicated_standard_n2d_standard_32_16SSD")
            }
            Self::Dedicated_standard_n2d_standard_64_24SSD => {
                write!(f, "dedicated_standard_n2d_standard_64_24SSD")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.complianceType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseCompliancetype {
    #[serde(rename = "hipaa")]
    #[default]
    Hipaa,
    #[serde(rename = "pci")]
    Pci,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseCompliancetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hipaa => write!(f, "hipaa"),
            Self::Pci => write!(f, "pci"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.profile`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseProfile {
    #[serde(rename = "v1-default")]
    #[default]
    V1_default,
    #[serde(rename = "v1-highmem-xs")]
    V1_highmem_xs,
    #[serde(rename = "v1-highmem-s")]
    V1_highmem_s,
    #[serde(rename = "v1-highmem-m")]
    V1_highmem_m,
    #[serde(rename = "v1-highmem-l")]
    V1_highmem_l,
    #[serde(rename = "v1-highmem-xl")]
    V1_highmem_xl,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1_default => write!(f, "v1-default"),
            Self::V1_highmem_xs => write!(f, "v1-highmem-xs"),
            Self::V1_highmem_s => write!(f, "v1-highmem-s"),
            Self::V1_highmem_m => write!(f, "v1-highmem-m"),
            Self::V1_highmem_l => write!(f, "v1-highmem-l"),
            Self::V1_highmem_xl => write!(f, "v1-highmem-xl"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.provider`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseProvider {
    #[serde(rename = "aws")]
    #[default]
    Aws,
    #[serde(rename = "gcp")]
    Gcp,
    #[serde(rename = "azure")]
    Azure,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Gcp => write!(f, "gcp"),
            Self::Azure => write!(f, "azure"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.region`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseRegion {
    #[serde(rename = "ap-northeast-1")]
    #[default]
    Ap_northeast_1,
    #[serde(rename = "ap-northeast-2")]
    Ap_northeast_2,
    #[serde(rename = "ap-south-1")]
    Ap_south_1,
    #[serde(rename = "ap-southeast-1")]
    Ap_southeast_1,
    #[serde(rename = "ap-southeast-2")]
    Ap_southeast_2,
    #[serde(rename = "ca-central-1")]
    Ca_central_1,
    #[serde(rename = "eu-central-1")]
    Eu_central_1,
    #[serde(rename = "eu-west-1")]
    Eu_west_1,
    #[serde(rename = "eu-west-2")]
    Eu_west_2,
    #[serde(rename = "il-central-1")]
    Il_central_1,
    #[serde(rename = "us-east-1")]
    Us_east_1,
    #[serde(rename = "us-east-2")]
    Us_east_2,
    #[serde(rename = "us-west-2")]
    Us_west_2,
    #[serde(rename = "us-east1")]
    Us_east1,
    #[serde(rename = "us-central1")]
    Us_central1,
    #[serde(rename = "europe-west2")]
    Europe_west2,
    #[serde(rename = "europe-west4")]
    Europe_west4,
    #[serde(rename = "asia-southeast1")]
    Asia_southeast1,
    #[serde(rename = "asia-northeast1")]
    Asia_northeast1,
    #[serde(rename = "eastus")]
    Eastus,
    #[serde(rename = "eastus2")]
    Eastus2,
    #[serde(rename = "westus3")]
    Westus3,
    #[serde(rename = "germanywestcentral")]
    Germanywestcentral,
    #[serde(rename = "centralus")]
    Centralus,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ap_northeast_1 => write!(f, "ap-northeast-1"),
            Self::Ap_northeast_2 => write!(f, "ap-northeast-2"),
            Self::Ap_south_1 => write!(f, "ap-south-1"),
            Self::Ap_southeast_1 => write!(f, "ap-southeast-1"),
            Self::Ap_southeast_2 => write!(f, "ap-southeast-2"),
            Self::Ca_central_1 => write!(f, "ca-central-1"),
            Self::Eu_central_1 => write!(f, "eu-central-1"),
            Self::Eu_west_1 => write!(f, "eu-west-1"),
            Self::Eu_west_2 => write!(f, "eu-west-2"),
            Self::Il_central_1 => write!(f, "il-central-1"),
            Self::Us_east_1 => write!(f, "us-east-1"),
            Self::Us_east_2 => write!(f, "us-east-2"),
            Self::Us_west_2 => write!(f, "us-west-2"),
            Self::Us_east1 => write!(f, "us-east1"),
            Self::Us_central1 => write!(f, "us-central1"),
            Self::Europe_west2 => write!(f, "europe-west2"),
            Self::Europe_west4 => write!(f, "europe-west4"),
            Self::Asia_southeast1 => write!(f, "asia-southeast1"),
            Self::Asia_northeast1 => write!(f, "asia-northeast1"),
            Self::Eastus => write!(f, "eastus"),
            Self::Eastus2 => write!(f, "eastus2"),
            Self::Westus3 => write!(f, "westus3"),
            Self::Germanywestcentral => write!(f, "germanywestcentral"),
            Self::Centralus => write!(f, "centralus"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.releaseChannel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseReleasechannel {
    #[serde(rename = "slow")]
    #[default]
    Slow,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "fast")]
    Fast,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseReleasechannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slow => write!(f, "slow"),
            Self::Default => write!(f, "default"),
            Self::Fast => write!(f, "fast"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.state`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseState {
    #[serde(rename = "starting")]
    #[default]
    Starting,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "terminating")]
    Terminating,
    #[serde(rename = "softdeleting")]
    Softdeleting,
    #[serde(rename = "awaking")]
    Awaking,
    #[serde(rename = "partially_running")]
    Partially_running,
    #[serde(rename = "provisioning")]
    Provisioning,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "terminated")]
    Terminated,
    #[serde(rename = "softdeleted")]
    Softdeleted,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "idle")]
    Idle,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Stopping => write!(f, "stopping"),
            Self::Terminating => write!(f, "terminating"),
            Self::Softdeleting => write!(f, "softdeleting"),
            Self::Awaking => write!(f, "awaking"),
            Self::Partially_running => write!(f, "partially_running"),
            Self::Provisioning => write!(f, "provisioning"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Terminated => write!(f, "terminated"),
            Self::Softdeleted => write!(f, "softdeleted"),
            Self::Degraded => write!(f, "degraded"),
            Self::Failed => write!(f, "failed"),
            Self::Idle => write!(f, "idle"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceScalingPatchResponse.tier`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceScalingPatchResponseTier {
    #[serde(rename = "development")]
    #[default]
    Development,
    #[serde(rename = "production")]
    Production,
    #[serde(rename = "dedicated_high_mem")]
    Dedicated_high_mem,
    #[serde(rename = "dedicated_high_cpu")]
    Dedicated_high_cpu,
    #[serde(rename = "dedicated_standard")]
    Dedicated_standard,
    #[serde(rename = "dedicated_standard_n2d_standard_4")]
    Dedicated_standard_n2d_standard_4,
    #[serde(rename = "dedicated_standard_n2d_standard_8")]
    Dedicated_standard_n2d_standard_8,
    #[serde(rename = "dedicated_standard_n2d_standard_32")]
    Dedicated_standard_n2d_standard_32,
    #[serde(rename = "dedicated_standard_n2d_standard_128")]
    Dedicated_standard_n2d_standard_128,
    #[serde(rename = "dedicated_standard_n2d_standard_32_16SSD")]
    Dedicated_standard_n2d_standard_32_16SSD,
    #[serde(rename = "dedicated_standard_n2d_standard_64_24SSD")]
    Dedicated_standard_n2d_standard_64_24SSD,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceScalingPatchResponseTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Dedicated_high_mem => write!(f, "dedicated_high_mem"),
            Self::Dedicated_high_cpu => write!(f, "dedicated_high_cpu"),
            Self::Dedicated_standard => write!(f, "dedicated_standard"),
            Self::Dedicated_standard_n2d_standard_4 => {
                write!(f, "dedicated_standard_n2d_standard_4")
            }
            Self::Dedicated_standard_n2d_standard_8 => {
                write!(f, "dedicated_standard_n2d_standard_8")
            }
            Self::Dedicated_standard_n2d_standard_32 => {
                write!(f, "dedicated_standard_n2d_standard_32")
            }
            Self::Dedicated_standard_n2d_standard_128 => {
                write!(f, "dedicated_standard_n2d_standard_128")
            }
            Self::Dedicated_standard_n2d_standard_32_16SSD => {
                write!(f, "dedicated_standard_n2d_standard_32_16SSD")
            }
            Self::Dedicated_standard_n2d_standard_64_24SSD => {
                write!(f, "dedicated_standard_n2d_standard_64_24SSD")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `ServiceStatePatchRequest.command`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ServiceStatePatchRequestCommand {
    #[serde(rename = "start")]
    #[default]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "awake")]
    Awake,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for ServiceStatePatchRequestCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::Stop => write!(f, "stop"),
            Self::Awake => write!(f, "awake"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Inline enum for `UsageCostRecord.entityType`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum UsageCostRecordEntitytype {
    #[serde(rename = "datawarehouse")]
    #[default]
    Datawarehouse,
    #[serde(rename = "service")]
    Service,
    #[serde(rename = "clickpipe")]
    Clickpipe,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for UsageCostRecordEntitytype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Datawarehouse => write!(f, "datawarehouse"),
            Self::Service => write!(f, "service"),
            Self::Clickpipe => write!(f, "clickpipe"),
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

/// `BackupBucket` - one of multiple variants.
///
/// Dispatched on the `bucketProvider` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucket {
    AwsBackupBucket(AwsBackupBucket),
    GcpBackupBucket(GcpBackupBucket),
    AzureBackupBucket(AzureBackupBucket),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    BackupBucket, "bucketProvider" {
        "AWS" => AwsBackupBucket,
        "GCP" => GcpBackupBucket,
        "AZURE" => AzureBackupBucket,
    }
}

impl std::fmt::Display for BackupBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucket(_) => write!(f, "AwsBackupBucket"),
            Self::GcpBackupBucket(_) => write!(f, "GcpBackupBucket"),
            Self::AzureBackupBucket(_) => write!(f, "AzureBackupBucket"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucketPatchRequest` - one of multiple variants.
///
/// Dispatched on the `bucketProvider` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucketPatchRequest {
    AwsBackupBucketPatchRequestV1(AwsBackupBucketPatchRequestV1),
    GcpBackupBucketPatchRequestV1(GcpBackupBucketPatchRequestV1),
    AzureBackupBucketPatchRequestV1(AzureBackupBucketPatchRequestV1),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    BackupBucketPatchRequest, "bucketProvider" {
        "AWS" => AwsBackupBucketPatchRequestV1,
        "GCP" => GcpBackupBucketPatchRequestV1,
        "AZURE" => AzureBackupBucketPatchRequestV1,
    }
}

impl std::fmt::Display for BackupBucketPatchRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucketPatchRequestV1(_) => write!(f, "AwsBackupBucketPatchRequestV1"),
            Self::GcpBackupBucketPatchRequestV1(_) => write!(f, "GcpBackupBucketPatchRequestV1"),
            Self::AzureBackupBucketPatchRequestV1(_) => {
                write!(f, "AzureBackupBucketPatchRequestV1")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucketPostRequest` - one of multiple variants.
///
/// Dispatched on the `bucketProvider` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucketPostRequest {
    AwsBackupBucketPostRequestV1(AwsBackupBucketPostRequestV1),
    GcpBackupBucketPostRequestV1(GcpBackupBucketPostRequestV1),
    AzureBackupBucketPostRequestV1(AzureBackupBucketPostRequestV1),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    BackupBucketPostRequest, "bucketProvider" {
        "AWS" => AwsBackupBucketPostRequestV1,
        "GCP" => GcpBackupBucketPostRequestV1,
        "AZURE" => AzureBackupBucketPostRequestV1,
    }
}

impl std::fmt::Display for BackupBucketPostRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucketPostRequestV1(_) => write!(f, "AwsBackupBucketPostRequestV1"),
            Self::GcpBackupBucketPostRequestV1(_) => write!(f, "GcpBackupBucketPostRequestV1"),
            Self::AzureBackupBucketPostRequestV1(_) => write!(f, "AzureBackupBucketPostRequestV1"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `BackupBucketProperties` - one of multiple variants.
///
/// Dispatched on the `bucketProvider` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BackupBucketProperties {
    AwsBackupBucketProperties(AwsBackupBucketProperties),
    GcpBackupBucketProperties(GcpBackupBucketProperties),
    AzureBackupBucketProperties(AzureBackupBucketProperties),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    BackupBucketProperties, "bucketProvider" {
        "AWS" => AwsBackupBucketProperties,
        "GCP" => GcpBackupBucketProperties,
        "AZURE" => AzureBackupBucketProperties,
    }
}

impl std::fmt::Display for BackupBucketProperties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsBackupBucketProperties(_) => write!(f, "AwsBackupBucketProperties"),
            Self::GcpBackupBucketProperties(_) => write!(f, "GcpBackupBucketProperties"),
            Self::AzureBackupBucketProperties(_) => write!(f, "AzureBackupBucketProperties"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertChannel` - one of multiple variants.
///
/// Dispatched on the `type` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertChannel {
    ClickStackAlertChannelEmail(ClickStackAlertChannelEmail),
    ClickStackAlertChannelWebhook(ClickStackAlertChannelWebhook),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertChannel, "type" {
        "email" => ClickStackAlertChannelEmail,
        "webhook" => ClickStackAlertChannelWebhook,
    }
}

impl std::fmt::Display for ClickStackAlertChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertChannelEmail(_) => write!(f, "ClickStackAlertChannelEmail"),
            Self::ClickStackAlertChannelWebhook(_) => write!(f, "ClickStackAlertChannelWebhook"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackAlertChannel` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackAlertChannel`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `type` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `type` this crate does not know, or a payload that
/// does not fit the variant its `type` selects, lands in `Unknown` with the
/// raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `type` and so would not deserialize back to the same
/// variant. Build a [`ClickStackAlertChannel`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackAlertChannelResponse {
    ClickStackAlertChannelEmail(ClickStackAlertChannelEmailResponse),
    ClickStackAlertChannelWebhook(ClickStackAlertChannelWebhookResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackAlertChannelResponse, "type" {
        "email" => ClickStackAlertChannelEmail,
        "webhook" => ClickStackAlertChannelWebhook,
    }
}

impl std::fmt::Display for ClickStackAlertChannelResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackAlertChannelEmail(_) => write!(f, "ClickStackAlertChannelEmail"),
            Self::ClickStackAlertChannelWebhook(_) => write!(f, "ClickStackAlertChannelWebhook"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackBarChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackBarChartConfig {
    ClickStackBarBuilderChartConfig(ClickStackBarBuilderChartConfig),
    ClickStackBarRawSqlChartConfig(ClickStackBarRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackBarChartConfig, "configType" {
        "sql" => ClickStackBarRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackBarBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackBarChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackBarBuilderChartConfig(_) => {
                write!(f, "ClickStackBarBuilderChartConfig")
            }
            Self::ClickStackBarRawSqlChartConfig(_) => write!(f, "ClickStackBarRawSqlChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackBarChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackBarChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackBarChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackBarChartConfigResponse {
    ClickStackBarRawSqlChartConfig(ClickStackBarRawSqlChartConfigResponse),
    ClickStackBarBuilderChartConfig(ClickStackBarBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackBarChartConfigResponse, "configType" {
        "sql" => ClickStackBarRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackBarBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackBarChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackBarRawSqlChartConfig(_) => write!(f, "ClickStackBarRawSqlChartConfig"),
            Self::ClickStackBarBuilderChartConfig(_) => {
                write!(f, "ClickStackBarBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackCategoricalBarChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackCategoricalBarChartConfig {
    ClickStackCategoricalBarBuilderChartConfig(ClickStackCategoricalBarBuilderChartConfig),
    ClickStackCategoricalBarRawSqlChartConfig(ClickStackCategoricalBarRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackCategoricalBarChartConfig, "configType" {
        "sql" => ClickStackCategoricalBarRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackCategoricalBarBuilderChartConfig,
    }
}

impl Default for ClickStackCategoricalBarChartConfig {
    fn default() -> Self {
        Self::ClickStackCategoricalBarBuilderChartConfig(
            ClickStackCategoricalBarBuilderChartConfig::default(),
        )
    }
}

impl std::fmt::Display for ClickStackCategoricalBarChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackCategoricalBarBuilderChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarBuilderChartConfig")
            }
            Self::ClickStackCategoricalBarRawSqlChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackCategoricalBarChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackCategoricalBarChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackCategoricalBarChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackCategoricalBarChartConfigResponse {
    ClickStackCategoricalBarRawSqlChartConfig(ClickStackCategoricalBarRawSqlChartConfigResponse),
    ClickStackCategoricalBarBuilderChartConfig(ClickStackCategoricalBarBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackCategoricalBarChartConfigResponse, "configType" {
        "sql" => ClickStackCategoricalBarRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackCategoricalBarBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackCategoricalBarChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackCategoricalBarRawSqlChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarRawSqlChartConfig")
            }
            Self::ClickStackCategoricalBarBuilderChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackDashboardChartSeries` - one of multiple variants.
///
/// Dispatched on the `type` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackDashboardChartSeries {
    ClickStackTimeChartSeries(ClickStackTimeChartSeries),
    ClickStackTableChartSeries(ClickStackTableChartSeries),
    ClickStackNumberChartSeries(ClickStackNumberChartSeries),
    ClickStackSearchChartSeries(ClickStackSearchChartSeries),
    ClickStackMarkdownChartSeries(ClickStackMarkdownChartSeries),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackDashboardChartSeries, "type" {
        "time" => ClickStackTimeChartSeries,
        "table" => ClickStackTableChartSeries,
        "number" => ClickStackNumberChartSeries,
        "search" => ClickStackSearchChartSeries,
        "markdown" => ClickStackMarkdownChartSeries,
    }
}

impl std::fmt::Display for ClickStackDashboardChartSeries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackTimeChartSeries(_) => write!(f, "ClickStackTimeChartSeries"),
            Self::ClickStackTableChartSeries(_) => write!(f, "ClickStackTableChartSeries"),
            Self::ClickStackNumberChartSeries(_) => write!(f, "ClickStackNumberChartSeries"),
            Self::ClickStackSearchChartSeries(_) => write!(f, "ClickStackSearchChartSeries"),
            Self::ClickStackMarkdownChartSeries(_) => write!(f, "ClickStackMarkdownChartSeries"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackLineChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackLineChartConfig {
    ClickStackLineBuilderChartConfig(ClickStackLineBuilderChartConfig),
    ClickStackLineRawSqlChartConfig(ClickStackLineRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackLineChartConfig, "configType" {
        "sql" => ClickStackLineRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackLineBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackLineChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLineBuilderChartConfig(_) => {
                write!(f, "ClickStackLineBuilderChartConfig")
            }
            Self::ClickStackLineRawSqlChartConfig(_) => {
                write!(f, "ClickStackLineRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackLineChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackLineChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackLineChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackLineChartConfigResponse {
    ClickStackLineRawSqlChartConfig(ClickStackLineRawSqlChartConfigResponse),
    ClickStackLineBuilderChartConfig(ClickStackLineBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackLineChartConfigResponse, "configType" {
        "sql" => ClickStackLineRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackLineBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackLineChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLineRawSqlChartConfig(_) => {
                write!(f, "ClickStackLineRawSqlChartConfig")
            }
            Self::ClickStackLineBuilderChartConfig(_) => {
                write!(f, "ClickStackLineBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackNumberChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackNumberChartConfig {
    ClickStackNumberBuilderChartConfig(ClickStackNumberBuilderChartConfig),
    ClickStackNumberRawSqlChartConfig(ClickStackNumberRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackNumberChartConfig, "configType" {
        "sql" => ClickStackNumberRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackNumberBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackNumberChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackNumberBuilderChartConfig(_) => {
                write!(f, "ClickStackNumberBuilderChartConfig")
            }
            Self::ClickStackNumberRawSqlChartConfig(_) => {
                write!(f, "ClickStackNumberRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackNumberChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackNumberChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackNumberChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackNumberChartConfigResponse {
    ClickStackNumberRawSqlChartConfig(ClickStackNumberRawSqlChartConfigResponse),
    ClickStackNumberBuilderChartConfig(ClickStackNumberBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackNumberChartConfigResponse, "configType" {
        "sql" => ClickStackNumberRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackNumberBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackNumberChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackNumberRawSqlChartConfig(_) => {
                write!(f, "ClickStackNumberRawSqlChartConfig")
            }
            Self::ClickStackNumberBuilderChartConfig(_) => {
                write!(f, "ClickStackNumberBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackNumberTileColorCondition` - one of multiple variants.
///
/// Dispatched on the `operator` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackNumberTileColorCondition {
    ClickStackNumericColorCondition(ClickStackNumericColorCondition),
    ClickStackBetweenColorCondition(ClickStackBetweenColorCondition),
    ClickStackEqualityColorCondition(ClickStackEqualityColorCondition),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackNumberTileColorCondition, "operator" {
        "gt" | "gte" | "lt" | "lte" => ClickStackNumericColorCondition,
        "between" => ClickStackBetweenColorCondition,
        "eq" | "neq" => ClickStackEqualityColorCondition,
    }
}

impl std::fmt::Display for ClickStackNumberTileColorCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackNumericColorCondition(_) => {
                write!(f, "ClickStackNumericColorCondition")
            }
            Self::ClickStackBetweenColorCondition(_) => {
                write!(f, "ClickStackBetweenColorCondition")
            }
            Self::ClickStackEqualityColorCondition(_) => {
                write!(f, "ClickStackEqualityColorCondition")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackNumberTileColorCondition` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackNumberTileColorCondition`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `operator` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `operator` this crate does not know, or a payload that
/// does not fit the variant its `operator` selects, lands in `Unknown` with the
/// raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `operator` and so would not deserialize back to the same
/// variant. Build a [`ClickStackNumberTileColorCondition`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackNumberTileColorConditionResponse {
    ClickStackNumericColorCondition(ClickStackNumericColorConditionResponse),
    ClickStackBetweenColorCondition(ClickStackBetweenColorConditionResponse),
    ClickStackEqualityColorCondition(ClickStackEqualityColorConditionResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackNumberTileColorConditionResponse, "operator" {
        "gt" | "gte" | "lt" | "lte" => ClickStackNumericColorCondition,
        "between" => ClickStackBetweenColorCondition,
        "eq" | "neq" => ClickStackEqualityColorCondition,
    }
}

impl std::fmt::Display for ClickStackNumberTileColorConditionResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackNumericColorCondition(_) => {
                write!(f, "ClickStackNumericColorCondition")
            }
            Self::ClickStackBetweenColorCondition(_) => {
                write!(f, "ClickStackBetweenColorCondition")
            }
            Self::ClickStackEqualityColorCondition(_) => {
                write!(f, "ClickStackEqualityColorCondition")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackOnClick` - one of multiple variants.
///
/// Dispatched on the `type` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackOnClick {
    ClickStackOnClickSearch(ClickStackOnClickSearch),
    ClickStackOnClickDashboard(ClickStackOnClickDashboard),
    ClickStackOnClickExternal(ClickStackOnClickExternal),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackOnClick, "type" {
        "search" => ClickStackOnClickSearch,
        "dashboard" => ClickStackOnClickDashboard,
        "external" => ClickStackOnClickExternal,
    }
}

impl Default for ClickStackOnClick {
    fn default() -> Self {
        Self::Unknown(serde_json::Value::Null)
    }
}

impl std::fmt::Display for ClickStackOnClick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackOnClickSearch(_) => write!(f, "ClickStackOnClickSearch"),
            Self::ClickStackOnClickDashboard(_) => write!(f, "ClickStackOnClickDashboard"),
            Self::ClickStackOnClickExternal(_) => write!(f, "ClickStackOnClickExternal"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackOnClick` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackOnClick`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `type` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `type` this crate does not know, or a payload that
/// does not fit the variant its `type` selects, lands in `Unknown` with the
/// raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `type` and so would not deserialize back to the same
/// variant. Build a [`ClickStackOnClick`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackOnClickResponse {
    ClickStackOnClickSearch(ClickStackOnClickSearchResponse),
    ClickStackOnClickDashboard(ClickStackOnClickDashboardResponse),
    ClickStackOnClickExternal(ClickStackOnClickExternalResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackOnClickResponse, "type" {
        "search" => ClickStackOnClickSearch,
        "dashboard" => ClickStackOnClickDashboard,
        "external" => ClickStackOnClickExternal,
    }
}

impl std::fmt::Display for ClickStackOnClickResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackOnClickSearch(_) => write!(f, "ClickStackOnClickSearch"),
            Self::ClickStackOnClickDashboard(_) => write!(f, "ClickStackOnClickDashboard"),
            Self::ClickStackOnClickExternal(_) => write!(f, "ClickStackOnClickExternal"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackOnClickTarget` - one of multiple variants.
///
/// Dispatched on the `mode` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackOnClickTarget {
    ClickStackOnClickTargetIdVariant(ClickStackOnClickTargetIdVariant),
    ClickStackOnClickTargetTemplateVariant(ClickStackOnClickTargetTemplateVariant),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackOnClickTarget, "mode" {
        "id" => ClickStackOnClickTargetIdVariant,
        "template" => ClickStackOnClickTargetTemplateVariant,
    }
}

impl Default for ClickStackOnClickTarget {
    fn default() -> Self {
        Self::Unknown(serde_json::Value::Null)
    }
}

impl std::fmt::Display for ClickStackOnClickTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackOnClickTargetIdVariant(_) => {
                write!(f, "ClickStackOnClickTargetIdVariant")
            }
            Self::ClickStackOnClickTargetTemplateVariant(_) => {
                write!(f, "ClickStackOnClickTargetTemplateVariant")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackOnClickTarget` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackOnClickTarget`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `mode` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `mode` this crate does not know, or a payload that
/// does not fit the variant its `mode` selects, lands in `Unknown` with the
/// raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `mode` and so would not deserialize back to the same
/// variant. Build a [`ClickStackOnClickTarget`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackOnClickTargetResponse {
    ClickStackOnClickTargetIdVariant(ClickStackOnClickTargetIdVariantResponse),
    ClickStackOnClickTargetTemplateVariant(ClickStackOnClickTargetTemplateVariantResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackOnClickTargetResponse, "mode" {
        "id" => ClickStackOnClickTargetIdVariant,
        "template" => ClickStackOnClickTargetTemplateVariant,
    }
}

impl std::fmt::Display for ClickStackOnClickTargetResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackOnClickTargetIdVariant(_) => {
                write!(f, "ClickStackOnClickTargetIdVariant")
            }
            Self::ClickStackOnClickTargetTemplateVariant(_) => {
                write!(f, "ClickStackOnClickTargetTemplateVariant")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackPieChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackPieChartConfig {
    ClickStackPieBuilderChartConfig(ClickStackPieBuilderChartConfig),
    ClickStackPieRawSqlChartConfig(ClickStackPieRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackPieChartConfig, "configType" {
        "sql" => ClickStackPieRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackPieBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackPieChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackPieBuilderChartConfig(_) => {
                write!(f, "ClickStackPieBuilderChartConfig")
            }
            Self::ClickStackPieRawSqlChartConfig(_) => write!(f, "ClickStackPieRawSqlChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackPieChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackPieChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackPieChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackPieChartConfigResponse {
    ClickStackPieRawSqlChartConfig(ClickStackPieRawSqlChartConfigResponse),
    ClickStackPieBuilderChartConfig(ClickStackPieBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackPieChartConfigResponse, "configType" {
        "sql" => ClickStackPieRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackPieBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackPieChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackPieRawSqlChartConfig(_) => write!(f, "ClickStackPieRawSqlChartConfig"),
            Self::ClickStackPieBuilderChartConfig(_) => {
                write!(f, "ClickStackPieBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackSource` - one of multiple variants.
///
/// Dispatched on the `kind` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackSource {
    ClickStackLogSource(ClickStackLogSource),
    ClickStackTraceSource(ClickStackTraceSource),
    ClickStackMetricSource(ClickStackMetricSource),
    ClickStackSessionSource(ClickStackSessionSource),
    ClickStackPromqlSource(ClickStackPromqlSource),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackSource, "kind" {
        "log" => ClickStackLogSource,
        "trace" => ClickStackTraceSource,
        "metric" => ClickStackMetricSource,
        "session" => ClickStackSessionSource,
        "promql" => ClickStackPromqlSource,
    }
}

impl std::fmt::Display for ClickStackSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLogSource(_) => write!(f, "ClickStackLogSource"),
            Self::ClickStackTraceSource(_) => write!(f, "ClickStackTraceSource"),
            Self::ClickStackMetricSource(_) => write!(f, "ClickStackMetricSource"),
            Self::ClickStackSessionSource(_) => write!(f, "ClickStackSessionSource"),
            Self::ClickStackPromqlSource(_) => write!(f, "ClickStackPromqlSource"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackSource` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackSource`]: each arm is the all-`Option`
/// response variant of its request struct, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `kind` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `kind` this crate does not know, or a payload that does
/// not fit the variant its `kind` selects, lands in `Unknown` with the raw JSON
/// intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `kind` and so would not deserialize back to the same
/// variant. Build a [`ClickStackSource`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackSourceResponse {
    ClickStackLogSource(ClickStackLogSourceResponse),
    ClickStackTraceSource(ClickStackTraceSourceResponse),
    ClickStackMetricSource(ClickStackMetricSourceResponse),
    ClickStackSessionSource(ClickStackSessionSourceResponse),
    ClickStackPromqlSource(ClickStackPromqlSourceResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackSourceResponse, "kind" {
        "log" => ClickStackLogSource,
        "trace" => ClickStackTraceSource,
        "metric" => ClickStackMetricSource,
        "session" => ClickStackSessionSource,
        "promql" => ClickStackPromqlSource,
    }
}

impl std::fmt::Display for ClickStackSourceResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLogSource(_) => write!(f, "ClickStackLogSource"),
            Self::ClickStackTraceSource(_) => write!(f, "ClickStackTraceSource"),
            Self::ClickStackMetricSource(_) => write!(f, "ClickStackMetricSource"),
            Self::ClickStackSessionSource(_) => write!(f, "ClickStackSessionSource"),
            Self::ClickStackPromqlSource(_) => write!(f, "ClickStackPromqlSource"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackTableChartConfig` - one of multiple variants.
///
/// Dispatched on the `configType` field (absent or non-string dispatches to the
/// builder variant, unless the payload carries a raw-SQL-only key); see the
/// `discriminated_union!` invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackTableChartConfig {
    ClickStackTableBuilderChartConfig(ClickStackTableBuilderChartConfig),
    ClickStackTableRawSqlChartConfig(ClickStackTableRawSqlChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackTableChartConfig, "configType" {
        "sql" => ClickStackTableRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackTableBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackTableChartConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackTableBuilderChartConfig(_) => {
                write!(f, "ClickStackTableBuilderChartConfig")
            }
            Self::ClickStackTableRawSqlChartConfig(_) => {
                write!(f, "ClickStackTableRawSqlChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackTableChartConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackTableChartConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `configType` field exactly as the request union is (absent
/// or non-string dispatches to the builder variant, unless the payload carries
/// a raw-SQL-only key): dispatch reads the raw JSON rather than trying each
/// variant's shape, so all-`Option` arms — which would match any object under
/// `untagged` matching — cannot misroute a payload, and the `unless` guard
/// keeps a raw-SQL payload with a dropped discriminator out of the total
/// builder arm. A payload that does not fit the variant its discriminator
/// selects lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: response values are produced by
/// deserialization, never constructed; build a [`ClickStackTableChartConfig`] instead when
/// writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackTableChartConfigResponse {
    ClickStackTableRawSqlChartConfig(ClickStackTableRawSqlChartConfigResponse),
    ClickStackTableBuilderChartConfig(ClickStackTableBuilderChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackTableChartConfigResponse, "configType" {
        "sql" => ClickStackTableRawSqlChartConfig,
        none unless "connectionId" | "sqlTemplate" => ClickStackTableBuilderChartConfig,
    }
}

impl std::fmt::Display for ClickStackTableChartConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackTableRawSqlChartConfig(_) => {
                write!(f, "ClickStackTableRawSqlChartConfig")
            }
            Self::ClickStackTableBuilderChartConfig(_) => {
                write!(f, "ClickStackTableBuilderChartConfig")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackTileConfig` - one of multiple variants.
///
/// Dispatched on the `displayType` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackTileConfig {
    ClickStackCategoricalBarChartConfig(ClickStackCategoricalBarChartConfig),
    ClickStackLineChartConfig(ClickStackLineChartConfig),
    ClickStackBarChartConfig(ClickStackBarChartConfig),
    ClickStackTableChartConfig(ClickStackTableChartConfig),
    ClickStackNumberChartConfig(ClickStackNumberChartConfig),
    ClickStackPieChartConfig(ClickStackPieChartConfig),
    ClickStackHeatmapChartConfig(ClickStackHeatmapChartConfig),
    ClickStackSearchChartConfig(ClickStackSearchChartConfig),
    ClickStackEventPatternsChartConfig(ClickStackEventPatternsChartConfig),
    ClickStackMarkdownChartConfig(ClickStackMarkdownChartConfig),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackTileConfig, "displayType" {
        "line" => ClickStackLineChartConfig,
        "stacked_bar" => ClickStackBarChartConfig,
        "bar" => ClickStackCategoricalBarChartConfig,
        "table" => ClickStackTableChartConfig,
        "number" => ClickStackNumberChartConfig,
        "pie" => ClickStackPieChartConfig,
        "heatmap" => ClickStackHeatmapChartConfig,
        "search" => ClickStackSearchChartConfig,
        "event_patterns" => ClickStackEventPatternsChartConfig,
        "markdown" => ClickStackMarkdownChartConfig,
    }
}

impl std::fmt::Display for ClickStackTileConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackCategoricalBarChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarChartConfig")
            }
            Self::ClickStackLineChartConfig(_) => write!(f, "ClickStackLineChartConfig"),
            Self::ClickStackBarChartConfig(_) => write!(f, "ClickStackBarChartConfig"),
            Self::ClickStackTableChartConfig(_) => write!(f, "ClickStackTableChartConfig"),
            Self::ClickStackNumberChartConfig(_) => write!(f, "ClickStackNumberChartConfig"),
            Self::ClickStackPieChartConfig(_) => write!(f, "ClickStackPieChartConfig"),
            Self::ClickStackHeatmapChartConfig(_) => write!(f, "ClickStackHeatmapChartConfig"),
            Self::ClickStackSearchChartConfig(_) => write!(f, "ClickStackSearchChartConfig"),
            Self::ClickStackEventPatternsChartConfig(_) => {
                write!(f, "ClickStackEventPatternsChartConfig")
            }
            Self::ClickStackMarkdownChartConfig(_) => write!(f, "ClickStackMarkdownChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackTileConfig` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackTileConfig`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `displayType` field, exactly as the request union is: dispatch
/// reads the raw JSON rather than trying each variant's shape, so all-`Option`
/// arms — which would match any object under `untagged` matching — cannot
/// misroute a payload. A `displayType` this crate does not know, or a payload that
/// does not fit the variant its `displayType` selects, lands in `Unknown` with the
/// raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `displayType` and so would not deserialize back to the same
/// variant. Build a [`ClickStackTileConfig`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackTileConfigResponse {
    ClickStackLineChartConfig(ClickStackLineChartConfigResponse),
    ClickStackBarChartConfig(ClickStackBarChartConfigResponse),
    ClickStackCategoricalBarChartConfig(ClickStackCategoricalBarChartConfigResponse),
    ClickStackTableChartConfig(ClickStackTableChartConfigResponse),
    ClickStackNumberChartConfig(ClickStackNumberChartConfigResponse),
    ClickStackPieChartConfig(ClickStackPieChartConfigResponse),
    ClickStackHeatmapChartConfig(ClickStackHeatmapChartConfigResponse),
    ClickStackSearchChartConfig(ClickStackSearchChartConfigResponse),
    ClickStackEventPatternsChartConfig(ClickStackEventPatternsChartConfigResponse),
    ClickStackMarkdownChartConfig(ClickStackMarkdownChartConfigResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackTileConfigResponse, "displayType" {
        "line" => ClickStackLineChartConfig,
        "stacked_bar" => ClickStackBarChartConfig,
        "bar" => ClickStackCategoricalBarChartConfig,
        "table" => ClickStackTableChartConfig,
        "number" => ClickStackNumberChartConfig,
        "pie" => ClickStackPieChartConfig,
        "heatmap" => ClickStackHeatmapChartConfig,
        "search" => ClickStackSearchChartConfig,
        "event_patterns" => ClickStackEventPatternsChartConfig,
        "markdown" => ClickStackMarkdownChartConfig,
    }
}

impl std::fmt::Display for ClickStackTileConfigResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackLineChartConfig(_) => write!(f, "ClickStackLineChartConfig"),
            Self::ClickStackBarChartConfig(_) => write!(f, "ClickStackBarChartConfig"),
            Self::ClickStackCategoricalBarChartConfig(_) => {
                write!(f, "ClickStackCategoricalBarChartConfig")
            }
            Self::ClickStackTableChartConfig(_) => write!(f, "ClickStackTableChartConfig"),
            Self::ClickStackNumberChartConfig(_) => write!(f, "ClickStackNumberChartConfig"),
            Self::ClickStackPieChartConfig(_) => write!(f, "ClickStackPieChartConfig"),
            Self::ClickStackHeatmapChartConfig(_) => write!(f, "ClickStackHeatmapChartConfig"),
            Self::ClickStackSearchChartConfig(_) => write!(f, "ClickStackSearchChartConfig"),
            Self::ClickStackEventPatternsChartConfig(_) => {
                write!(f, "ClickStackEventPatternsChartConfig")
            }
            Self::ClickStackMarkdownChartConfig(_) => write!(f, "ClickStackMarkdownChartConfig"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackWebhook` - one of multiple variants.
///
/// Dispatched on the `service` field; see the `discriminated_union!`
/// invocation below for the wire values.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackWebhook {
    ClickStackSlackWebhook(ClickStackSlackWebhook),
    ClickStackIncidentIOWebhook(ClickStackIncidentIOWebhook),
    ClickStackGenericWebhook(ClickStackGenericWebhook),
    ClickStackSlackAPIWebhook(ClickStackSlackAPIWebhook),
    ClickStackPagerDutyAPIWebhook(ClickStackPagerDutyAPIWebhook),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackWebhook, "service" {
        "slack" => ClickStackSlackWebhook,
        "incidentio" => ClickStackIncidentIOWebhook,
        "generic" => ClickStackGenericWebhook,
        "slack_api" => ClickStackSlackAPIWebhook,
        "pagerduty_api" => ClickStackPagerDutyAPIWebhook,
    }
}

impl std::fmt::Display for ClickStackWebhook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackSlackWebhook(_) => write!(f, "ClickStackSlackWebhook"),
            Self::ClickStackIncidentIOWebhook(_) => write!(f, "ClickStackIncidentIOWebhook"),
            Self::ClickStackGenericWebhook(_) => write!(f, "ClickStackGenericWebhook"),
            Self::ClickStackSlackAPIWebhook(_) => write!(f, "ClickStackSlackAPIWebhook"),
            Self::ClickStackPagerDutyAPIWebhook(_) => write!(f, "ClickStackPagerDutyAPIWebhook"),
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// Type alias for `ClickStackCASLPermissionConditions`.
pub type ClickStackCASLPermissionConditions = serde_json::Value;

/// Type alias for `ClickStackValidateDashboardResponseNormalized`.
pub type ClickStackValidateDashboardResponseNormalized = serde_json::Value;

/// Type alias for `ClickStackWebhookInputHeaders`.
pub type ClickStackWebhookInputHeaders = std::collections::BTreeMap<String, String>;

/// Type alias for `ClickStackWebhookInputQueryParams`.
pub type ClickStackWebhookInputQueryParams = std::collections::BTreeMap<String, String>;

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

/// `Activity` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "actorDetails", skip_serializing_if = "Option::is_none")]
    pub actor_details: Option<String>,
    #[serde(rename = "actorId", skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    #[serde(rename = "actorIpAddress", skip_serializing_if = "Option::is_none")]
    pub actor_ip_address: Option<String>,
    #[serde(rename = "actorType", skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<ActivityActortype>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "keyUpdateType", skip_serializing_if = "Option::is_none")]
    pub key_update_type: Option<ActivityKeyupdatetype>,
    #[serde(rename = "organizationId", skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(rename = "targetKeyId", skip_serializing_if = "Option::is_none")]
    pub target_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ActivityType>,
    #[serde(rename = "userAgent", skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}

/// `ApiKey` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKey {
    #[serde(rename = "assignedRoles", skip_serializing_if = "Option::is_none")]
    pub assigned_roles: Option<Vec<AssignedRole>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessListEntryResponse>>,
    #[serde(rename = "keySuffix", skip_serializing_if = "Option::is_none")]
    pub key_suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ApiKeyState>,
    #[serde(rename = "usedAt", skip_serializing_if = "Option::is_none")]
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ApiKeyHashData` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyHashData {
    #[serde(rename = "keyIdHash")]
    pub key_id_hash: String,
    #[serde(rename = "keyIdSuffix")]
    pub key_id_suffix: String,
    #[serde(rename = "keySecretHash")]
    pub key_secret_hash: String,
}

/// `ApiKeyPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPatchRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none")]
    pub assigned_role_ids: Option<Vec<uuid::Uuid>>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessListEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ApiKeyPatchRequestState>,
}

/// `ApiKeyPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPostRequest {
    #[serde(rename = "assignedRoleIds")]
    pub assigned_role_ids: Vec<uuid::Uuid>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "hashData", skip_serializing_if = "Option::is_none")]
    pub hash_data: Option<ApiKeyHashData>,
    #[serde(rename = "ipAccessList")]
    pub ip_access_list: Vec<IpAccessListEntry>,
    pub name: String,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    pub state: ApiKeyPostRequestState,
}

/// `ApiKeyPostResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyPostResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<ApiKey>,
    #[serde(rename = "keyId", skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    #[serde(rename = "keySecret", skip_serializing_if = "Option::is_none")]
    pub key_secret: Option<String>,
}

/// `AssignedRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AssignedRole {
    #[serde(rename = "roleId", skip_serializing_if = "Option::is_none")]
    pub role_id: Option<uuid::Uuid>,
    #[serde(rename = "roleName", skip_serializing_if = "Option::is_none")]
    pub role_name: Option<String>,
    #[serde(rename = "roleType", skip_serializing_if = "Option::is_none")]
    pub role_type: Option<AssignedRoleRoletype>,
}

/// `AwsBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucket {
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none")]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none")]
    pub bucket_provider: Option<AwsBackupBucketBucketprovider>,
    #[serde(rename = "iamRoleArn", skip_serializing_if = "Option::is_none")]
    pub iam_role_arn: Option<String>,
    #[serde(rename = "iamRoleSessionName", skip_serializing_if = "Option::is_none")]
    pub iam_role_session_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
}

/// `AwsBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketPatchRequestV1 {
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AwsBackupBucketPatchRequestV1Bucketprovider,
    #[serde(rename = "iamRoleArn")]
    pub iam_role_arn: String,
    #[serde(rename = "iamRoleSessionName", skip_serializing_if = "Option::is_none")]
    pub iam_role_session_name: Option<String>,
}

/// `AwsBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketPostRequestV1 {
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AwsBackupBucketPostRequestV1Bucketprovider,
    #[serde(rename = "iamRoleArn")]
    pub iam_role_arn: String,
    #[serde(rename = "iamRoleSessionName")]
    pub iam_role_session_name: String,
}

/// `AwsBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AwsBackupBucketProperties {
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AwsBackupBucketPropertiesBucketprovider,
    #[serde(rename = "iamRoleArn")]
    pub iam_role_arn: String,
    #[serde(rename = "iamRoleSessionName")]
    pub iam_role_session_name: String,
}

/// `AzureBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucket {
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none")]
    pub bucket_provider: Option<AzureBackupBucketBucketprovider>,
    #[serde(rename = "containerName", skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
}

/// `AzureBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketPatchRequestV1 {
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AzureBackupBucketPatchRequestV1Bucketprovider,
    #[serde(rename = "connectionString")]
    pub connection_string: String,
    #[serde(rename = "containerName")]
    pub container_name: String,
}

/// `AzureBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketPostRequestV1 {
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AzureBackupBucketPostRequestV1Bucketprovider,
    #[serde(rename = "connectionString")]
    pub connection_string: String,
    #[serde(rename = "containerName")]
    pub container_name: String,
}

/// `AzureBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureBackupBucketProperties {
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: AzureBackupBucketPropertiesBucketprovider,
    #[serde(rename = "containerName")]
    pub container_name: String,
}

/// `AzureEventHub` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AzureEventHub {
    #[serde(rename = "connectionString")]
    pub connection_string: String,
}

/// `Backup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Backup {
    #[serde(rename = "backupName", skip_serializing_if = "Option::is_none")]
    pub backup_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<serde_json::Value>,
    #[serde(rename = "durationInSeconds", skip_serializing_if = "Option::is_none")]
    pub duration_in_seconds: Option<f64>,
    #[serde(rename = "finishedAt", skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
    #[serde(rename = "sizeInBytes", skip_serializing_if = "Option::is_none")]
    pub size_in_bytes: Option<f64>,
    #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BackupStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<BackupType>,
}

/// `BackupConfiguration` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BackupConfiguration {
    #[serde(
        rename = "backupPeriodInHours",
        skip_serializing_if = "Option::is_none"
    )]
    pub backup_period_in_hours: Option<f64>,
    #[serde(
        rename = "backupRetentionPeriodInHours",
        skip_serializing_if = "Option::is_none"
    )]
    pub backup_retention_period_in_hours: Option<f64>,
    #[serde(rename = "backupStartTime", skip_serializing_if = "Option::is_none")]
    pub backup_start_time: Option<String>,
}

/// `BackupConfigurationPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BackupConfigurationPatchRequest {
    #[serde(
        rename = "backupPeriodInHours",
        skip_serializing_if = "Option::is_none"
    )]
    pub backup_period_in_hours: Option<f64>,
    #[serde(
        rename = "backupRetentionPeriodInHours",
        skip_serializing_if = "Option::is_none"
    )]
    pub backup_retention_period_in_hours: Option<f64>,
    #[serde(rename = "backupStartTime", skip_serializing_if = "Option::is_none")]
    pub backup_start_time: Option<String>,
}

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

/// `ByocConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocConfig {
    #[serde(rename = "accountName", skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<ByocConfigCloudprovider>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "regionId", skip_serializing_if = "Option::is_none")]
    pub region_id: Option<ByocConfigRegionid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ByocConfigState>,
}

/// `ByocInfrastructurePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructurePatchRequest {
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// `ByocInfrastructurePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ByocInfrastructurePostRequest {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "availabilityZoneSuffixes")]
    pub availability_zone_suffixes: Vec<String>,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "regionId")]
    pub region_id: ByocInfrastructurePostRequestRegionid,
    #[serde(rename = "vpcCidrRange")]
    pub vpc_cidr_range: String,
}

/// `ClickPipe` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipe {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<ClickPipeDestination>,
    #[serde(rename = "fieldMappings", skip_serializing_if = "Option::is_none")]
    pub field_mappings: Option<Vec<ClickPipeFieldMappingResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling: Option<ClickPipeScalingResponse>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeSettingsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ClickPipeSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ClickPipeState>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickPipeBigQueryPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeSettings {
    #[serde(rename = "allowNullableColumns")]
    pub allow_nullable_columns: bool,
    #[serde(rename = "initialLoadParallelism")]
    pub initial_load_parallelism: f64,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeBigQueryPipeSettingsReplicationmode,
    #[serde(rename = "snapshotNumRowsPerPartition")]
    pub snapshot_num_rows_per_partition: f64,
    #[serde(rename = "snapshotNumberOfParallelTables")]
    pub snapshot_number_of_parallel_tables: f64,
}

/// `ClickPipeBigQueryPipeSettings` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeBigQueryPipeSettings`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeSettingsResponse {
    #[serde(
        rename = "allowNullableColumns",
        skip_serializing_if = "Option::is_none"
    )]
    pub allow_nullable_columns: Option<bool>,
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<f64>,
    #[serde(rename = "replicationMode", skip_serializing_if = "Option::is_none")]
    pub replication_mode: Option<ClickPipeBigQueryPipeSettingsReplicationmode>,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<f64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<f64>,
}

/// `ClickPipeBigQueryPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeTableMapping {
    #[serde(rename = "excludedColumns")]
    pub excluded_columns: Vec<String>,
    #[serde(rename = "sortingKeys")]
    pub sorting_keys: Vec<String>,
    #[serde(rename = "sourceDatasetName")]
    pub source_dataset_name: String,
    #[serde(rename = "sourceTable")]
    pub source_table: String,
    #[serde(rename = "tableEngine")]
    pub table_engine: ClickPipeBigQueryPipeTableMappingTableengine,
    #[serde(rename = "targetTable")]
    pub target_table: String,
    #[serde(rename = "useCustomSortingKey")]
    pub use_custom_sorting_key: bool,
}

/// `ClickPipeBigQueryPipeTableMapping` from the ClickHouse Cloud API, in
/// response position.
///
/// Response variant of [`ClickPipeBigQueryPipeTableMapping`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQueryPipeTableMappingResponse {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceDatasetName", skip_serializing_if = "Option::is_none")]
    pub source_dataset_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeBigQueryPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
    #[serde(
        rename = "useCustomSortingKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipeBigQuerySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeBigQuerySource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeBigQueryPipeSettingsResponse>,
    #[serde(
        rename = "snapshotStagingPath",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_staging_path: Option<String>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none")]
    pub table_mappings: Option<Vec<ClickPipeBigQueryPipeTableMappingResponse>>,
}

/// `ClickPipeDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ClickPipeDestinationColumnResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(rename = "managedTable", skip_serializing_if = "Option::is_none")]
    pub managed_table: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(rename = "tableDefinition", skip_serializing_if = "Option::is_none")]
    pub table_definition: Option<ClickPipeDestinationTableDefinitionResponse>,
}

/// `ClickPipeDestinationColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationColumn {
    pub name: String,
    pub r#type: String,
}

/// `ClickPipeDestinationColumn` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeDestinationColumn`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationColumnResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// `ClickPipeDestinationTableDefinition` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableDefinition {
    pub engine: ClickPipeDestinationTableEngine,
    // API rejects empty strings / empty arrays for these keys. Spec has no
    // `required` array so the description-heuristic treats them as required;
    // skip at serialize time when unset instead of modeling as Option<T>.
    #[serde(rename = "partitionBy", skip_serializing_if = "String::is_empty")]
    pub partition_by: String,
    #[serde(rename = "primaryKey", skip_serializing_if = "String::is_empty")]
    pub primary_key: String,
    #[serde(rename = "sortingKey", skip_serializing_if = "Vec::is_empty")]
    pub sorting_key: Vec<String>,
}

/// `ClickPipeDestinationTableDefinition` from the ClickHouse Cloud API, in
/// response position.
///
/// Response variant of [`ClickPipeDestinationTableDefinition`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableDefinitionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<ClickPipeDestinationTableEngineResponse>,
    #[serde(rename = "partitionBy", skip_serializing_if = "Option::is_none")]
    pub partition_by: Option<String>,
    #[serde(rename = "primaryKey", skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<String>,
    #[serde(rename = "sortingKey", skip_serializing_if = "Option::is_none")]
    pub sorting_key: Option<Vec<String>>,
}

/// `ClickPipeDestinationTableEngine` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableEngine {
    // columnIds only valid for SummingMergeTree. Skip when empty to avoid API
    // rejection for MergeTree/ReplacingMergeTree/Null engines. Spec has no
    // `required` array so the heuristic treats this as required; API rejects
    // empty values despite that.
    #[serde(rename = "columnIds", skip_serializing_if = "Vec::is_empty")]
    pub column_ids: Vec<String>,
    pub r#type: ClickPipeDestinationTableEngineType,
    #[serde(rename = "versionColumnId", skip_serializing_if = "Option::is_none")]
    pub version_column_id: Option<String>,
}

/// `ClickPipeDestinationTableEngine` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeDestinationTableEngine`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeDestinationTableEngineResponse {
    #[serde(rename = "columnIds", skip_serializing_if = "Option::is_none")]
    pub column_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeDestinationTableEngineType>,
    #[serde(rename = "versionColumnId", skip_serializing_if = "Option::is_none")]
    pub version_column_id: Option<String>,
}

/// `ClickPipeFieldMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeFieldMapping {
    #[serde(rename = "destinationField")]
    pub destination_field: String,
    #[serde(rename = "sourceField")]
    pub source_field: String,
}

/// `ClickPipeFieldMapping` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickPipeFieldMapping`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeFieldMappingResponse {
    #[serde(rename = "destinationField", skip_serializing_if = "Option::is_none")]
    pub destination_field: Option<String>,
    #[serde(rename = "sourceField", skip_serializing_if = "Option::is_none")]
    pub source_field: Option<String>,
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

/// `ClickPipeKafkaSchemaRegistry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeKafkaSchemaRegistry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeKafkaSchemaRegistryAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeKafkaSourceType>,
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
    #[serde(rename = "streamName", skip_serializing_if = "Option::is_none")]
    pub stream_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(rename = "useEnhancedFanOut", skip_serializing_if = "Option::is_none")]
    pub use_enhanced_fan_out: Option<bool>,
}

/// `ClickPipeMongoDBPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeSettings {
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none")]
    pub delete_on_merge: Option<bool>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeMongoDBPipeSettingsReplicationmode,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
    #[serde(
        rename = "useJsonNativeFormat",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_json_native_format: Option<bool>,
}

/// `ClickPipeMongoDBPipeSettings` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeMongoDBPipeSettings`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeSettingsResponse {
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none")]
    pub delete_on_merge: Option<bool>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode", skip_serializing_if = "Option::is_none")]
    pub replication_mode: Option<ClickPipeMongoDBPipeSettingsReplicationmode>,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
    #[serde(
        rename = "useJsonNativeFormat",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_json_native_format: Option<bool>,
}

/// `ClickPipeMongoDBPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeTableMapping {
    #[serde(rename = "sourceCollection")]
    pub source_collection: String,
    #[serde(rename = "sourceDatabaseName")]
    pub source_database_name: String,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeMongoDBPipeTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: String,
}

/// `ClickPipeMongoDBPipeTableMapping` from the ClickHouse Cloud API, in
/// response position.
///
/// Response variant of [`ClickPipeMongoDBPipeTableMapping`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBPipeTableMappingResponse {
    #[serde(rename = "sourceCollection", skip_serializing_if = "Option::is_none")]
    pub source_collection: Option<String>,
    #[serde(rename = "sourceDatabaseName", skip_serializing_if = "Option::is_none")]
    pub source_database_name: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeMongoDBPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
}

/// `ClickPipeMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference", skip_serializing_if = "Option::is_none")]
    pub read_preference: Option<ClickPipeMongoDBSourceReadpreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeMongoDBPipeSettingsResponse>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none")]
    pub table_mappings: Option<Vec<ClickPipeMongoDBPipeTableMappingResponse>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// `ClickPipeMutateBigQuerySource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateBigQuerySource {
    pub credentials: ServiceAccount,
    pub settings: ClickPipeBigQueryPipeSettings,
    #[serde(rename = "snapshotStagingPath")]
    pub snapshot_staging_path: String,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeBigQueryPipeTableMapping>,
}

/// `ClickPipeMutateDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateDestination {
    // The spec describes `columns`, `managedTable`, `table`, and
    // `tableDefinition` as "Required field for all pipe types except database
    // pipes (Postgres, MySQL, BigQuery)" — all four must be omitted entirely
    // for database pipes. Modeled with skip-when-empty / Option so callers can
    // build a single destination type and database pipes serialize cleanly.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<ClickPipeDestinationColumn>,
    pub database: String,
    #[serde(rename = "managedTable", skip_serializing_if = "Option::is_none")]
    pub managed_table: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(rename = "tableDefinition", skip_serializing_if = "Option::is_none")]
    pub table_definition: Option<ClickPipeDestinationTableDefinition>,
}

/// `ClickPipeMutateKafkaSchemaRegistry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateKafkaSchemaRegistry {
    pub authentication: ClickPipeMutateKafkaSchemaRegistryAuthentication,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    pub credentials: ClickPipeKafkaSchemaRegistryCredentials,
    pub url: String,
}

/// `ClickPipeMutateMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference")]
    pub read_preference: ClickPipeMutateMongoDBSourceReadpreference,
    pub settings: ClickPipeMongoDBPipeSettings,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeMongoDBPipeTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    pub uri: String,
}

/// `ClickPipeMutateMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutateMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeMutateMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    pub host: String,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    pub port: i64,
    #[serde(rename = "serverId", skip_serializing_if = "Option::is_none")]
    pub server_id: Option<i64>,
    pub settings: ClickPipeMySQLPipeSettings,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipeMySQLPipeTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeMutateMySQLSourceType>,
}

/// `ClickPipeMutatePostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMutatePostgresSource {
    pub authentication: ClickPipeMutatePostgresSourceAuthentication,
    // caCertificate is `undefinedOr(isValidPEMCertificate)` server-side — sending
    // `""` (the bare-String default) fails PEM validation. Modeled as
    // `Option<String>` so callers can omit it.
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    pub credentials: PLAIN,
    pub database: String,
    #[serde(rename = "disableTls")]
    pub disable_tls: bool,
    pub host: String,
    // iamRole only applies to RDS-style Postgres + IAM_ROLE auth. Spec marks
    // it required but the server rejects "" for Basic-auth Postgres. Modeled
    // as Option<String> so callers can omit it; same pattern as ca_certificate.
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    pub port: i64,
    pub settings: ClickPipePostgresPipeSettings,
    #[serde(rename = "skipCertVerification")]
    pub skip_cert_verification: bool,
    #[serde(rename = "tableMappings")]
    pub table_mappings: Vec<ClickPipePostgresPipeTableMapping>,
    // tlsHost is only set when the broker cert SAN doesn't match `host`.
    // Optional in practice; server rejects "" with PEM-style validation.
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeMutatePostgresSourceType>,
}

/// `ClickPipeMySQLPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeSettings {
    #[serde(
        rename = "allowNullableColumns",
        skip_serializing_if = "Option::is_none"
    )]
    pub allow_nullable_columns: Option<bool>,
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none")]
    pub delete_on_merge: Option<bool>,
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(
        rename = "replicationMechanism",
        skip_serializing_if = "Option::is_none"
    )]
    pub replication_mechanism: Option<ClickPipeMySQLPipeSettingsReplicationmechanism>,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipeMySQLPipeSettingsReplicationmode,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useCompression", skip_serializing_if = "Option::is_none")]
    pub use_compression: Option<bool>,
}

/// `ClickPipeMySQLPipeSettings` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeMySQLPipeSettings`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeSettingsResponse {
    #[serde(
        rename = "allowNullableColumns",
        skip_serializing_if = "Option::is_none"
    )]
    pub allow_nullable_columns: Option<bool>,
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none")]
    pub delete_on_merge: Option<bool>,
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(
        rename = "replicationMechanism",
        skip_serializing_if = "Option::is_none"
    )]
    pub replication_mechanism: Option<ClickPipeMySQLPipeSettingsReplicationmechanism>,
    #[serde(rename = "replicationMode", skip_serializing_if = "Option::is_none")]
    pub replication_mode: Option<ClickPipeMySQLPipeSettingsReplicationmode>,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useCompression", skip_serializing_if = "Option::is_none")]
    pub use_compression: Option<bool>,
}

/// `ClickPipeMySQLPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeTableMapping {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceSchemaName")]
    pub source_schema_name: String,
    #[serde(rename = "sourceTable")]
    pub source_table: String,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeMySQLPipeTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: String,
    #[serde(
        rename = "useCustomSortingKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipeMySQLPipeTableMapping` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipeMySQLPipeTableMapping`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLPipeTableMappingResponse {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceSchemaName", skip_serializing_if = "Option::is_none")]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipeMySQLPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
    #[serde(
        rename = "useCustomSortingKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipeMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(rename = "serverId", skip_serializing_if = "Option::is_none")]
    pub server_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeMySQLPipeSettingsResponse>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none")]
    pub table_mappings: Option<Vec<ClickPipeMySQLPipeTableMappingResponse>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeMySQLSourceType>,
}

/// `ClickPipeObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeObjectStorageSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipeObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none")]
    pub azure_container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<ClickPipeObjectStorageSourceCompression>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ClickPipeObjectStorageSourceFormat>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(rename = "isContinuous", skip_serializing_if = "Option::is_none")]
    pub is_continuous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "queueUrl", skip_serializing_if = "Option::is_none")]
    pub queue_url: Option<String>,
    #[serde(rename = "skipInitialLoad", skip_serializing_if = "Option::is_none")]
    pub skip_initial_load: Option<bool>,
    #[serde(rename = "startAfter", skip_serializing_if = "Option::is_none")]
    pub start_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipeObjectStorageSourceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// `ClickPipePatchDestination` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchDestination {
    pub columns: Vec<ClickPipeDestinationColumn>,
}

/// `ClickPipePatchKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchKafkaSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePatchKafkaSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    pub credentials: serde_json::Value,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(rename = "reversePrivateEndpointIds")]
    pub reverse_private_endpoint_ids: Vec<String>,
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

/// `ClickPipePatchMongoDBPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBPipeRemoveTableMapping {
    #[serde(rename = "sourceCollection")]
    pub source_collection: Option<String>,
    #[serde(rename = "sourceDatabaseName")]
    pub source_database_name: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipePatchMongoDBPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: Option<String>,
}

/// `ClickPipePatchMongoDBPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePatchMongoDBSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMongoDBSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(rename = "readPreference", skip_serializing_if = "Option::is_none")]
    pub read_preference: Option<ClickPipePatchMongoDBSourceReadpreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipePatchMongoDBPipeSettings>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappingsToAdd", skip_serializing_if = "Option::is_none")]
    pub table_mappings_to_add: Option<Vec<ClickPipeMongoDBPipeTableMapping>>,
    #[serde(
        rename = "tableMappingsToRemove",
        skip_serializing_if = "Option::is_none"
    )]
    pub table_mappings_to_remove: Option<Vec<ClickPipePatchMongoDBPipeRemoveTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    pub uri: Option<String>,
}

/// `ClickPipePatchMySQLPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLPipeRemoveTableMapping {
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
    #[serde(rename = "sourceSchemaName")]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipePatchMySQLPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable")]
    pub target_table: Option<String>,
}

/// `ClickPipePatchMySQLPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
    #[serde(rename = "useCompression", skip_serializing_if = "Option::is_none")]
    pub use_compression: Option<bool>,
}

/// `ClickPipePatchMySQLSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchMySQLSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePatchMySQLSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<PLAIN>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    pub host: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    pub port: Option<i64>,
    #[serde(rename = "serverId", skip_serializing_if = "Option::is_none")]
    pub server_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipePatchMySQLPipeSettings>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappingsToAdd", skip_serializing_if = "Option::is_none")]
    pub table_mappings_to_add: Option<Vec<ClickPipeMySQLPipeTableMapping>>,
    #[serde(
        rename = "tableMappingsToRemove",
        skip_serializing_if = "Option::is_none"
    )]
    pub table_mappings_to_remove: Option<Vec<ClickPipePatchMySQLPipeRemoveTableMapping>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
}

/// `ClickPipePatchObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchObjectStorageSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none")]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePatchObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none")]
    pub azure_container_name: Option<String>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "serviceAccountKey", skip_serializing_if = "Option::is_none")]
    pub service_account_key: Option<String>,
    #[serde(rename = "skipInitialLoad", skip_serializing_if = "Option::is_none")]
    pub skip_initial_load: Option<bool>,
    #[serde(rename = "startAfter", skip_serializing_if = "Option::is_none")]
    pub start_after: Option<String>,
}

/// `ClickPipePatchPostgresPipeRemoveTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresPipeRemoveTableMapping {
    #[serde(rename = "partitionByExpr", skip_serializing_if = "Option::is_none")]
    pub partition_by_expr: Option<String>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
    #[serde(rename = "sourceSchemaName", skip_serializing_if = "Option::is_none")]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipePatchPostgresPipeRemoveTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
}

/// `ClickPipePatchPostgresPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresPipeSettings {
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePatchPostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPostgresSource {
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    pub credentials: PLAIN,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    pub settings: ClickPipePatchPostgresPipeSettings,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappingsToAdd")]
    pub table_mappings_to_add: Vec<ClickPipePostgresPipeTableMapping>,
    #[serde(rename = "tableMappingsToRemove")]
    pub table_mappings_to_remove: Vec<ClickPipePatchPostgresPipeRemoveTableMapping>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
}

/// `ClickPipePatchPubSubSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchPubSubSource {
    #[serde(rename = "ackDeadline", skip_serializing_if = "Option::is_none")]
    pub ack_deadline: Option<i64>,
    pub authentication: Option<ClickPipePatchPubSubSourceAuthentication>,
    #[serde(rename = "serviceAccountKey")]
    pub service_account_key: Option<ServiceAccount>,
}

/// `ClickPipePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<ClickPipePatchDestination>,
    #[serde(rename = "fieldMappings", skip_serializing_if = "Option::is_none")]
    pub field_mappings: Option<Vec<ClickPipeFieldMapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ClickPipePatchSource>,
}

/// `ClickPipePatchSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePatchSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<ClickPipePatchKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinesis: Option<ClickPipePatchKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mongodb: Option<ClickPipePatchMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mysql: Option<ClickPipePatchMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none")]
    pub object_storage: Option<ClickPipePatchObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgres: Option<ClickPipePatchPostgresSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubsub: Option<ClickPipePatchPubSubSource>,
    #[serde(rename = "validateSamples")]
    pub validate_samples: bool,
}

/// `ClickPipePostKafkaSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostKafkaSource {
    pub authentication: ClickPipePostKafkaSourceAuthentication,
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
    pub r#type: ClickPipePostKafkaSourceType,
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
    #[serde(rename = "streamName")]
    pub stream_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(rename = "useEnhancedFanOut", skip_serializing_if = "Option::is_none")]
    pub use_enhanced_fan_out: Option<bool>,
}

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
}

/// `ClickPipePostObjectStorageSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostObjectStorageSource {
    #[serde(rename = "accessKey", skip_serializing_if = "Option::is_none")]
    pub access_key: Option<MskIamUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePostObjectStorageSourceAuthentication>,
    #[serde(rename = "azureContainerName", skip_serializing_if = "Option::is_none")]
    pub azure_container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<ClickPipePostObjectStorageSourceCompression>,
    #[serde(rename = "connectionString", skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    pub format: ClickPipePostObjectStorageSourceFormat,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(rename = "isContinuous", skip_serializing_if = "Option::is_none")]
    pub is_continuous: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "queueUrl", skip_serializing_if = "Option::is_none")]
    pub queue_url: Option<String>,
    #[serde(rename = "serviceAccountKey", skip_serializing_if = "Option::is_none")]
    pub service_account_key: Option<String>,
    #[serde(rename = "skipInitialLoad", skip_serializing_if = "Option::is_none")]
    pub skip_initial_load: Option<bool>,
    #[serde(rename = "startAfter", skip_serializing_if = "Option::is_none")]
    pub start_after: Option<String>,
    pub r#type: ClickPipePostObjectStorageSourceType,
    pub url: String,
}

/// `ClickPipePostPubSubSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostPubSubSource {
    #[serde(rename = "ackDeadline", skip_serializing_if = "Option::is_none")]
    pub ack_deadline: Option<i64>,
    pub authentication: ClickPipePostPubSubSourceAuthentication,
    #[serde(rename = "enableOrdering", skip_serializing_if = "Option::is_none")]
    pub enable_ordering: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    pub format: ClickPipePostPubSubSourceFormat,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "seekTimestamp", skip_serializing_if = "Option::is_none")]
    pub seek_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "seekType")]
    pub seek_type: ClickPipePostPubSubSourceSeektype,
    #[serde(rename = "serviceAccountKey")]
    pub service_account_key: ServiceAccount,
    pub topic: String,
}

/// `ClickPipePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostRequest {
    pub destination: ClickPipeMutateDestination,
    // Empty arrays rejected by some API paths and never useful on create —
    // skip when empty. Non-Option to match the spec description heuristic.
    #[serde(rename = "fieldMappings", skip_serializing_if = "Vec::is_empty")]
    pub field_mappings: Vec<ClickPipeFieldMapping>,
    pub name: String,
    // scaling block default-serializes as {replicas: 0, ...} which the API
    // rejects ("replicas: Not between 1 and 40"). Modeled as Option so the
    // whole block is omitted when the caller doesn't set it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling: Option<ClickPipeScaling>,
    // settings default-serializes as `{}` which the API also rejects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipeSettings>,
    pub source: ClickPipePostSource,
}

/// `ClickPipePostSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bigquery: Option<ClickPipeMutateBigQuerySource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<ClickPipePostKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinesis: Option<ClickPipePostKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mongodb: Option<ClickPipeMutateMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mysql: Option<ClickPipeMutateMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none")]
    pub object_storage: Option<ClickPipePostObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgres: Option<ClickPipeMutatePostgresSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubsub: Option<ClickPipePostPubSubSource>,
    #[serde(rename = "validateSamples")]
    pub validate_samples: bool,
}

/// `ClickPipePostgresPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeSettings {
    #[serde(rename = "allowNullableColumns")]
    pub allow_nullable_columns: bool,
    #[serde(rename = "deleteOnMerge")]
    pub delete_on_merge: bool,
    #[serde(rename = "enableFailoverSlots")]
    pub enable_failover_slots: bool,
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "publicationName", skip_serializing_if = "Option::is_none")]
    pub publication_name: Option<String>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode")]
    pub replication_mode: ClickPipePostgresPipeSettingsReplicationmode,
    #[serde(
        rename = "replicationSlotName",
        skip_serializing_if = "Option::is_none"
    )]
    pub replication_slot_name: Option<String>,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePostgresPipeSettings` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`ClickPipePostgresPipeSettings`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeSettingsResponse {
    #[serde(
        rename = "allowNullableColumns",
        skip_serializing_if = "Option::is_none"
    )]
    pub allow_nullable_columns: Option<bool>,
    #[serde(rename = "deleteOnMerge", skip_serializing_if = "Option::is_none")]
    pub delete_on_merge: Option<bool>,
    #[serde(
        rename = "enableFailoverSlots",
        skip_serializing_if = "Option::is_none"
    )]
    pub enable_failover_slots: Option<bool>,
    #[serde(
        rename = "initialLoadParallelism",
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_load_parallelism: Option<i64>,
    #[serde(rename = "publicationName", skip_serializing_if = "Option::is_none")]
    pub publication_name: Option<String>,
    #[serde(rename = "pullBatchSize", skip_serializing_if = "Option::is_none")]
    pub pull_batch_size: Option<i64>,
    #[serde(rename = "replicationMode", skip_serializing_if = "Option::is_none")]
    pub replication_mode: Option<ClickPipePostgresPipeSettingsReplicationmode>,
    #[serde(
        rename = "replicationSlotName",
        skip_serializing_if = "Option::is_none"
    )]
    pub replication_slot_name: Option<String>,
    #[serde(
        rename = "snapshotNumRowsPerPartition",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_num_rows_per_partition: Option<i64>,
    #[serde(
        rename = "snapshotNumberOfParallelTables",
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_number_of_parallel_tables: Option<i64>,
    #[serde(
        rename = "syncIntervalSeconds",
        skip_serializing_if = "Option::is_none"
    )]
    pub sync_interval_seconds: Option<i64>,
}

/// `ClickPipePostgresPipeTableMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeTableMapping {
    #[serde(rename = "excludedColumns")]
    pub excluded_columns: Vec<String>,
    #[serde(rename = "partitionByExpr")]
    pub partition_by_expr: String,
    #[serde(rename = "partitionKey")]
    pub partition_key: String,
    #[serde(rename = "sortingKeys")]
    pub sorting_keys: Vec<String>,
    #[serde(rename = "sourceSchemaName")]
    pub source_schema_name: String,
    #[serde(rename = "sourceTable")]
    pub source_table: String,
    #[serde(rename = "tableEngine")]
    pub table_engine: ClickPipePostgresPipeTableMappingTableengine,
    #[serde(rename = "targetTable")]
    pub target_table: String,
    #[serde(rename = "useCustomSortingKey")]
    pub use_custom_sorting_key: bool,
}

/// `ClickPipePostgresPipeTableMapping` from the ClickHouse Cloud API, in
/// response position.
///
/// Response variant of [`ClickPipePostgresPipeTableMapping`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresPipeTableMappingResponse {
    #[serde(rename = "excludedColumns", skip_serializing_if = "Option::is_none")]
    pub excluded_columns: Option<Vec<String>>,
    #[serde(rename = "partitionByExpr", skip_serializing_if = "Option::is_none")]
    pub partition_by_expr: Option<String>,
    #[serde(rename = "partitionKey", skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<String>,
    #[serde(rename = "sortingKeys", skip_serializing_if = "Option::is_none")]
    pub sorting_keys: Option<Vec<String>>,
    #[serde(rename = "sourceSchemaName", skip_serializing_if = "Option::is_none")]
    pub source_schema_name: Option<String>,
    #[serde(rename = "sourceTable", skip_serializing_if = "Option::is_none")]
    pub source_table: Option<String>,
    #[serde(rename = "tableEngine", skip_serializing_if = "Option::is_none")]
    pub table_engine: Option<ClickPipePostgresPipeTableMappingTableengine>,
    #[serde(rename = "targetTable", skip_serializing_if = "Option::is_none")]
    pub target_table: Option<String>,
    #[serde(
        rename = "useCustomSortingKey",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_custom_sorting_key: Option<bool>,
}

/// `ClickPipePostgresSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePostgresSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePostgresSourceAuthentication>,
    #[serde(rename = "caCertificate", skip_serializing_if = "Option::is_none")]
    pub ca_certificate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(rename = "disableTls", skip_serializing_if = "Option::is_none")]
    pub disable_tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ClickPipePostgresPipeSettingsResponse>,
    #[serde(
        rename = "skipCertVerification",
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_cert_verification: Option<bool>,
    #[serde(rename = "tableMappings", skip_serializing_if = "Option::is_none")]
    pub table_mappings: Option<Vec<ClickPipePostgresPipeTableMappingResponse>>,
    #[serde(rename = "tlsHost", skip_serializing_if = "Option::is_none")]
    pub tls_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickPipePostgresSourceType>,
}

/// `ClickPipePubSubSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipePubSubSource {
    #[serde(rename = "ackDeadline", skip_serializing_if = "Option::is_none")]
    pub ack_deadline: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<ClickPipePubSubSourceAuthentication>,
    #[serde(rename = "enableOrdering", skip_serializing_if = "Option::is_none")]
    pub enable_ordering: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ClickPipePubSubSourceFormat>,
    #[serde(rename = "projectId", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(rename = "seekTimestamp", skip_serializing_if = "Option::is_none")]
    pub seek_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "seekType", skip_serializing_if = "Option::is_none")]
    pub seek_type: Option<ClickPipePubSubSourceSeektype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

/// `ClickPipeScaling` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeScaling {
    #[cfg(feature = "deprecated-fields")]
    pub concurrency: i64,
    #[serde(rename = "replicaCpuMillicores")]
    pub replica_cpu_millicores: i64,
    #[serde(rename = "replicaMemoryGb")]
    pub replica_memory_gb: f64,
    pub replicas: i64,
}

/// `ClickPipeScaling` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickPipeScaling`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeScalingResponse {
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<i64>,
    #[serde(
        rename = "replicaCpuMillicores",
        skip_serializing_if = "Option::is_none"
    )]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<i64>,
}

/// `ClickPipeScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeScalingPatchRequest {
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<i64>,
    #[serde(
        rename = "replicaCpuMillicores",
        skip_serializing_if = "Option::is_none"
    )]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<i64>,
}

/// `ClickPipeSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_download_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_insert_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_min_insert_block_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_distributed_insert_select: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_view_processing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_file_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_insert_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_polling_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_use_cluster_function: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_max_insert_wait_ms: Option<i64>,
}

/// `ClickPipeSettings` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickPipeSettings`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSettingsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_download_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_insert_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_min_insert_block_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_distributed_insert_select: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_view_processing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_file_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_insert_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_polling_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_use_cluster_function: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_max_insert_wait_ms: Option<i64>,
}

/// `ClickPipeSettingsPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSettingsPutRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_download_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_insert_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_max_threads: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_min_insert_block_size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_distributed_insert_select: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickhouse_parallel_view_processing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_concurrency: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_file_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_max_insert_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_polling_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_storage_use_cluster_function: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming_max_insert_wait_ms: Option<i64>,
}

/// `ClickPipeSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bigquery: Option<ClickPipeBigQuerySource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<ClickPipeKafkaSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinesis: Option<ClickPipeKinesisSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mongodb: Option<ClickPipeMongoDBSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mysql: Option<ClickPipeMySQLSource>,
    #[serde(rename = "objectStorage", skip_serializing_if = "Option::is_none")]
    pub object_storage: Option<ClickPipeObjectStorageSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postgres: Option<ClickPipePostgresSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubsub: Option<ClickPipePubSubSource>,
}

/// `ClickPipeStatePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipeStatePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<ClickPipeStatePatchRequestCommand>,
}

/// `ClickPipesCdcScaling` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipesCdcScaling {
    #[serde(
        rename = "replicaCpuMillicores",
        skip_serializing_if = "Option::is_none"
    )]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
}

/// `ClickPipesCdcScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickPipesCdcScalingPatchRequest {
    #[serde(
        rename = "replicaCpuMillicores",
        skip_serializing_if = "Option::is_none"
    )]
    pub replica_cpu_millicores: Option<i64>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
}

/// `ClickStackAggregatedColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAggregatedColumn {
    #[serde(rename = "aggFn")]
    pub agg_fn: String,
    #[serde(rename = "mvColumn")]
    pub mv_column: String,
    #[serde(rename = "sourceColumn", skip_serializing_if = "Option::is_none")]
    pub source_column: Option<String>,
}

/// `ClickStackAggregatedColumn` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAggregatedColumn`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAggregatedColumnResponse {
    #[serde(rename = "aggFn", skip_serializing_if = "Option::is_none")]
    pub agg_fn: Option<String>,
    #[serde(rename = "mvColumn", skip_serializing_if = "Option::is_none")]
    pub mv_column: Option<String>,
    #[serde(rename = "sourceColumn", skip_serializing_if = "Option::is_none")]
    pub source_column: Option<String>,
}

/// `ClickStackAlertChannelEmail` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelEmail {
    #[serde(rename = "emailRecipients")]
    pub email_recipients: Vec<String>,
    pub r#type: ClickStackAlertChannelEmailType,
}

/// `ClickStackAlertChannelEmail` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertChannelEmail`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelEmailResponse {
    #[serde(rename = "emailRecipients", skip_serializing_if = "Option::is_none")]
    pub email_recipients: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackAlertChannelEmailType>,
}

/// `ClickStackAlertChannelWebhook` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelWebhook {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<ClickStackAlertChannelWebhookSeverity>,
    #[serde(rename = "slackChannelId", skip_serializing_if = "Option::is_none")]
    pub slack_channel_id: Option<String>,
    pub r#type: ClickStackAlertChannelWebhookType,
    #[serde(rename = "webhookId")]
    pub webhook_id: String,
    #[serde(rename = "webhookService", skip_serializing_if = "Option::is_none")]
    pub webhook_service: Option<String>,
}

/// `ClickStackAlertChannelWebhook` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackAlertChannelWebhook`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertChannelWebhookResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<ClickStackAlertChannelWebhookSeverity>,
    #[serde(rename = "slackChannelId", skip_serializing_if = "Option::is_none")]
    pub slack_channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackAlertChannelWebhookType>,
    #[serde(rename = "webhookId", skip_serializing_if = "Option::is_none")]
    pub webhook_id: Option<String>,
    #[serde(rename = "webhookService", skip_serializing_if = "Option::is_none")]
    pub webhook_service: Option<String>,
}

/// `ClickStackAlertExecutionError` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertExecutionError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackAlertExecutionErrorType>,
}

/// `ClickStackAlertResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<ClickStackAlertChannelResponse>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none")]
    pub dashboard_id: Option<String>,
    #[serde(rename = "executionErrors", skip_serializing_if = "Option::is_none")]
    pub execution_errors: Option<Vec<ClickStackAlertExecutionError>>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<ClickStackAlertResponseInterval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(
        rename = "numConsecutiveWindows",
        skip_serializing_if = "Option::is_none"
    )]
    pub num_consecutive_windows: Option<i64>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none")]
    pub saved_search_id: Option<String>,
    #[serde(
        rename = "scheduleOffsetMinutes",
        skip_serializing_if = "Option::is_none"
    )]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none")]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silenced: Option<ClickStackAlertSilenced>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ClickStackAlertResponseSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ClickStackAlertResponseState>,
    #[serde(rename = "teamId", skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    #[serde(rename = "thresholdMax", skip_serializing_if = "Option::is_none")]
    pub threshold_max: Option<f64>,
    #[serde(rename = "thresholdType", skip_serializing_if = "Option::is_none")]
    pub threshold_type: Option<ClickStackAlertResponseThresholdtype>,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none")]
    pub tile_id: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickStackAlertSilenced` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackAlertSilenced {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickStackBackgroundChart` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBackgroundChart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    pub r#type: ClickStackBackgroundChartType,
}

/// `ClickStackBackgroundChart` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackBackgroundChart`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBackgroundChartResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackBackgroundChartType>,
}

/// `ClickStackBarBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarBuilderChartConfig {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackBarBuilderChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackBarBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackBarBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarBuilderChartConfigResponse {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackBarBuilderChartConfigDisplaytype>,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackBarRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarRawSqlChartConfig {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackBarRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackBarRawSqlChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackBarRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackBarRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBarRawSqlChartConfigResponse {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackBarRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackBarRawSqlChartConfigDisplaytype>,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackBetweenColorCondition` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBetweenColorCondition {
    pub color: ClickStackChartColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub operator: ClickStackBetweenColorConditionOperator,
    pub value: Vec<f64>,
}

/// `ClickStackBetweenColorCondition` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackBetweenColorCondition`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackBetweenColorConditionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<ClickStackBetweenColorConditionOperator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Vec<f64>>,
}

/// `ClickStackCASLPermission` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCASLPermission {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ClickStackCASLPermissionConditions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    pub subject: String,
}

/// `ClickStackCASLPermission` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackCASLPermission`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCASLPermissionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ClickStackCASLPermissionConditions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

/// `ClickStackCategoricalBarBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCategoricalBarBuilderChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackCategoricalBarBuilderChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackCategoricalBarBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackCategoricalBarBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCategoricalBarBuilderChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackCategoricalBarBuilderChartConfigDisplaytype>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackCategoricalBarRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCategoricalBarRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackCategoricalBarRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackCategoricalBarRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackCategoricalBarRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackCategoricalBarRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCategoricalBarRawSqlChartConfigResponse {
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackCategoricalBarRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackCategoricalBarRawSqlChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackConnection` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackConnection {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(
        rename = "hyperdxSettingPrefix",
        skip_serializing_if = "Option::is_none"
    )]
    pub hyperdx_setting_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "isPrometheusEndpoint",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_prometheus_endpoint: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// `ClickStackCreateAlertRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateAlertRequest {
    pub channel: ClickStackAlertChannel,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none")]
    pub dashboard_id: Option<String>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    pub interval: ClickStackCreateAlertRequestInterval,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(
        rename = "numConsecutiveWindows",
        skip_serializing_if = "Option::is_none"
    )]
    pub num_consecutive_windows: Option<i64>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none")]
    pub saved_search_id: Option<String>,
    #[serde(
        rename = "scheduleOffsetMinutes",
        skip_serializing_if = "Option::is_none"
    )]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none")]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source: ClickStackCreateAlertRequestSource,
    pub threshold: f64,
    #[serde(rename = "thresholdMax", skip_serializing_if = "Option::is_none")]
    pub threshold_max: Option<f64>,
    #[serde(rename = "thresholdType")]
    pub threshold_type: ClickStackCreateAlertRequestThresholdtype,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none")]
    pub tile_id: Option<String>,
}

/// `ClickStackCreateConnectionRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateConnectionRequest {
    pub host: String,
    #[serde(
        rename = "hyperdxSettingPrefix",
        skip_serializing_if = "Option::is_none"
    )]
    pub hyperdx_setting_prefix: Option<String>,
    #[serde(
        rename = "isPrometheusEndpoint",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_prometheus_endpoint: Option<bool>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub username: String,
}

/// `ClickStackCreateDashboardRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateDashboardRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers: Option<Vec<ClickStackDashboardContainer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackFilterInput>>,
    pub name: String,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none")]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValue>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none")]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none")]
    pub saved_query_language: Option<ClickStackCreateDashboardRequestSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub tiles: Vec<ClickStackTileInput>,
}

/// `ClickStackCreateRoleRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateRoleRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub name: String,
    pub permissions: Vec<ClickStackCASLPermission>,
}

/// `ClickStackDashboardContainer` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardContainer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bordered: Option<bool>,
    pub collapsed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsible: Option<bool>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabs: Option<Vec<ClickStackDashboardContainerTab>>,
    pub title: String,
}

/// `ClickStackDashboardContainer` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackDashboardContainer`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardContainerResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bordered: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tabs: Option<Vec<ClickStackDashboardContainerTabResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// `ClickStackDashboardContainerTab` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardContainerTab {
    pub id: String,
    pub title: String,
}

/// `ClickStackDashboardContainerTab` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackDashboardContainerTab`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardContainerTabResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// `ClickStackDashboardResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackDashboardResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers: Option<Vec<ClickStackDashboardContainerResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackFilterResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none")]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValueResponse>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none")]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none")]
    pub saved_query_language: Option<ClickStackDashboardResponseSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiles: Option<Vec<ClickStackTileOutput>>,
}

/// `ClickStackEqualityColorCondition` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackEqualityColorCondition {
    pub color: ClickStackChartColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub operator: ClickStackEqualityColorConditionOperator,
    /// A finite number or a string; the spec models this as `oneOf number|string`.
    pub value: serde_json::Value,
}

/// `ClickStackEqualityColorCondition` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackEqualityColorCondition`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackEqualityColorConditionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<ClickStackEqualityColorConditionOperator>,
    /// A finite number or a string; the spec models this as `oneOf number|string`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

/// `ClickStackEventPatternsChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackEventPatternsChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackEventPatternsChartConfigDisplaytype,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackEventPatternsChartConfigWherelanguage>,
}

/// `ClickStackEventPatternsChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackEventPatternsChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackEventPatternsChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackEventPatternsChartConfigDisplaytype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackEventPatternsChartConfigWherelanguage>,
}

/// `ClickStackFilter` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilter {
    #[serde(rename = "appliesToSourceIds", skip_serializing_if = "Option::is_none")]
    pub applies_to_source_ids: Option<Vec<String>>,
    pub expression: String,
    pub id: String,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "sourceMetricType", skip_serializing_if = "Option::is_none")]
    pub source_metric_type: Option<ClickStackFilterSourcemetrictype>,
    pub r#type: ClickStackFilterType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackFilterWherelanguage>,
}

/// `ClickStackFilter` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackFilter`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilterResponse {
    #[serde(rename = "appliesToSourceIds", skip_serializing_if = "Option::is_none")]
    pub applies_to_source_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sourceMetricType", skip_serializing_if = "Option::is_none")]
    pub source_metric_type: Option<ClickStackFilterSourcemetrictype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackFilterType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackFilterWherelanguage>,
}

/// `ClickStackFilterInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilterInput {
    #[serde(rename = "appliesToSourceIds", skip_serializing_if = "Option::is_none")]
    pub applies_to_source_ids: Option<Vec<String>>,
    pub expression: String,
    pub name: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "sourceMetricType", skip_serializing_if = "Option::is_none")]
    pub source_metric_type: Option<ClickStackFilterInputSourcemetrictype>,
    pub r#type: ClickStackFilterInputType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackFilterInputWherelanguage>,
}

/// `ClickStackFilterSettingsColumn` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilterSettingsColumn {
    pub label: String,
    pub name: String,
}

/// `ClickStackFilterSettingsColumn` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackFilterSettingsColumn`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackFilterSettingsColumnResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `ClickStackGenericWebhook` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackGenericWebhook {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ClickStackGenericWebhookService>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// `ClickStackHeatmapChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHeatmapChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackHeatmapChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackHeatmapSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackHeatmapChartConfigWherelanguage>,
}

/// `ClickStackHeatmapChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackHeatmapChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHeatmapChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackHeatmapChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackHeatmapSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackHeatmapChartConfigWherelanguage>,
}

/// `ClickStackHeatmapSelectItem` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHeatmapSelectItem {
    #[serde(rename = "countExpression", skip_serializing_if = "Option::is_none")]
    pub count_expression: Option<String>,
    #[serde(rename = "heatmapScaleType", skip_serializing_if = "Option::is_none")]
    pub heatmap_scale_type: Option<ClickStackHeatmapSelectItemHeatmapscaletype>,
    #[serde(rename = "valueExpression")]
    pub value_expression: String,
}

/// `ClickStackHeatmapSelectItem` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackHeatmapSelectItem`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHeatmapSelectItemResponse {
    #[serde(rename = "countExpression", skip_serializing_if = "Option::is_none")]
    pub count_expression: Option<String>,
    #[serde(rename = "heatmapScaleType", skip_serializing_if = "Option::is_none")]
    pub heatmap_scale_type: Option<ClickStackHeatmapSelectItemHeatmapscaletype>,
    #[serde(rename = "valueExpression", skip_serializing_if = "Option::is_none")]
    pub value_expression: Option<String>,
}

/// `ClickStackHighlightedAttributeExpression` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHighlightedAttributeExpression {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(rename = "luceneExpression", skip_serializing_if = "Option::is_none")]
    pub lucene_expression: Option<String>,
    #[serde(rename = "sqlExpression")]
    pub sql_expression: String,
}

/// `ClickStackHighlightedAttributeExpression` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackHighlightedAttributeExpression`]: every
/// field is `Option<T>`, so a field the API drops or sends as `null`
/// deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackHighlightedAttributeExpressionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(rename = "luceneExpression", skip_serializing_if = "Option::is_none")]
    pub lucene_expression: Option<String>,
    #[serde(rename = "sqlExpression", skip_serializing_if = "Option::is_none")]
    pub sql_expression: Option<String>,
}

/// `ClickStackIncidentIOWebhook` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackIncidentIOWebhook {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ClickStackIncidentIOWebhookService>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// `ClickStackLineBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineBuilderChartConfig {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(
        rename = "compareToPreviousPeriod",
        skip_serializing_if = "Option::is_none"
    )]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackLineBuilderChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "fitYAxisToData", skip_serializing_if = "Option::is_none")]
    pub fit_y_axis_to_data: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackLineBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackLineBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineBuilderChartConfigResponse {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(
        rename = "compareToPreviousPeriod",
        skip_serializing_if = "Option::is_none"
    )]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackLineBuilderChartConfigDisplaytype>,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "fitYAxisToData", skip_serializing_if = "Option::is_none")]
    pub fit_y_axis_to_data: Option<bool>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackLineRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineRawSqlChartConfig {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(
        rename = "compareToPreviousPeriod",
        skip_serializing_if = "Option::is_none"
    )]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackLineRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackLineRawSqlChartConfigDisplaytype,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "fitYAxisToData", skip_serializing_if = "Option::is_none")]
    pub fit_y_axis_to_data: Option<bool>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackLineRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackLineRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLineRawSqlChartConfigResponse {
    #[serde(
        rename = "alignDateRangeToGranularity",
        skip_serializing_if = "Option::is_none"
    )]
    pub align_date_range_to_granularity: Option<bool>,
    #[serde(
        rename = "compareToPreviousPeriod",
        skip_serializing_if = "Option::is_none"
    )]
    pub compare_to_previous_period: Option<bool>,
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackLineRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackLineRawSqlChartConfigDisplaytype>,
    #[serde(rename = "fillNulls", skip_serializing_if = "Option::is_none")]
    pub fill_nulls: Option<bool>,
    #[serde(rename = "fitYAxisToData", skip_serializing_if = "Option::is_none")]
    pub fit_y_axis_to_data: Option<bool>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackLogSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLogSource {
    #[serde(rename = "bodyExpression", skip_serializing_if = "Option::is_none")]
    pub body_expression: Option<String>,
    pub connection: String,
    #[serde(rename = "defaultTableSelectExpression")]
    pub default_table_select_expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(
        rename = "displayedTimestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub displayed_timestamp_value_expression: Option<String>,
    #[serde(
        rename = "eventAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub event_attributes_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none")]
    pub filter_settings: Option<ClickStackSourceFilterSettings>,
    pub from: ClickStackSourceFrom,
    #[serde(
        rename = "highlightedRowAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_row_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(
        rename = "highlightedTraceAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_trace_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "implicitColumnExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub implicit_column_expression: Option<String>,
    pub kind: ClickStackLogSourceKind,
    #[serde(
        rename = "knownColumnsListExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub known_columns_list_expression: Option<String>,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none")]
    pub materialized_views: Option<Vec<ClickStackMaterializedView>>,
    #[serde(
        rename = "metadataMaterializedViews",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata_materialized_views: Option<ClickStackLogSourceMetadataMaterializedViews>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none")]
    pub metric_source_id: Option<String>,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "serviceNameExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_name_expression: Option<String>,
    #[serde(
        rename = "severityTextExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub severity_text_expression: Option<String>,
    #[serde(rename = "spanIdExpression", skip_serializing_if = "Option::is_none")]
    pub span_id_expression: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
    #[serde(rename = "traceIdExpression", skip_serializing_if = "Option::is_none")]
    pub trace_id_expression: Option<String>,
    #[serde(rename = "traceSourceId", skip_serializing_if = "Option::is_none")]
    pub trace_source_id: Option<String>,
    #[serde(
        rename = "useTextIndexForImplicitColumn",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_text_index_for_implicit_column:
        Option<ClickStackLogSourceUsetextindexforimplicitcolumn>,
}

/// `ClickStackLogSource` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackLogSource`]: every field is `Option<T>`, so
/// a field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLogSourceResponse {
    #[serde(rename = "bodyExpression", skip_serializing_if = "Option::is_none")]
    pub body_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(
        rename = "defaultTableSelectExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub default_table_select_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(
        rename = "displayedTimestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub displayed_timestamp_value_expression: Option<String>,
    #[serde(
        rename = "eventAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub event_attributes_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none")]
    pub filter_settings: Option<ClickStackSourceFilterSettingsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackSourceFromResponse>,
    #[serde(
        rename = "highlightedRowAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_row_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpressionResponse>>,
    #[serde(
        rename = "highlightedTraceAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_trace_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpressionResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "implicitColumnExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub implicit_column_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackLogSourceKind>,
    #[serde(
        rename = "knownColumnsListExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub known_columns_list_expression: Option<String>,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none")]
    pub materialized_views: Option<Vec<ClickStackMaterializedViewResponse>>,
    #[serde(
        rename = "metadataMaterializedViews",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata_materialized_views: Option<ClickStackLogSourceMetadataMaterializedViewsResponse>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none")]
    pub metric_source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySettingResponse>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "serviceNameExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_name_expression: Option<String>,
    #[serde(
        rename = "severityTextExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub severity_text_expression: Option<String>,
    #[serde(rename = "spanIdExpression", skip_serializing_if = "Option::is_none")]
    pub span_id_expression: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
    #[serde(rename = "traceIdExpression", skip_serializing_if = "Option::is_none")]
    pub trace_id_expression: Option<String>,
    #[serde(rename = "traceSourceId", skip_serializing_if = "Option::is_none")]
    pub trace_source_id: Option<String>,
    #[serde(
        rename = "useTextIndexForImplicitColumn",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_text_index_for_implicit_column:
        Option<ClickStackLogSourceUsetextindexforimplicitcolumn>,
}

/// `ClickStackLogSourceMetadataMaterializedViews` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLogSourceMetadataMaterializedViews {
    pub granularity: String,
    #[serde(rename = "keyRollupTable")]
    pub key_rollup_table: String,
    #[serde(rename = "kvRollupTable")]
    pub kv_rollup_table: String,
}

/// `ClickStackLogSourceMetadataMaterializedViews` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackLogSourceMetadataMaterializedViews`]: every
/// field is `Option<T>`, so a field the API drops or sends as `null`
/// deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackLogSourceMetadataMaterializedViewsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<String>,
    #[serde(rename = "keyRollupTable", skip_serializing_if = "Option::is_none")]
    pub key_rollup_table: Option<String>,
    #[serde(rename = "kvRollupTable", skip_serializing_if = "Option::is_none")]
    pub kv_rollup_table: Option<String>,
}

/// `ClickStackMarkdownChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMarkdownChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackMarkdownChartConfigDisplaytype,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

/// `ClickStackMarkdownChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMarkdownChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMarkdownChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackMarkdownChartConfigDisplaytype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

/// `ClickStackMarkdownChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMarkdownChartSeries {
    pub content: String,
    pub r#type: ClickStackMarkdownChartSeriesType,
}

/// `ClickStackMaterializedView` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMaterializedView {
    #[serde(rename = "aggregatedColumns")]
    pub aggregated_columns: Vec<ClickStackAggregatedColumn>,
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "dimensionColumns")]
    pub dimension_columns: String,
    #[serde(rename = "minDate", skip_serializing_if = "Option::is_none")]
    pub min_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "minGranularity")]
    pub min_granularity: ClickStackMaterializedViewMingranularity,
    #[serde(rename = "tableName")]
    pub table_name: String,
    #[serde(rename = "timestampColumn")]
    pub timestamp_column: String,
}

/// `ClickStackMaterializedView` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMaterializedView`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMaterializedViewResponse {
    #[serde(rename = "aggregatedColumns", skip_serializing_if = "Option::is_none")]
    pub aggregated_columns: Option<Vec<ClickStackAggregatedColumnResponse>>,
    #[serde(rename = "databaseName", skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "dimensionColumns", skip_serializing_if = "Option::is_none")]
    pub dimension_columns: Option<String>,
    #[serde(rename = "minDate", skip_serializing_if = "Option::is_none")]
    pub min_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "minGranularity", skip_serializing_if = "Option::is_none")]
    pub min_granularity: Option<ClickStackMaterializedViewMingranularity>,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
    #[serde(rename = "timestampColumn", skip_serializing_if = "Option::is_none")]
    pub timestamp_column: Option<String>,
}

/// `ClickStackMetricSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSource {
    pub connection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    pub from: ClickStackMetricSourceFrom,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub kind: ClickStackMetricSourceKind,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none")]
    pub log_source_id: Option<String>,
    #[serde(rename = "metricTables")]
    pub metric_tables: ClickStackMetricTables,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(rename = "resourceAttributesExpression")]
    pub resource_attributes_expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
}

/// `ClickStackMetricSource` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMetricSource`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSourceResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackMetricSourceFromResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackMetricSourceKind>,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none")]
    pub log_source_id: Option<String>,
    #[serde(rename = "metricTables", skip_serializing_if = "Option::is_none")]
    pub metric_tables: Option<ClickStackMetricTablesResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySettingResponse>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
}

/// `ClickStackMetricSourceFrom` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSourceFrom {
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
}

/// `ClickStackMetricSourceFrom` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMetricSourceFrom`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricSourceFromResponse {
    #[serde(rename = "databaseName", skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
}

/// `ClickStackMetricTables` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricTables {
    #[serde(rename = "exponential histogram")]
    pub exponential_histogram: String,
    pub gauge: String,
    pub histogram: String,
    pub sum: String,
    pub summary: String,
}

/// `ClickStackMetricTables` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMetricTables`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMetricTablesResponse {
    #[serde(
        rename = "exponential histogram",
        skip_serializing_if = "Option::is_none"
    )]
    pub exponential_histogram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gauge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub histogram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// `ClickStackNumberBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberBuilderChartConfig {
    #[serde(rename = "backgroundChart", skip_serializing_if = "Option::is_none")]
    pub background_chart: Option<ClickStackBackgroundChart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(rename = "colorRules", skip_serializing_if = "Option::is_none")]
    pub color_rules: Option<Vec<ClickStackNumberTileColorCondition>>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackNumberBuilderChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackNumberBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackNumberBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberBuilderChartConfigResponse {
    #[serde(rename = "backgroundChart", skip_serializing_if = "Option::is_none")]
    pub background_chart: Option<ClickStackBackgroundChartResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(rename = "colorRules", skip_serializing_if = "Option::is_none")]
    pub color_rules: Option<Vec<ClickStackNumberTileColorConditionResponse>>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackNumberBuilderChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackNumberChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackNumberChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none")]
    pub metric_data_type: Option<ClickStackNumberChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackNumberChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackNumberChartSeriesWherelanguage,
}

/// `ClickStackNumberFormat` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberFormat {
    pub average: bool,
    #[serde(rename = "currencySymbol")]
    pub currency_symbol: String,
    #[serde(rename = "decimalBytes")]
    pub decimal_bytes: bool,
    pub factor: f64,
    pub mantissa: i64,
    #[serde(rename = "numericUnit")]
    pub numeric_unit: ClickStackNumberFormatNumericunit,
    pub output: ClickStackNumberFormatOutput,
    #[serde(rename = "thousandSeparated")]
    pub thousand_separated: bool,
    pub unit: String,
}

/// `ClickStackNumberFormat` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackNumberFormat`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberFormatResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average: Option<bool>,
    #[serde(rename = "currencySymbol", skip_serializing_if = "Option::is_none")]
    pub currency_symbol: Option<String>,
    #[serde(rename = "decimalBytes", skip_serializing_if = "Option::is_none")]
    pub decimal_bytes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub factor: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mantissa: Option<i64>,
    #[serde(rename = "numericUnit", skip_serializing_if = "Option::is_none")]
    pub numeric_unit: Option<ClickStackNumberFormatNumericunit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<ClickStackNumberFormatOutput>,
    #[serde(rename = "thousandSeparated", skip_serializing_if = "Option::is_none")]
    pub thousand_separated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// `ClickStackNumberRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberRawSqlChartConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(rename = "configType")]
    pub config_type: ClickStackNumberRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackNumberRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackNumberRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackNumberRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumberRawSqlChartConfigResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackNumberRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackNumberRawSqlChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackNumericColorCondition` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumericColorCondition {
    pub color: ClickStackChartColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub operator: ClickStackNumericColorConditionOperator,
    pub value: f64,
}

/// `ClickStackNumericColorCondition` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackNumericColorCondition`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackNumericColorConditionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ClickStackChartColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<ClickStackNumericColorConditionOperator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

/// `ClickStackOnClickDashboard` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickDashboard {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackOnClickFilterTemplate>>,
    pub target: ClickStackOnClickTarget,
    pub r#type: ClickStackOnClickDashboardType,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackOnClickDashboardWherelanguage>,
    #[serde(rename = "whereTemplate", skip_serializing_if = "Option::is_none")]
    pub where_template: Option<String>,
}

/// `ClickStackOnClickDashboard` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickDashboard`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickDashboardResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackOnClickFilterTemplateResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<ClickStackOnClickTargetResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackOnClickDashboardType>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackOnClickDashboardWherelanguage>,
    #[serde(rename = "whereTemplate", skip_serializing_if = "Option::is_none")]
    pub where_template: Option<String>,
}

/// `ClickStackOnClickExternal` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickExternal {
    pub r#type: ClickStackOnClickExternalType,
    #[serde(rename = "urlTemplate")]
    pub url_template: String,
}

/// `ClickStackOnClickExternal` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickExternal`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickExternalResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackOnClickExternalType>,
    #[serde(rename = "urlTemplate", skip_serializing_if = "Option::is_none")]
    pub url_template: Option<String>,
}

/// `ClickStackOnClickFilterTemplate` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickFilterTemplate {
    pub expression: String,
    pub kind: ClickStackOnClickFilterTemplateKind,
    pub template: String,
}

/// `ClickStackOnClickFilterTemplate` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickFilterTemplate`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickFilterTemplateResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackOnClickFilterTemplateKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

/// `ClickStackOnClickSearch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackOnClickFilterTemplate>>,
    pub target: ClickStackOnClickTarget,
    pub r#type: ClickStackOnClickSearchType,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackOnClickSearchWherelanguage>,
    #[serde(rename = "whereTemplate", skip_serializing_if = "Option::is_none")]
    pub where_template: Option<String>,
}

/// `ClickStackOnClickSearch` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickSearch`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickSearchResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackOnClickFilterTemplateResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<ClickStackOnClickTargetResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackOnClickSearchType>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackOnClickSearchWherelanguage>,
    #[serde(rename = "whereTemplate", skip_serializing_if = "Option::is_none")]
    pub where_template: Option<String>,
}

/// `ClickStackOnClickTargetIdVariant` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickTargetIdVariant {
    pub id: String,
    pub mode: ClickStackOnClickTargetIdVariantMode,
}

/// `ClickStackOnClickTargetIdVariant` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickTargetIdVariant`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickTargetIdVariantResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ClickStackOnClickTargetIdVariantMode>,
}

/// `ClickStackOnClickTargetTemplateVariant` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickTargetTemplateVariant {
    pub mode: ClickStackOnClickTargetTemplateVariantMode,
    pub template: String,
}

/// `ClickStackOnClickTargetTemplateVariant` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackOnClickTargetTemplateVariant`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackOnClickTargetTemplateVariantResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ClickStackOnClickTargetTemplateVariantMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

/// `ClickStackPagerDutyAPIWebhook` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPagerDutyAPIWebhook {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ClickStackPagerDutyAPIWebhookService>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// `ClickStackPieBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieBuilderChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackPieBuilderChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackPieBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackPieBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieBuilderChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackPieBuilderChartConfigDisplaytype>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackPieRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackPieRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackPieRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackPieRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackPieRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPieRawSqlChartConfigResponse {
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackPieRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackPieRawSqlChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackPromqlSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPromqlSource {
    pub connection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    pub from: ClickStackSourceFrom,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub kind: ClickStackPromqlSourceKind,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
}

/// `ClickStackPromqlSource` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackPromqlSource`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackPromqlSourceResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackSourceFromResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackPromqlSourceKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySettingResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
}

/// `ClickStackQuerySetting` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackQuerySetting {
    pub setting: String,
    pub value: String,
}

/// `ClickStackQuerySetting` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackQuerySetting`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackQuerySettingResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// `ClickStackRole` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackRole {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "isPredefined", skip_serializing_if = "Option::is_none")]
    pub is_predefined: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<ClickStackCASLPermissionResponse>>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ClickStackSavedFilterValue` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedFilterValue {
    pub condition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackSavedFilterValueType>,
}

/// `ClickStackSavedFilterValue` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSavedFilterValue`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedFilterValueResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackSavedFilterValueType>,
}

/// `ClickStackSavedSearch` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedSearch {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackSavedSearchFilterResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "teamId", skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackSavedSearchWherelanguage>,
}

/// `ClickStackSavedSearchFilter` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedSearchFilter {
    pub condition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackSavedSearchFilterType>,
}

/// `ClickStackSavedSearchFilter` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSavedSearchFilter`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedSearchFilterResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ClickStackSavedSearchFilterType>,
}

/// `ClickStackSavedSearchInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSavedSearchInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackSavedSearchFilter>>,
    pub name: String,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackSavedSearchInputWherelanguage>,
}

/// `ClickStackSearchChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSearchChartConfig {
    #[serde(rename = "displayType")]
    pub display_type: ClickStackSearchChartConfigDisplaytype,
    pub select: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackSearchChartConfigWherelanguage,
}

/// `ClickStackSearchChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSearchChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSearchChartConfigResponse {
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackSearchChartConfigDisplaytype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackSearchChartConfigWherelanguage>,
}

/// `ClickStackSearchChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSearchChartSeries {
    pub fields: Vec<String>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackSearchChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackSearchChartSeriesWherelanguage,
}

/// `ClickStackSelectItem` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSelectItem {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackSelectItemAggfn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<ClickStackSelectItemLevel>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(rename = "metricType", skip_serializing_if = "Option::is_none")]
    pub metric_type: Option<ClickStackSelectItemMetrictype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "periodAggFn", skip_serializing_if = "Option::is_none")]
    pub period_agg_fn: Option<ClickStackSelectItemPeriodaggfn>,
    #[serde(rename = "valueExpression", skip_serializing_if = "Option::is_none")]
    pub value_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackSelectItemWherelanguage>,
}

/// `ClickStackSelectItem` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSelectItem`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSelectItemResponse {
    #[serde(rename = "aggFn", skip_serializing_if = "Option::is_none")]
    pub agg_fn: Option<ClickStackSelectItemAggfn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<ClickStackSelectItemLevel>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(rename = "metricType", skip_serializing_if = "Option::is_none")]
    pub metric_type: Option<ClickStackSelectItemMetrictype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "periodAggFn", skip_serializing_if = "Option::is_none")]
    pub period_agg_fn: Option<ClickStackSelectItemPeriodaggfn>,
    #[serde(rename = "valueExpression", skip_serializing_if = "Option::is_none")]
    pub value_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackSelectItemWherelanguage>,
}

/// `ClickStackSessionSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSessionSource {
    pub connection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    pub from: ClickStackSourceFrom,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub kind: ClickStackSessionSourceKind,
    pub name: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
    #[serde(rename = "traceSourceId")]
    pub trace_source_id: String,
}

/// `ClickStackSessionSource` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSessionSource`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSessionSourceResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackSourceFromResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackSessionSourceKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySettingResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
    #[serde(rename = "traceSourceId", skip_serializing_if = "Option::is_none")]
    pub trace_source_id: Option<String>,
}

/// `ClickStackSlackAPIWebhook` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSlackAPIWebhook {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ClickStackSlackAPIWebhookService>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// `ClickStackSlackWebhook` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSlackWebhook {
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ClickStackSlackWebhookService>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// `ClickStackSourceFilterSettings` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFilterSettings {
    pub columns: Vec<ClickStackFilterSettingsColumn>,
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
}

/// `ClickStackSourceFilterSettings` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSourceFilterSettings`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFilterSettingsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ClickStackFilterSettingsColumnResponse>>,
    #[serde(rename = "databaseName", skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
}

/// `ClickStackSourceFrom` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFrom {
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
}

/// `ClickStackSourceFrom` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackSourceFrom`]: every field is `Option<T>`, so
/// a field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackSourceFromResponse {
    #[serde(rename = "databaseName", skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
}

/// `ClickStackTableBuilderChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableBuilderChartConfig {
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackTableBuilderChartConfigDisplaytype,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(
        rename = "groupByColumnsOnLeft",
        skip_serializing_if = "Option::is_none"
    )]
    pub group_by_columns_on_left: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub having: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "onClick", skip_serializing_if = "Option::is_none")]
    pub on_click: Option<ClickStackOnClick>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    pub select: Vec<ClickStackSelectItem>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
}

/// `ClickStackTableBuilderChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackTableBuilderChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableBuilderChartConfigResponse {
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackTableBuilderChartConfigDisplaytype>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    #[serde(
        rename = "groupByColumnsOnLeft",
        skip_serializing_if = "Option::is_none"
    )]
    pub group_by_columns_on_left: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub having: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "onClick", skip_serializing_if = "Option::is_none")]
    pub on_click: Option<ClickStackOnClickResponse>,
    #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<ClickStackSelectItemResponse>>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// `ClickStackTableChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackTableChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(rename = "groupBy")]
    pub group_by: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none")]
    pub metric_data_type: Option<ClickStackTableChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sortOrder", skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<ClickStackTableChartSeriesSortorder>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackTableChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackTableChartSeriesWherelanguage,
}

/// `ClickStackTableRawSqlChartConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableRawSqlChartConfig {
    #[serde(rename = "configType")]
    pub config_type: ClickStackTableRawSqlChartConfigConfigtype,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    #[serde(rename = "displayType")]
    pub display_type: ClickStackTableRawSqlChartConfigDisplaytype,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "onClick", skip_serializing_if = "Option::is_none")]
    pub on_click: Option<ClickStackOnClick>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate")]
    pub sql_template: String,
}

/// `ClickStackTableRawSqlChartConfig` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackTableRawSqlChartConfig`]: every field is `Option<T>`, so a field
/// the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTableRawSqlChartConfigResponse {
    #[serde(rename = "configType", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ClickStackTableRawSqlChartConfigConfigtype>,
    #[serde(rename = "connectionId", skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackTableRawSqlChartConfigDisplaytype>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormatResponse>,
    #[serde(rename = "onClick", skip_serializing_if = "Option::is_none")]
    pub on_click: Option<ClickStackOnClickResponse>,
    #[serde(rename = "sourceId", skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(rename = "sqlTemplate", skip_serializing_if = "Option::is_none")]
    pub sql_template: Option<String>,
}

/// `ClickStackTileInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTileInput {
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "asRatio", skip_serializing_if = "Option::is_none")]
    pub as_ratio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ClickStackTileConfig>,
    #[serde(rename = "containerId", skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    pub h: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<Vec<ClickStackDashboardChartSeries>>,
    #[serde(rename = "tabId", skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    pub w: i64,
    pub x: i64,
    pub y: i64,
}

/// `ClickStackTileOutput` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTileOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ClickStackTileConfigResponse>,
    #[serde(rename = "containerId", skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "tabId", skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i64>,
}

/// `ClickStackTimeChartSeries` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTimeChartSeries {
    #[serde(rename = "aggFn")]
    pub agg_fn: ClickStackTimeChartSeriesAggfn,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(rename = "displayType", skip_serializing_if = "Option::is_none")]
    pub display_type: Option<ClickStackTimeChartSeriesDisplaytype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(rename = "groupBy")]
    pub group_by: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    #[serde(rename = "metricDataType", skip_serializing_if = "Option::is_none")]
    pub metric_data_type: Option<ClickStackTimeChartSeriesMetricdatatype>,
    #[serde(rename = "metricName", skip_serializing_if = "Option::is_none")]
    pub metric_name: Option<String>,
    #[serde(rename = "numberFormat", skip_serializing_if = "Option::is_none")]
    pub number_format: Option<ClickStackNumberFormat>,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub r#type: ClickStackTimeChartSeriesType,
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackTimeChartSeriesWherelanguage,
}

/// `ClickStackTraceSource` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTraceSource {
    pub connection: String,
    #[serde(rename = "defaultTableSelectExpression")]
    pub default_table_select_expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(rename = "durationExpression")]
    pub duration_expression: String,
    #[serde(rename = "durationPrecision")]
    pub duration_precision: i64,
    #[serde(
        rename = "eventAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub event_attributes_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none")]
    pub filter_settings: Option<ClickStackSourceFilterSettings>,
    pub from: ClickStackSourceFrom,
    #[serde(
        rename = "highlightedRowAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_row_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(
        rename = "highlightedTraceAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_trace_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpression>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "implicitColumnExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub implicit_column_expression: Option<String>,
    pub kind: ClickStackTraceSourceKind,
    #[serde(
        rename = "knownColumnsListExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub known_columns_list_expression: Option<String>,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none")]
    pub log_source_id: Option<String>,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none")]
    pub materialized_views: Option<Vec<ClickStackMaterializedView>>,
    #[serde(
        rename = "metadataMaterializedViews",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata_materialized_views: Option<ClickStackTraceSourceMetadataMaterializedViews>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none")]
    pub metric_source_id: Option<String>,
    pub name: String,
    #[serde(rename = "parentSpanIdExpression")]
    pub parent_span_id_expression: String,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySetting>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "serviceNameExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_name_expression: Option<String>,
    #[serde(rename = "sessionSourceId", skip_serializing_if = "Option::is_none")]
    pub session_source_id: Option<String>,
    #[serde(
        rename = "spanEventsValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub span_events_value_expression: Option<String>,
    #[serde(rename = "spanIdExpression")]
    pub span_id_expression: String,
    #[serde(rename = "spanKindExpression")]
    pub span_kind_expression: String,
    #[serde(rename = "spanNameExpression")]
    pub span_name_expression: String,
    #[serde(
        rename = "statusCodeExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_code_expression: Option<String>,
    #[serde(
        rename = "statusMessageExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_message_expression: Option<String>,
    #[serde(rename = "timestampValueExpression")]
    pub timestamp_value_expression: String,
    #[serde(rename = "traceIdExpression")]
    pub trace_id_expression: String,
    #[serde(
        rename = "useTextIndexForImplicitColumn",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_text_index_for_implicit_column:
        Option<ClickStackTraceSourceUsetextindexforimplicitcolumn>,
}

/// `ClickStackTraceSource` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackTraceSource`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTraceSourceResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(
        rename = "defaultTableSelectExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub default_table_select_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    #[serde(rename = "durationExpression", skip_serializing_if = "Option::is_none")]
    pub duration_expression: Option<String>,
    #[serde(rename = "durationPrecision", skip_serializing_if = "Option::is_none")]
    pub duration_precision: Option<i64>,
    #[serde(
        rename = "eventAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub event_attributes_expression: Option<String>,
    #[serde(rename = "filterSettings", skip_serializing_if = "Option::is_none")]
    pub filter_settings: Option<ClickStackSourceFilterSettingsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackSourceFromResponse>,
    #[serde(
        rename = "highlightedRowAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_row_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpressionResponse>>,
    #[serde(
        rename = "highlightedTraceAttributeExpressions",
        skip_serializing_if = "Option::is_none"
    )]
    pub highlighted_trace_attribute_expressions:
        Option<Vec<ClickStackHighlightedAttributeExpressionResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(
        rename = "implicitColumnExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub implicit_column_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ClickStackTraceSourceKind>,
    #[serde(
        rename = "knownColumnsListExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub known_columns_list_expression: Option<String>,
    #[serde(rename = "logSourceId", skip_serializing_if = "Option::is_none")]
    pub log_source_id: Option<String>,
    #[serde(rename = "materializedViews", skip_serializing_if = "Option::is_none")]
    pub materialized_views: Option<Vec<ClickStackMaterializedViewResponse>>,
    #[serde(
        rename = "metadataMaterializedViews",
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata_materialized_views: Option<ClickStackTraceSourceMetadataMaterializedViewsResponse>,
    #[serde(rename = "metricSourceId", skip_serializing_if = "Option::is_none")]
    pub metric_source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        rename = "parentSpanIdExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub parent_span_id_expression: Option<String>,
    #[serde(rename = "querySettings", skip_serializing_if = "Option::is_none")]
    pub query_settings: Option<Vec<ClickStackQuerySettingResponse>>,
    #[serde(
        rename = "resourceAttributesExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_attributes_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        rename = "serviceNameExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_name_expression: Option<String>,
    #[serde(rename = "sessionSourceId", skip_serializing_if = "Option::is_none")]
    pub session_source_id: Option<String>,
    #[serde(
        rename = "spanEventsValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub span_events_value_expression: Option<String>,
    #[serde(rename = "spanIdExpression", skip_serializing_if = "Option::is_none")]
    pub span_id_expression: Option<String>,
    #[serde(rename = "spanKindExpression", skip_serializing_if = "Option::is_none")]
    pub span_kind_expression: Option<String>,
    #[serde(rename = "spanNameExpression", skip_serializing_if = "Option::is_none")]
    pub span_name_expression: Option<String>,
    #[serde(
        rename = "statusCodeExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_code_expression: Option<String>,
    #[serde(
        rename = "statusMessageExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_message_expression: Option<String>,
    #[serde(
        rename = "timestampValueExpression",
        skip_serializing_if = "Option::is_none"
    )]
    pub timestamp_value_expression: Option<String>,
    #[serde(rename = "traceIdExpression", skip_serializing_if = "Option::is_none")]
    pub trace_id_expression: Option<String>,
    #[serde(
        rename = "useTextIndexForImplicitColumn",
        skip_serializing_if = "Option::is_none"
    )]
    pub use_text_index_for_implicit_column:
        Option<ClickStackTraceSourceUsetextindexforimplicitcolumn>,
}

/// `ClickStackTraceSourceMetadataMaterializedViews` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTraceSourceMetadataMaterializedViews {
    pub granularity: String,
    #[serde(rename = "keyRollupTable")]
    pub key_rollup_table: String,
    #[serde(rename = "kvRollupTable")]
    pub kv_rollup_table: String,
}

/// `ClickStackTraceSourceMetadataMaterializedViews` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackTraceSourceMetadataMaterializedViews`]:
/// every field is `Option<T>`, so a field the API drops or sends as `null`
/// deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackTraceSourceMetadataMaterializedViewsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<String>,
    #[serde(rename = "keyRollupTable", skip_serializing_if = "Option::is_none")]
    pub key_rollup_table: Option<String>,
    #[serde(rename = "kvRollupTable", skip_serializing_if = "Option::is_none")]
    pub kv_rollup_table: Option<String>,
}

/// `ClickStackUpdateAlertRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateAlertRequest {
    pub channel: ClickStackAlertChannel,
    #[serde(rename = "dashboardId", skip_serializing_if = "Option::is_none")]
    pub dashboard_id: Option<String>,
    #[serde(rename = "groupBy", skip_serializing_if = "Option::is_none")]
    pub group_by: Option<String>,
    pub interval: ClickStackUpdateAlertRequestInterval,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(
        rename = "numConsecutiveWindows",
        skip_serializing_if = "Option::is_none"
    )]
    pub num_consecutive_windows: Option<i64>,
    #[serde(rename = "savedSearchId", skip_serializing_if = "Option::is_none")]
    pub saved_search_id: Option<String>,
    #[serde(
        rename = "scheduleOffsetMinutes",
        skip_serializing_if = "Option::is_none"
    )]
    pub schedule_offset_minutes: Option<i64>,
    #[serde(rename = "scheduleStartAt", skip_serializing_if = "Option::is_none")]
    pub schedule_start_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source: ClickStackUpdateAlertRequestSource,
    pub threshold: f64,
    #[serde(rename = "thresholdMax", skip_serializing_if = "Option::is_none")]
    pub threshold_max: Option<f64>,
    #[serde(rename = "thresholdType")]
    pub threshold_type: ClickStackUpdateAlertRequestThresholdtype,
    #[serde(rename = "tileId", skip_serializing_if = "Option::is_none")]
    pub tile_id: Option<String>,
}

/// `ClickStackUpdateConnectionRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateConnectionRequest {
    pub host: String,
    #[serde(
        rename = "hyperdxSettingPrefix",
        skip_serializing_if = "Option::is_none"
    )]
    pub hyperdx_setting_prefix: Option<String>,
    #[serde(
        rename = "isPrometheusEndpoint",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_prometheus_endpoint: Option<bool>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub username: String,
}

/// `ClickStackUpdateDashboardRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateDashboardRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containers: Option<Vec<ClickStackDashboardContainer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<ClickStackFilter>>,
    pub name: String,
    #[serde(rename = "savedFilterValues", skip_serializing_if = "Option::is_none")]
    pub saved_filter_values: Option<Vec<ClickStackSavedFilterValue>>,
    #[serde(rename = "savedQuery", skip_serializing_if = "Option::is_none")]
    pub saved_query: Option<String>,
    #[serde(rename = "savedQueryLanguage", skip_serializing_if = "Option::is_none")]
    pub saved_query_language: Option<ClickStackUpdateDashboardRequestSavedquerylanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub tiles: Vec<ClickStackTileInput>,
}

/// `ClickStackUpdateRoleRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateRoleRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub permissions: Vec<ClickStackCASLPermission>,
}

/// `ClickStackValidateDashboardError` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackValidateDashboardError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// `ClickStackValidateDashboardResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackValidateDashboardResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ClickStackValidateDashboardError>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized: Option<ClickStackValidateDashboardResponseNormalized>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid: Option<bool>,
}

/// `ClickStackWebhookInput` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackWebhookInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<ClickStackWebhookInputHeaders>,
    pub name: String,
    #[serde(rename = "queryParams", skip_serializing_if = "Option::is_none")]
    pub query_params: Option<ClickStackWebhookInputQueryParams>,
    pub service: ClickStackWebhookInputService,
    pub url: String,
}

/// `CreateReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CreateReversePrivateEndpoint {
    #[serde(
        rename = "customPrivateDnsMappings",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_private_dns_mappings: Option<Vec<CustomPrivateDnsMapping>>,
    pub description: String,
    #[serde(
        rename = "gcpServiceAttachment",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_service_attachment: Option<String>,
    #[serde(rename = "mskAuthentication", skip_serializing_if = "Option::is_none")]
    pub msk_authentication: Option<CreateReversePrivateEndpointMskauthentication>,
    #[serde(rename = "mskClusterArn", skip_serializing_if = "Option::is_none")]
    pub msk_cluster_arn: Option<String>,
    pub r#type: CreateReversePrivateEndpointType,
    #[serde(
        rename = "vpcEndpointServiceName",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_endpoint_service_name: Option<String>,
    #[serde(
        rename = "vpcResourceConfigurationId",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_configuration_id: Option<String>,
    #[serde(
        rename = "vpcResourceShareArn",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_share_arn: Option<String>,
}

/// `CurrentScaling` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CurrentScaling {
    #[serde(rename = "activeEntryId", skip_serializing_if = "Option::is_none")]
    pub active_entry_id: Option<uuid::Uuid>,
    #[serde(
        rename = "effectiveAutoscalingMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_autoscaling_mode: Option<CurrentScalingEffectiveautoscalingmode>,
    #[serde(
        rename = "effectiveIdleScaling",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_idle_scaling: Option<bool>,
    #[serde(
        rename = "effectiveIdleTimeoutMinutes",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_idle_timeout_minutes: Option<i64>,
    #[serde(
        rename = "effectiveMaxReplicaMemoryGb",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_max_replica_memory_gb: Option<f64>,
    #[serde(
        rename = "effectiveMaxReplicas",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_max_replicas: Option<i64>,
    #[serde(
        rename = "effectiveMinReplicaMemoryGb",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_min_replica_memory_gb: Option<f64>,
    #[serde(
        rename = "effectiveMinReplicas",
        skip_serializing_if = "Option::is_none"
    )]
    pub effective_min_replicas: Option<i64>,
}

/// `CustomPrivateDnsMapping` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CustomPrivateDnsMapping {
    #[serde(rename = "privateDnsName", skip_serializing_if = "Option::is_none")]
    pub private_dns_name: Option<String>,
}

/// `CustomPrivateDnsMapping` from the ClickHouse Cloud API, in response
/// position.
///
/// Response variant of [`CustomPrivateDnsMapping`]: every field is `Option<T>`,
/// so a field the API drops or sends as `null` deserializes to `None` instead
/// of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CustomPrivateDnsMappingResponse {
    #[serde(rename = "privateDnsName", skip_serializing_if = "Option::is_none")]
    pub private_dns_name: Option<String>,
}

/// `GcpBackupBucket` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucket {
    #[serde(rename = "accessKeyId", skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(rename = "bucketPath", skip_serializing_if = "Option::is_none")]
    pub bucket_path: Option<String>,
    #[serde(rename = "bucketProvider", skip_serializing_if = "Option::is_none")]
    pub bucket_provider: Option<GcpBackupBucketBucketprovider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
}

/// `GcpBackupBucketPatchRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketPatchRequestV1 {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: GcpBackupBucketPatchRequestV1Bucketprovider,
    #[serde(rename = "secretAccessKey")]
    pub secret_access_key: String,
}

/// `GcpBackupBucketPostRequestV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketPostRequestV1 {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: GcpBackupBucketPostRequestV1Bucketprovider,
    #[serde(rename = "secretAccessKey")]
    pub secret_access_key: String,
}

/// `GcpBackupBucketProperties` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GcpBackupBucketProperties {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "bucketPath")]
    pub bucket_path: String,
    #[serde(rename = "bucketProvider")]
    pub bucket_provider: GcpBackupBucketPropertiesBucketprovider,
}

/// `InstancePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstancePrivateEndpoint {
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<InstancePrivateEndpointCloudprovider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<InstancePrivateEndpointRegion>,
}

/// `InstancePrivateEndpointsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstancePrivateEndpointsPatch {
    pub add: Vec<String>,
    pub remove: Vec<String>,
}

/// `InstanceServiceQueryApiEndpointsPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstanceServiceQueryApiEndpointsPostRequest {
    #[serde(rename = "allowedOrigins")]
    pub allowed_origins: String,
    #[serde(rename = "openApiKeys")]
    pub open_api_keys: Vec<String>,
    pub roles: Vec<String>,
}

/// `InstanceTagsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InstanceTagsPatch {
    pub add: Vec<ResourceTagsV1>,
    pub remove: Vec<ResourceTagsV1>,
}

/// `Invitation` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Invitation {
    #[serde(rename = "assignedRoles", skip_serializing_if = "Option::is_none")]
    pub assigned_roles: Option<Vec<AssignedRole>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "expireAt", skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<InvitationRole>,
}

/// `InvitationPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InvitationPostRequest {
    #[serde(rename = "assignedRoleIds")]
    pub assigned_role_ids: Vec<String>,
    pub email: String,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<InvitationPostRequestRole>,
}

/// `IpAccessListEntry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IpAccessListEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source: String,
}

/// `IpAccessListEntry` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`IpAccessListEntry`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IpAccessListEntryResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// `IpAccessListPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IpAccessListPatch {
    pub add: Vec<IpAccessListEntry>,
    pub remove: Vec<IpAccessListEntry>,
}

/// `License` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct License {
    #[serde(rename = "environmentFingerprint")]
    pub environment_fingerprint: String,
    pub expiration: String,
    pub id: String,
    pub memory: String,
    pub name: String,
}

/// `Member` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Member {
    #[serde(rename = "assignedRoles", skip_serializing_if = "Option::is_none")]
    pub assigned_roles: Option<Vec<AssignedRole>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "joinedAt", skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MemberRole>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// `MemberPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemberPatchRequest {
    #[serde(rename = "assignedRoleIds", skip_serializing_if = "Option::is_none")]
    pub assigned_role_ids: Option<Vec<String>>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MemberPatchRequestRole>,
}

/// `MskIamUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MskIamUser {
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    #[serde(rename = "secretKey")]
    pub secret_key: String,
}

/// `MutualTLS` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MutualTLS {
    pub certificate: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

/// `Organization` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Organization {
    #[serde(rename = "byocConfig", skip_serializing_if = "Option::is_none")]
    pub byoc_config: Option<Vec<ByocConfig>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "privateEndpoints", skip_serializing_if = "Option::is_none")]
    pub private_endpoints: Option<Vec<OrganizationPrivateEndpoint>>,
}

/// `OrganizationCloudRegionPrivateEndpointConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationCloudRegionPrivateEndpointConfig {
    #[serde(rename = "endpointServiceId", skip_serializing_if = "Option::is_none")]
    pub endpoint_service_id: Option<String>,
}

/// `OrganizationPatchPrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPatchPrivateEndpoint {
    #[serde(rename = "cloudProvider")]
    pub cloud_provider: OrganizationPatchPrivateEndpointCloudprovider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub id: String,
    pub region: OrganizationPatchPrivateEndpointRegion,
}

/// `OrganizationPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPatchRequest {
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "privateEndpoints", skip_serializing_if = "Option::is_none")]
    pub private_endpoints: Option<OrganizationPrivateEndpointsPatch>,
}

/// `OrganizationPrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPrivateEndpoint {
    #[serde(rename = "cloudProvider", skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<OrganizationPrivateEndpointCloudprovider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<OrganizationPrivateEndpointRegion>,
}

/// `OrganizationPrivateEndpointsPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationPrivateEndpointsPatch {
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add: Option<Vec<OrganizationPatchPrivateEndpoint>>,
    pub remove: Vec<OrganizationPatchPrivateEndpoint>,
}

/// `OrganizationQuota` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrganizationQuota {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjustable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "quotaCode", skip_serializing_if = "Option::is_none")]
    pub quota_code: Option<OrganizationQuotaQuotacode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<OrganizationQuotaScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
}

/// `PLAIN` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PLAIN {
    pub password: String,
    pub username: String,
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

/// `PrivateEndpointConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PrivateEndpointConfig {
    #[serde(rename = "endpointServiceId", skip_serializing_if = "Option::is_none")]
    pub endpoint_service_id: Option<String>,
    #[serde(rename = "privateDnsHostname", skip_serializing_if = "Option::is_none")]
    pub private_dns_hostname: Option<String>,
}

/// `RBACPolicy` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicy {
    #[serde(rename = "allowDeny", skip_serializing_if = "Option::is_none")]
    pub allow_deny: Option<RBACPolicyAllowdeny>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<String>>,
    #[serde(rename = "roleId", skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<RBACPolicyTagsResponse>,
    #[serde(rename = "tenantId", skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// `RBACPolicyCreateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicyCreateRequest {
    #[serde(rename = "allowDeny")]
    pub allow_deny: RBACPolicyCreateRequestAllowdeny,
    pub permissions: Vec<String>,
    pub resources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<RBACPolicyTags>,
}

/// `RBACPolicyTags` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicyTags {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grants: Option<Vec<String>>,
    #[serde(rename = "roleV2", skip_serializing_if = "Option::is_none")]
    pub role_v2: Option<RBACPolicyTagsRolev2>,
}

/// `RBACPolicyTags` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`RBACPolicyTags`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACPolicyTagsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grants: Option<Vec<String>>,
    #[serde(rename = "roleV2", skip_serializing_if = "Option::is_none")]
    pub role_v2: Option<RBACPolicyTagsRolev2>,
}

/// `RBACRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RBACRole {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actors: Option<Vec<String>>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "ownerId", skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policies: Option<Vec<RBACPolicy>>,
    #[serde(rename = "tenantId", skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<RBACRoleType>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `ResourceTagsV1` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ResourceTagsV1 {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// `ResourceTagsV1` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ResourceTagsV1`]: every field is `Option<T>`, so a
/// field the API drops or sends as `null` deserializes to `None` instead of
/// failing. Writing a fetched tag back to the API goes through
/// `TryFrom<ResourceTagsV1Response>` (see [`crate::convert`]), because a tag
/// without a key cannot be sent.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ResourceTagsV1Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// `ReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ReversePrivateEndpoint {
    #[serde(
        rename = "customPrivateDnsMappings",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_private_dns_mappings: Option<Vec<CustomPrivateDnsMappingResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "dnsNames", skip_serializing_if = "Option::is_none")]
    pub dns_names: Option<Vec<String>>,
    #[serde(rename = "endpointId", skip_serializing_if = "Option::is_none")]
    pub endpoint_id: Option<String>,
    #[serde(
        rename = "gcpServiceAttachment",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcp_service_attachment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "mskAuthentication", skip_serializing_if = "Option::is_none")]
    pub msk_authentication: Option<ReversePrivateEndpointMskauthentication>,
    #[serde(rename = "mskClusterArn", skip_serializing_if = "Option::is_none")]
    pub msk_cluster_arn: Option<String>,
    #[serde(rename = "privateDnsNames", skip_serializing_if = "Option::is_none")]
    pub private_dns_names: Option<Vec<String>>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ReversePrivateEndpointStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ReversePrivateEndpointType>,
    #[serde(
        rename = "vpcEndpointServiceName",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_endpoint_service_name: Option<String>,
    #[serde(
        rename = "vpcResourceConfigurationId",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_configuration_id: Option<String>,
    #[serde(
        rename = "vpcResourceShareArn",
        skip_serializing_if = "Option::is_none"
    )]
    pub vpc_resource_share_arn: Option<String>,
}

/// `RoleCreateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoleCreateRequest {
    pub actors: Vec<String>,
    pub name: String,
    pub policies: Vec<RBACPolicyCreateRequest>,
}

/// `RoleUpdateRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoleUpdateRequest {
    pub actors: Vec<String>,
    pub name: String,
    pub policies: Vec<RBACPolicyCreateRequest>,
}

/// `ScalingSchedule` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScalingSchedule {
    #[serde(rename = "activeEntryId", skip_serializing_if = "Option::is_none")]
    pub active_entry_id: Option<uuid::Uuid>,
    #[serde(rename = "baseConfig", skip_serializing_if = "Option::is_none")]
    pub base_config: Option<ScalingScheduleBaseConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<ScalingScheduleEntry>>,
}

/// `ScalingScheduleBaseConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScalingScheduleBaseConfig {
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<i64>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
}

/// `ScalingScheduleEntry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScalingScheduleEntry {
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "endHourUtc", skip_serializing_if = "Option::is_none")]
    pub end_hour_utc: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<i64>,
    #[serde(rename = "isActiveNow", skip_serializing_if = "Option::is_none")]
    pub is_active_now: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "startHourUtc", skip_serializing_if = "Option::is_none")]
    pub start_hour_utc: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekdays: Option<Vec<i64>>,
}

/// `ScalingScheduleEntryRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScalingScheduleEntryRequest {
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "endHourUtc")]
    pub end_hour_utc: i64,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<i64>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    pub name: String,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
    #[serde(rename = "startHourUtc")]
    pub start_hour_utc: i64,
    pub weekdays: Vec<i64>,
}

/// `ScalingSchedulePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScalingSchedulePostRequest {
    pub entries: Vec<ScalingScheduleEntryRequest>,
}

// The `Scim*` family below is strict in both directions, deliberately. The spec
// defines the SCIM schemas but declares no SCIM path, so no `Client` method
// sends or returns one: the family is reachable from neither a request root nor
// a response root, and the analyzer resolves such operation-unreferenced schemas
// in request position. Making the SCIM list/response envelopes all-`Option`
// would therefore report `FieldOptionalityMismatch` drift while protecting no
// actual response. `scim_models_are_outside_the_response_tree` in
// `tests/spec_coverage_test.rs` pins that premise: if SCIM operations are ever
// added to `client.rs`, the envelopes they return must be split first.

/// `ScimEnterpriseManager` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimEnterpriseManager {
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub value: String,
}

/// `ScimEnterpriseUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimEnterpriseUser {
    #[serde(rename = "costCenter")]
    pub cost_center: String,
    pub department: String,
    pub division: String,
    #[serde(rename = "employeeNumber")]
    pub employee_number: String,
    pub manager: ScimEnterpriseManager,
    pub organization: String,
}

/// `ScimGroup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroup {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<ScimGroupMember>>,
    pub meta: ScimGroupMeta,
    pub schemas: Vec<String>,
}

/// `ScimGroupListResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupListResponse {
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimGroup>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    pub schemas: Vec<String>,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
}

/// `ScimGroupMember` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupMember {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    pub value: String,
}

/// `ScimGroupMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupMeta {
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "lastModified")]
    pub last_modified: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimGroupPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupPostRequest {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<ScimGroupMember>>,
    pub schemas: Vec<String>,
}

/// `ScimGroupPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimGroupPutRequest {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<ScimGroupMember>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ScimGroupMeta>,
    pub schemas: Vec<String>,
}

/// `ScimListResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimListResponse {
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimUser>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    pub schemas: Vec<String>,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
}

/// `ScimPatchOp` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimPatchOp {
    #[serde(rename = "Operations")]
    pub operations: Vec<ScimPatchOperation>,
    pub schemas: Vec<String>,
}

/// `ScimPatchOperation` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimPatchOperation {
    pub op: ScimPatchOperationOp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// `ScimUser` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUser {
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addresses: Option<Vec<ScimUserAddress>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub emails: Vec<ScimUserEmail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlements: Option<Vec<ScimUserEntitlement>>,
    #[serde(
        rename = "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
        skip_serializing_if = "Option::is_none"
    )]
    pub enterprise_user: Option<ScimEnterpriseUser>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<ScimUserGroup>>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ims: Option<Vec<ScimUserIm>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    pub meta: ScimUserMeta,
    pub name: ScimUserName,
    #[serde(rename = "nickName", skip_serializing_if = "Option::is_none")]
    pub nick_name: Option<String>,
    #[serde(rename = "phoneNumbers", skip_serializing_if = "Option::is_none")]
    pub phone_numbers: Option<Vec<ScimUserPhoneNumber>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photos: Option<Vec<ScimUserPhoto>>,
    #[serde(rename = "preferredLanguage", skip_serializing_if = "Option::is_none")]
    pub preferred_language: Option<String>,
    #[serde(rename = "profileUrl", skip_serializing_if = "Option::is_none")]
    pub profile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<ScimUserRole>>,
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userType", skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
    #[serde(rename = "x509Certificates", skip_serializing_if = "Option::is_none")]
    pub x509_certificates: Option<Vec<ScimX509Certificate>>,
}

/// `ScimUserAddress` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserAddress {
    pub country: String,
    pub formatted: String,
    pub locality: String,
    #[serde(rename = "postalCode")]
    pub postal_code: String,
    pub primary: bool,
    pub region: String,
    #[serde(rename = "streetAddress")]
    pub street_address: String,
    pub r#type: String,
}

/// `ScimUserEmail` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserEmail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    pub value: String,
}

/// `ScimUserEntitlement` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserEntitlement {
    pub display: String,
    pub primary: bool,
    pub r#type: String,
    pub value: String,
}

/// `ScimUserGroup` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserGroup {
    pub display: String,
    pub r#type: String,
    pub value: String,
}

/// `ScimUserIm` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserIm {
    pub primary: bool,
    pub r#type: String,
    pub value: String,
}

/// `ScimUserMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserMeta {
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "lastModified")]
    pub last_modified: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimUserName` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserName {
    #[serde(rename = "familyName")]
    pub family_name: String,
    pub formatted: String,
    #[serde(rename = "givenName")]
    pub given_name: String,
    #[serde(rename = "honorificPrefix")]
    pub honorific_prefix: String,
    #[serde(rename = "honorificSuffix")]
    pub honorific_suffix: String,
    #[serde(rename = "middleName")]
    pub middle_name: String,
}

/// `ScimUserPhoneNumber` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPhoneNumber {
    pub primary: bool,
    pub r#type: String,
    pub value: String,
}

/// `ScimUserPhoto` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPhoto {
    pub primary: bool,
    pub r#type: String,
    pub value: String,
}

/// `ScimUserPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPostRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addresses: Option<Vec<ScimUserAddress>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub emails: Vec<ScimUserEmail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlements: Option<Vec<ScimUserEntitlement>>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<ScimUserGroup>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ims: Option<Vec<ScimUserIm>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<ScimUserName>,
    #[serde(rename = "nickName", skip_serializing_if = "Option::is_none")]
    pub nick_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename = "phoneNumbers", skip_serializing_if = "Option::is_none")]
    pub phone_numbers: Option<Vec<ScimUserPhoneNumber>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photos: Option<Vec<ScimUserPhoto>>,
    #[serde(rename = "preferredLanguage", skip_serializing_if = "Option::is_none")]
    pub preferred_language: Option<String>,
    #[serde(rename = "profileUrl", skip_serializing_if = "Option::is_none")]
    pub profile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<ScimUserRole>>,
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(
        rename = "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
        skip_serializing_if = "Option::is_none"
    )]
    pub urn_ietf_params_scim_schemas_extension_enterprise_2_0_user: Option<ScimEnterpriseUser>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userType", skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
    #[serde(rename = "x509Certificates", skip_serializing_if = "Option::is_none")]
    pub x509_certificates: Option<Vec<ScimX509Certificate>>,
}

/// `ScimUserPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserPutRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addresses: Option<Vec<ScimUserAddress>>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub emails: Vec<ScimUserEmail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlements: Option<Vec<ScimUserEntitlement>>,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<ScimUserGroup>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ims: Option<Vec<ScimUserIm>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ScimUserMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<ScimUserName>,
    #[serde(rename = "nickName", skip_serializing_if = "Option::is_none")]
    pub nick_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(rename = "phoneNumbers", skip_serializing_if = "Option::is_none")]
    pub phone_numbers: Option<Vec<ScimUserPhoneNumber>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photos: Option<Vec<ScimUserPhoto>>,
    #[serde(rename = "preferredLanguage", skip_serializing_if = "Option::is_none")]
    pub preferred_language: Option<String>,
    #[serde(rename = "profileUrl", skip_serializing_if = "Option::is_none")]
    pub profile_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<ScimUserRole>>,
    pub schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(
        rename = "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User",
        skip_serializing_if = "Option::is_none"
    )]
    pub urn_ietf_params_scim_schemas_extension_enterprise_2_0_user: Option<ScimEnterpriseUser>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userType", skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
    #[serde(rename = "x509Certificates", skip_serializing_if = "Option::is_none")]
    pub x509_certificates: Option<Vec<ScimX509Certificate>>,
}

/// `ScimUserRole` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimUserRole {
    pub display: String,
    pub primary: bool,
    pub r#type: String,
    pub value: String,
}

/// `ScimX509Certificate` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimX509Certificate {
    pub value: String,
}

/// `ScimAuthenticationScheme` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimAuthenticationScheme {
    pub description: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<bool>,
    #[serde(rename = "specUri", skip_serializing_if = "Option::is_none")]
    pub spec_uri: Option<String>,
    #[serde(rename = "type")]
    pub r#type: String,
}

/// `ScimBooleanFeature` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimBooleanFeature {
    pub supported: bool,
}

/// `ScimResourceType` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimResourceType {
    pub description: String,
    pub endpoint: String,
    pub id: String,
    pub meta: ScimResourceTypeMeta,
    pub name: String,
    pub schema: String,
    #[serde(rename = "schemaExtensions")]
    pub schema_extensions: Vec<ScimSchemaExtension>,
    pub schemas: Vec<String>,
}

/// `ScimResourceTypeListResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimResourceTypeListResponse {
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimResourceType>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    pub schemas: Vec<String>,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
}

/// `ScimResourceTypeMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimResourceTypeMeta {
    pub location: String,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimSchema` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimSchema {
    pub attributes: Vec<ScimSchemaAttribute>,
    pub description: String,
    pub id: String,
    pub meta: ScimSchemaMeta,
    pub name: String,
    pub schemas: Vec<String>,
}

/// `ScimSchemaAttribute` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimSchemaAttribute {
    #[serde(rename = "canonicalValues", skip_serializing_if = "Option::is_none")]
    pub canonical_values: Option<Vec<String>>,
    #[serde(rename = "caseExact", skip_serializing_if = "Option::is_none")]
    pub case_exact: Option<bool>,
    pub description: String,
    #[serde(rename = "multiValued")]
    pub multi_valued: bool,
    pub mutability: String,
    pub name: String,
    #[serde(rename = "referenceTypes", skip_serializing_if = "Option::is_none")]
    pub reference_types: Option<Vec<String>>,
    pub required: bool,
    pub returned: String,
    #[serde(rename = "subAttributes", skip_serializing_if = "Option::is_none")]
    pub sub_attributes: Option<Vec<ScimSchemaAttribute>>,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniqueness: Option<String>,
}

/// `ScimSchemaExtension` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimSchemaExtension {
    pub required: bool,
    pub schema: String,
}

/// `ScimSchemaListResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimSchemaListResponse {
    #[serde(rename = "Resources")]
    pub resources: Vec<ScimSchema>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: i64,
    pub schemas: Vec<String>,
    #[serde(rename = "startIndex")]
    pub start_index: i64,
    #[serde(rename = "totalResults")]
    pub total_results: i64,
}

/// `ScimSchemaMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimSchemaMeta {
    pub location: String,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimServiceProviderConfig` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimServiceProviderConfig {
    #[serde(rename = "authenticationSchemes")]
    pub authentication_schemes: Vec<ScimAuthenticationScheme>,
    pub bulk: ScimServiceProviderConfigBulk,
    #[serde(rename = "changePassword")]
    pub change_password: ScimBooleanFeature,
    #[serde(rename = "documentationUri", skip_serializing_if = "Option::is_none")]
    pub documentation_uri: Option<String>,
    pub etag: ScimBooleanFeature,
    pub filter: ScimServiceProviderConfigFilter,
    pub meta: ScimServiceProviderConfigMeta,
    pub patch: ScimServiceProviderConfigPatch,
    pub schemas: Vec<String>,
    pub sort: ScimBooleanFeature,
}

/// `ScimServiceProviderConfigBulk` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimServiceProviderConfigBulk {
    #[serde(rename = "maxOperations")]
    pub max_operations: i64,
    #[serde(rename = "maxPayloadSize")]
    pub max_payload_size: i64,
    pub supported: bool,
}

/// `ScimServiceProviderConfigFilter` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimServiceProviderConfigFilter {
    #[serde(rename = "maxResults")]
    pub max_results: i64,
    pub supported: bool,
}

/// `ScimServiceProviderConfigMeta` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimServiceProviderConfigMeta {
    pub location: String,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

/// `ScimServiceProviderConfigPatch` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ScimServiceProviderConfigPatch {
    pub supported: bool,
}

/// `ServicPrivateEndpointePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicPrivateEndpointePostRequest {
    pub description: String,
    pub id: String,
}

/// `Service` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Service {
    #[serde(
        rename = "availablePrivateEndpointIds",
        skip_serializing_if = "Option::is_none"
    )]
    pub available_private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "byocId", skip_serializing_if = "Option::is_none")]
    pub byoc_id: Option<String>,
    #[serde(rename = "clickhouseVersion", skip_serializing_if = "Option::is_none")]
    pub clickhouse_version: Option<String>,
    #[serde(rename = "complianceType", skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<ServiceCompliancetype>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "currentScaling", skip_serializing_if = "Option::is_none")]
    pub current_scaling: Option<CurrentScaling>,
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(
        rename = "encryptionAssumedRoleIdentifier",
        skip_serializing_if = "Option::is_none"
    )]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none")]
    pub encryption_key: Option<String>,
    #[serde(rename = "encryptionRoleId", skip_serializing_if = "Option::is_none")]
    pub encryption_role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpoint>>,
    #[serde(
        rename = "hasTransparentDataEncryption",
        skip_serializing_if = "Option::is_none"
    )]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessListEntryResponse>>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    #[serde(rename = "isReadonly", skip_serializing_if = "Option::is_none")]
    pub is_readonly: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ServiceProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ServiceProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<ServiceRegion>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ServiceReleasechannel>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
    #[serde(rename = "scalingSchedule", skip_serializing_if = "Option::is_none")]
    pub scaling_schedule: Option<ScalingSchedule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ServiceState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTagsV1Response>>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<ServiceTier>,
    #[serde(
        rename = "transparentDataEncryptionKeyId",
        skip_serializing_if = "Option::is_none"
    )]
    pub transparent_data_encryption_key_id: Option<String>,
}

/// `ServiceAccount` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceAccount {
    #[serde(rename = "serviceAccountFile")]
    pub service_account_file: String,
}

/// `ServiceClickhouseSetting` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSetting {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// `ServiceClickhouseSettingSchemaEntry` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingSchemaEntry {
    #[serde(rename = "deprecationNotice", skip_serializing_if = "Option::is_none")]
    pub deprecation_notice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// `ServiceClickhouseSettingWarning` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingWarning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `ServiceClickhouseSettingsList` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingsList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Vec<ServiceClickhouseSetting>>,
}

/// `ServiceClickhouseSettingsPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingsPatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
}

/// `ServiceClickhouseSettingsPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingsPatchResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<ServiceClickhouseSettingWarning>>,
}

/// `ServiceClickhouseSettingsSchema` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceClickhouseSettingsSchema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Vec<ServiceClickhouseSettingSchemaEntry>>,
}

/// `ServiceEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // The schema currently says `number`, but Cloud endpoints are TCP ports and
    // the API sends integral values. Keep JSON output integral for consumers.
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<ServiceEndpointProtocol>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// `ServiceEndpointChange` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceEndpointChange {
    pub enabled: bool,
    pub protocol: ServiceEndpointChangeProtocol,
}

/// `ServicePasswordPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePasswordPatchRequest {
    #[serde(rename = "newDoubleSha1Hash", skip_serializing_if = "Option::is_none")]
    pub new_double_sha1_hash: Option<String>,
    #[serde(rename = "newPasswordHash", skip_serializing_if = "Option::is_none")]
    pub new_password_hash: Option<String>,
}

/// `ServicePasswordPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePasswordPatchResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// `ServicePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePatchRequest {
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpointChange>>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<IpAccessListPatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<InstancePrivateEndpointsPatch>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ServicePatchRequestReleasechannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<InstanceTagsPatch>,
    #[serde(
        rename = "transparentDataEncryptionKeyId",
        skip_serializing_if = "Option::is_none"
    )]
    pub transparent_data_encryption_key_id: Option<String>,
}

/// `ServicePostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePostRequest {
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "backupId", skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<uuid::Uuid>,
    #[serde(rename = "byocId", skip_serializing_if = "Option::is_none")]
    pub byoc_id: Option<String>,
    #[serde(rename = "complianceType", skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<ServicePostRequestCompliancetype>,
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(
        rename = "encryptionAssumedRoleIdentifier",
        skip_serializing_if = "Option::is_none"
    )]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none")]
    pub encryption_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpointChange>>,
    #[serde(
        rename = "hasTransparentDataEncryption",
        skip_serializing_if = "Option::is_none"
    )]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "ipAccessList")]
    pub ip_access_list: Vec<IpAccessListEntry>,
    #[serde(rename = "isReadonly", skip_serializing_if = "Option::is_none")]
    pub is_readonly: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_total_memory_gb: Option<f64>,
    pub name: String,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(
        rename = "privatePreviewTermsChecked",
        skip_serializing_if = "Option::is_none"
    )]
    pub private_preview_terms_checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ServicePostRequestProfile>,
    pub provider: ServicePostRequestProvider,
    pub region: ServicePostRequestRegion,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ServicePostRequestReleasechannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTagsV1>>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<ServicePostRequestTier>,
}

/// `ServicePostResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServicePostResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<Service>,
}

/// `ServiceQueryAPIEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceQueryAPIEndpoint {
    #[serde(rename = "allowedOrigins", skip_serializing_if = "Option::is_none")]
    pub allowed_origins: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "openApiKeys", skip_serializing_if = "Option::is_none")]
    pub open_api_keys: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
}

/// `ServiceReplicaScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceReplicaScalingPatchRequest {
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
}

/// `ServiceScalingPatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceScalingPatchRequest {
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_total_memory_gb: Option<f64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_total_memory_gb: Option<f64>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
}

/// `ServiceScalingPatchResponse` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceScalingPatchResponse {
    #[serde(
        rename = "availablePrivateEndpointIds",
        skip_serializing_if = "Option::is_none"
    )]
    pub available_private_endpoint_ids: Option<Vec<String>>,
    #[serde(rename = "autoscalingMode", skip_serializing_if = "Option::is_none")]
    pub autoscaling_mode: Option<AutoscalingMode>,
    #[serde(rename = "byocId", skip_serializing_if = "Option::is_none")]
    pub byoc_id: Option<String>,
    #[serde(rename = "clickhouseVersion", skip_serializing_if = "Option::is_none")]
    pub clickhouse_version: Option<String>,
    #[serde(rename = "complianceType", skip_serializing_if = "Option::is_none")]
    pub compliance_type: Option<ServiceScalingPatchResponseCompliancetype>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "currentScaling", skip_serializing_if = "Option::is_none")]
    pub current_scaling: Option<CurrentScaling>,
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<String>,
    #[serde(rename = "enableCoreDumps", skip_serializing_if = "Option::is_none")]
    pub enable_core_dumps: Option<bool>,
    #[serde(
        rename = "encryptionAssumedRoleIdentifier",
        skip_serializing_if = "Option::is_none"
    )]
    pub encryption_assumed_role_identifier: Option<String>,
    #[serde(rename = "encryptionKey", skip_serializing_if = "Option::is_none")]
    pub encryption_key: Option<String>,
    #[serde(rename = "encryptionRoleId", skip_serializing_if = "Option::is_none")]
    pub encryption_role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<ServiceEndpoint>>,
    #[serde(
        rename = "hasTransparentDataEncryption",
        skip_serializing_if = "Option::is_none"
    )]
    pub has_transparent_data_encryption: Option<bool>,
    #[serde(rename = "iamRole", skip_serializing_if = "Option::is_none")]
    pub iam_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "idleScaling", skip_serializing_if = "Option::is_none")]
    pub idle_scaling: Option<bool>,
    #[serde(rename = "idleTimeoutMinutes", skip_serializing_if = "Option::is_none")]
    pub idle_timeout_minutes: Option<f64>,
    #[serde(rename = "ipAccessList", skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<IpAccessListEntryResponse>>,
    #[serde(rename = "isPrimary", skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    #[serde(rename = "isReadonly", skip_serializing_if = "Option::is_none")]
    pub is_readonly: Option<bool>,
    #[serde(rename = "maxReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_replica_memory_gb: Option<f64>,
    #[serde(rename = "maxReplicas", skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "maxTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub max_total_memory_gb: Option<f64>,
    #[serde(rename = "minReplicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_replica_memory_gb: Option<f64>,
    #[serde(rename = "minReplicas", skip_serializing_if = "Option::is_none")]
    pub min_replicas: Option<i64>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(rename = "minTotalMemoryGb", skip_serializing_if = "Option::is_none")]
    pub min_total_memory_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "numReplicas", skip_serializing_if = "Option::is_none")]
    pub num_replicas: Option<i64>,
    #[serde(rename = "privateEndpointIds", skip_serializing_if = "Option::is_none")]
    pub private_endpoint_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ServiceScalingPatchResponseProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ServiceScalingPatchResponseProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<ServiceScalingPatchResponseRegion>,
    #[serde(rename = "releaseChannel", skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ServiceScalingPatchResponseReleasechannel>,
    #[serde(rename = "replicaMemoryGb", skip_serializing_if = "Option::is_none")]
    pub replica_memory_gb: Option<f64>,
    #[serde(rename = "scalingSchedule", skip_serializing_if = "Option::is_none")]
    pub scaling_schedule: Option<ScalingSchedule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ServiceScalingPatchResponseState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<ResourceTagsV1Response>>,
    #[cfg(feature = "deprecated-fields")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<ServiceScalingPatchResponseTier>,
    #[serde(
        rename = "transparentDataEncryptionKeyId",
        skip_serializing_if = "Option::is_none"
    )]
    pub transparent_data_encryption_key_id: Option<String>,
}

/// `ServiceStatePatchRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ServiceStatePatchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<ServiceStatePatchRequestCommand>,
}

/// `UpgradeWindow` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpgradeWindow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<i64>,
    #[serde(rename = "startHourUtc", skip_serializing_if = "Option::is_none")]
    pub start_hour_utc: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekday: Option<i64>,
}

/// `UpgradeWindowPutRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpgradeWindowPutRequest {
    #[serde(rename = "startHourUtc")]
    pub start_hour_utc: i64,
    pub weekday: i64,
}

/// `UpdateReversePrivateEndpoint` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UpdateReversePrivateEndpoint {
    #[serde(
        rename = "customPrivateDnsMappings",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_private_dns_mappings: Option<Vec<CustomPrivateDnsMapping>>,
}

/// `UsageCost` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCost {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub costs: Option<Vec<UsageCostRecord>>,
    #[serde(rename = "grandTotalCHC", skip_serializing_if = "Option::is_none")]
    pub grand_total_chc: Option<f64>,
}

/// `UsageCostMetrics` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCostMetrics {
    #[serde(rename = "backupCHC", skip_serializing_if = "Option::is_none")]
    pub backup_chc: Option<f64>,
    #[serde(rename = "computeCHC", skip_serializing_if = "Option::is_none")]
    pub compute_chc: Option<f64>,
    #[serde(rename = "dataTransferCHC", skip_serializing_if = "Option::is_none")]
    pub data_transfer_chc: Option<f64>,
    #[serde(rename = "initialLoadCHC", skip_serializing_if = "Option::is_none")]
    pub initial_load_chc: Option<f64>,
    #[serde(
        rename = "interRegionTier1DataTransferCHC",
        skip_serializing_if = "Option::is_none"
    )]
    pub inter_region_tier1_data_transfer_chc: Option<f64>,
    #[serde(
        rename = "interRegionTier2DataTransferCHC",
        skip_serializing_if = "Option::is_none"
    )]
    pub inter_region_tier2_data_transfer_chc: Option<f64>,
    #[serde(
        rename = "interRegionTier3DataTransferCHC",
        skip_serializing_if = "Option::is_none"
    )]
    pub inter_region_tier3_data_transfer_chc: Option<f64>,
    #[serde(
        rename = "interRegionTier4DataTransferCHC",
        skip_serializing_if = "Option::is_none"
    )]
    pub inter_region_tier4_data_transfer_chc: Option<f64>,
    #[serde(
        rename = "publicDataTransferCHC",
        skip_serializing_if = "Option::is_none"
    )]
    pub public_data_transfer_chc: Option<f64>,
    #[serde(rename = "storageCHC", skip_serializing_if = "Option::is_none")]
    pub storage_chc: Option<f64>,
}

/// `UsageCostRecord` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageCostRecord {
    #[serde(rename = "dataWarehouseId", skip_serializing_if = "Option::is_none")]
    pub data_warehouse_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(rename = "entityId", skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<uuid::Uuid>,
    #[serde(rename = "entityName", skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
    #[serde(rename = "entityType", skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<UsageCostRecordEntitytype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<UsageCostMetrics>,
    #[serde(rename = "serviceId", skip_serializing_if = "Option::is_none")]
    pub service_id: Option<uuid::Uuid>,
    #[serde(rename = "totalCHC", skip_serializing_if = "Option::is_none")]
    pub total_chc: Option<f64>,
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

/// Standard API response wrapper.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestId")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Default for BackupBucket {
    fn default() -> Self {
        // Every field of a response variant is `Option<T>`, so the derived
        // `AwsBackupBucket::default()` leaves `bucketProvider` absent and
        // serializes to `{}` — which deserializes back through the
        // discriminator dispatch as `Unknown`, not as this variant. Naming the
        // variant's own wire value keeps the default round-tripping.
        Self::AwsBackupBucket(AwsBackupBucket {
            bucket_provider: Some(AwsBackupBucketBucketprovider::default()),
            ..AwsBackupBucket::default()
        })
    }
}

impl Default for BackupBucketPatchRequest {
    fn default() -> Self {
        Self::AwsBackupBucketPatchRequestV1(AwsBackupBucketPatchRequestV1::default())
    }
}

impl Default for BackupBucketPostRequest {
    fn default() -> Self {
        Self::AwsBackupBucketPostRequestV1(AwsBackupBucketPostRequestV1::default())
    }
}

impl Default for BackupBucketProperties {
    fn default() -> Self {
        Self::AwsBackupBucketProperties(AwsBackupBucketProperties::default())
    }
}

impl Default for ClickStackAlertChannel {
    fn default() -> Self {
        Self::ClickStackAlertChannelEmail(ClickStackAlertChannelEmail::default())
    }
}

impl Default for ClickStackBarChartConfig {
    fn default() -> Self {
        Self::ClickStackBarBuilderChartConfig(ClickStackBarBuilderChartConfig::default())
    }
}

impl Default for ClickStackDashboardChartSeries {
    fn default() -> Self {
        Self::ClickStackTimeChartSeries(ClickStackTimeChartSeries::default())
    }
}

impl Default for ClickStackLineChartConfig {
    fn default() -> Self {
        Self::ClickStackLineBuilderChartConfig(ClickStackLineBuilderChartConfig::default())
    }
}

impl Default for ClickStackNumberChartConfig {
    fn default() -> Self {
        Self::ClickStackNumberBuilderChartConfig(ClickStackNumberBuilderChartConfig::default())
    }
}

impl Default for ClickStackPieChartConfig {
    fn default() -> Self {
        Self::ClickStackPieBuilderChartConfig(ClickStackPieBuilderChartConfig::default())
    }
}

impl Default for ClickStackSource {
    fn default() -> Self {
        Self::ClickStackLogSource(ClickStackLogSource::default())
    }
}

impl Default for ClickStackTableChartConfig {
    fn default() -> Self {
        Self::ClickStackTableBuilderChartConfig(ClickStackTableBuilderChartConfig::default())
    }
}

impl Default for ClickStackTileConfig {
    fn default() -> Self {
        Self::ClickStackLineChartConfig(ClickStackLineChartConfig::default())
    }
}

impl Default for ClickStackWebhook {
    fn default() -> Self {
        // Every field of this response-only union's variants is `Option<T>`,
        // so the derived `ClickStackSlackWebhook::default()` leaves `service`
        // absent and serializes to `{}` — which deserializes back through the
        // discriminator dispatch as `Unknown`, not as this variant. Naming the
        // variant's own wire value keeps the default round-tripping.
        Self::ClickStackSlackWebhook(ClickStackSlackWebhook {
            service: Some(ClickStackSlackWebhookService::default()),
            ..ClickStackSlackWebhook::default()
        })
    }
}
