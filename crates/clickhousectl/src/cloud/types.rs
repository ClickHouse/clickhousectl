use serde::{Deserialize, Serialize};

/// Delete service success payload returned directly by the API without a result wrapper.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResponse {
    pub status: f64,
    pub request_id: String,
}

/// Body for POST {queries_host}/service/{service_id}/run.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunQueryRequest {
    pub run_id: String,
    pub sql: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
}
