//! Name selectors and background-change progress shared across domains (src/cloud/shared.rs).

use serde_json::Value;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::*;

/// Human mode names the check that confirms a change applied in the background
/// (#1241): the `get` command for a CDC scaling change or a Postgres resize,
/// and `pg_stat_replication` for an HA change, whose `haType` flips at once so
/// no `get` confirms it. A rename applies at once, so it gets no note.
#[tokio::test]
async fn background_changes_name_their_progress_check_in_human_output() {
    let postgres_id = "11111111-2222-3333-4444-555555555555";
    let postgres_path = format!("/v1/organizations/org-1/postgres/{postgres_id}");
    let postgres_check = format!("clickhousectl cloud postgres get {postgres_id} --org-id org-1");
    let ha_check = "pg_stat_replication";
    // (args, identifiers stderr must name, identifiers it must not)
    let cases: [(Vec<&str>, Vec<&str>, Vec<&str>); 5] = [
        (
            vec![
                "clickpipe",
                "cdc-scaling",
                "update",
                "svc-1",
                "--cpu-millicores",
                "2000",
                "--memory-gb",
                "8",
            ],
            vec!["clickhousectl cloud clickpipe cdc-scaling get svc-1 --org-id org-1"],
            vec![],
        ),
        (
            vec!["postgres", "update", postgres_id, "--size", "c6gd.large"],
            vec![&postgres_check],
            vec![ha_check],
        ),
        (
            vec!["postgres", "update", postgres_id, "--ha-type", "async"],
            vec![ha_check],
            vec![&postgres_check],
        ),
        (
            vec![
                "postgres",
                "update",
                postgres_id,
                "--size",
                "c6gd.large",
                "--ha-type",
                "async",
            ],
            vec![&postgres_check, ha_check],
            vec![],
        ),
        (
            vec!["postgres", "update", postgres_id, "--new-name", "renamed"],
            vec![],
            vec![],
        ),
    ];
    for (mut args, names, omits) in cases {
        let mock = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(CDC_SCALING_PATH))
            .respond_with(cdc_scaling_envelope(
                serde_json::json!({ "replicaMemoryGb": 4.0 }),
            ))
            .mount(&mock)
            .await;
        let postgres = serde_json::json!({ "result": { "id": postgres_id, "tags": [] } });
        for verb in ["GET", "PATCH"] {
            Mock::given(method(verb))
                .and(path(&postgres_path))
                .respond_with(ResponseTemplate::new(200).set_body_json(postgres.clone()))
                .mount(&mock)
                .await;
        }
        args.extend(["--org-id", "org-1"]);

        let output = invoke_cli_with_cloud_credentials_human(&mock, &args);

        assert_success(&output);
        assert!(!output.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        if names.is_empty() {
            assert!(stderr.is_empty(), "{args:?}: {stderr}");
        }
        for name in names {
            assert!(stderr.contains(name), "{args:?}: {stderr}");
        }
        for name in omits {
            assert!(!stderr.contains(name), "{args:?}: {stderr}");
        }
    }
}

// #891: exact, scoped selectors resolve before the existing target handler.
#[tokio::test]
async fn selectors_use_exact_scoped_names_and_ids_do_not_require_list_access() {
    const ID: &str = "11111111-2222-3333-4444-555555555555";
    type SelectorCase = (
        &'static [&'static str],
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    );
    let cases: &[SelectorCase] = &[
        (
            &["service", "get"],
            "services",
            "services",
            "GET",
            "name",
            "id",
        ),
        (
            &["postgres", "get"],
            "postgres",
            "postgres",
            "GET",
            "name",
            "id",
        ),
        (&["key", "get"], "keys", "keys", "GET", "name", "id"),
        (
            &["org", "role", "get"],
            "roles",
            "roles",
            "GET",
            "name",
            "id",
        ),
        (
            &["member", "get"],
            "members",
            "members",
            "GET",
            "email",
            "userId",
        ),
        (
            &["invitation", "get"],
            "invitations",
            "invitations",
            "GET",
            "email",
            "id",
        ),
        // Destructive commands resolve in their own dispatcher arm, so each
        // has a run of its own.
        (
            &["org", "role", "delete"],
            "roles",
            "roles",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["member", "remove"],
            "members",
            "members",
            "DELETE",
            "email",
            "userId",
        ),
        (
            &["invitation", "delete"],
            "invitations",
            "invitations",
            "DELETE",
            "email",
            "id",
        ),
        (
            &["clickpipe", "get", "parent-service"],
            "services/parent-service/clickpipes",
            "services/parent-service/clickpipes",
            "GET",
            "name",
            "id",
        ),
        (
            &["org", "byoc", "delete"],
            "",
            "byocInfrastructure",
            "DELETE",
            "displayName",
            "id",
        ),
        (
            &["clickstack", "source", "delete", "parent-service"],
            "services/parent-service/clickstack/sources",
            "services/parent-service/clickstack/sources",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["clickstack", "role", "delete", "parent-service"],
            "services/parent-service/clickstack/roles",
            "services/parent-service/clickstack/roles",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["clickstack", "dashboard", "delete", "parent-service"],
            "services/parent-service/clickstack/dashboards",
            "services/parent-service/clickstack/dashboards",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["clickstack", "alert", "delete", "parent-service"],
            "services/parent-service/clickstack/alerts",
            "services/parent-service/clickstack/alerts",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["clickstack", "webhook", "delete", "parent-service"],
            "services/parent-service/clickstack/webhooks",
            "services/parent-service/clickstack/webhooks",
            "DELETE",
            "name",
            "id",
        ),
        (
            &["clickstack", "saved-search", "delete", "parent-service"],
            "services/parent-service/clickstack/saved-searches",
            "services/parent-service/clickstack/saved-searches",
            "DELETE",
            "name",
            "id",
        ),
    ];
    // Every resource's name lookup (its list route and name field) once. A
    // positional ID skips the list in `NameSelector::resolve` for all of them,
    // so only the first case also runs by ID.
    for (index, (prefix, list_suffix, target_suffix, verb, name_field, id_field)) in
        cases.iter().enumerate()
    {
        let forms: &[bool] = if index == 0 { &[false, true] } else { &[true] };
        for &named in forms {
            let mock = MockServer::start().await;
            let list_path = if list_suffix.is_empty() {
                "/v1/organizations/org-1".to_owned()
            } else {
                format!("/v1/organizations/org-1/{list_suffix}")
            };
            let mut row = serde_json::json!({*name_field: "Exact", *id_field: ID});
            if prefix.get(1) == Some(&"source") {
                row["kind"] = "promql".into();
            }
            if prefix.get(1) == Some(&"webhook") {
                row["service"] = "slack".into();
            }
            let mut other = row.clone();
            other[*name_field] = "exact".into();
            other[*id_field] = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into();
            let rows = serde_json::json!([other, row]);
            let result = if list_suffix.is_empty() {
                serde_json::json!({"byocConfig": rows})
            } else {
                rows
            };
            Mock::given(method("GET"))
                .and(path(&list_path))
                .respond_with(if named {
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": result}))
                } else {
                    ResponseTemplate::new(403).set_body_string("list forbidden")
                })
                .expect(if named { 1 } else { 0 })
                .mount(&mock)
                .await;
            let target_path = format!("/v1/organizations/org-1/{target_suffix}/{ID}");
            Mock::given(method(*verb))
                .and(path(&target_path))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(serde_json::json!({"result": {}, "status": 200})),
                )
                .expect(1)
                .mount(&mock)
                .await;
            let selector = if *name_field == "email" {
                "--email"
            } else {
                "--name"
            };
            let mut args = prefix.to_vec();
            if named {
                args.extend([selector, "Exact"]);
            } else {
                args.push(ID);
            }
            args.extend(["--org-id", "org-1"]);
            let output = invoke_cli_with_cloud_credentials(&mock, &args);
            assert!(
                output.status.success(),
                "{args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                mock.received_requests().await.unwrap().len(),
                if named { 2 } else { 1 }
            );
        }
    }
}

#[tokio::test]
async fn selectors_refuse_bad_lists_before_mutation_and_preserve_list_errors() {
    // Which lists cannot resolve a name is
    // `shared::exact_resolution_rejects_incomplete_or_ambiguous_collections`;
    // here a refused list and a failed list each stop the update.
    for (status, body, exit) in [
        (
            200,
            serde_json::json!({"result": [{"name": "target", "id": "11111111-2222-3333-4444-555555555555"}, {"name": "target", "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"}]}),
            1,
        ),
        (401, serde_json::json!({"error": "list forbidden"}), 4),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/services"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "service",
                "update",
                "--name",
                "target",
                "--new-name",
                "replacement",
                "--org-id",
                "org-1",
            ],
        );
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET");
    }
}

#[tokio::test]
async fn selectors_rename_body_and_organization_name_scope_are_independent() {
    let org = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    let id = "11111111-2222-3333-4444-555555555555";
    for resource in ["service", "postgres", "key", "org"] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"result": [{"id": org, "name": "Production"}]}),
                ),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let suffix = match resource {
            "service" => "services",
            "postgres" => "postgres",
            "key" => "keys",
            _ => "",
        };
        let target_path = if resource == "org" {
            format!("/v1/organizations/{org}")
        } else {
            format!("/v1/organizations/{org}/{suffix}/{id}")
        };
        if resource != "org" {
            Mock::given(method("GET"))
                .and(path(format!("/v1/organizations/{org}/{suffix}")))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"result": [{"id": id, "name": "target"}]}),
                    ),
                )
                .expect(1)
                .mount(&mock)
                .await;
        }
        Mock::given(method("PATCH"))
            .and(path(&target_path))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": {}})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let mut args = vec![
            resource,
            "update",
            "--org-name",
            "Production",
            "--new-name",
            "replacement",
        ];
        if resource != "org" {
            args.extend(["--name", "target"]);
        }
        if resource == "postgres" {
            args.push("--clear-tags");
        }
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        let patch = requests.iter().find(|r| r.method == "PATCH").unwrap();
        let body: Value = serde_json::from_slice(&patch.body).unwrap();
        assert_eq!(body["name"], "replacement");
        assert!(!body.as_object().unwrap().contains_key("newName"));
        assert_eq!(requests.len(), if resource == "org" { 2 } else { 3 });
    }
}

#[tokio::test]
async fn selectors_query_endpoints_read_all_cursor_pages_before_mutating() {
    let first_id = "11111111-2222-3333-4444-555555555555";
    let second_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    for second in [
        serde_json::json!({"result": [], "nextCursor": null}),
        serde_json::json!({"result": [{"id": second_id, "name": "target"}], "nextCursor": null}),
        serde_json::json!({"result": null, "nextCursor": null}),
        serde_json::json!({"result": [], "nextCursor": "page-2"}),
    ] {
        let unique = second["nextCursor"].is_null()
            && second["result"].as_array().is_some_and(Vec::is_empty);
        let mock = MockServer::start().await;
        let list_path = "/v1/organizations/org-1/services/parent/query-api-endpoints";
        Mock::given(method("GET"))
            .and(path(list_path))
            .and(query_param("limit", "100"))
            .and(wiremock::matchers::query_param_is_missing("cursor"))
            .respond_with(query_api_endpoint_list_page(
                serde_json::json!([{"id": first_id, "name": "target"}]),
                Some("page-2"),
                None,
                Some(100),
            ))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("GET"))
            .and(path(list_path))
            .and(query_param("cursor", "page-2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(second))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("DELETE"))
            .and(path(format!("{list_path}/{first_id}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": 200})),
            )
            .expect(if unique { 1 } else { 0 })
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "query-api-endpoint",
                "delete",
                "parent",
                "--name",
                "target",
                "--org-id",
                "org-1",
            ],
        );
        assert_eq!(
            output.status.success(),
            unique,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            mock.received_requests().await.unwrap().len(),
            if unique { 3 } else { 2 }
        );
    }
}

#[tokio::test]
async fn selectors_clickstack_exhaust_offset_pages_before_mutating() {
    // Each resource's offset page fetch once; late duplicates are
    // `clickstack::name_pagination_rejects_duplicates_on_later_pages`.
    for (command, resource) in [
        ("alert", "alerts"),
        ("webhook", "webhooks"),
        ("saved-search", "saved-searches"),
    ] {
        let mock = MockServer::start().await;
        let list_path = format!("/v1/organizations/org-1/services/parent/clickstack/{resource}");
        let first: Vec<Value> = (0..1000)
            .map(|index| {
                serde_json::json!({
                    "id": format!("00000000-0000-0000-0000-{index:012}"),
                    "name": format!("other-{index}"),
                    "service": "slack"
                })
            })
            .collect();
        Mock::given(method("GET"))
            .and(path(&list_path))
            .and(query_param("limit", "1000"))
            .and(query_param("offset", "0"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": first})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("GET"))
            .and(path(&list_path))
            .and(query_param("limit", "1000"))
            .and(query_param("offset", "1000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": [{"id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "name": "target", "service": "slack"}]})))
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("DELETE"))
            .and(path(format!(
                "{list_path}/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
            )))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": 200})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "clickstack",
                command,
                "delete",
                "parent",
                "--name",
                "target",
                "--org-id",
                "org-1",
            ],
        );
        assert_success(&output);
        assert_eq!(
            mock.received_requests().await.unwrap().len(),
            3,
            "{command}"
        );
    }
}

#[tokio::test]
async fn selectors_unknown_clickstack_variants_never_mutate() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/v1/organizations/org-1/services/parent/clickstack/sources",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({"result": [{"id": "id", "name": "target", "kind": "future-variant"}]}),
        ))
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "clickstack",
            "source",
            "delete",
            "parent",
            "--name",
            "target",
            "--org-id",
            "org-1",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn selectors_postgres_sources_keep_new_resource_names_in_the_body() {
    // Each command resolves `--source-name` once; a positional ID skips the
    // lookup in `SourceSelector::resolve`.
    let id = "11111111-2222-3333-4444-555555555555";
    for (prefix, endpoint) in [
        (vec!["postgres", "restore"], "restoredService"),
        (vec!["postgres", "read-replica", "create"], "readReplica"),
    ] {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/org-1/postgres"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"result": [{"id": id, "name": "source"}]})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v1/organizations/org-1/postgres/{id}/{endpoint}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"result": restored_service_response(
                    &format!("/{endpoint}"),
                    &serde_json::json!({}),
                )}),
            ))
            .expect(1)
            .mount(&mock)
            .await;
        let mut args = prefix.clone();
        args.extend(["--source-name", "source"]);
        args.extend(["--name", "created", "--org-id", "org-1"]);
        if endpoint == "restoredService" {
            args.extend([
                "--restore-target",
                "2026-09-01T12:00:00Z",
                "--block-public-access",
            ]);
        }
        let output = invoke_cli_with_cloud_credentials(&mock, &args);
        assert_success(&output);
        let requests = mock.received_requests().await.unwrap();
        let request = requests.iter().find(|r| r.method == "POST").unwrap();
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["name"], "created");
        assert_eq!(requests.len(), 2);
    }
}

#[tokio::test]
async fn selectors_ambiguous_organization_name_fails_without_mutation() {
    // Other incomplete lists: `shared::exact_resolution_rejects_incomplete_or_ambiguous_collections`.
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": [
                { "name": "production", "id": "11111111-2222-3333-4444-555555555555" },
                { "name": "production", "id": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee" }
            ]})),
        )
        .expect(1)
        .mount(&mock)
        .await;
    let output = invoke_cli_with_cloud_credentials(
        &mock,
        &[
            "org",
            "update",
            "--org-name",
            "production",
            "--new-name",
            "replacement",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn selectors_endpoint_pagination_integrity_and_later_errors_block_mutation() {
    let target_id = "11111111-2222-3333-4444-555555555555";
    let other_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    for (status, second, exit) in [
        (
            200,
            serde_json::json!({"result": [{"id": other_id, "name": "other"}], "totalCount": 3, "nextCursor": null}),
            1,
        ),
        (
            200,
            serde_json::json!({"result": [], "totalCount": 4, "nextCursor": null}),
            1,
        ),
        (
            200,
            serde_json::json!({"result": [], "totalCount": 3, "nextCursor": null}),
            1,
        ),
        (401, serde_json::json!({"error": "later page forbidden"}), 4),
    ] {
        let mock = MockServer::start().await;
        let list_path = "/v1/organizations/org-1/services/parent/query-api-endpoints";
        Mock::given(method("GET")).and(path(list_path)).and(wiremock::matchers::query_param_is_missing("cursor"))
            .respond_with(query_api_endpoint_list_page(serde_json::json!([{"id": target_id, "name": "target"}, {"id": other_id, "name": "other"}]), Some("page-2"), Some(3), Some(100)))
            .expect(1).mount(&mock).await;
        Mock::given(method("GET"))
            .and(path(list_path))
            .and(query_param("cursor", "page-2"))
            .respond_with(ResponseTemplate::new(status).set_body_json(second))
            .expect(1)
            .mount(&mock)
            .await;
        let output = invoke_cli_with_cloud_credentials(
            &mock,
            &[
                "query-api-endpoint",
                "delete",
                "parent",
                "--name",
                "target",
                "--org-id",
                "org-1",
            ],
        );
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests.iter().all(|request| request.method == "GET"));
    }
}
