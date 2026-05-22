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
