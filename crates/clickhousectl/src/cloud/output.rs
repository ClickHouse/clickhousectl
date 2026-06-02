//! Human-readable rendering of cloud API response types.
//!
//! [`print_human`] serializes any `Serialize` value to a [`serde_json::Value`]
//! and renders it as an indented `key: value` tree. Driving human output
//! through serde (rather than hand-written `println!` blocks) means it shares
//! the library's serialization behaviour — most importantly, deprecated fields
//! marked with `#[cfg_attr(not(feature = "deprecated-fields"), serde(skip_serializing))]`
//! are absent from both `--json` and human output, so the CLI never surfaces a
//! field the API has deprecated.
//!
//! `serde_json`'s `preserve_order` feature keeps `to_value` output in struct
//! declaration order, so fields render in a stable, source-defined order.

use serde::Serialize;
use serde_json::{Map, Value};

const INDENT: &str = "  ";

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
        assert_eq!(
            render_to_string(&v),
            "name: svc\nport: 9000\nsecure: true"
        );
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
        assert!(!rendered.contains("tier"), "deprecated tier leaked: {rendered}");
        assert!(!rendered.contains("minTotalMemoryGb"));
        assert!(!rendered.contains("maxTotalMemoryGb"));
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
