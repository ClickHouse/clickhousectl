use serde::{Deserialize, Serialize};

/// Values of `WhoamiUser.actorType` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum WhoamiUserActortype {
    #[serde(rename = "user")]
    #[default]
    User,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for WhoamiUserActortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// Values of `WhoamiApiKey.actorType` in the Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum WhoamiApiKeyActortype {
    #[serde(rename = "apiKey")]
    #[default]
    ApiKey,
    /// Catch-all for unknown or newly-added values.
    #[serde(untagged)]
    Unknown(String),
}

impl std::fmt::Display for WhoamiApiKeyActortype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey => write!(f, "apiKey"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}

/// `WhoamiOrganization` from the ClickHouse Cloud API: an organization the
/// calling user belongs to.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WhoamiOrganization {
    #[serde(rename = "organizationId", skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<uuid::Uuid>,
    #[serde(rename = "organizationName", skip_serializing_if = "Option::is_none")]
    pub organization_name: Option<String>,
}

/// `WhoamiUser` from the ClickHouse Cloud API: the caller is a user
/// (OAuth or JWT).
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WhoamiUser {
    #[serde(rename = "actorType", skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<WhoamiUserActortype>,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<uuid::Uuid>,
    #[serde(rename = "email", skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Organizations the user belongs to.
    #[serde(rename = "organizations", skip_serializing_if = "Option::is_none")]
    pub organizations: Option<Vec<WhoamiOrganization>>,
}

/// `WhoamiApiKey` from the ClickHouse Cloud API: the caller is an
/// organization API key.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WhoamiApiKey {
    #[serde(rename = "actorType", skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<WhoamiApiKeyActortype>,
    #[serde(rename = "keyId", skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Organization that owns the API key.
    #[serde(rename = "organizationId", skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<uuid::Uuid>,
}

/// `Whoami` from the ClickHouse Cloud API: the authenticated caller, selected
/// by `actorType`.
///
/// An absent or unrecognized `actorType` keeps the payload verbatim in
/// `Unknown` rather than guessing a variant.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Whoami {
    WhoamiUser(WhoamiUser),
    WhoamiApiKey(WhoamiApiKey),
    /// Catch-all for unknown or newly-added values.
    ///
    /// Holds the raw payload as `serde_json::Value` so it round-trips
    /// losslessly; its `Display` emits the payload as compact JSON.
    Unknown(serde_json::Value),
}

discriminated_union! {
    Whoami, "actorType" {
        "user" => WhoamiUser,
        "apiKey" => WhoamiApiKey,
    }
}

impl std::fmt::Display for Whoami {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WhoamiUser(_) => write!(f, "WhoamiUser"),
            Self::WhoamiApiKey(_) => write!(f, "WhoamiApiKey"),
            Self::Unknown(value) => write!(f, "{value}"),
        }
    }
}
