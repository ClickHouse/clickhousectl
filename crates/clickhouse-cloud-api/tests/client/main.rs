mod activity;
mod api_keys;
mod backups;
mod byoc;
mod clickpipes;
mod clickstack;
mod client;
mod invitations;
mod members;
mod organizations;
mod postgres;
mod query_api_endpoints;
mod saved_queries;
mod services;
mod udfs;
mod whoami;

use clickhouse_cloud_api::Client;
use wiremock::{MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn ok_json(result: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "req-test",
        "result": result
    }))
}

fn ok_empty() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "status": 200,
        "requestId": "req-test"
    }))
}

fn created_json(result: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(201).set_body_json(serde_json::json!({
        "status": 201,
        "requestId": "req-test",
        "result": result
    }))
}

async fn setup() -> (MockServer, Client) {
    let s = MockServer::start().await;
    let c = Client::with_base_url(s.uri(), "key", "secret");
    (s, c)
}
