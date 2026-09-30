use serde::{Deserialize, Serialize};

/// `PublicSavedQuery` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PublicSavedQuery {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "sql", skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    #[serde(rename = "database", skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    /// Query parameters, keyed by parameter name.
    #[serde(rename = "parameters", skip_serializing_if = "Option::is_none")]
    pub parameters: Option<std::collections::BTreeMap<String, String>>,
}

/// `PublicSavedQueryListItem` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PublicSavedQueryListItem {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "database", skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
}

/// `PublicSavedQueryRequest` from the ClickHouse Cloud API.
///
/// Body of both `saved_query_create` and `saved_query_update`; an update
/// replaces the whole saved query.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PublicSavedQueryRequest {
    /// Name of the saved query; must be non-empty and unique within the service.
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "sql")]
    pub sql: String,
    #[serde(rename = "database")]
    pub database: String,
    /// Default query parameters. The API treats an omitted map as empty.
    #[serde(rename = "parameters", skip_serializing_if = "Option::is_none")]
    pub parameters: Option<std::collections::BTreeMap<String, String>>,
}
