mod common;

use std::collections::BTreeSet;
use std::future::Future;

use clickhouse_cloud_api::{Client, Error, models::*};
use common::support::*;

/// Limit=1 exercises real continuation pages without assuming an empty org.
/// totalCount is best-effort and nullable; never use it to decide when to stop.
async fn pages<T, F, Fut>(mut fetch: F) -> TestResult<Vec<T>>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: Future<Output = Result<ApiResponse<Vec<T>>, Error>>,
{
    let mut cursor = None;
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for _ in 0..1000 {
        let page = fetch(cursor).await?;
        let result = page.result.ok_or("UDF list omitted result")?;
        if result.len() > 1 || page.limit != Some(1) {
            return Err("UDF list did not respect limit=1".into());
        }
        items.extend(result);
        cursor = page.next_cursor;
        let Some(next) = &cursor else {
            return Ok(items);
        };
        if !seen.insert(next.clone()) {
            return Err("UDF list repeated a cursor".into());
        }
    }
    Err("UDF pagination exceeded 1000 pages".into())
}

async fn upload(client: &Client, org: &str) -> TestResult<String> {
    let session = client
        .udf_upload_session_create(org)
        .await?
        .result
        .ok_or("upload session omitted result")?;
    let id = session.upload_id.ok_or("upload session omitted uploadId")?;
    let url = session.upload_url.ok_or("upload session omitted URL")?;
    // Dedicated unauthenticated client. Transport errors can contain signed URLs.
    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?
        .put(url)
        .header("content-type", "application/zip")
        .body(include_bytes!("fixtures/udf_source.zip").as_slice())
        .send()
        .await
        .map_err(|_| "UDF artifact upload transport failed")?;
    if !response.status().is_success() {
        return Err(format!("UDF artifact upload returned {}", response.status()).into());
    }
    Ok(id)
}

async fn ready(client: &Client, ctx: &TestContext, name: &str, version: i64) -> TestResult<()> {
    poll_until(
        "UDF build",
        ctx.create_timeout,
        ctx.poll_interval,
        || async {
            let udf = client
                .udf_get(&ctx.org_id, name)
                .await?
                .result
                .ok_or("UDF get omitted result")?;
            if udf.status == Some(UdfStatus::Error) {
                return Err("disposable UDF build failed".into());
            }
            Ok(
                (udf.version == Some(version) && udf.status == Some(UdfStatus::Ready))
                    .then_some(()),
            )
        },
    )
    .await
}

#[tokio::test]
#[ignore = "requires live Cloud credentials; creates and cleans up disposable UDFs and a service"]
async fn cloud_udf_list_lifecycle() -> TestResult<()> {
    let ctx = TestContext::from_env()?;
    let client = create_client()?;
    let org = ctx.org_id.as_str();
    let mut cleanup = CleanupRegistry::default();
    let mut names = Vec::new();
    let mut attachment_target = None;
    let test_result: TestResult<()> = async {
        let client = &client;
        log_run_header("cloud_udf_list_lifecycle", &ctx);
        // Verify read-only decoding before creating anything.
        pages(|cursor| async move { client.udf_list(org, cursor.as_deref(), Some(1)).await }).await?;
        for _ in 0..2 {
            let name = format!("chctl_uat01_{}", uuid::Uuid::new_v4().simple());
            match client.udf_get(org, &name).await {
                Err(Error::Api { status:404, .. }) => {}
                _ => return Err("disposable UDF name is not available".into()),
            }
            // Register the known-unused name before sending the create, so
            // teardown also handles a successful write with a lost response.
            names.push(name.clone());
            let request: UdfCreateRequest = serde_json::from_value(serde_json::json!({
                "functionName":name, "type":"executable", "runtime":"python3.11",
                "arguments":[{"name":"x","type":"UInt64"}], "returnType":"UInt64",
                "format":"TabSeparated", "uploadId":upload(client, org).await?
            }))?;
            let udf = client.udf_create(org, &request).await?.result.ok_or("UDF create omitted result")?;
            ready(client, &ctx, &name, udf.version.ok_or("UDF create omitted version")?).await?;
            let empty = client.udf_attachment_list(org, &name, None, Some(1)).await?;
            if empty.result.as_ref().is_none_or(|items| !items.is_empty()) || empty.next_cursor.is_some() {
                return Err("new UDF attachments should be an empty final page".into());
            }
        }
        let listed = pages(|cursor| async move { client.udf_list(org, cursor.as_deref(), Some(1)).await }).await?;
        for name in &names {
            if listed.iter().filter(|udf| udf.function_name.as_ref() == Some(name)).count() != 1 {
                return Err("organization list did not contain each disposable UDF exactly once".into());
            }
        }
        let name = names[0].as_str();
        let request: UdfVersionCreateRequest = serde_json::from_value(serde_json::json!({
            "type":"executable", "runtime":"python3.11",
            "arguments":[{"name":"x","type":"UInt64"}], "returnType":"UInt64",
            "format":"TabSeparated", "uploadId":upload(client, org).await?
        }))?;
        let latest = client.udf_version_create(org, name, &request).await?.result.ok_or("version create omitted result")?;
        let version = latest.version.ok_or("version create omitted version")?;
        ready(client, &ctx, name, version).await?;
        let versions = pages(|cursor| async move { client.udf_version_list(org, name, cursor.as_deref(), Some(1)).await }).await?;
        let version_ids: BTreeSet<_> = versions.iter().filter_map(|udf| udf.version).collect();
        if versions.len() != 2 || version_ids.len() != 2 || !version_ids.contains(&version) {
            return Err("version pagination lost or repeated a version".into());
        }

        let created = client.instance_create(org, &ServicePostRequest {
            name:format!("chctl-udf-{}", uuid::Uuid::new_v4().simple()),
            provider:ServicePostRequestProvider::Unknown(ctx.provider.clone()),
            region:ServicePostRequestRegion::Unknown(ctx.region.clone()),
            min_replica_memory_gb:Some(8.0), max_replica_memory_gb:Some(8.0), num_replicas:Some(1),
            // The lifecycle needs no SQL ingress.
            ip_access_list:vec![], ..Default::default()
        }).await?.result.ok_or("service create omitted result")?;
        let id = created.service.and_then(|service| service.id).ok_or("service create omitted ID")?.to_string();
        cleanup.register_service(id.clone());

        poll_until("UDF service running", ctx.create_timeout, ctx.poll_interval, || async {
            let service = client.instance_get(org, &id).await?.result.ok_or("service get omitted result")?;
            Ok((service_state(&service) == "running").then_some(()))
        }).await?;
        attachment_target = Some((name.to_string(), id.clone()));
        client.udf_attach(org, name, &id, Some(version)).await?;
        poll_until("UDF attachment deployed", ctx.create_timeout, ctx.poll_interval, || async {
            let attachment = client.udf_attachment_get(org, name, &id).await?.result.ok_or("attachment get omitted result")?;
            if attachment.status == Some(UdfAttachmentStatus::Error) { return Err("disposable UDF attachment failed".into()); }
            Ok((attachment.status == Some(UdfAttachmentStatus::Deployed)).then_some(()))
        }).await?;
        let attachments = pages(|cursor| async move { client.udf_attachment_list(org, name, cursor.as_deref(), Some(1)).await }).await?;
        if attachments.len() != 1 || attachments[0].service_id.as_deref() != Some(&id) || attachments[0].version != Some(version) {
            return Err("attachment list did not preserve the deployed service and version".into());
        }
        client.udf_detach(org, name, &id).await?;
        poll_until("UDF attachments empty", ctx.delete_timeout, ctx.poll_interval, || async {
            let page = client.udf_attachment_list(org, name, None, Some(1)).await?;
            Ok(page.result.filter(|items| items.is_empty()).map(|_| ()))
        }).await?;
        attachment_target = None;
        let previous = *version_ids.iter().find(|id| **id != version).ok_or("previous version missing")?;
        client.udf_version_delete(org, name, previous).await?;
        let remaining = pages(|cursor| async move { client.udf_version_list(org, name, cursor.as_deref(), Some(1)).await }).await?;
        if remaining.len() != 1 || remaining[0].version != Some(version) { return Err("version list did not reflect deletion".into()); }
        eprintln!("UDF organization/version/attachment lists: empty, nonempty, limit=1 continuation and lifecycle verified");
        Ok(())
    }.await;

    // Always run every cleanup, including after a failed assertion/request.
    let mut cleanup_errors = Vec::new();
    if let Some((name, id)) = &attachment_target {
        match client.udf_detach(org, name, id).await {
            Ok(_) | Err(Error::Api { status: 404, .. }) => {}
            Err(error) => cleanup_errors.push(format!("detach {name}: {error}")),
        }
    }
    for name in names.iter().rev() {
        match client.udf_delete(org, name).await {
            Ok(_) | Err(Error::Api { status: 404, .. }) => {}
            Err(error) => cleanup_errors.push(format!("delete {name}: {error}")),
        }
    }
    if let Err(error) = cleanup
        .cleanup(&client, org, ctx.delete_timeout, ctx.poll_interval, None)
        .await
    {
        cleanup_errors.push(error);
    }
    if !cleanup_errors.is_empty() {
        return Err(format!(
            "test: {test_result:?}; cleanup: {}",
            cleanup_errors.join("; ")
        )
        .into());
    }
    test_result
}
