//! Serde helpers used by generated models.

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
