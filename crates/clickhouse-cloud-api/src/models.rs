//! Typed models for ClickHouse Cloud API schemas.
//!
//! Derived from the OpenAPI specification and kept in step with it by the drift
//! analyzer, which parses the private module tree rooted at this facade. Every
//! model struct, enum and type alias remains literal source in that tree.
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

mod activity;
mod api_keys;
mod backups;
mod byoc;
mod clickpipes;
mod invitations;
mod members;
mod organization_private_endpoints;
mod organizations;
mod postgres;
mod quotas;
mod rbac;
mod scim;
mod services;
mod shared;
mod udfs;

pub use activity::{Activity, ActivityActortype, ActivityKeyupdatetype, ActivityType};
pub use api_keys::{
    ApiKey, ApiKeyHashData, ApiKeyPatchRequest, ApiKeyPatchRequestState, ApiKeyPostRequest,
    ApiKeyPostRequestState, ApiKeyPostResponse, ApiKeyState,
};
pub use backups::{
    AwsBackupBucket, AwsBackupBucketBucketprovider, AwsBackupBucketPatchRequestV1,
    AwsBackupBucketPatchRequestV1Bucketprovider, AwsBackupBucketPostRequestV1,
    AwsBackupBucketPostRequestV1Bucketprovider, AwsBackupBucketProperties,
    AwsBackupBucketPropertiesBucketprovider, AzureBackupBucket, AzureBackupBucketBucketprovider,
    AzureBackupBucketPatchRequestV1, AzureBackupBucketPatchRequestV1Bucketprovider,
    AzureBackupBucketPostRequestV1, AzureBackupBucketPostRequestV1Bucketprovider,
    AzureBackupBucketProperties, AzureBackupBucketPropertiesBucketprovider, Backup, BackupBucket,
    BackupBucketPatchRequest, BackupBucketPostRequest, BackupBucketProperties, BackupConfiguration,
    BackupConfigurationPatchRequest, BackupStatus, BackupType, GcpBackupBucket,
    GcpBackupBucketBucketprovider, GcpBackupBucketPatchRequestV1,
    GcpBackupBucketPatchRequestV1Bucketprovider, GcpBackupBucketPostRequestV1,
    GcpBackupBucketPostRequestV1Bucketprovider, GcpBackupBucketProperties,
    GcpBackupBucketPropertiesBucketprovider,
};
pub use byoc::{
    ByocConfig, ByocConfigCloudprovider, ByocConfigRegionid, ByocConfigState,
    ByocInfrastructurePatchRequest, ByocInfrastructurePostRequest,
    ByocInfrastructurePostRequestRegionid,
};
pub use clickpipes::*;
pub use invitations::{
    Invitation, InvitationPostRequest, InvitationPostRequestRole, InvitationRole,
};
pub use members::{Member, MemberPatchRequest, MemberPatchRequestRole, MemberRole};
pub use organization_private_endpoints::{
    OrganizationCloudRegionPrivateEndpointConfig, OrganizationPatchPrivateEndpoint,
    OrganizationPatchPrivateEndpointCloudprovider, OrganizationPatchPrivateEndpointRegion,
    OrganizationPrivateEndpoint, OrganizationPrivateEndpointCloudprovider,
    OrganizationPrivateEndpointRegion, OrganizationPrivateEndpointsPatch,
};
pub use organizations::{Organization, OrganizationPatchRequest};
pub use postgres::{
    BasePostgresService, PgBouncerConfig, PgBouncerConfigResponse, PgConfig,
    PgConfigDefaultTransactionIsolation, PgConfigResponse, PgConfigSslMinProtocolVersion,
    PgConfigWalCompression, PgCreatedAtProperty, PgHaType, PgIdProperty, PgIsPrimaryProperty,
    PgNameProperty, PgPassword, PgPitrRestoreTargetProperty, PgProvider, PgRegion, PgSize,
    PgStateProperty, PgStorageSize, PgTags, PgTagsResponse, PgVersion, PostgresInstanceConfig,
    PostgresInstanceConfigResponse, PostgresInstanceUpdateConfigResponse, PostgresMetric,
    PostgresMetricDataPoint, PostgresMetricSeries, PostgresMetrics, PostgresQueryExecution,
    PostgresService, PostgresServiceListItem, PostgresServicePasswordResource,
    PostgresServicePatchRequest, PostgresServicePostRequest, PostgresServiceReadReplicaRequest,
    PostgresServiceRestoreRequest, PostgresServiceSetPassword, PostgresServiceSetState,
    PostgresServiceSetStateCommand, PostgresSlowQueryPattern, PostgresSlowQueryPatternDetail,
};
pub use quotas::{OrganizationQuota, OrganizationQuotaQuotacode, OrganizationQuotaScope};
pub use rbac::{
    RBACPolicy, RBACPolicyAllowdeny, RBACPolicyCreateRequest, RBACPolicyCreateRequestAllowdeny,
    RBACPolicyTags, RBACPolicyTagsResponse, RBACPolicyTagsRolev2, RBACRole, RBACRoleType,
    RoleCreateRequest, RoleUpdateRequest,
};
pub use scim::{
    ScimAuthenticationScheme, ScimBooleanFeature, ScimEnterpriseManager, ScimEnterpriseUser,
    ScimGroup, ScimGroupListResponse, ScimGroupMember, ScimGroupMeta, ScimGroupPostRequest,
    ScimGroupPutRequest, ScimListResponse, ScimPatchOp, ScimPatchOperation, ScimPatchOperationOp,
    ScimResourceType, ScimResourceTypeListResponse, ScimResourceTypeMeta, ScimSchema,
    ScimSchemaAttribute, ScimSchemaExtension, ScimSchemaListResponse, ScimSchemaMeta,
    ScimServiceProviderConfig, ScimServiceProviderConfigBulk, ScimServiceProviderConfigFilter,
    ScimServiceProviderConfigMeta, ScimServiceProviderConfigPatch, ScimUser, ScimUserAddress,
    ScimUserEmail, ScimUserEntitlement, ScimUserGroup, ScimUserIm, ScimUserMeta, ScimUserName,
    ScimUserPhoneNumber, ScimUserPhoto, ScimUserPostRequest, ScimUserPutRequest, ScimUserRole,
    ScimX509Certificate,
};
pub use services::{
    AutoscalingMode, CurrentScaling, CurrentScalingEffectiveautoscalingmode,
    InstancePrivateEndpoint, InstancePrivateEndpointCloudprovider, InstancePrivateEndpointRegion,
    InstancePrivateEndpointsPatch, InstanceServiceQueryApiEndpointsPostRequest, InstanceTagsPatch,
    PrivateEndpointConfig, ScalingSchedule, ScalingScheduleBaseConfig, ScalingScheduleEntry,
    ScalingScheduleEntryRequest, ScalingSchedulePostRequest, ServicPrivateEndpointePostRequest,
    Service, ServiceClickhouseSetting, ServiceClickhouseSettingSchemaEntry,
    ServiceClickhouseSettingWarning, ServiceClickhouseSettingsList,
    ServiceClickhouseSettingsPatchRequest, ServiceClickhouseSettingsPatchResponse,
    ServiceClickhouseSettingsSchema, ServiceCompliancetype, ServiceEndpoint, ServiceEndpointChange,
    ServiceEndpointChangeProtocol, ServiceEndpointProtocol, ServicePasswordPatchRequest,
    ServicePasswordPatchResponse, ServicePatchRequest, ServicePatchRequestReleasechannel,
    ServicePostRequest, ServicePostRequestCompliancetype, ServicePostRequestProfile,
    ServicePostRequestProvider, ServicePostRequestRegion, ServicePostRequestReleasechannel,
    ServicePostRequestTier, ServicePostResponse, ServiceProfile, ServiceProvider,
    ServiceQueryAPIEndpoint, ServiceRegion, ServiceReleasechannel,
    ServiceReplicaScalingPatchRequest, ServiceScalingPatchRequest, ServiceScalingPatchResponse,
    ServiceScalingPatchResponseCompliancetype, ServiceScalingPatchResponseProfile,
    ServiceScalingPatchResponseProvider, ServiceScalingPatchResponseRegion,
    ServiceScalingPatchResponseReleasechannel, ServiceScalingPatchResponseState,
    ServiceScalingPatchResponseTier, ServiceState, ServiceStatePatchRequest,
    ServiceStatePatchRequestCommand, ServiceTier, UpgradeWindow, UpgradeWindowPutRequest,
    UsageCost, UsageCostMetrics, UsageCostRecord, UsageCostRecordEntitytype,
};
pub use shared::{
    ApiResponse, AssignedRole, AssignedRoleRoletype, IpAccessListEntry, IpAccessListEntryResponse,
    IpAccessListPatch, License, ResourceTagsV1, ResourceTagsV1Response,
};
pub use udfs::{
    Pagination, Udf, UdfArgument, UdfArgumentResponse, UdfAttachment, UdfAttachmentListResponse,
    UdfAttachmentStatus, UdfCreateRequest, UdfCreateRequestV1, UdfCreateRequestV1Type,
    UdfCreateRequestV2, UdfCreateRequestV2Type, UdfListResponse, UdfRuntime, UdfSandboxType,
    UdfSandboxVersion, UdfStatus, UdfType, UdfUploadSession, UdfVersionCreateRequest,
    UdfVersionCreateRequestV1, UdfVersionCreateRequestV1Type, UdfVersionCreateRequestV2,
    UdfVersionCreateRequestV2Type, UdfVersionListResponse,
};

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
