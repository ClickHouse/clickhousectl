//! Explicit conversions from response models back into request models.
//!
//! Response models are tolerant by construction: every field is `Option<T>`, so
//! a field the API drops or sends as `null` deserializes to `None` instead of
//! failing. Request models are strict: a field the API requires is `T`.
//!
//! A caller that fetches a resource, edits it, and writes it back therefore has
//! to resolve absence explicitly, and that is the point of this module — the old
//! `#[serde(default)]` policy silently fabricated `""`/`0`/`false` for a dropped
//! field and persisted it on the next write. A conversion that can lose that
//! information is a [`TryFrom`] reporting the missing wire field names; a
//! conversion that cannot is a [`From`].
//!
//! The ClickStack source tree converts as a group:
//! [`TryFrom<crate::models::ClickStackSourceResponse>`] for
//! [`crate::models::ClickStackSource`] is the entry point, and every nested object
//! in that tree carries its own conversion so a missing field is named at the
//! level it is missing from.

use std::fmt;

mod clickstack;
mod postgres;
mod service;
mod shared;

/// The response omitted fields that the matching request model requires.
///
/// Field names are the wire (spec) names, so an error message points at the
/// JSON the API returned rather than at Rust identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingRequiredFields {
    fields: Vec<&'static str>,
}

impl MissingRequiredFields {
    pub(crate) fn new(fields: Vec<&'static str>) -> Self {
        Self { fields }
    }

    /// The missing wire field names, in declaration order.
    pub fn fields(&self) -> &[&'static str] {
        &self.fields
    }
}

impl fmt::Display for MissingRequiredFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the API response is missing required field(s): {}",
            self.fields.join(", ")
        )
    }
}

impl std::error::Error for MissingRequiredFields {}
