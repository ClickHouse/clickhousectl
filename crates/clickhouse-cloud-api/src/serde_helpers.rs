//! Serde helpers used by generated models.

use serde::Deserialize;

use crate::models::{ApiResponse, ServiceEndpoint, ServiceEndpointProtocol};

/// Deserialize an optional integer from either JSON integer syntax or an
/// integral JSON float such as `200.0`.
pub fn deserialize_optional_integral_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    const MAX_SAFE_FLOAT_INTEGER_EXCLUSIVE: f64 = (1_u64 << f64::MANTISSA_DIGITS) as f64;

    Option::<serde_json::Number>::deserialize(deserializer)?
        .map(|number| {
            if let Some(value) = number.as_i64() {
                return Ok(value);
            }
            if let Some(value) = number.as_u64() {
                return i64::try_from(value).map_err(serde::de::Error::custom);
            }
            if let Some(value) = number.as_f64()
                && value.fract() == 0.0
                && value > -MAX_SAFE_FLOAT_INTEGER_EXCLUSIVE
                && value < MAX_SAFE_FLOAT_INTEGER_EXCLUSIVE
            {
                return Ok(value as i64);
            }
            Err(serde::de::Error::custom(format!(
                "expected an integer-valued i64, got {number}"
            )))
        })
        .transpose()
}

#[derive(Deserialize)]
pub(crate) struct ServiceEndpointWire {
    host: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_integral_i64")]
    port: Option<i64>,
    protocol: Option<ServiceEndpointProtocol>,
    username: Option<String>,
}

impl From<ServiceEndpointWire> for ServiceEndpoint {
    fn from(value: ServiceEndpointWire) -> Self {
        Self {
            host: value.host,
            port: value.port,
            protocol: value.protocol,
            username: value.username,
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct ApiResponseWire<T> {
    #[serde(default, deserialize_with = "deserialize_optional_integral_i64")]
    status: Option<i64>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    result: Option<T>,
    error: Option<String>,
}

impl<T> From<ApiResponseWire<T>> for ApiResponse<T> {
    fn from(value: ApiResponseWire<T>) -> Self {
        Self {
            status: value.status,
            request_id: value.request_id,
            result: value.result,
            error: value.error,
        }
    }
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
