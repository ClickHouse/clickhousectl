use super::Pagination;
use serde::{Deserialize, Serialize};

/// Values of `PublicQueryApiEndpoint.ownerType` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PublicQueryApiEndpointOwnertype {
    #[serde(rename = "user")]
    #[default]
    User,
    #[serde(rename = "queryApiEndpoint")]
    QueryApiEndpoint,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PublicQueryApiEndpointOwnertype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::QueryApiEndpoint => write!(f, "queryApiEndpoint"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `PublicQueryApiEndpoint` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PublicQueryApiEndpoint {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "sql", skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    #[serde(rename = "database", skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(rename = "parameters", skip_serializing_if = "Option::is_none")]
    pub parameters: Option<std::collections::BTreeMap<String, String>>,
    #[serde(rename = "apiKeyIds", skip_serializing_if = "Option::is_none")]
    pub api_key_ids: Option<Vec<uuid::Uuid>>,
    #[serde(rename = "roles", skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(rename = "allowedOrigins", skip_serializing_if = "Option::is_none")]
    pub allowed_origins: Option<Vec<String>>,
    #[serde(rename = "url", skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "ownerType", skip_serializing_if = "Option::is_none")]
    pub owner_type: Option<PublicQueryApiEndpointOwnertype>,
}

/// Values of `PublicQueryApiEndpointListItem.ownerType` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum PublicQueryApiEndpointListItemOwnertype {
    #[serde(rename = "user")]
    #[default]
    User,
    #[serde(rename = "queryApiEndpoint")]
    QueryApiEndpoint,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for PublicQueryApiEndpointListItemOwnertype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::QueryApiEndpoint => write!(f, "queryApiEndpoint"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `PublicQueryApiEndpointListItem` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PublicQueryApiEndpointListItem {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<uuid::Uuid>,
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "database", skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(rename = "apiKeyIds", skip_serializing_if = "Option::is_none")]
    pub api_key_ids: Option<Vec<uuid::Uuid>>,
    #[serde(rename = "roles", skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(rename = "allowedOrigins", skip_serializing_if = "Option::is_none")]
    pub allowed_origins: Option<Vec<String>>,
    #[serde(rename = "url", skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "ownerType", skip_serializing_if = "Option::is_none")]
    pub owner_type: Option<PublicQueryApiEndpointListItemOwnertype>,
}

/// `PublicQueryApiEndpointRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PublicQueryApiEndpointRequest {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "sql")]
    pub sql: String,
    #[serde(rename = "database")]
    pub database: String,
    #[serde(rename = "parameters", skip_serializing_if = "Option::is_none")]
    pub parameters: Option<std::collections::BTreeMap<String, String>>,
    #[serde(rename = "apiKeyIds")]
    pub api_key_ids: Vec<uuid::Uuid>,
    #[serde(rename = "roles")]
    pub roles: Vec<String>,
    #[serde(rename = "allowedOrigins", skip_serializing_if = "Option::is_none")]
    pub allowed_origins: Option<Vec<String>>,
}

/// `QueryApiEndpointListResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QueryApiEndpointListResponse {
    #[serde(rename = "items", skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<PublicQueryApiEndpointListItem>>,
    #[serde(rename = "pagination", skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
}
