//! Serde helpers used by generated models.

use serde::{Deserialize, Deserializer};

/// Deserialize a `Vec<T>` field, treating an explicit JSON `null` the same as
/// an empty array. Required because the ClickHouse Cloud API emits `null` for
/// some array-valued fields that its OpenAPI spec declares as non-nullable
/// `array`s (e.g. `reversePrivateEndpointIds` on Kafka sources). With plain
/// `#[serde(default)]`, a missing field works but an explicit `null` fails
/// with "invalid type: null, expected a sequence".
pub fn null_to_empty<'de, T, D>(d: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<Vec<T>>::deserialize(d).map(Option::unwrap_or_default)
}

/// Deserialize a buffered payload into `T`, handing the payload back unchanged
/// if it does not fit `T`.
///
/// Used by the `discriminated_union!` macro in `models.rs` so a union whose
/// discriminator selects a variant the rest of the payload no longer fits
/// degrades to that union's `Unknown(serde_json::Value)` catch-all instead of
/// failing the whole response. Field-level tolerance covers a field the API
/// stops sending; this covers a field whose *shape* the API changes.
pub fn deserialize_or_raw<T>(value: serde_json::Value) -> Result<T, serde_json::Value>
where
    T: serde::de::DeserializeOwned,
{
    match serde_json::from_value(value.clone()) {
        Ok(parsed) => Ok(parsed),
        Err(_) => Err(value),
    }
}
