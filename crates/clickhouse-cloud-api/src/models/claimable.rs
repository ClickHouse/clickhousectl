use serde::Deserialize;

/// Resource limits for a claimable organisation. The shape is informative
/// and subject to change; kept as free-form JSON for now.
pub type ClaimableOrganizationLimits = serde_json::Value;

/// The API key pair returned by `POST /v1/claimable`.
///
/// The key secret is a write-once value: the CLI persists it to the
/// credentials file and must never log, print or store it elsewhere.
/// `Serialize` is intentionally absent — a serialization that could
/// accidentally emit `key_secret` to stdout would be a silent data-exposure
/// bug.
#[derive(Debug, Clone, Deserialize)]
pub struct ClaimableOrganizationApiKey {
    #[serde(rename = "keyId", skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    #[serde(rename = "keySecret", skip_serializing_if = "Option::is_none")]
    pub key_secret: Option<String>,
}

/// The result body of `POST /v1/claimable`.
#[derive(Debug, Clone, Deserialize)]
pub struct ClaimableOrganization {
    #[serde(rename = "organizationId", skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<ClaimableOrganizationApiKey>,
    #[serde(rename = "expiresAt", skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<jiff::Timestamp>,
    #[serde(rename = "limits", skip_serializing_if = "Option::is_none")]
    pub limits: Option<ClaimableOrganizationLimits>,
}
