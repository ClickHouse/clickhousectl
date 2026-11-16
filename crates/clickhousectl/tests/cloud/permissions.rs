//! Permission help derived from the declared OpenAPI calls (src/cloud/permissions.rs).

use std::process::Command;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

#[tokio::test]
async fn permission_help_is_offline_and_exposes_multi_permission_operations() {
    let mock = MockServer::start().await;
    for flag in ["-h", "--help"] {
        let help = permission_help(&mock, &["postgres", "read-replica", "create"], flag);
        let advertised = advertised_permission_ids(&help);
        let required =
            clickhouse_cloud_api::meta::operations::POSTGRES_INSTANCE_CREATE_READ_REPLICA
                .required_permissions;
        assert!(required.len() > 1, "fixture must exercise conjunctions");
        for permission in required {
            assert!(advertised.contains(permission), "missing {permission}");
        }
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn permission_help_does_not_attach_leaf_permissions_to_parent_groups() {
    let mock = MockServer::start().await;
    for command in [&["service"][..], &["postgres", "read-replica"][..]] {
        let help = permission_help(&mock, command, "--help");
        assert!(advertised_permission_ids(&help).is_empty());
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn permission_help_covers_force_delete_calls() {
    let mock = MockServer::start().await;
    let help = permission_help(&mock, &["service", "delete"], "--help");
    assert!(mock.received_requests().await.unwrap().is_empty());
    let service_path = format!("/v1/organizations/org-1/services/{TEST_SERVICE_ID}");
    Mock::given(method("GET"))
        .and(path(&service_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 200,
            "result": {"id": TEST_SERVICE_ID, "state": "stopped"},
        })))
        .expect(1)
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(&service_path))
        .respond_with(successful_delete_response("permission-service-delete"))
        .expect(1)
        .mount(&mock)
        .await;
    let project = tempfile::tempdir().unwrap();
    let output = Command::new(clickhousectl_binary())
        .isolated_env()
        .env("DO_NOT_TRACK", "1")
        .env("HOME", project.path())
        .env("CLICKHOUSE_CLOUD_API_KEY", "fake-key-for-tests")
        .env("CLICKHOUSE_CLOUD_API_SECRET", "fake-secret-for-tests")
        .current_dir(project.path())
        .args([
            "cloud",
            "--url",
            &mock.uri(),
            "--json",
            "--org-id",
            "org-1",
            "service",
            "delete",
            TEST_SERVICE_ID,
            "--force",
        ])
        .output()
        .expect("run compound service deletion");
    assert_success(&output);
    assert_eq!(mock.received_requests().await.unwrap().len(), 2);
    assert_observed_permissions_advertised(&mock, &help).await;
}
