use super::enums::*;
use serde::{Deserialize, Serialize};

/// `ClickStackCASLPermission` - one of multiple variants.
///
/// Dispatched on the `integration` field, which each variant requires with its
/// own single wire value; see the `discriminated_union!` invocation below.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackCASLPermission {
    ClickStackMongoDBCASLPermission(ClickStackMongoDBCASLPermission),
    ClickStackClickHouseCASLPermission(ClickStackClickHouseCASLPermission),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackCASLPermission, "integration" {
        "mongodb" => ClickStackMongoDBCASLPermission,
        "clickhouse" => ClickStackClickHouseCASLPermission,
    }
}

impl std::fmt::Display for ClickStackCASLPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackMongoDBCASLPermission(_) => {
                write!(f, "ClickStackMongoDBCASLPermission")
            }
            Self::ClickStackClickHouseCASLPermission(_) => {
                write!(f, "ClickStackClickHouseCASLPermission")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackCASLPermission` - one of multiple variants, in response position.
///
/// Response variant of [`ClickStackCASLPermission`]: each arm is the all-`Option`
/// response variant of its request type, so a field the API drops or sends as
/// `null` deserializes to `None` instead of failing.
///
/// Dispatched on the `integration` field, exactly as the request union is:
/// dispatch reads the raw JSON rather than trying each variant's shape, so the
/// all-`Option` arms — which would match any object under `untagged` matching —
/// cannot misroute a payload. An `integration` this crate does not know, a
/// payload without one, or a payload that does not fit the variant its
/// `integration` selects, lands in `Unknown` with the raw JSON intact.
///
/// Deliberately has no `Default`: every arm's default would serialize to `{}`,
/// which carries no `integration` and so would not deserialize back to the
/// same variant. Build a [`ClickStackCASLPermission`] instead when writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ClickStackCASLPermissionResponse {
    ClickStackMongoDBCASLPermission(ClickStackMongoDBCASLPermissionResponse),
    ClickStackClickHouseCASLPermission(ClickStackClickHouseCASLPermissionResponse),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    ClickStackCASLPermissionResponse, "integration" {
        "mongodb" => ClickStackMongoDBCASLPermission,
        "clickhouse" => ClickStackClickHouseCASLPermission,
    }
}

impl std::fmt::Display for ClickStackCASLPermissionResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClickStackMongoDBCASLPermission(_) => {
                write!(f, "ClickStackMongoDBCASLPermission")
            }
            Self::ClickStackClickHouseCASLPermission(_) => {
                write!(f, "ClickStackClickHouseCASLPermission")
            }
            Self::Unknown(s) => write!(f, "{s}"),
        }
    }
}

/// `ClickStackMongoDBCASLPermissionConditions` from the ClickHouse Cloud API.
///
/// A MongoDB-style query keyed by document field name. Each value is either a
/// string to match exactly or a `$regex` pattern.
pub type ClickStackMongoDBCASLPermissionConditions =
    std::collections::BTreeMap<String, ClickStackMongoDBCASLPermissionConditionValue>;

/// `ClickStackMongoDBCASLPermissionConditions` from the ClickHouse Cloud API, in response position.
///
/// Tolerant response map: keeps every field name, and each value as raw JSON
/// so a value shape the API adds later is preserved without coercion.
pub type ClickStackMongoDBCASLPermissionConditionsResponse =
    std::collections::BTreeMap<String, serde_json::Value>;

/// One value of [`ClickStackMongoDBCASLPermissionConditions`]: the spec's
/// `oneOf` of a string and a `$regex` object.
///
/// The two shapes are disjoint JSON types (a string and an object), so
/// `untagged` matching cannot route a value to the wrong variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClickStackMongoDBCASLPermissionConditionValue {
    String(String),
    Regex(ClickStackMongoDBCASLPermissionRegexCondition),
}

/// `ClickStackClickHouseCASLPermission` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermission {
    pub action: ClickStackCASLPermissionAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ClickStackClickHouseCASLPermissionConditions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    pub integration: ClickStackClickHouseCASLPermissionIntegration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    pub subject: ClickStackCASLPermissionSubject,
}

/// `ClickStackClickHouseCASLPermission` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackClickHouseCASLPermission`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermissionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ClickStackCASLPermissionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ClickStackClickHouseCASLPermissionConditionsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<ClickStackClickHouseCASLPermissionIntegration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<ClickStackCASLPermissionSubject>,
}

/// `ClickStackClickHouseCASLPermissionConditions` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermissionConditions {
    pub connection: String,
    pub from: ClickStackClickHouseCASLPermissionConditionsFrom,
    #[serde(rename = "where")]
    pub r#where: String,
    #[serde(rename = "whereLanguage")]
    pub where_language: ClickStackClickHouseCASLPermissionConditionsWherelanguage,
}

/// `ClickStackClickHouseCASLPermissionConditions` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackClickHouseCASLPermissionConditions`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermissionConditionsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ClickStackClickHouseCASLPermissionConditionsFromResponse>,
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(rename = "whereLanguage", skip_serializing_if = "Option::is_none")]
    pub where_language: Option<ClickStackClickHouseCASLPermissionConditionsWherelanguage>,
}

/// `ClickStackClickHouseCASLPermissionConditionsFrom` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermissionConditionsFrom {
    #[serde(rename = "databaseName")]
    pub database_name: String,
    #[serde(rename = "tableName")]
    pub table_name: String,
}

/// `ClickStackClickHouseCASLPermissionConditionsFrom` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackClickHouseCASLPermissionConditionsFrom`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackClickHouseCASLPermissionConditionsFromResponse {
    #[serde(rename = "databaseName", skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(rename = "tableName", skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
}

/// `ClickStackCreateRoleRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackCreateRoleRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub name: String,
    pub permissions: Vec<ClickStackCASLPermission>,
}

/// `ClickStackMongoDBCASLPermission` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMongoDBCASLPermission {
    pub action: ClickStackCASLPermissionAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ClickStackMongoDBCASLPermissionConditions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    pub integration: ClickStackMongoDBCASLPermissionIntegration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    pub subject: ClickStackCASLPermissionSubject,
}

/// `ClickStackMongoDBCASLPermission` from the ClickHouse Cloud API, in response position.
///
/// Response variant of [`ClickStackMongoDBCASLPermission`]: every field is
/// `Option<T>`, so a field the API drops or sends as `null` deserializes to
/// `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMongoDBCASLPermissionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ClickStackCASLPermissionAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<ClickStackMongoDBCASLPermissionConditionsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<ClickStackMongoDBCASLPermissionIntegration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<ClickStackCASLPermissionSubject>,
}

/// The `$regex` object branch of a [`ClickStackMongoDBCASLPermissionConditionValue`].
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackMongoDBCASLPermissionRegexCondition {
    #[serde(rename = "$regex")]
    pub regex: String,
    #[serde(rename = "$options", skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
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

/// `ClickStackUpdateRoleRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackUpdateRoleRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub permissions: Vec<ClickStackCASLPermission>,
}
