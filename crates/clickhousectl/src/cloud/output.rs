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

// ── structured errors (issues #644, #825) ───────────────────────────────────

/// Stable machine-readable code for a cloud failure. Literal wire values only,
/// and never widened by anything the API sends: the vocabulary is this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudErrorCode {
    AuthRequired,
    Cancelled,
    Io,
    Transport,
    #[serde(rename = "http_4xx")]
    Http4xx,
    #[serde(rename = "http_5xx")]
    Http5xx,
    SqlError,
    ServiceStopped,
    Timeout,
    RateLimited,
    Other,
    /// The Query API gateway stopped waiting for the statement (#644).
    QueryTimeout,
    /// `cloud service query` found the authenticated API key already bound to
    /// the service's Query API endpoint, yet the Query API kept rejecting it
    /// for the whole readiness window; no upsert was made, since rebinding
    /// cannot help (#1043).
    QueryKeyBoundRejected,
    /// `cloud service query` had to bind the authenticated API key to the
    /// service's Query API endpoint, and the Cloud API refused the write
    /// (`403`), typically because the key is read-only. Nothing was changed;
    /// `command` inspects the endpoint and `api_key_id` names the key (#1325).
    QueryKeyBindForbidden,
    /// A by-identifier request named a resource that does not exist: the API
    /// answered 404, or rejected a well-formed UUID on a get or delete
    /// (#666), or a `--name`/`--email` matched nothing (#1237). `command`
    /// lists what does exist in the same scope.
    ResourceNotFound,
    /// A UDF definition is not valid JSON or fails validation; the same code
    /// `local udf deploy` reports.
    UdfDefinitionInvalid,
    /// A UDF source directory cannot be packaged: it has no `udf.json` or
    /// entrypoint, or contains a symbolic link; the same code
    /// `local udf deploy` reports.
    UdfSourceInvalid,
    /// `cloud udf attach` found the service idle (the API's typed
    /// `SERVICE_IDLE`); rerun with `--wake` or wake the service first.
    ServiceIdle,
    /// `cloud udf attach` found the service neither running, idle nor
    /// stopped (the API's typed `SERVICE_NOT_RUNNING`, e.g. starting): wait
    /// for it to reach `running`, or start it.
    ServiceNotRunning,
    /// `cloud clickpipe start` reached a CDC pipe that is still `Pausing`: the
    /// API answers 200 but does not apply the start (#1228). Poll `command`
    /// until the pipe is `Paused`, then rerun start.
    #[serde(rename = "clickpipe_still_pausing")]
    ClickPipeStillPausing,
    /// `cloud clickpipe start` was answered with `Paused`: a CDC pipe that has
    /// only just finished pausing accepts the start but does not apply it
    /// (#1228). Wait a few seconds and rerun `command`, the same start.
    #[serde(rename = "clickpipe_start_not_applied")]
    ClickPipeStartNotApplied,
    /// `cloud clickpipe start` was answered with `Completed`: the API accepts a
    /// start on a completed pipe but does not re-run it (#926). Nothing to
    /// retry, so there is no `command`.
    #[serde(rename = "clickpipe_start_no_effect")]
    ClickPipeStartNoEffect,
    /// `update` found an npm or pip/uv/pipx install and replaced nothing:
    /// `command` is the package manager's upgrade command (#1305).
    PackageManaged,
    /// `update` downloaded an archive that the release's `SHA256SUMS` does
    /// not list or whose digest differs, and replaced nothing (#1307).
    VerificationFailed,
    /// `cloud service client` was pointed at a ClickHouse build whose version
    /// is malformed; the same code `local client` reports (#1443).
    InvalidVersion,
    /// `cloud service client` found no installed local build to run, or not
    /// the requested one; `command` lists or installs one. The same code
    /// `local client` reports (#1443).
    VersionNotInstalled,
    /// `cloud service client` found several installed builds and no default;
    /// pass `--version` or set a default. The same code `local client`
    /// reports (#1443).
    VersionSelectionRequired,
    /// The selected local build is installed but cannot be launched;
    /// `command` reinstalls it. The same code `local client` reports (#1443).
    BinaryNotLaunchable,
    /// The selected local build's client predates `--login`, which
    /// `cloud service client` needs; `command` installs a newer one (#1443).
    UnsupportedClientVersion,
}

impl CloudErrorCode {
    pub fn from_failure(kind: crate::failure::FailureKind) -> Self {
        use crate::failure::FailureKind;
        match kind {
            FailureKind::Io => Self::Io,
            FailureKind::Transport => Self::Transport,
            FailureKind::Http4xx => Self::Http4xx,
            FailureKind::Http5xx => Self::Http5xx,
            FailureKind::SqlError => Self::SqlError,
            FailureKind::ServiceStopped => Self::ServiceStopped,
            FailureKind::Timeout => Self::Timeout,
            FailureKind::RateLimited => Self::RateLimited,
            FailureKind::Other => Self::Other,
        }
    }

    /// The Cloud code for a local client pre-flight failure that
    /// `cloud service client` shares with `local client` (#1443).
    fn from_local(code: crate::local::output::LocalErrorCode) -> Self {
        use crate::local::output::LocalErrorCode;
        match code {
            LocalErrorCode::InvalidVersion => Self::InvalidVersion,
            LocalErrorCode::VersionNotInstalled => Self::VersionNotInstalled,
            LocalErrorCode::VersionSelectionRequired => Self::VersionSelectionRequired,
            LocalErrorCode::BinaryNotLaunchable => Self::BinaryNotLaunchable,
            LocalErrorCode::UnsupportedClientVersion => Self::UnsupportedClientVersion,
            _ => Self::Other,
        }
    }

    /// Every code, for closed-vocabulary tests.
    #[cfg(test)]
    const ALL: &'static [Self] = &[
        Self::AuthRequired,
        Self::Cancelled,
        Self::Io,
        Self::Transport,
        Self::Http4xx,
        Self::Http5xx,
        Self::SqlError,
        Self::ServiceStopped,
        Self::Timeout,
        Self::RateLimited,
        Self::Other,
        Self::QueryTimeout,
        Self::QueryKeyBoundRejected,
        Self::QueryKeyBindForbidden,
        Self::ResourceNotFound,
        Self::UdfDefinitionInvalid,
        Self::UdfSourceInvalid,
        Self::ServiceIdle,
        Self::ServiceNotRunning,
        Self::ClickPipeStillPausing,
        Self::ClickPipeStartNotApplied,
        Self::ClickPipeStartNoEffect,
        Self::PackageManaged,
        Self::VerificationFailed,
        Self::InvalidVersion,
        Self::VersionNotInstalled,
        Self::VersionSelectionRequired,
        Self::BinaryNotLaunchable,
        Self::UnsupportedClientVersion,
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
    /// Management resource ID of the API key a `query_key_*` failure is
    /// about. A resource ID, never the credential pair.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<String>,
}

impl CloudErrorDetail {
    pub fn new(code: CloudErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            host: None,
            port: None,
            command: None,
            api_key_id: None,
        }
    }

    /// Runtime errors outside the CloudClient boundary (including auth and
    /// cancellation) use the same envelope without changing their exit codes.
    /// Unclassified failures use `other`; never inspect their message for a code.
    fn from_error(error: &crate::error::Error) -> std::borrow::Cow<'_, Self> {
        use crate::error::Error;
        let code = match error {
            Error::CloudDetailed(detail) => return std::borrow::Cow::Borrowed(detail),
            Error::AuthRequired(_) => CloudErrorCode::AuthRequired,
            Error::Cancelled => CloudErrorCode::Cancelled,
            Error::Io(_) | Error::SqlInputOpen { .. } | Error::SqlInputRead(_) => {
                CloudErrorCode::Io
            }
            Error::Http(error) if error.is_timeout() => CloudErrorCode::Timeout,
            Error::Http(_) => CloudErrorCode::Transport,
            Error::UpdateCheck { failure, .. } => CloudErrorCode::from_failure(*failure),
            Error::PackageManagedInstall { command, .. } => {
                let mut detail = Self::new(CloudErrorCode::PackageManaged, error.to_string());
                detail.command = Some((*command).to_owned());
                return std::borrow::Cow::Owned(detail);
            }
            Error::ArchiveVerification(_) => CloudErrorCode::VerificationFailed,
            _ => match crate::local::output::installed_client_error(error) {
                Some((code, command)) => {
                    let mut detail = Self::new(CloudErrorCode::from_local(code), error.to_string());
                    detail.command = command;
                    return std::borrow::Cow::Owned(detail);
                }
                None => CloudErrorCode::Other,
            },
        };
        std::borrow::Cow::Owned(Self::new(code, error.to_string()))
    }
}

#[cfg(test)]
type CloudErrorOutput<'a> = crate::stdout::ErrorEnvelope<'a, CloudErrorDetail>;

/// Write exactly one cloud error object to stderr, for `--json` mode, in the
/// envelope local failures share (`crate::stdout::eprint_error_envelope`).
pub fn print_error(error: &crate::error::Error) {
    let detail = CloudErrorDetail::from_error(error);
    crate::stdout::eprint_error_envelope(detail.as_ref());
}

// ── next-step polling hints (issue #1162) ───────────────────────────────────

/// How to confirm that an accepted asynchronous operation finished, added to
/// the operation's JSON output as `next`. A command that carries `next` does
/// not wait: exit 0 means the API accepted the request, and this names the
/// read that shows when it is done. Most Cloud commands do not wait; the
/// exceptions are `service delete --force` (it waits for the stop before
/// deleting), `udf deploy` and `udf attach --wake`.
/// `docs/reference/automation.md` documents the shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct NextStep {
    /// The read command to repeat, with resolved IDs, `--org-id` and `--json`.
    pub(crate) command: String,
    /// Top-level field of that command's JSON output to check.
    pub(crate) field: &'static str,
    /// Values of `field` that mean the operation finished.
    pub(crate) until: Vec<Value>,
    /// Values of `field` that mean it will not finish, so polling should stop.
    pub(crate) fail_on: Vec<Value>,
    /// A `--json` error code from `command` that also means finished: the
    /// resource is gone after a delete.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) until_error_code: Option<CloudErrorCode>,
    /// Suggested delay between polls.
    pub(crate) interval_seconds: u64,
}

impl NextStep {
    /// Poll `clickhousectl cloud <args> --org-id <org_id> --json` and read
    /// `field`. Each argument is shell-quoted when it needs to be.
    pub(crate) fn poll(args: &[&str], org_id: &str, field: &'static str) -> Self {
        Self {
            command: format!("{} --json", cloud_command(args, org_id)),
            field,
            until: Vec::new(),
            fail_on: Vec::new(),
            until_error_code: None,
            interval_seconds: 10,
        }
    }

    pub(crate) fn until<T: Serialize>(mut self, values: &[T]) -> Self {
        self.until = hint_values(values);
        self
    }

    pub(crate) fn fail_on<T: Serialize>(mut self, values: &[T]) -> Self {
        self.fail_on = hint_values(values);
        self
    }

    pub(crate) fn until_error_code(mut self, code: CloudErrorCode) -> Self {
        self.until_error_code = Some(code);
        self
    }

    pub(crate) fn every(mut self, seconds: u64) -> Self {
        self.interval_seconds = seconds;
        self
    }
}

/// `clickhousectl cloud <args> --org-id <org_id>`, each argument shell-quoted
/// when it needs to be.
pub(crate) fn cloud_command(args: &[&str], org_id: &str) -> String {
    let quote = |arg: &str| {
        shlex::try_quote(arg)
            .map(std::borrow::Cow::into_owned)
            .unwrap_or_else(|_| arg.to_string())
    };
    let mut command = String::from("clickhousectl cloud");
    for arg in args.iter().copied().chain(["--org-id", org_id]) {
        command.push(' ');
        command.push_str(&quote(arg));
    }
    command
}

/// Human-mode note, on stderr, for a change the API accepted but applies in
/// the background, so the response printed with it still shows the old value
/// (#1241). `check` is the read to repeat; JSON mode carries `next` instead.
pub(crate) fn eprint_background_change(change: &str, check: &str) {
    eprintln!(
        "{change} applies in the background and can take several minutes; the values shown \
         may not include it yet. Check progress with: {check}"
    );
}

/// Serialize the library's own values, so a hint names exactly what the poll
/// command prints. Every value here is a field enum, a bool or a number, which
/// cannot fail to serialize.
fn hint_values<T: Serialize>(values: &[T]) -> Vec<Value> {
    values
        .iter()
        .filter_map(|value| serde_json::to_value(value).ok())
        .collect()
}

/// `value` as pretty JSON, with `next` added when `value` is an object.
pub(crate) fn json_with_next<T: Serialize>(
    value: &T,
    next: Option<NextStep>,
) -> serde_json::Result<String> {
    let mut value = serde_json::to_value(value)?;
    if let (Some(next), Some(object)) = (next, value.as_object_mut()) {
        object.insert("next".to_string(), serde_json::to_value(next)?);
    }
    serde_json::to_string_pretty(&value)
}

/// Serialize `value` and print it as an indented, human-readable tree.
///
/// - Object keys are printed verbatim (camelCase, as the API returns them).
/// - String values are unquoted.
/// - Arrays of scalars render inline (`key: [a, b, c]`); arrays of objects
///   render as `-` bullet blocks.
/// - Present empty objects, strings, and arrays remain visible as `{}`, `""`,
///   and `[]`; nulls are omitted.
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
    matches!(value, Value::Null)
}

fn is_scalar(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_))
}

fn scalar_string(value: &Value) -> Option<String> {
    match value {
        // The single place a string scalar becomes human text, so summarizing
        // PEM here covers object fields, nested objects and arrays of strings
        // alike.
        Value::String(s) if s.is_empty() => Some("\"\"".to_string()),
        Value::String(s) => Some(pem_summary(s).unwrap_or_else(|| s.clone())),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

// ── PEM summaries in human output (issue #665) ────────────────────────────
//
// A Postgres ClickPipe's `source.postgres.caCertificate` is a whole PEM block:
// roughly 1.5 KB of base64 that scrolls the rest of `clickpipe get` off the
// screen while telling the reader nothing. Human output therefore replaces
// every well-formed RFC 7468 block *in place* with a one-line summary of that
// block, keeping whatever text surrounds it — a bundle's header comments, an
// `openssl x509 -text` dump, prose between two blocks — so nothing the value
// carried is silently dropped. `--json` is untouched: it serializes the model
// directly, so a caller that needs the certificate still gets the bytes
// verbatim.
//
// The test is the value's *format*, not its key name. A client certificate, a
// private key, or a certificate nested anywhere else in any response gets the
// same treatment with no per-field allowlist to keep in sync.

/// The framing prefix that opens an RFC 7468 block.
const PEM_BEGIN: &str = "-----BEGIN ";
/// The five-dash run that opens and closes every framing line.
const PEM_DASHES: &str = "-----";
/// The longest label treated as a label. RFC 7468 sets no limit; this one, with
/// [`is_pem_label`], bounds how much of its own text a hostile value can paste
/// into a summary line.
const PEM_LABEL_MAX_LEN: usize = 64;
/// The labels whose body is an X.509 structure, and so the only labels this
/// module fingerprints: the digest is the one `openssl x509`, `openssl req` or
/// `openssl crl` prints with `-fingerprint -sha256`, so a reader can check the
/// value against the file they hold. Every other label — a private key above
/// all — is reported by size only. A stable identifier of secret material left
/// in scrollback is not something any caller needs.
const PEM_CERTIFICATE_LABELS: &[&str] = &[
    "CERTIFICATE",
    "TRUSTED CERTIFICATE",
    "CERTIFICATE REQUEST",
    "NEW CERTIFICATE REQUEST",
    "X509 CRL",
];

/// One well-formed RFC 7468 block found inside a string.
struct PemBlock<'a> {
    /// Byte range of the whole block, both framing lines included, in the
    /// string it was found in. Summarizing substitutes over exactly this span.
    span: std::ops::Range<usize>,
    label: &'a str,
    /// The raw text between the framing lines, base64 and any line breaks or
    /// RFC 1421 headers included. Never printed.
    body: &'a str,
}

/// RFC 7468 section 3's `label`, tightened for terminal output.
///
/// The grammar is `label = [ labelchar *( ["-" / SP ] labelchar ) ]` with
/// `labelchar = %x21-2C / %x2E-7E`: printable ASCII except the hyphen-minus the
/// framing itself is made of, with single hyphens and spaces allowed between
/// label characters. This accepts exactly that character set, rejects the empty
/// label the grammar permits, rejects a leading or trailing hyphen or space,
/// and caps the length.
///
/// The label is the one part of a block that reaches the summary line, so the
/// bound matters: a value carrying a newline, a control character, an ANSI
/// escape, non-ASCII bytes or a kilobyte of text where a label belongs is not a
/// block at all, and the string is then printed unchanged as it always was.
fn is_pem_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= PEM_LABEL_MAX_LEN
        && label.chars().all(|c| c == ' ' || c.is_ascii_graphic())
        && !label.starts_with([' ', '-'])
        && !label.ends_with([' ', '-'])
}

/// Find every well-formed RFC 7468 block in `text`, in order.
///
/// A block is a `BEGIN` frame with a valid label followed by an `END` frame
/// carrying the same label. A frame that fails either test is not a block: the
/// scan steps past that `-----BEGIN ` and keeps looking, so one malformed frame
/// cannot hide the blocks after it.
fn pem_blocks(text: &str) -> Vec<PemBlock<'_>> {
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find(PEM_BEGIN) {
        let begin = cursor + offset;
        let label_start = begin + PEM_BEGIN.len();
        // Whatever this frame turns out to be, never look at it again.
        cursor = label_start;
        let Some(label_len) = text[label_start..].find(PEM_DASHES) else {
            continue;
        };
        let label = &text[label_start..label_start + label_len];
        if !is_pem_label(label) {
            continue;
        }
        let body_start = label_start + label_len + PEM_DASHES.len();
        let end_marker = format!("{PEM_DASHES}END {label}{PEM_DASHES}");
        let Some(body_len) = text[body_start..].find(&end_marker) else {
            continue;
        };
        let body_end = body_start + body_len;
        blocks.push(PemBlock {
            span: begin..body_end + end_marker.len(),
            label,
            body: &text[body_start..body_end],
        });
        cursor = body_end + end_marker.len();
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

/// Summarize one block as `<PEM {label}, …>`.
///
/// A certificate-family block is identified by the SHA-256 fingerprint of its
/// own DER, so each block of a bundle is checkable on its own terms and a
/// mixed bundle never mislabels one block with another's label. Anything else,
/// and a certificate whose base64 does not decode, is reported as the size of
/// the block's body text. The body itself is never part of the result.
fn pem_block_summary(block: &PemBlock<'_>) -> String {
    let label = block.label;
    if PEM_CERTIFICATE_LABELS.contains(&label)
        && let Some(der) = pem_body_der(block.body)
    {
        return format!(
            "<PEM {label}, SHA-256 fingerprint {}>",
            sha256_fingerprint(&der)
        );
    }
    format!("<PEM {label}, {} bytes>", block.body.len())
}

/// Whether `blocks` account for all of `value` apart from whitespace.
fn pem_blocks_are_the_whole_value(value: &str, blocks: &[PemBlock<'_>]) -> bool {
    let mut cursor = 0;
    for block in blocks {
        if !value[cursor..block.span.start].trim().is_empty() {
            return false;
        }
        cursor = block.span.end;
    }
    value[cursor..].trim().is_empty()
}

/// Summarize the PEM blocks in `value` for human output, or return `None` when
/// `value` holds no well-formed block and should be printed as-is.
///
/// A value that is nothing but blocks and whitespace — the common case, a
/// single certificate or a chain — becomes one line of comma-separated
/// summaries. Any other value keeps its text and has each block replaced where
/// it stood, since text a bundle carries around its blocks (a `cacert.pem`
/// header, an `openssl x509 -text` dump, a byte-order mark) is the reader's to
/// keep and rendered as-is before this existed.
fn pem_summary(value: &str) -> Option<String> {
    let blocks = pem_blocks(value);
    if blocks.is_empty() {
        return None;
    }
    let summaries = blocks.iter().map(pem_block_summary);
    if pem_blocks_are_the_whole_value(value, &blocks) {
        return Some(summaries.collect::<Vec<_>>().join(", "));
    }
    let mut summarized = String::new();
    let mut cursor = 0;
    for (block, summary) in blocks.iter().zip(summaries) {
        summarized.push_str(&value[cursor..block.span.start]);
        summarized.push_str(&summary);
        cursor = block.span.end;
    }
    summarized.push_str(&value[cursor..]);
    Some(summarized)
}

/// Render any value at `indent`. The first line emitted (if any) is the natural
/// anchor a caller can retrofit a `-` bullet onto.
fn render(lines: &mut Vec<String>, indent: usize, value: &Value) {
    match value {
        Value::Object(map) if map.is_empty() => lines.push(format!("{}{{}}", pad(indent))),
        Value::Array(items) if items.is_empty() => lines.push(format!("{}[]", pad(indent))),
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
            Value::Object(inner) if inner.is_empty() => {
                lines.push(format!("{}{}: {{}}", pad(indent), key));
            }
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
    fn omits_null_but_renders_present_empty_values() {
        let v = json!({
            "name": "svc",
            "note": null,
            "empty": "",
            "tags": [],
            "meta": {},
            "count": 0,
            "flag": false
        });
        // Missing fields serialize as absent; null is omitted while explicit
        // empty values, 0 and false remain distinguishable.
        assert_eq!(
            render_to_string(&v),
            "name: svc\nempty: \"\"\ntags: []\nmeta: {}\ncount: 0\nflag: false"
        );
    }

    #[test]
    fn renders_explicit_empty_objects_at_every_depth() {
        assert_eq!(render_to_string(&json!({})), "{}");
        assert_eq!(render_to_string(&json!([])), "[]");
        assert_eq!(render_to_string(&json!("")), "\"\"");
        assert_eq!(
            render_to_string(&json!({"pgConfig": {}, "pgBouncerConfig": {}})),
            "pgConfig: {}\npgBouncerConfig: {}"
        );
        assert_eq!(render_to_string(&json!({"items": [{}]})), "items:\n  - {}");
        assert_eq!(render_to_string(&json!({"absent": {"value": null}})), "");
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
                "auth_required",
                "cancelled",
                "io",
                "transport",
                "http_4xx",
                "http_5xx",
                "sql_error",
                "service_stopped",
                "timeout",
                "rate_limited",
                "other",
                "query_timeout",
                "query_key_bound_rejected",
                "query_key_bind_forbidden",
                "resource_not_found",
                "udf_definition_invalid",
                "udf_source_invalid",
                "service_idle",
                "service_not_running",
                "clickpipe_still_pausing",
                "clickpipe_start_not_applied",
                "clickpipe_start_no_effect",
                "package_managed",
                "verification_failed",
                "invalid_version",
                "version_not_installed",
                "version_selection_required",
                "binary_not_launchable",
                "unsupported_client_version",
            ]
        );
    }

    #[test]
    fn package_managed_update_names_the_upgrade_command() {
        let error = crate::error::Error::PackageManagedInstall {
            manager: "npm",
            command: "npm install -g clickhousectl@latest",
        };
        let detail = CloudErrorDetail::from_error(&error);
        let value = serde_json::to_value(CloudErrorOutput { error: &detail }).unwrap();
        assert_eq!(
            value,
            json!({"error": {
                "code": "package_managed",
                "message": error.to_string(),
                "command": "npm install -g clickhousectl@latest",
            }})
        );
        assert_eq!(error.exit_code(), 1);
    }

    #[test]
    fn cloud_runtime_errors_preserve_the_message_and_exit_code() {
        use crate::error::Error;
        for (error, code, exit_code) in [
            (
                Error::AuthRequired("login required".into()),
                "auth_required",
                4,
            ),
            (Error::Cancelled, "cancelled", 3),
            (Error::Skills("validation failed".into()), "other", 1),
            (
                Error::UpdateCheck {
                    message: "timed out".into(),
                    failure: crate::failure::FailureKind::Timeout,
                },
                "timeout",
                1,
            ),
            (
                Error::UpdateCheck {
                    message: "unreachable".into(),
                    failure: crate::failure::FailureKind::Transport,
                },
                "transport",
                1,
            ),
            (Error::Io(std::io::Error::other("read failed")), "io", 1),
            (
                Error::ArchiveVerification("digest differs".into()),
                "verification_failed",
                1,
            ),
        ] {
            let detail = CloudErrorDetail::from_error(&error);
            let value = serde_json::to_value(CloudErrorOutput { error: &detail }).unwrap();
            assert_eq!(
                value,
                json!({"error": {"code": code, "message": error.to_string()}})
            );
            assert_eq!(error.exit_code(), exit_code);
        }
    }

    /// `cloud service client` runs the local client pre-flight, so a missing,
    /// ambiguous, broken or too-old build gets the same typed code and
    /// runnable repair `local client` reports, not `other` (#1443).
    #[test]
    fn local_client_preflight_errors_carry_a_typed_code_and_runnable_command() {
        use crate::error::{BinaryLaunchProblem, Error};
        let not_launchable = |problem| Error::BinaryNotLaunchable {
            version: "25.12.1.1".into(),
            problem,
            path: "/home/u/.clickhouse/versions/25.12.1.1/clickhouse".into(),
        };
        for (error, code, command) in [
            (
                Error::InvalidVersion("bad".into()),
                "invalid_version",
                "clickhousectl local install --help",
            ),
            (
                Error::VersionNotFound("25.12.1.1".into()),
                "version_not_installed",
                "clickhousectl local list",
            ),
            (
                Error::StaleDefaultVersion("25.12.1.1".into()),
                "version_not_installed",
                "clickhousectl local list",
            ),
            (
                Error::NoClientVersionInstalled,
                "version_not_installed",
                "clickhousectl local install latest",
            ),
            (
                Error::ClientVersionNotInstalled("25.12".into()),
                "version_not_installed",
                "clickhousectl local install 25.12",
            ),
            (
                Error::NoDefaultVersion,
                "version_selection_required",
                "clickhousectl local list",
            ),
            (
                Error::AmbiguousClientVersion,
                "version_selection_required",
                "clickhousectl local list",
            ),
            (
                not_launchable(BinaryLaunchProblem::NotExecutable),
                "binary_not_launchable",
                "clickhousectl local install --force 25.12.1.1",
            ),
            (
                not_launchable(BinaryLaunchProblem::NotAFile),
                "binary_not_launchable",
                "clickhousectl local remove 25.12.1.1 && clickhousectl local install 25.12.1.1",
            ),
            (
                Error::ClientLoginUnsupported {
                    version: "25.11.1.1".into(),
                    minimum: "25.12",
                },
                "unsupported_client_version",
                "clickhousectl local install latest",
            ),
        ] {
            let detail = CloudErrorDetail::from_error(&error);
            let value = serde_json::to_value(CloudErrorOutput { error: &detail }).unwrap();
            assert_eq!(
                value,
                json!({"error": {
                    "code": code,
                    "message": error.to_string(),
                    "command": command,
                }}),
                "{error:?}"
            );
            assert_eq!(error.exit_code(), 1);
            for step in command.split(" && ") {
                let argv = shlex::split(step).expect("shell-quoted command");
                match <crate::cli::Cli as clap::Parser>::try_parse_from(&argv) {
                    Ok(cli) => assert!(
                        matches!(cli.command, crate::cli::Commands::Local(_)),
                        "{step}"
                    ),
                    Err(e) => assert_eq!(e.kind(), clap::error::ErrorKind::DisplayHelp, "{step}"),
                }
            }
        }
    }

    #[test]
    fn cloud_error_detail_omits_absent_fields() {
        let detail = CloudErrorDetail {
            code: CloudErrorCode::QueryKeyBoundRejected,
            message: "rejected".into(),
            host: None,
            port: None,
            command: Some("clickhousectl cloud service query-endpoint get svc-1".into()),
            api_key_id: Some("key-1".into()),
        };
        let value = serde_json::to_value(CloudErrorOutput { error: &detail }).unwrap();
        assert_eq!(
            value,
            json!({
                "error": {
                    "code": "query_key_bound_rejected",
                    "message": "rejected",
                    "command": "clickhousectl cloud service query-endpoint get svc-1",
                    "api_key_id": "key-1",
                }
            })
        );
    }

    #[test]
    fn next_step_serializes_a_runnable_poll() {
        let next = NextStep::poll(&["service", "get", "svc 1"], "org-1", "state")
            .until(&["running"])
            .every(5);
        assert_eq!(
            serde_json::to_value(&next).unwrap(),
            json!({
                "command": "clickhousectl cloud service get 'svc 1' --org-id org-1 --json",
                "field": "state",
                "until": ["running"],
                "fail_on": [],
                "interval_seconds": 5,
            })
        );
        let gone = next.until_error_code(CloudErrorCode::ResourceNotFound);
        assert_eq!(
            serde_json::to_value(&gone).unwrap()["until_error_code"],
            "resource_not_found"
        );
    }

    #[test]
    fn json_with_next_adds_next_to_objects_only() {
        let next = NextStep::poll(&["service", "get", "svc-1"], "org-1", "state");
        let with: Value = serde_json::from_str(
            &json_with_next(&json!({"id": "svc-1"}), Some(next.clone())).unwrap(),
        )
        .unwrap();
        assert_eq!(with["id"], "svc-1");
        assert_eq!(with["next"]["field"], "state");
        for (value, next) in [(json!({"id": "svc-1"}), None), (json!(["a"]), Some(next))] {
            let printed: Value =
                serde_json::from_str(&json_with_next(&value, next).unwrap()).unwrap();
            assert_eq!(printed, value);
        }
    }

    // ── PEM summaries (issue #665) ─────────────────────────────────────────

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
    /// ever appears in human output, the certificate was not summarized.
    const PEM_FIXTURE_BODY_MARKER: &str =
        "ByqGSM49AgEGCCqGSM49AwEHA0IABNTPygUG2umVvTqod5jJXCgp1o9qwrx2wLf7";

    /// A private key over the four bytes `01 02 03 04`, whose SHA-256 is the
    /// constant below. Its presence in a summary would mean a key was
    /// fingerprinted.
    const PEM_KEY_FIXTURE: &str =
        "-----BEGIN EC PRIVATE KEY-----\nAQIDBA==\n-----END EC PRIVATE KEY-----\n";
    const FINGERPRINT_OF_01020304: &str = "9F:64:A7:47:E1:B9:7F:13:1F:AB:B6:B4:47:29:6C:9B:6F:02:01:E7:9F:B3:C5:35:6E:6C:77:E8:9B:6A:80:6A";

    /// The summary [`PEM_FIXTURE`] renders as, wherever it appears.
    fn fixture_summary() -> String {
        format!("<PEM CERTIFICATE, SHA-256 fingerprint {PEM_FIXTURE_FINGERPRINT}>")
    }

    #[test]
    fn pem_summary_reports_the_label_and_the_openssl_fingerprint() {
        assert_eq!(pem_summary(PEM_FIXTURE).unwrap(), fixture_summary());
    }

    #[test]
    fn pem_summary_never_contains_the_body() {
        let summary = pem_summary(PEM_FIXTURE).unwrap();
        assert!(
            !summary.contains(PEM_FIXTURE_BODY_MARKER),
            "body leaked: {summary}"
        );
        assert!(!summary.contains(PEM_DASHES), "framing leaked: {summary}");
    }

    #[test]
    fn pem_summary_puts_a_whitespace_separated_chain_on_one_line() {
        // The common case: nothing but blocks, so the reader gets one line
        // with one summary per certificate of the chain.
        let chain = format!("{PEM_FIXTURE}\n{PEM_FIXTURE}\n{PEM_FIXTURE}");
        let summary = pem_summary(&chain).unwrap();
        let expected = fixture_summary();
        assert_eq!(summary, format!("{expected}, {expected}, {expected}"));
        assert!(!summary.contains('\n'), "chain spans lines: {summary}");
    }

    #[test]
    fn pem_summary_never_fingerprints_a_key_in_a_mixed_bundle() {
        // Each block is summarized on its own terms: the certificate by its
        // own fingerprint, the key by size and never by digest.
        let bundle = format!("{PEM_FIXTURE}{PEM_KEY_FIXTURE}");
        let summary = pem_summary(&bundle).unwrap();
        assert_eq!(
            summary,
            format!("{}, <PEM EC PRIVATE KEY, 10 bytes>", fixture_summary())
        );
        assert!(
            !summary.contains(FINGERPRINT_OF_01020304),
            "the key was fingerprinted: {summary}"
        );
    }

    #[test]
    fn pem_summary_keeps_text_before_a_block() {
        // A `cacert.pem`-style bundle: header comments, then the block. The
        // comments are the reader's to keep; only the block is summarized.
        let bundle = format!(
            "## Bundle of CA Root Certificates\n## Certificate data from Mozilla\n\n{PEM_FIXTURE}"
        );
        assert_eq!(
            pem_summary(&bundle).unwrap(),
            format!(
                "## Bundle of CA Root Certificates\n## Certificate data from Mozilla\n\n{}\n",
                fixture_summary()
            )
        );
    }

    #[test]
    fn pem_summary_keeps_text_between_two_blocks() {
        let bundle = format!(
            "{}\nissued by the internal CA\n{}\n",
            PEM_FIXTURE.trim(),
            PEM_FIXTURE.trim()
        );
        let expected = fixture_summary();
        assert_eq!(
            pem_summary(&bundle).unwrap(),
            format!("{expected}\nissued by the internal CA\n{expected}\n")
        );
    }

    #[test]
    fn pem_summary_keeps_a_byte_order_mark() {
        // A BOM is not whitespace, so the value is not blocks-only: the mark
        // survives and the block is still summarized where it stood.
        let bom_prefixed = format!("\u{feff}{PEM_FIXTURE}");
        assert_eq!(
            pem_summary(&bom_prefixed).unwrap(),
            format!("\u{feff}{}\n", fixture_summary())
        );
    }

    #[test]
    fn a_frame_whose_label_is_not_a_label_is_not_a_block() {
        // The label is the only part of a block that reaches the summary line.
        // A newline, a control character or an unbounded run of text where a
        // label belongs must not produce a summary at all: no synthesized
        // line, nothing of the value's own injected into one.
        let long_label = "A".repeat(200);
        for value in [
            "-----BEGIN CERTIFICATE\nx-----\nAQIDBA==\n-----END CERTIFICATE\nx-----".to_string(),
            "-----BEGIN \u{1b}[31mCERTIFICATE-----\nAQIDBA==\n-----END \u{1b}[31mCERTIFICATE-----"
                .to_string(),
            format!("-----BEGIN {long_label}-----\nAQIDBA==\n-----END {long_label}-----"),
        ] {
            assert_eq!(
                pem_summary(&value),
                None,
                "unexpectedly summarized {value:?}"
            );
            let rendered = render_to_string(&json!({ "caCertificate": value }));
            assert!(
                !rendered.contains("<PEM"),
                "synthesized a summary: {rendered}"
            );
            assert_eq!(rendered, format!("caCertificate: {value}"));
        }
    }

    #[test]
    fn pem_summary_handles_crlf_line_endings() {
        let crlf = PEM_FIXTURE.replace('\n', "\r\n");
        assert_eq!(pem_summary(&crlf).unwrap(), fixture_summary());
    }

    #[test]
    fn pem_summary_tolerates_surrounding_whitespace() {
        let padded = format!("\n  {}\n\n", PEM_FIXTURE.trim());
        assert_eq!(pem_summary(&padded).unwrap(), fixture_summary());
    }

    #[test]
    fn pem_summary_falls_back_to_a_byte_count_when_the_body_is_not_base64() {
        let broken = "-----BEGIN CERTIFICATE-----\n**not base64**\n-----END CERTIFICATE-----";
        assert_eq!(pem_summary(broken).unwrap(), "<PEM CERTIFICATE, 16 bytes>");
    }

    #[test]
    fn pem_summary_reports_rfc_1421_headers_as_a_byte_count_only() {
        // An encrypted key carries `Proc-Type`/`DEK-Info` headers before the
        // base64, so the body does not decode. The count stands in for it and
        // no header text reaches the summary.
        let encrypted = "-----BEGIN RSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\nDEK-Info: AES-128-CBC,0123456789ABCDEF\n\nAQIDBA==\n-----END RSA PRIVATE KEY-----";
        let summary = pem_summary(encrypted).unwrap();
        assert_eq!(summary, "<PEM RSA PRIVATE KEY, 73 bytes>");
        assert!(!summary.contains("Proc-Type"), "header leaked: {summary}");
        assert!(!summary.contains("DEK-Info"), "header leaked: {summary}");
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
            // No space after `BEGIN`, so there is no frame at all.
            "-----BEGIN-----\nAQIDBA==\n-----END-----",
        ] {
            assert_eq!(
                pem_summary(value),
                None,
                "unexpectedly summarized {value:?}"
            );
        }
    }

    #[test]
    fn render_summarizes_a_certificate_field_and_keeps_its_siblings() {
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
                "source:\n  postgres:\n    host: db.example.com\n    caCertificate: {}\n    database: postgres",
                fixture_summary()
            )
        );
        assert!(
            !rendered.contains(PEM_BEGIN),
            "certificate body rendered: {rendered}"
        );
    }

    #[test]
    fn render_summarizes_certificates_inside_arrays_and_bullets() {
        let v = json!({
            "trustChain": [PEM_FIXTURE, PEM_FIXTURE],
            "endpoints": [{ "clientCertificate": PEM_FIXTURE }],
        });
        let rendered = render_to_string(&v);
        assert!(!rendered.contains(PEM_BEGIN), "body rendered: {rendered}");
        assert_eq!(rendered.matches(&fixture_summary()).count(), 3);
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
