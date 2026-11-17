use wiremock::matchers::{basic_auth, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::setup;

// ===========================================================================
// Organization roles
// ===========================================================================

#[tokio::test]
async fn list_roles_pagination_encodes_parameters_and_preserves_envelope() {
    let (server, client) = setup().await;
    let cursor = "opaque +/=&?雪";
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/roles"))
        .and(basic_auth("key", "secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200, "requestId": "req-page",
            "result": [{"id": "role-1", "name": "Admin", "type": "system"}],
            "limit": 25, "nextCursor": cursor
        })))
        .expect(4)
        .mount(&server)
        .await;
    for (limit, cursor_arg) in [
        (None, None),
        (Some(25), None),
        (None, Some(cursor)),
        (Some(25), Some(cursor)),
    ] {
        let response = client
            .organization_roles_get_list("org-1", limit, cursor_arg)
            .await
            .unwrap();
        let roles = response.result.unwrap();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0].id.as_deref(), Some("role-1"));
        assert_eq!(response.limit, Some(25));
        assert_eq!(response.next_cursor.as_deref(), Some(cursor));
    }
    let requests = server.received_requests().await.unwrap();
    let queries: Vec<Vec<(String, String)>> = requests
        .iter()
        .map(|request| request.url.query_pairs().into_owned().collect())
        .collect();
    assert_eq!(
        queries,
        vec![
            vec![],
            vec![("limit".into(), "25".into())],
            vec![("cursor".into(), cursor.into())],
            vec![
                ("limit".into(), "25".into()),
                ("cursor".into(), cursor.into())
            ],
        ]
    );
}

#[tokio::test]
async fn list_roles_last_page_has_no_next_cursor() {
    let (server, client) = setup().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/org-1/roles"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": [], "limit": 250, "nextCursor": null
        })))
        .mount(&server)
        .await;
    let response = client
        .organization_roles_get_list("org-1", None, None)
        .await
        .unwrap();
    assert_eq!(response.result, Some(vec![]));
    assert_eq!(response.next_cursor, None);
}
