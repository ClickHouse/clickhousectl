use super::enums::*;
use serde::{Deserialize, Serialize};

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

/// Type alias for `ClickStackValidateWebhookResponseNormalized`.
pub type ClickStackValidateWebhookResponseNormalized = serde_json::Value;

/// Type alias for `ClickStackWebhookInputHeaders`.
pub type ClickStackWebhookInputHeaders = std::collections::BTreeMap<String, String>;

/// Type alias for `ClickStackWebhookInputQueryParams`.
pub type ClickStackWebhookInputQueryParams = std::collections::BTreeMap<String, String>;

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

/// `ClickStackValidateWebhookResponse` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackValidateWebhookResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ClickStackValidateWebhookResponseErrorsItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized: Option<ClickStackValidateWebhookResponseNormalized>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid: Option<bool>,
}

/// `ClickStackValidateWebhookResponseErrorsItem` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClickStackValidateWebhookResponseErrorsItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
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
