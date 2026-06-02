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
//! The `beta_operations_match_spec` test in `tests/spec_coverage_test.rs` fails
//! if this list drifts from the spec.

/// Snake-case operation IDs (matching [`crate::client::Client`] method names)
/// that the OpenAPI spec marks Beta via `x-badges`.
pub const BETA_OPERATIONS: &[&str] = &[
    "backup_bucket_create",
    "backup_bucket_delete",
    "backup_bucket_get",
    "backup_bucket_update",
    "click_stack_create_alert",
    "click_stack_create_dashboard",
    "click_stack_delete_alert",
    "click_stack_delete_dashboard",
    "click_stack_get_alert",
    "click_stack_get_dashboard",
    "click_stack_list_alerts",
    "click_stack_list_dashboards",
    "click_stack_list_sources",
    "click_stack_list_webhooks",
    "click_stack_update_alert",
    "click_stack_update_dashboard",
    "postgres_instance_config_get",
    "postgres_instance_config_patch",
    "postgres_instance_config_post",
    "postgres_instance_create_read_replica",
    "postgres_instance_prometheus_get",
    "postgres_instance_restore",
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
    "service_clickhouse_setting_get",
    "service_clickhouse_settings_list_get",
    "service_clickhouse_settings_schema_get",
    "service_clickhouse_settings_update",
];

/// Returns `true` if `name` matches a client method backed by a Beta endpoint.
///
/// `name` is the snake-case method name (e.g. `"postgres_service_get_list"`).
pub fn is_beta_operation(name: &str) -> bool {
    BETA_OPERATIONS.binary_search(&name).is_ok()
}

/// Response/display schema fields the OpenAPI spec marks `deprecated: true`,
/// as `(RustStructName, specFieldName)` pairs.
///
/// These fields are hidden during serialization unless the `deprecated-fields`
/// Cargo feature is enabled — each one carries a
/// `#[cfg_attr(not(feature = "deprecated-fields"), serde(skip_serializing))]`
/// marker in [`crate::models`]. Deserialization is unaffected, so the values
/// are still parsed into the structs; only serialized output (e.g. this crate's
/// CLI rendering responses) omits them.
///
/// Request-side schemas (`*Request`, `*Patch`, `*Input`) are intentionally
/// excluded — callers may still need to send a deprecated field. The list is
/// kept sorted so [`is_deprecated_output_field`] can use `binary_search` and
/// snapshot diffs stay readable.
///
/// Regenerate from the snapshot with:
///
/// ```text
/// python3 scripts/regenerate-deprecated-fields.py
/// ```
///
/// The `deprecated_output_fields_match_spec` test in
/// `tests/spec_coverage_test.rs` fails if this list drifts from the spec, and
/// `deprecated_output_fields_hidden` fails if a field here lacks the
/// `skip_serializing` marker in `models.rs` (or vice versa).
pub const DEPRECATED_OUTPUT_FIELDS: &[(&str, &str)] = &[
    ("ApiKey", "roles"),
    ("ClickPipeScaling", "concurrency"),
    ("Invitation", "role"),
    ("Member", "role"),
    ("Service", "maxTotalMemoryGb"),
    ("Service", "minTotalMemoryGb"),
    ("Service", "tier"),
    ("ServiceScalingPatchResponse", "maxTotalMemoryGb"),
    ("ServiceScalingPatchResponse", "minTotalMemoryGb"),
    ("ServiceScalingPatchResponse", "tier"),
];

/// Returns `true` if `(struct_name, field_name)` is a deprecated response field
/// that this crate hides from serialized output by default.
///
/// `field_name` is the spec (camelCase) field name, e.g.
/// `is_deprecated_output_field("Service", "tier")`.
pub fn is_deprecated_output_field(struct_name: &str, field_name: &str) -> bool {
    DEPRECATED_OUTPUT_FIELDS
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
    fn deprecated_output_fields_are_sorted_and_unique() {
        for pair in DEPRECATED_OUTPUT_FIELDS.windows(2) {
            assert!(
                pair[0] < pair[1],
                "DEPRECATED_OUTPUT_FIELDS must be sorted and unique; {:?} >= {:?}",
                pair[0],
                pair[1],
            );
        }
    }

    #[test]
    fn is_deprecated_output_field_matches_constant() {
        assert!(is_deprecated_output_field("Service", "tier"));
        assert!(is_deprecated_output_field("ApiKey", "roles"));
        assert!(!is_deprecated_output_field("Service", "name"));
        assert!(!is_deprecated_output_field("NotAStruct", "tier"));
    }
}
