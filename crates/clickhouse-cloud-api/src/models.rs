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
/// New discriminated unions in the model tree should use this macro rather than
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
mod clickstack;
mod clickstack_enums;
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
    ByocAvailabilityZoneSuffix, ByocConfig, ByocConfigCloudprovider, ByocConfigRegionid,
    ByocConfigState, ByocInfrastructurePatchRequest, ByocInfrastructurePostRequest,
    ByocInfrastructurePostRequestRegionid,
};
pub use clickpipes::*;
pub use clickstack::*;
pub use clickstack_enums::*;
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
pub use organizations::{
    ActiveBalance, ActiveBalances, Organization, OrganizationPatchRequest,
    PrometheusDiscoveryLabels, PrometheusDiscoveryTargetGroup,
};
pub use postgres::{
    BasePostgresService, PgBouncerConfig, PgBouncerConfigResponse, PgConfig,
    PgConfigDefaultTransactionIsolation, PgConfigResponse, PgConfigSslMinProtocolVersion,
    PgConfigWalCompression, PgCreatedAtProperty, PgHaType, PgIdProperty, PgIsPrimaryProperty,
    PgNameProperty, PgPassword, PgPitrRestoreTargetProperty, PgProvider, PgRegion, PgSize,
    PgStateProperty, PgStorageSize, PgTags, PgTagsResponse, PgVersion, PostgresInstanceConfig,
    PostgresInstanceConfigResponse, PostgresInstanceUpdateConfigResponse, PostgresLogEntry,
    PostgresLogsGetListSortorder, PostgresMetric, PostgresMetricDataPoint, PostgresMetricSeries,
    PostgresMetrics, PostgresQueryExecution, PostgresService, PostgresServiceListItem,
    PostgresServicePasswordResource, PostgresServicePatchRequest, PostgresServicePostRequest,
    PostgresServiceReadReplicaRequest, PostgresServiceRestoreRequest, PostgresServiceSetPassword,
    PostgresServiceSetState, PostgresServiceSetStateCommand, PostgresSlowQueryPattern,
    PostgresSlowQueryPatternDetail, SlowQueryPatternsGetListSortby,
    SlowQueryPatternsGetListSortorder,
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
    PrivateEndpointConfig, QueryEndpointRole, ScalingSchedule, ScalingScheduleBaseConfig,
    ScalingScheduleEntry, ScalingScheduleEntryRequest, ScalingSchedulePostRequest,
    ServicPrivateEndpointePostRequest, Service, ServiceClickhouseSetting,
    ServiceClickhouseSettingSchemaEntry, ServiceClickhouseSettingWarning,
    ServiceClickhouseSettingsList, ServiceClickhouseSettingsPatchRequest,
    ServiceClickhouseSettingsPatchResponse, ServiceClickhouseSettingsSchema, ServiceCompliancetype,
    ServiceEndpoint, ServiceEndpointChange, ServiceEndpointChangeProtocol, ServiceEndpointProtocol,
    ServicePasswordPatchRequest, ServicePasswordPatchResponse, ServicePatchRequest,
    ServicePatchRequestReleasechannel, ServicePostRequest, ServicePostRequestCompliancetype,
    ServicePostRequestProfile, ServicePostRequestProvider, ServicePostRequestRegion,
    ServicePostRequestReleasechannel, ServicePostRequestTier, ServicePostResponse, ServiceProfile,
    ServiceProvider, ServiceQueryAPIEndpoint, ServiceRegion, ServiceReleasechannel,
    ServiceReplicaScalingPatchRequest, ServiceScalingPatchRequest, ServiceScalingPatchResponse,
    ServiceScalingPatchResponseCompliancetype, ServiceScalingPatchResponseProfile,
    ServiceScalingPatchResponseProvider, ServiceScalingPatchResponseRegion,
    ServiceScalingPatchResponseReleasechannel, ServiceScalingPatchResponseState,
    ServiceScalingPatchResponseTier, ServiceState, ServiceStatePatchRequest,
    ServiceStatePatchRequestCommand, ServiceTier, UpgradeWindow, UpgradeWindowDuration,
    UpgradeWindowPutRequest, UpgradeWindowStartHourUtc, UsageCost, UsageCostMetrics,
    UsageCostRecord, UsageCostRecordEntitytype,
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
