//! Human-readable rendering of cloud API response types.
//!
//! [`print_human`] serializes any `Serialize` value to a [`serde_json::Value`]
//! and renders it as an indented `key: value` tree. Driving human output
//! through serde (rather than hand-written `println!` blocks) means it shares
//! the library's serialization behaviour — most importantly, deprecated fields
//! marked with `#[cfg(feature = "deprecated-fields")]` are absent from the
//! struct (and so from both `--json` and human output) by default, so the CLI
//! never surfaces a field the API has deprecated.
//!
//! `serde_json`'s `preserve_order` feature keeps `to_value` output in struct
//! declaration order, so fields render in a stable, source-defined order.

use serde::Serialize;
use serde_json::{Map, Value};

const INDENT: &str = "  ";

/// Placeholder for a field the API did not return.
///
/// Every field of a response model is `Option<T>`, so absence is a normal
/// outcome rather than an error: list tables and plain output render it as this
/// placeholder instead of unwrapping or fabricating a value.
pub(crate) const ABSENT: &str = "-";

/// Renders an absent (`None`) response field as [`ABSENT`].
pub(crate) fn or_absent<T: std::fmt::Display>(value: Option<T>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| ABSENT.to_string())
}

/// Write one line to stderr, discarding a write failure.
///
/// `eprintln!` *panics* when the write fails, and the write fails with
/// `BrokenPipe` as soon as whatever was reading stderr goes away (a pager the
/// user quit, a supervising harness that stopped reading). That turns a
/// long-running command into a panic and exit 101 — see #598, where
/// `cloud service delete --force` streamed stop-poll progress for minutes and
/// crashed instead of deleting the service. Progress and status lines are never
/// worth a panic: the exit code, not the line, reports the outcome.
///
/// Same rule as `telemetry::print_first_run_notice` and
/// `update::print_cached_update_notice`.
pub(crate) fn eprint_line(line: impl std::fmt::Display) {
    use std::io::Write;
    let _ = writeln!(std::io::stderr(), "{line}");
}

/// Write one line to stdout, discarding a write failure.
///
/// The stdout counterpart of [`eprint_line`], for a result line printed after
/// the operation it describes already succeeded: a closed stdout must not
/// convert a completed deletion into a panic.
pub(crate) fn print_line(line: impl std::fmt::Display) {
    use std::io::Write;
    let _ = writeln!(std::io::stdout(), "{line}");
}

// ── structured errors (issue #644) ─────────────────────────────────────────
//
// Cloud failures are prose on stderr: `Error: <message>`. That is fine for a
// human, but an agent asked to recover from one has to parse it. A failure
// whose remedy is a concrete command therefore also carries a machine-readable
// detail, emitted under `--json` in the same envelope local errors use
// (#475/#608): one object on stderr, `{"error": {"code": ..., "message": ...}}`,
// with a `command` a caller can run.

/// Stable machine-readable code for a cloud failure that carries structured
/// remediation. Literal wire values only, and never widened by anything the
/// API sends: the vocabulary is this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
// The shared `Query` prefix is part of the wire value (`query_timeout`,
// `query_key_*`), which names the command family the code belongs to; it is
// not a Rust naming accident to strip.
#[allow(clippy::enum_variant_names)]
pub enum CloudErrorCode {
    /// The Query API gateway stopped waiting for the statement (#644).
    QueryTimeout,
    /// The stored Query API key no longer exists in the organization, so the
    /// stale local record was removed; the next query re-provisions (#528).
    QueryKeyDeleted,
    /// The stored Query API key exists but an administrator disabled it; it
    /// was neither replaced nor removed (#528).
    QueryKeyDisabled,
    /// The stored Query API key exists but its `expireAt` has passed; it was
    /// neither replaced nor removed (#528).
    QueryKeyExpired,
    /// The stored Query API key is enabled but no longer bound to the
    /// service's Query API endpoint; it was neither replaced nor removed
    /// (#528).
    QueryKeyUnbound,
    /// The stored Query API key is enabled, unexpired and bound, yet the
    /// Query API still rejects it: its IP access list or the local secret is
    /// the likely cause; nothing was changed (#528).
    QueryKeyRejected,
    /// The stored Query API key's management record could not be read, so
    /// the rejection could not be classified; nothing was changed (#528).
    QueryKeyUnverified,
}

impl CloudErrorCode {
    /// Every code, for closed-vocabulary tests.
    #[cfg(test)]
    const ALL: &'static [Self] = &[
        Self::QueryTimeout,
        Self::QueryKeyDeleted,
        Self::QueryKeyDisabled,
        Self::QueryKeyExpired,
        Self::QueryKeyUnbound,
        Self::QueryKeyRejected,
        Self::QueryKeyUnverified,
    ];
}

/// One cloud failure, as `--json` reports it.
///
/// `message` is the same text human mode prints, so the two modes never
/// disagree. The remaining fields are the structured form of the hint: absent
/// fields are omitted rather than serialized as `null`, matching the response
/// models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CloudErrorDetail {
    pub code: CloudErrorCode,
    pub message: String,
    /// Native-protocol host to reconnect to, when the API returned one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Native-protocol port paired with `host`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    /// A command that acts on the failure. Never carries the user's SQL or
    /// any credential: both are placeholders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Management resource ID of the stored Query API key a `query_key_*`
    /// failure is about (#528). A resource ID, never the credential pair.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<String>,
    /// The key's IP access list (CIDRs only), when the rejection may be an
    /// allowlist miss (#528).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_access_list: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct CloudErrorOutput<'a> {
    error: &'a CloudErrorDetail,
}

/// Write exactly one cloud error object to stderr, for `--json` mode.
///
/// Serialization failure is swallowed for the same reason [`eprint_line`]
/// swallows a write failure: the exit code reports the outcome, so a closed
/// stderr must not become a panic.
pub fn print_error(detail: &CloudErrorDetail) {
    use std::io::Write;
    let stderr = std::io::stderr();
    let mut stderr = stderr.lock();
    if serde_json::to_writer_pretty(&mut stderr, &CloudErrorOutput { error: detail }).is_ok() {
        let _ = writeln!(stderr);
    }
}

/// Serialize `value` and print it as an indented, human-readable tree.
///
/// - Object keys are printed verbatim (camelCase, as the API returns them).
/// - String values are unquoted.
/// - Arrays of scalars render inline (`key: [a, b, c]`); arrays of objects
///   render as `-` bullet blocks.
/// - Null values and empty strings/arrays/objects are omitted.
pub fn print_human<T: Serialize>(value: &T) -> Result<(), serde_json::Error> {
    let value = serde_json::to_value(value)?;
    let mut lines = Vec::new();
    render(&mut lines, 0, &value);
    for line in &lines {
        println!("{line}");
    }
    Ok(())
}

fn pad(indent: usize) -> String {
    INDENT.repeat(indent)
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

fn is_scalar(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_))
}

fn scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Render any value at `indent`. The first line emitted (if any) is the natural
/// anchor a caller can retrofit a `-` bullet onto.
fn render(lines: &mut Vec<String>, indent: usize, value: &Value) {
    match value {
        Value::Object(map) => render_object(lines, indent, map),
        Value::Array(items) => render_array(lines, indent, items),
        scalar => {
            if let Some(s) = scalar_string(scalar) {
                lines.push(format!("{}{}", pad(indent), s));
            }
        }
    }
}

fn render_object(lines: &mut Vec<String>, indent: usize, map: &Map<String, Value>) {
    for (key, value) in map {
        if is_empty(value) {
            continue;
        }
        match value {
            Value::Object(inner) => {
                let start = lines.len();
                lines.push(format!("{}{}:", pad(indent), key));
                render_object(lines, indent + 1, inner);
                // Drop the header if every field underneath was empty.
                if lines.len() == start + 1 {
                    lines.pop();
                }
            }
            Value::Array(items) => render_array_field(lines, indent, key, items),
            scalar => lines.push(format!(
                "{}{}: {}",
                pad(indent),
                key,
                scalar_string(scalar).unwrap_or_default()
            )),
        }
    }
}

fn render_array_field(lines: &mut Vec<String>, indent: usize, key: &str, items: &[Value]) {
    if items.iter().all(is_scalar) {
        let joined = items
            .iter()
            .filter_map(scalar_string)
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("{}{}: [{}]", pad(indent), key, joined));
    } else {
        let start = lines.len();
        lines.push(format!("{}{}:", pad(indent), key));
        render_array(lines, indent + 1, items);
        // Drop the header if every item rendered empty.
        if lines.len() == start + 1 {
            lines.pop();
        }
    }
}

/// Render array items as `-` bullet blocks. Each item's content is rendered one
/// level deeper than `indent`; the `- ` bullet then occupies the `indent` slot.
fn render_array(lines: &mut Vec<String>, indent: usize, items: &[Value]) {
    for item in items {
        if is_empty(item) {
            continue;
        }
        let start = lines.len();
        render(lines, indent + 1, item);
        // Retrofit a `- ` bullet onto the first line this item produced by
        // replacing the two pad spaces that sit at the array's indent level.
        if lines.len() > start {
            let bullet_pos = indent * INDENT.len();
            let line = &mut lines[start];
            if line.len() >= bullet_pos + INDENT.len()
                && &line[bullet_pos..bullet_pos + INDENT.len()] == INDENT
            {
                line.replace_range(bullet_pos..bullet_pos + INDENT.len(), "- ");
            } else {
                line.insert_str(bullet_pos, "- ");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Render to a single string for assertions (mirrors what `print_human`
    /// would print, minus the trailing newline).
    fn render_to_string(value: &Value) -> String {
        let mut lines = Vec::new();
        render(&mut lines, 0, value);
        lines.join("\n")
    }

    #[test]
    fn renders_flat_object() {
        let v = json!({"name": "svc", "port": 9000, "secure": true});
        assert_eq!(render_to_string(&v), "name: svc\nport: 9000\nsecure: true");
    }

    #[test]
    fn omits_null_and_empty_values() {
        let v = json!({
            "name": "svc",
            "note": null,
            "empty": "",
            "tags": [],
            "meta": {},
            "count": 0,
            "flag": false
        });
        // null/empty-string/empty-array/empty-object are dropped; 0 and false stay.
        assert_eq!(render_to_string(&v), "name: svc\ncount: 0\nflag: false");
    }

    #[test]
    fn renders_nested_object_indented() {
        let v = json!({"service": {"name": "svc", "region": "us-east-1"}});
        assert_eq!(
            render_to_string(&v),
            "service:\n  name: svc\n  region: us-east-1"
        );
    }

    #[test]
    fn renders_scalar_array_inline() {
        let v = json!({"providers": ["aws", "gcp", "azure"]});
        assert_eq!(render_to_string(&v), "providers: [aws, gcp, azure]");
    }

    #[test]
    fn renders_object_array_as_bullets() {
        let v = json!({
            "endpoints": [
                {"protocol": "https", "port": 8443},
                {"protocol": "nativesecure", "port": 9440}
            ]
        });
        assert_eq!(
            render_to_string(&v),
            "endpoints:\n  - protocol: https\n    port: 8443\n  - protocol: nativesecure\n    port: 9440"
        );
    }

    #[test]
    fn renders_nested_object_under_bullet() {
        let v = json!({
            "columns": [
                {"name": "id", "type": {"kind": "UInt64", "nullable": false}}
            ]
        });
        assert_eq!(
            render_to_string(&v),
            "columns:\n  - name: id\n    type:\n      kind: UInt64\n      nullable: false"
        );
    }

    // In the default build the deprecated fields don't exist on `Service`, so
    // `to_value` can't emit them. With the `deprecated-fields` feature they are
    // present and serialized, so this default-behaviour assertion only holds
    // without the feature.
    #[cfg(not(feature = "deprecated-fields"))]
    #[test]
    fn service_get_render_omits_deprecated_tier() {
        // End-to-end: a real library `Service` rendered through the same
        // `to_value` path `print_human` uses. `tier`/`minTotalMemoryGb`/
        // `maxTotalMemoryGb` are deprecated and must not appear in output.
        let svc: clickhouse_cloud_api::models::Service = serde_json::from_str(
            r#"{
                "name": "analytics",
                "provider": "aws",
                "region": "us-east-1",
                "tier": "production",
                "minTotalMemoryGb": 24,
                "maxTotalMemoryGb": 48,
                "numReplicas": 3
            }"#,
        )
        .unwrap();
        let rendered = render_to_string(&serde_json::to_value(&svc).unwrap());
        assert!(rendered.contains("name: analytics"));
        assert!(rendered.contains("numReplicas: 3"));
        assert!(
            !rendered.contains("tier"),
            "deprecated tier leaked: {rendered}"
        );
        assert!(!rendered.contains("minTotalMemoryGb"));
        assert!(!rendered.contains("maxTotalMemoryGb"));
    }

    /// The error codes are a closed vocabulary of snake_case literals: an
    /// agent branching on `error.code` can enumerate them.
    #[test]
    fn cloud_error_codes_are_stable_snake_case_literals() {
        let wire: Vec<String> = CloudErrorCode::ALL
            .iter()
            .map(|code| {
                serde_json::to_value(code)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(
            wire,
            [
                "query_timeout",
                "query_key_deleted",
                "query_key_disabled",
                "query_key_expired",
                "query_key_unbound",
                "query_key_rejected",
                "query_key_unverified",
            ]
        );
    }

    #[test]
    fn cloud_error_detail_omits_absent_fields() {
        let detail = CloudErrorDetail {
            code: CloudErrorCode::QueryKeyDisabled,
            message: "disabled".into(),
            host: None,
            port: None,
            command: Some("clickhousectl cloud service repair-query-key svc-1".into()),
            api_key_id: Some("key-1".into()),
            ip_access_list: None,
        };
        let value = serde_json::to_value(CloudErrorOutput { error: &detail }).unwrap();
        assert_eq!(
            value,
            json!({
                "error": {
                    "code": "query_key_disabled",
                    "message": "disabled",
                    "command": "clickhousectl cloud service repair-query-key svc-1",
                    "api_key_id": "key-1",
                }
            })
        );
    }

    #[test]
    fn deprecated_field_absent_means_omitted() {
        // Simulates a serialized Service where serde already dropped `tier`.
        // print_human renders only what serde produced — nothing extra to do.
        let v = json!({"name": "svc", "state": "running"});
        let out = render_to_string(&v);
        assert!(!out.contains("tier"));
        assert_eq!(out, "name: svc\nstate: running");
    }
}
