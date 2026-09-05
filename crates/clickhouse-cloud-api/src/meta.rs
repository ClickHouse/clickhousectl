//! Operation stability metadata.
//!
//! `BETA_OPERATIONS` mirrors `x-badges` entries on operations in the ClickHouse
//! Cloud OpenAPI spec. The list is kept sorted so [`is_beta_operation`] can use
//! `binary_search` and snapshot diffs stay readable.
//!
//! Consumers — including this crate's own CLI — can use [`is_beta_operation`]
//! to render a "(Beta)" affordance derived from the spec rather than maintained
//! by hand.
//!
//! Regenerate from the snapshot with:
//!
//! ```text
//! python3 scripts/regenerate-beta-lists.py
//! ```
//!
//! The shared OpenAPI analyzer reports drift if this list differs from the
//! snapshot or live spec.

/// Snake-case operation IDs (matching [`crate::client::Client`] method names)
/// that the OpenAPI spec marks Beta via `x-badges`.
pub const BETA_OPERATIONS: &[&str] = &[
    "backup_bucket_create",
    "backup_bucket_delete",
    "backup_bucket_get",
    "backup_bucket_update",
    "click_pipe_schema_discovery",
    "click_pipes_service_context_get",
    "click_stack_create_alert",
    "click_stack_create_dashboard",
    "click_stack_create_role",
    "click_stack_create_saved_search",
    "click_stack_create_source",
    "click_stack_create_webhook",
    "click_stack_delete_alert",
    "click_stack_delete_dashboard",
    "click_stack_delete_role",
    "click_stack_delete_saved_search",
    "click_stack_delete_source",
    "click_stack_delete_webhook",
    "click_stack_get_alert",
    "click_stack_get_dashboard",
    "click_stack_get_role",
    "click_stack_get_saved_search",
    "click_stack_get_source",
    "click_stack_list_alerts",
    "click_stack_list_dashboards",
    "click_stack_list_roles",
    "click_stack_list_saved_searches",
    "click_stack_list_sources",
    "click_stack_list_webhooks",
    "click_stack_update_alert",
    "click_stack_update_dashboard",
    "click_stack_update_role",
    "click_stack_update_saved_search",
    "click_stack_update_source",
    "click_stack_update_webhook",
    "click_stack_validate_dashboard",
    "credit_balances_get",
    "organization_prometheus_discovery_get",
    "organization_quota_get",
    "organization_quotas_get_list",
    "postgres_instance_config_get",
    "postgres_instance_config_patch",
    "postgres_instance_config_post",
    "postgres_instance_create_read_replica",
    "postgres_instance_metrics_get",
    "postgres_instance_prometheus_get",
    "postgres_instance_restore",
    "postgres_logs_get_list",
    "postgres_org_prometheus_get",
    "postgres_service_certs_get",
    "postgres_service_create",
    "postgres_service_delete",
    "postgres_service_get",
    "postgres_service_get_list",
    "postgres_service_patch",
    "postgres_service_patch_state",
    "postgres_service_set_password",
    "scaling_schedule_delete",
    "scaling_schedule_get",
    "scaling_schedule_upsert",
    "service_clickhouse_setting_delete",
    "service_clickhouse_setting_get",
    "service_clickhouse_settings_list_get",
    "service_clickhouse_settings_schema_get",
    "service_clickhouse_settings_update",
    "slow_query_pattern_get",
    "slow_query_patterns_get_list",
    "udf_attach",
    "udf_attachment_get",
    "udf_attachment_list",
    "udf_create",
    "udf_delete",
    "udf_detach",
    "udf_get",
    "udf_list",
    "udf_upload_session_create",
    "udf_version_create",
    "udf_version_delete",
    "udf_version_list",
];

/// Returns `true` if `name` matches a client method backed by a Beta endpoint.
///
/// `name` is the snake-case method name (e.g. `"postgres_service_get_list"`).
pub fn is_beta_operation(name: &str) -> bool {
    BETA_OPERATIONS.binary_search(&name).is_ok()
}

/// Schema fields the OpenAPI spec marks `deprecated: true`, as
/// `(RustStructName, specFieldName)` pairs. Covers both response-side and
/// request-side schemas.
///
/// These fields are removed from the struct entirely unless the
/// `deprecated-fields` Cargo feature is enabled — each one carries a
/// `#[cfg(feature = "deprecated-fields")]` marker in [`crate::models`]. By
/// default the field does not exist, so:
///
/// - On response structs, referencing it is a compile error and it never
///   appears in serialized output; deserializing a payload that still contains
///   it just ignores the extra key.
/// - On request structs, callers cannot set it and `skip_serializing_if` keeps
///   it off the wire entirely.
///
/// The list is kept sorted so [`is_deprecated_field`] can use `binary_search`
/// and snapshot diffs stay readable.
///
/// Regenerate from the snapshot with:
///
/// ```text
/// python3 scripts/regenerate-deprecated-fields.py
/// ```
///
/// The script derives struct names from spec schema names alone, so a schema
/// modeled as both a request and a response type needs the `{Name}Response`
/// entry added by hand after regenerating (e.g. `ClickPipeScalingResponse`).
/// The analyzer expects the pair once per Rust type the schema maps to, so a
/// dropped response-variant entry fails the drift check rather than passing
/// silently.
///
/// The shared OpenAPI analyzer reports drift if this list differs from the
/// spec or if a field here lacks the `#[cfg(feature = "deprecated-fields")]`
/// marker in `models.rs` (or vice versa).
pub const DEPRECATED_FIELDS: &[(&str, &str)] = &[
    ("ApiKey", "roles"),
    ("ApiKeyPatchRequest", "roles"),
    ("ApiKeyPostRequest", "roles"),
    ("ClickPipeScaling", "concurrency"),
    ("ClickPipeScalingPatchRequest", "concurrency"),
    ("ClickPipeScalingResponse", "concurrency"),
    ("ClickStackTileInput", "asRatio"),
    ("ClickStackTileInput", "series"),
    ("Invitation", "role"),
    ("InvitationPostRequest", "role"),
    ("Member", "role"),
    ("MemberPatchRequest", "role"),
    ("OrganizationPrivateEndpointsPatch", "add"),
    ("Service", "maxTotalMemoryGb"),
    ("Service", "minTotalMemoryGb"),
    ("Service", "tier"),
    ("ServicePostRequest", "maxTotalMemoryGb"),
    ("ServicePostRequest", "minTotalMemoryGb"),
    ("ServicePostRequest", "privateEndpointIds"),
    ("ServicePostRequest", "tier"),
    ("ServiceScalingPatchRequest", "maxTotalMemoryGb"),
    ("ServiceScalingPatchRequest", "minTotalMemoryGb"),
    ("ServiceScalingPatchResponse", "maxTotalMemoryGb"),
    ("ServiceScalingPatchResponse", "minTotalMemoryGb"),
    ("ServiceScalingPatchResponse", "tier"),
];

/// Returns `true` if `(struct_name, field_name)` is a deprecated field that
/// this crate removes from the generated struct by default.
///
/// `field_name` is the spec (camelCase) field name, e.g.
/// `is_deprecated_field("Service", "tier")`.
pub fn is_deprecated_field(struct_name: &str, field_name: &str) -> bool {
    DEPRECATED_FIELDS
        .binary_search(&(struct_name, field_name))
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_sorted_and_unique() {
        for pair in BETA_OPERATIONS.windows(2) {
            assert!(
                pair[0] < pair[1],
                "BETA_OPERATIONS must be sorted and unique; {:?} >= {:?}",
                pair[0],
                pair[1],
            );
        }
    }

    #[test]
    fn is_beta_operation_matches_constant() {
        assert!(is_beta_operation("scaling_schedule_get"));
        assert!(is_beta_operation("postgres_service_get_list"));
        assert!(!is_beta_operation("services_list"));
        assert!(!is_beta_operation("not_a_real_op"));
    }

    #[test]
    fn deprecated_fields_are_sorted_and_unique() {
        for pair in DEPRECATED_FIELDS.windows(2) {
            assert!(
                pair[0] < pair[1],
                "DEPRECATED_FIELDS must be sorted and unique; {:?} >= {:?}",
                pair[0],
                pair[1],
            );
        }
    }

    #[test]
    fn is_deprecated_field_matches_constant() {
        assert!(is_deprecated_field("Service", "tier"));
        assert!(is_deprecated_field("ApiKey", "roles"));
        assert!(is_deprecated_field("ServicePostRequest", "tier"));
        assert!(is_deprecated_field("InvitationPostRequest", "role"));
        assert!(!is_deprecated_field("Service", "name"));
        assert!(!is_deprecated_field("NotAStruct", "tier"));
    }
}
