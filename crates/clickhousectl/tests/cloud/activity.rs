//! `cloud activity` (src/cloud/activity.rs).

use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

/// The wiring test for `activity list`: the default start is computed from the
/// named `--to-date`, and the API is sent the day after it. Both date rules are
/// unit tests in `cloud::activity`.
#[tokio::test]
async fn activity_list_without_from_date_covers_the_seven_days_ending_on_to_date() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/activities"))
        .and(query_param("from_date", "2024-02-24"))
        .and(query_param("to_date", "2024-03-02"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "requestId": "stub-activity-list",
            "result": []
        })))
        .expect(1)
        .mount(&mock)
        .await;

    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "activity",
            "list",
            "--org-id",
            "org-1",
            "--to-date",
            "2024-03-01",
        ],
    );

    assert_success(&output);
}
