use serde::{Deserialize, Serialize};
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
