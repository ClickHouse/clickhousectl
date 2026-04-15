use serde::{Deserialize, Serialize};

/// Delete service success payload returned directly by the API without a result wrapper.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResponse {
    pub status: f64,
    pub request_id: String,
}
