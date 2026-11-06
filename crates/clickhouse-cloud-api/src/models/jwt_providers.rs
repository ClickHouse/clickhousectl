use serde::{Deserialize, Serialize};

/// `JwtProvider` from the ClickHouse Cloud API.
///
/// Used in response position only: every field is `Option<T>`, so a field the
/// API drops or sends as `null` deserializes to `None` instead of failing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct JwtProvider {
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "issuer", skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
    #[serde(rename = "audience", skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
    #[serde(rename = "jwksUrl", skip_serializing_if = "Option::is_none")]
    pub jwks_url: Option<String>,
    /// Absent when no roles claim is configured.
    #[serde(rename = "rolesClaim", skip_serializing_if = "Option::is_none")]
    pub roles_claim: Option<String>,
}

/// `JwtProviderPostRequest` from the ClickHouse Cloud API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct JwtProviderPostRequest {
    /// Unique within the service: lowercase letters, digits and hyphens, at
    /// most 59 characters.
    #[serde(rename = "name")]
    pub name: String,
    /// Expected `iss` claim of the tokens.
    #[serde(rename = "issuer")]
    pub issuer: String,
    /// Expected `aud` claim of the tokens.
    #[serde(rename = "audience")]
    pub audience: String,
    /// Public `https://` JSON Web Key Set URL with at least one RS256 key.
    #[serde(rename = "jwksUrl")]
    pub jwks_url: String,
    /// Token claim listing the ClickHouse roles to grant. `None` configures no
    /// roles claim.
    #[serde(rename = "rolesClaim", skip_serializing_if = "Option::is_none")]
    pub roles_claim: Option<String>,
}

/// `JwtProviderPutRequest` from the ClickHouse Cloud API.
///
/// Replaces the whole provider configuration; the name is the path parameter.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct JwtProviderPutRequest {
    /// Expected `iss` claim of the tokens.
    #[serde(rename = "issuer")]
    pub issuer: String,
    /// Expected `aud` claim of the tokens.
    #[serde(rename = "audience")]
    pub audience: String,
    /// Public `https://` JSON Web Key Set URL with at least one RS256 key.
    #[serde(rename = "jwksUrl")]
    pub jwks_url: String,
    /// Token claim listing the ClickHouse roles to grant. `None` removes an
    /// existing roles claim.
    #[serde(rename = "rolesClaim", skip_serializing_if = "Option::is_none")]
    pub roles_claim: Option<String>,
}
