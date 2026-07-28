use serde::{Deserialize, Serialize};

/// Delete service success payload returned directly by the API without a result wrapper.
///
/// Fields the API omitted stay `None` and are omitted from `--json` output,
/// matching the library-wide all-`Option` response policy — never fabricate a
/// `0.0` status or empty request ID the server did not send.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}
