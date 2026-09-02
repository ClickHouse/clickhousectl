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
    /// The stored Query API key no longer exists in the organization; the
    /// record was kept and `repair-query-key` replaces it (#528).
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
    /// `repair-query-key` replaced and stored the key, but the Query API kept
    /// rejecting the replacement for the whole readiness window. The repair
    /// is committed; rerunning it would rotate a key that may be fine (#658).
    QueryKeyRepairUnverified,
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
        Self::QueryKeyRepairUnverified,
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
        // The single place a string scalar becomes human text, so eliding here
        // covers object fields, nested objects and arrays of strings alike.
        Value::String(s) => Some(pem_summary(s).unwrap_or_else(|| s.clone())),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

// ── PEM elision in human output (issue #665) ─────────────────────────────
//
// A Postgres ClickPipe's `source.postgres.caCertificate` is a whole PEM block:
// roughly 1.5 KB of base64 that scrolls the rest of `clickpipe get` off the
// screen while telling the reader nothing. Human output therefore renders a
// PEM-framed string as a one-line summary. `--json` is untouched: it
// serializes the model directly, so a caller that needs the certificate still
// gets the bytes verbatim.
//
// The test is the value's *format*, not its key name. A client certificate, a
// private key, or a certificate nested anywhere else in any response gets the
// same treatment with no per-field allowlist to keep in sync, and a string
// that merely happens to carry PEM framing is exactly what should be elided.

/// The framing prefix that opens an RFC 7468 block.
const PEM_BEGIN: &str = "-----BEGIN ";
/// The five-dash run that closes a framing line and opens an `END` marker.
const PEM_DASHES: &str = "-----";

/// One RFC 7468 block: its label and the raw text between the framing lines.
struct PemBlock<'a> {
    label: &'a str,
    body: &'a str,
}

/// Split `text` into the RFC 7468 blocks it contains, stopping at the first
/// unterminated or unlabelled frame. Only well-formed `BEGIN`/`END` pairs with
/// matching labels count as blocks.
fn pem_blocks(text: &str) -> Vec<PemBlock<'_>> {
    let mut blocks = Vec::new();
    let mut rest = text;
    while let Some(begin) = rest.find(PEM_BEGIN) {
        let after_begin = &rest[begin + PEM_BEGIN.len()..];
        let Some(label_len) = after_begin.find(PEM_DASHES) else {
            break;
        };
        let label = &after_begin[..label_len];
        if label.is_empty() {
            break;
        }
        let after_label = &after_begin[label_len + PEM_DASHES.len()..];
        let end_marker = format!("-----END {label}-----");
        let Some(body_len) = after_label.find(&end_marker) else {
            break;
        };
        blocks.push(PemBlock {
            label,
            body: &after_label[..body_len],
        });
        rest = &after_label[body_len + end_marker.len()..];
    }
    blocks
}

/// Decode a block body to its DER bytes, or `None` when the base64 is not
/// decodable (an encrypted key with RFC 1421 headers, a truncated paste).
fn pem_body_der(body: &str) -> Option<Vec<u8>> {
    let base64: String = body.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let der = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &base64).ok()?;
    if der.is_empty() { None } else { Some(der) }
}

/// SHA-256 of `der` as colon-separated uppercase hex, the form
/// `openssl x509 -fingerprint -sha256` prints.
fn sha256_fingerprint(der: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(der)
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Summarize a PEM text for human output, or return `None` when `value` is not
/// PEM and should be printed as-is.
///
/// The body is never part of the result. The fingerprint identifies the *first*
/// block, which is the leaf certificate of a chain; when that block's base64
/// does not decode the summary reports the text's size instead, so the value is
/// still accounted for without printing it.
fn pem_summary(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with(PEM_BEGIN) {
        return None;
    }
    let blocks = pem_blocks(trimmed);
    let first = blocks.first()?;
    let count = blocks.len();
    let label = first.label;
    Some(match pem_body_der(first.body) {
        Some(der) => format!(
            "<PEM: {count} {label} block(s), SHA-256 fingerprint {}>",
            sha256_fingerprint(&der)
        ),
        None => format!("<PEM: {count} {label} block(s), {} bytes>", trimmed.len()),
    })
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
                "query_key_repair_unverified",
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

    // ── PEM elision (issue #665) ───────────────────────────────────────────

    /// A real self-signed EC certificate. Its SHA-256 fingerprint below was
    /// taken from `openssl x509 -fingerprint -sha256`, so the assertion pins
    /// the CLI's output against the tool a user would reach for to check it.
    const PEM_FIXTURE: &str = "\
-----BEGIN CERTIFICATE-----
MIIBjDCCATOgAwIBAgIUajES1wl65zexYYPuWX8ShldYw4YwCgYIKoZIzj0EAwIw
HDEaMBgGA1UEAwwRY2hjdGwtcGVtLWZpeHR1cmUwHhcNMjYwOTAyMTc1NDI3WhcN
NDYwODI4MTc1NDI3WjAcMRowGAYDVQQDDBFjaGN0bC1wZW0tZml4dHVyZTBZMBMG
ByqGSM49AgEGCCqGSM49AwEHA0IABNTPygUG2umVvTqod5jJXCgp1o9qwrx2wLf7
p+2PyHYm5ZdIS+kqT25Xm2SGM3th4dB43l3fd5kF0g6CzvGNt42jUzBRMB0GA1Ud
DgQWBBQcL9JNezOJ8vzT0lR1Pj4sMoH2STAfBgNVHSMEGDAWgBQcL9JNezOJ8vzT
0lR1Pj4sMoH2STAPBgNVHRMBAf8EBTADAQH/MAoGCCqGSM49BAMCA0cAMEQCIAhv
iLjfMqcnJ10gmKoyEIMDDRJP2UwtGRcJZU/FnaIEAiBeUmN+nJBGIq0tFHIxz1Xl
LaBMf6qZANMrXRQaETxhIA==
-----END CERTIFICATE-----
";

    /// `openssl x509 -in fixture.crt -noout -fingerprint -sha256`.
    const PEM_FIXTURE_FINGERPRINT: &str = "5A:6D:67:FD:14:1B:1E:61:4A:F4:E2:7D:F1:F8:67:E2:75:85:DF:92:E3:66:31:85:75:AB:2C:C3:F4:8C:9A:D8";

    /// A distinctive run from the middle of the fixture's base64 body: if this
    /// ever appears in human output, the certificate was not elided.
    const PEM_FIXTURE_BODY_MARKER: &str =
        "ByqGSM49AgEGCCqGSM49AwEHA0IABNTPygUG2umVvTqod5jJXCgp1o9qwrx2wLf7";

    #[test]
    fn pem_summary_reports_count_label_and_openssl_fingerprint() {
        assert_eq!(
            pem_summary(PEM_FIXTURE).unwrap(),
            format!("<PEM: 1 CERTIFICATE block(s), SHA-256 fingerprint {PEM_FIXTURE_FINGERPRINT}>")
        );
    }

    #[test]
    fn pem_summary_never_contains_the_body() {
        let summary = pem_summary(PEM_FIXTURE).unwrap();
        assert!(
            !summary.contains(PEM_FIXTURE_BODY_MARKER),
            "body leaked: {summary}"
        );
        assert!(!summary.contains("-----BEGIN"), "framing leaked: {summary}");
    }

    #[test]
    fn pem_summary_counts_concatenated_blocks_and_fingerprints_the_first() {
        // A chain: leaf then issuer. The count reports both, the fingerprint
        // identifies the leaf.
        let chain = format!("{PEM_FIXTURE}{PEM_FIXTURE}");
        assert_eq!(
            pem_summary(&chain).unwrap(),
            format!("<PEM: 2 CERTIFICATE block(s), SHA-256 fingerprint {PEM_FIXTURE_FINGERPRINT}>")
        );
    }

    #[test]
    fn pem_summary_uses_the_blocks_own_label() {
        // Elision is not certificate-specific: a private key that somehow
        // reached human output is summarized the same way, body withheld.
        // `AQIDBA==` is the four bytes 01 02 03 04; the fingerprint is their
        // SHA-256, so the expected hex is a constant.
        let key = "-----BEGIN PRIVATE KEY-----\nAQIDBA==\n-----END PRIVATE KEY-----";
        const FINGERPRINT_OF_01020304: &str = "9F:64:A7:47:E1:B9:7F:13:1F:AB:B6:B4:47:29:6C:9B:6F:02:01:E7:9F:B3:C5:35:6E:6C:77:E8:9B:6A:80:6A";
        assert_eq!(
            pem_summary(key).unwrap(),
            format!("<PEM: 1 PRIVATE KEY block(s), SHA-256 fingerprint {FINGERPRINT_OF_01020304}>")
        );
    }

    #[test]
    fn pem_summary_tolerates_surrounding_whitespace() {
        let padded = format!("\n  {}\n\n", PEM_FIXTURE.trim());
        assert!(
            pem_summary(&padded)
                .unwrap()
                .starts_with("<PEM: 1 CERTIFICATE block(s),")
        );
    }

    #[test]
    fn pem_summary_falls_back_to_a_byte_count_when_the_body_is_not_base64() {
        let broken = "-----BEGIN CERTIFICATE-----\n**not base64**\n-----END CERTIFICATE-----";
        assert_eq!(broken.len(), 68);
        assert_eq!(
            pem_summary(broken).unwrap(),
            "<PEM: 1 CERTIFICATE block(s), 68 bytes>"
        );
    }

    #[test]
    fn pem_summary_leaves_non_pem_strings_alone() {
        for value in [
            "",
            "running",
            "us-east-1",
            // Framing that never closes is not a block.
            "-----BEGIN CERTIFICATE-----\nAQIDBA==\n",
            // A mismatched END label does not close the BEGIN.
            "-----BEGIN CERTIFICATE-----\nAQIDBA==\n-----END PRIVATE KEY-----",
            // An unlabelled frame is not a block.
            "-----BEGIN -----\nAQIDBA==\n-----END -----",
            // PEM further in is not the value's format; only a leading frame
            // makes the whole string a certificate.
            "see attached: -----BEGIN CERTIFICATE-----\nAQIDBA==\n-----END CERTIFICATE-----",
        ] {
            assert_eq!(pem_summary(value), None, "unexpectedly elided {value:?}");
        }
    }

    #[test]
    fn render_elides_a_certificate_field_and_keeps_its_siblings() {
        let v = json!({
            "source": {
                "postgres": {
                    "host": "db.example.com",
                    "caCertificate": PEM_FIXTURE,
                    "database": "postgres",
                }
            }
        });
        let rendered = render_to_string(&v);
        assert_eq!(
            rendered,
            format!(
                "source:\n  postgres:\n    host: db.example.com\n    caCertificate: <PEM: 1 CERTIFICATE block(s), SHA-256 fingerprint {PEM_FIXTURE_FINGERPRINT}>\n    database: postgres"
            )
        );
        assert!(
            !rendered.contains("-----BEGIN"),
            "certificate body rendered: {rendered}"
        );
    }

    #[test]
    fn render_elides_certificates_inside_arrays_and_bullets() {
        let v = json!({
            "trustChain": [PEM_FIXTURE, PEM_FIXTURE],
            "endpoints": [{ "clientCertificate": PEM_FIXTURE }],
        });
        let rendered = render_to_string(&v);
        assert!(
            !rendered.contains("-----BEGIN"),
            "body rendered: {rendered}"
        );
        assert_eq!(rendered.matches("<PEM: 1 CERTIFICATE block(s)").count(), 3);
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
