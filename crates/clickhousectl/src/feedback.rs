//! `clickhousectl feedback` (#1211): send a short free-text message to the
//! clickhousectl team.
//!
//! Unlike telemetry, the payload deliberately carries text the user typed, so
//! running the command is the consent: it is not gated by the telemetry
//! preference or `DO_NOT_TRACK`, which control passive collection only. The
//! message is validated here so the user sees why it was refused, and the
//! ingest worker sanitises it again on arrival; neither side relies on the
//! other's checks.
//!
//! The send is a synchronous POST through [`crate::http::client_builder`], so
//! the user learns whether it arrived and the request carries the same
//! User-Agent and agent session/trace headers as every other request this CLI
//! makes. Delivery is decided from the HTTP status alone, never from the
//! response text.

use std::time::Duration;

use unicode_normalization::UnicodeNormalization;

use crate::cli::FeedbackArgs;
use crate::client_facts::{ClientFacts, real_env_lookup};
use crate::error::{Error, Result};

/// Production ingest endpoint (the same Cloudflare worker as telemetry).
const DEFAULT_ENDPOINT: &str = "https://chctl.clickhouse.com/v1/feedback";

/// Overrides the ingest endpoint (integration tests, local worker dev).
const URL_ENV: &str = "CHCTL_FEEDBACK_URL";

/// Bounds the whole send; tests shorten it with
/// [`crate::timing::FEEDBACK_SEND_TIMEOUT_ENV`].
const SEND_TIMEOUT: Duration = Duration::from_secs(10);

/// Longest accepted message, in Unicode scalar values after trimming and NFC
/// normalisation. The worker enforces the same cap on the same text, counted
/// the same way.
const MAX_MESSAGE_CHARS: usize = 500;

/// The wire payload: one JSON object, never an array.
#[derive(serde::Serialize)]
struct Payload<'a> {
    message: &'a str,
    /// `is_agent`, `agent`, `ci`, `version`, `os`, `arch`.
    #[serde(flatten)]
    client: ClientFacts,
}

/// Why a message is refused before anything is sent.
#[derive(Debug, PartialEq, Eq)]
enum InvalidMessage {
    Empty,
    TooLong(usize),
    ControlCharacter,
}

impl std::fmt::Display for InvalidMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("the feedback message is empty"),
            Self::TooLong(chars) => write!(
                f,
                "the feedback message is {chars} characters long; the limit is {MAX_MESSAGE_CHARS}"
            ),
            Self::ControlCharacter => f.write_str(
                "the feedback message contains a control character; only newline and tab are allowed",
            ),
        }
    }
}

/// The message as it will be sent: trimmed, NFC-normalised (as the worker
/// stores it, so the cap is measured on the stored text), non-empty, at most
/// [`MAX_MESSAGE_CHARS`] scalar values, and free of control characters other
/// than `\n` and `\t`.
fn validate(message: &str) -> std::result::Result<String, InvalidMessage> {
    let message: String = message.trim().nfc().collect();
    if message.is_empty() {
        return Err(InvalidMessage::Empty);
    }
    if message
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(InvalidMessage::ControlCharacter);
    }
    let chars = message.chars().count();
    if chars > MAX_MESSAGE_CHARS {
        return Err(InvalidMessage::TooLong(chars));
    }
    Ok(message)
}

/// Why the feedback service did not accept a request, from its HTTP status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rejection {
    /// 400: the service found the message invalid.
    Invalid,
    /// 413: the request was too large. The worker answers 413 both for an
    /// oversized body and for an over-length message, so this does not claim
    /// which.
    TooLarge,
    /// 429: rate limited.
    RateLimited,
    /// 5xx.
    Unavailable,
    /// Any other unsuccessful status.
    Refused,
}

impl Rejection {
    fn from_status(status: u16) -> Self {
        match status {
            400 => Self::Invalid,
            413 => Self::TooLarge,
            429 => Self::RateLimited,
            500.. => Self::Unavailable,
            _ => Self::Refused,
        }
    }
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Invalid => "the feedback service rejected the message as invalid",
            Self::TooLarge => "the request was too large for the feedback service",
            Self::RateLimited => "too many feedback messages were sent recently; try again later",
            Self::Unavailable => "the feedback service is unavailable; try again later",
            Self::Refused => "the feedback service refused the request",
        })
    }
}

/// The error for an HTTP status that means the message was not accepted.
fn rejection(status: u16) -> Error {
    let reason = Rejection::from_status(status);
    Error::Feedback(format!("Feedback was not sent: {reason} (HTTP {status})"))
}

/// A timeout for a human: whole seconds in production, milliseconds when a
/// test override is not a whole second.
fn describe_timeout(timeout: Duration) -> String {
    if timeout.subsec_nanos() == 0 {
        format!("{}s", timeout.as_secs())
    } else {
        format!("{}ms", timeout.as_millis())
    }
}

pub async fn run(args: FeedbackArgs, json: bool) -> Result<()> {
    let joined = args.message.join(" ");
    let message = validate(&joined).map_err(|invalid| {
        Error::Usage(Box::new(clap::Error::raw(
            clap::error::ErrorKind::ValueValidation,
            format!("{invalid}\n"),
        )))
    })?;
    let payload = Payload {
        message: &message,
        client: ClientFacts::detect(&real_env_lookup),
    };
    let url = std::env::var(URL_ENV).unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string());
    send(&url, &payload).await?;

    let length = message.chars().count();
    if json {
        #[derive(serde::Serialize)]
        struct Output {
            sent: bool,
            length: usize,
        }
        let output = Output { sent: true, length };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Thanks! Your feedback was sent to the clickhousectl team.");
    }
    Ok(())
}

async fn send(url: &str, payload: &Payload<'_>) -> Result<()> {
    let timeout =
        crate::timing::duration_override(crate::timing::FEEDBACK_SEND_TIMEOUT_ENV, SEND_TIMEOUT);
    let client = crate::http::client_builder().timeout(timeout).build()?;
    let response = client.post(url).json(payload).send().await.map_err(|e| {
        Error::Feedback(if e.is_timeout() {
            format!(
                "Feedback was not sent: the feedback service did not respond within {}",
                describe_timeout(timeout)
            )
        } else {
            format!("Feedback was not sent: could not reach the feedback service: {e}")
        })
    })?;
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        Err(rejection(status.as_u16()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_whitespace_only_messages_are_refused() {
        for message in ["", " ", "   \n\t  "] {
            assert_eq!(validate(message), Err(InvalidMessage::Empty), "{message:?}");
        }
    }

    #[test]
    fn the_message_is_trimmed_before_it_is_checked() {
        assert_eq!(
            validate("  needs a wait flag \n").as_deref(),
            Ok("needs a wait flag")
        );
        let padded = format!("  {}  ", "a".repeat(MAX_MESSAGE_CHARS));
        assert_eq!(validate(&padded).map(|m| m.len()), Ok(MAX_MESSAGE_CHARS));
    }

    #[test]
    fn the_cap_counts_unicode_scalar_values_not_bytes() {
        for unit in ["a", "é", "日", "😀"] {
            let at_cap = unit.repeat(MAX_MESSAGE_CHARS);
            assert_eq!(validate(&at_cap).as_deref(), Ok(at_cap.as_str()), "{unit}");
            let over = unit.repeat(MAX_MESSAGE_CHARS + 1);
            assert_eq!(
                validate(&over),
                Err(InvalidMessage::TooLong(MAX_MESSAGE_CHARS + 1)),
                "{unit}"
            );
        }
    }

    #[test]
    fn control_characters_other_than_newline_and_tab_are_refused() {
        for c in [
            '\0', '\u{7}', '\u{8}', '\r', '\u{b}', '\u{1b}', '\u{7f}', '\u{85}', '\u{9b}',
        ] {
            let message = format!("before{c}after");
            assert_eq!(
                validate(&message),
                Err(InvalidMessage::ControlCharacter),
                "{c:?}"
            );
        }
        assert_eq!(
            validate("line one\nline two\tindented").as_deref(),
            Ok("line one\nline two\tindented")
        );
    }

    #[test]
    fn a_control_character_is_refused_even_in_an_overlong_message() {
        let message = format!("{}\u{1b}", "a".repeat(MAX_MESSAGE_CHARS + 1));
        assert_eq!(validate(&message), Err(InvalidMessage::ControlCharacter));
    }

    #[test]
    fn the_cap_is_measured_after_nfc_expansion() {
        // Composition exclusions: NFC leaves each of these decomposed, so
        // U+0958 becomes two scalar values and U+FB2C three.
        for (unit, expanded) in [("\u{0958}", 2), ("\u{FB2C}", 3)] {
            assert_eq!(
                validate(unit).map(|m| m.chars().count()),
                Ok(expanded),
                "{unit:?}"
            );
            let fits = unit.repeat(MAX_MESSAGE_CHARS / expanded);
            let fits_chars = MAX_MESSAGE_CHARS / expanded * expanded;
            assert_eq!(
                validate(&fits).map(|m| m.chars().count()),
                Ok(fits_chars),
                "{unit:?}"
            );
            let over = unit.repeat(MAX_MESSAGE_CHARS / expanded + 1);
            assert_eq!(
                validate(&over),
                Err(InvalidMessage::TooLong(fits_chars + expanded)),
                "{unit:?}"
            );
        }
    }

    #[test]
    fn decomposed_text_is_composed_before_it_is_counted_and_sent() {
        let decomposed = "e\u{301}";
        assert_eq!(decomposed.chars().count(), 2);
        assert_eq!(validate(decomposed).as_deref(), Ok("\u{e9}"));
        // Twice the cap in scalar values before NFC, exactly the cap after.
        let at_cap = decomposed.repeat(MAX_MESSAGE_CHARS);
        assert_eq!(
            validate(&at_cap).map(|m| m.chars().count()),
            Ok(MAX_MESSAGE_CHARS)
        );
    }

    #[test]
    fn each_unsuccessful_status_maps_to_its_rejection() {
        for (status, expected) in [
            (400, Rejection::Invalid),
            (413, Rejection::TooLarge),
            (429, Rejection::RateLimited),
            (500, Rejection::Unavailable),
            (502, Rejection::Unavailable),
            (503, Rejection::Unavailable),
            (599, Rejection::Unavailable),
            (401, Rejection::Refused),
            (403, Rejection::Refused),
            (404, Rejection::Refused),
            (302, Rejection::Refused),
        ] {
            assert_eq!(Rejection::from_status(status), expected, "{status}");
        }
    }

    #[test]
    fn a_rejection_is_a_feedback_error_naming_the_status() {
        for status in [400, 403, 413, 429, 503] {
            let Error::Feedback(message) = rejection(status) else {
                panic!("{status} is not a feedback error");
            };
            assert!(message.contains(&format!("HTTP {status}")), "{message}");
        }
    }

    #[test]
    fn the_payload_keys_follow_the_wire_contract() {
        let payload = Payload {
            message: "m",
            client: ClientFacts::detect(&|_| None),
        };
        let value = serde_json::to_value(&payload).unwrap();
        let keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "message", "is_agent", "agent", "ci", "version", "os", "arch"
            ]
        );
        assert_eq!(value["message"], "m");
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["os"], std::env::consts::OS);
        assert_eq!(value["arch"], std::env::consts::ARCH);
        assert_eq!(value["ci"], false);
    }
}
