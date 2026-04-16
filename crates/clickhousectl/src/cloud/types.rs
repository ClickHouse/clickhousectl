use serde::{Deserialize, Serialize};

/// Delete service success payload returned directly by the API without a result wrapper.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResponse {
    pub status: f64,
    pub request_id: String,
}

/// Request body for the query-run endpoint at `queries.clickhouse.cloud`.
///
/// Unlike the OpenAPI spec types, this endpoint lives outside the main Cloud API
/// and takes a JSON body with run-scoped fields. Content-type is `text/plain`
/// (the server is lenient about this) but the body is JSON.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunQueryRequest<'a> {
    pub run_id: String,
    pub sql: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<&'a str>,
}
