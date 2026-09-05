use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::config::{deserialize_strict_config, read_config_value};
use crate::cloud::output::{or_absent, print_human};
use crate::cloud::shared::resolve_org_id;
use crate::cloud::types::DeleteResponse;
use clap::{Args, Subcommand};
use clickhouse_cloud_api::models::*;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use tabled::{Table, Tabled, settings::Style};

#[derive(Args)]
pub struct UdfArgs {
    /// Organization ID (auto-detected only if you have one org)
    #[arg(long, global = true)]
    org_id: Option<String>,
    #[command(subcommand)]
    command: UdfCommands,
}

#[derive(Subcommand)]
pub enum UdfCommands {
    /// List UDFs
    List(UdfPageArgs),
    /// Get UDF details
    Get(UdfNameArgs),
    /// Create a UDF
    Create(UdfCreateArgs),
    /// Delete a UDF
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Deletes every version and detaches the UDF from all services.\n  Service removal completes asynchronously."
    )]
    Delete(UdfNameArgs),
    /// Attach a UDF to a service
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Replaces the service's attached version; omission selects the latest ready version.\n  The service must be running; wake an idle service before attaching."
    )]
    Attach {
        #[command(flatten)]
        target: UdfAttachmentArgs,
        /// UDF version number
        #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
        version: Option<i64>,
    },
    /// Detach a UDF from a service
    Detach(UdfAttachmentArgs),
    /// Manage UDF service attachments
    Attachment {
        #[command(subcommand)]
        command: UdfAttachmentCommands,
    },
    /// Manage UDF versions
    Version {
        #[command(subcommand)]
        command: UdfVersionCommands,
    },
}

#[derive(Subcommand)]
pub enum UdfAttachmentCommands {
    /// List UDF attachments
    List {
        #[command(flatten)]
        name: UdfNameArgs,
        #[command(flatten)]
        page: UdfPageArgs,
    },
    /// Get UDF attachment details
    Get(UdfAttachmentArgs),
}

#[derive(Subcommand)]
pub enum UdfVersionCommands {
    /// List UDF versions
    List {
        #[command(flatten)]
        name: UdfNameArgs,
        #[command(flatten)]
        page: UdfPageArgs,
    },
    /// Create a UDF version
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Supply the complete definition; omitted options use defaults, not previous values.\n  Each retry uploads a fresh archive and consumes a new upload session."
    )]
    Create {
        #[command(flatten)]
        name: UdfNameArgs,
        #[command(flatten)]
        input: UdfCreateArgs,
    },
    /// Delete a UDF version
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Detach the UDF from every service before deleting a version.\n  The latest version and versions still building cannot be deleted."
    )]
    Delete {
        #[command(flatten)]
        name: UdfNameArgs,
        /// UDF version number
        #[arg(value_parser = clap::value_parser!(i64).range(1..))]
        version: i64,
    },
}

#[derive(Args)]
pub struct UdfNameArgs {
    /// UDF function name
    #[arg(value_parser = parse_udf_name)]
    function_name: String,
}

#[derive(Args)]
pub struct UdfAttachmentArgs {
    #[command(flatten)]
    name: UdfNameArgs,
    /// Service ID
    service_id: String,
}

#[derive(Args)]
pub struct UdfPageArgs {
    /// Cursor from pagination.nextCursor
    #[arg(long)]
    cursor: Option<String>,
    /// Maximum records per page (1–100)
    #[arg(long, value_parser = clap::value_parser!(i64).range(1..=100))]
    limit: Option<i64>,
}

#[derive(Args)]
pub struct UdfCreateArgs {
    /// Complete JSON definition without uploadId (file path or - for stdin)
    #[arg(long = "config-file", alias = "config")]
    config: String,
    /// Source archive path in ZIP format
    #[arg(long)]
    artifact: PathBuf,
}

impl UdfArgs {
    pub fn is_write(&self) -> bool {
        match &self.command {
            UdfCommands::List(_) | UdfCommands::Get(_) => false,
            UdfCommands::Create(_)
            | UdfCommands::Delete(_)
            | UdfCommands::Attach { .. }
            | UdfCommands::Detach(_) => true,
            UdfCommands::Attachment { command } => match command {
                UdfAttachmentCommands::List { .. } | UdfAttachmentCommands::Get(_) => false,
            },
            UdfCommands::Version { command } => match command {
                UdfVersionCommands::List { .. } => false,
                UdfVersionCommands::Create { .. } | UdfVersionCommands::Delete { .. } => true,
            },
        }
    }
}

fn parse_udf_name(value: &str) -> Result<String, String> {
    let mut bytes = value.bytes();
    if !bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
        || !bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("Use a letter followed by letters, digits or underscores".into());
    }
    Ok(value.to_owned())
}

pub async fn run(client: &CloudClient, args: UdfArgs, json: bool) -> CloudResult<()> {
    // Parse and validate input before even auto-resolving the organization.
    match args.command {
        UdfCommands::Create(input) => {
            let mut request =
                build_udf_create_request(read_config_value(&input.config)?, "pending")?;
            let file = open_artifact(&input.artifact).await?;
            let org = resolve_org_id(client, args.org_id.as_deref()).await?;
            let upload_id = upload_artifact(client, &org, file).await?;
            match &mut request {
                UdfCreateRequest::UdfCreateRequestV1(body) => body.upload_id = upload_id,
                UdfCreateRequest::UdfCreateRequestV2(body) => body.upload_id = upload_id,
                UdfCreateRequest::Unknown(_) => unreachable!("builder accepts known variants only"),
            }
            output(&client.create_udf(&org, &request).await?, json)
        }
        UdfCommands::Version {
            command: UdfVersionCommands::Create { name, input },
        } => {
            let mut request =
                build_udf_version_create_request(read_config_value(&input.config)?, "pending")?;
            let file = open_artifact(&input.artifact).await?;
            let org = resolve_org_id(client, args.org_id.as_deref()).await?;
            let upload_id = upload_artifact(client, &org, file).await?;
            match &mut request {
                UdfVersionCreateRequest::UdfVersionCreateRequestV1(body) => {
                    body.upload_id = upload_id
                }
                UdfVersionCreateRequest::UdfVersionCreateRequestV2(body) => {
                    body.upload_id = upload_id
                }
                UdfVersionCreateRequest::Unknown(_) => {
                    unreachable!("builder accepts known variants only")
                }
            }
            output(
                &client
                    .create_udf_version(&org, &name.function_name, &request)
                    .await?,
                json,
            )
        }
        command => {
            let org = resolve_org_id(client, args.org_id.as_deref()).await?;
            match command {
                UdfCommands::List(page) => {
                    let data = client
                        .list_udfs(&org, page.cursor.as_deref(), page.limit)
                        .await?;
                    if json {
                        output(&data, true)
                    } else {
                        print_udfs(data.items, data.pagination)
                    }
                }
                UdfCommands::Get(name) => {
                    output(&client.get_udf(&org, &name.function_name).await?, json)
                }
                UdfCommands::Delete(name) => {
                    output(&client.delete_udf(&org, &name.function_name).await?, json)
                }
                UdfCommands::Attach { target, version } => output(
                    &client
                        .attach_udf(
                            &org,
                            &target.name.function_name,
                            &target.service_id,
                            version,
                        )
                        .await?,
                    json,
                ),
                UdfCommands::Detach(target) => output(
                    &client
                        .detach_udf(&org, &target.name.function_name, &target.service_id)
                        .await?,
                    json,
                ),
                UdfCommands::Attachment { command } => match command {
                    UdfAttachmentCommands::Get(target) => output(
                        &client
                            .get_udf_attachment(
                                &org,
                                &target.name.function_name,
                                &target.service_id,
                            )
                            .await?,
                        json,
                    ),
                    UdfAttachmentCommands::List { name, page } => {
                        let data = client
                            .list_udf_attachments(
                                &org,
                                &name.function_name,
                                page.cursor.as_deref(),
                                page.limit,
                            )
                            .await?;
                        if json {
                            output(&data, true)
                        } else {
                            print_attachments(data.items, data.pagination)
                        }
                    }
                },
                UdfCommands::Version { command } => match command {
                    UdfVersionCommands::List { name, page } => {
                        let data = client
                            .list_udf_versions(
                                &org,
                                &name.function_name,
                                page.cursor.as_deref(),
                                page.limit,
                            )
                            .await?;
                        if json {
                            output(&data, true)
                        } else {
                            print_udfs(data.items, data.pagination)
                        }
                    }
                    UdfVersionCommands::Delete { name, version } => output(
                        &client
                            .delete_udf_version(&org, &name.function_name, version)
                            .await?,
                        json,
                    ),
                    UdfVersionCommands::Create { .. } => unreachable!("handled above"),
                },
                UdfCommands::Create(_) => unreachable!("handled above"),
            }
        }
    }
}

fn output<T: Serialize>(data: &T, json: bool) -> CloudResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(data)?);
    } else {
        print_human(data)?;
    }
    Ok(())
}

fn print_udfs(items: Option<Vec<Udf>>, pagination: Option<Pagination>) -> CloudResult<()> {
    #[derive(Tabled)]
    struct Row {
        name: String,
        version: String,
        runtime: String,
        status: String,
    }
    if let Some(items) = items {
        let rows = items.into_iter().map(|item| Row {
            name: or_absent(item.function_name),
            version: or_absent(item.version),
            runtime: or_absent(item.runtime),
            status: or_absent(item.status),
        });
        println!("{}", Table::new(rows).with(Style::markdown()));
    } else {
        println!("UDFs: -");
    }
    if let Some(page) = pagination {
        print_human(&page)?;
    }
    Ok(())
}

fn print_attachments(
    items: Option<Vec<UdfAttachment>>,
    pagination: Option<Pagination>,
) -> CloudResult<()> {
    #[derive(Tabled)]
    struct Row {
        name: String,
        service_id: String,
        version: String,
        status: String,
    }
    if let Some(items) = items {
        let rows = items.into_iter().map(|item| Row {
            name: or_absent(item.function_name),
            service_id: or_absent(item.service_id),
            version: or_absent(item.version),
            status: or_absent(item.status),
        });
        println!("{}", Table::new(rows).with(Style::markdown()));
    } else {
        println!("Attachments: -");
    }
    if let Some(page) = pagination {
        print_human(&page)?;
    }
    Ok(())
}

fn validate_udf_config(value: &mut Value, upload_id: &str, create: bool) -> CloudResult<String> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| CloudError::new("UDF definition must be a JSON object"))?;
    if object.contains_key("uploadId") {
        return Err(CloudError::new(
            "Omit uploadId; --artifact creates a fresh upload session",
        ));
    }
    for name in [
        "commandReadTimeout",
        "commandWriteTimeout",
        "maxCommandExecutionTime",
        "poolSize",
        "memoryLimitMib",
    ] {
        if let Some(value) = object.get(name).filter(|v| !v.is_null()) {
            let max = if name == "memoryLimitMib" {
                1_048_576
            } else {
                i64::MAX
            };
            if !value.as_i64().is_some_and(|v| (1..=max).contains(&v)) {
                return Err(CloudError::new(format!(
                    "UDF {name} must be an integer from 1 to {max}"
                )));
            }
        }
    }
    let kind = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| CloudError::new("UDF definition requires type"))?
        .to_owned();
    // Option<T> is also used for non-nullable optional request fields; reject
    // explicit null here instead of silently converting it into omission.
    for name in [
        "runtime",
        "type",
        "deterministic",
        "sendChunkHeader",
        "format",
        "sandboxType",
        "sandboxVersion",
        "commandReadTimeout",
        "commandWriteTimeout",
    ] {
        if object.get(name).is_some_and(Value::is_null) {
            return Err(CloudError::new(format!("UDF {name} cannot be null")));
        }
    }
    if kind == "executable_pool" {
        for name in ["poolSize", "maxCommandExecutionTime"] {
            if object.get(name).is_some_and(Value::is_null) {
                return Err(CloudError::new(format!(
                    "UDF {name} cannot be null for executable_pool"
                )));
            }
        }
    }
    for name in ["returnName", "functionName"] {
        if let Some(value) = object.get(name).filter(|v| !v.is_null()) {
            let valid = value.as_str().is_some_and(|v| parse_udf_name(v).is_ok());
            if !valid {
                return Err(CloudError::new(format!("Invalid UDF {name}")));
            }
        }
    }
    if create && !object.get("functionName").is_some_and(Value::is_string) {
        return Err(CloudError::new("UDF definition requires functionName"));
    }
    if let Some(arguments) = object.get("arguments").and_then(Value::as_array) {
        for argument in arguments {
            if argument
                .get("name")
                .and_then(Value::as_str)
                .is_none_or(|v| parse_udf_name(v).is_err())
            {
                return Err(CloudError::new("Every UDF argument requires a valid name"));
            }
        }
    }
    object.insert("uploadId".into(), Value::String(upload_id.to_owned()));
    Ok(kind)
}

fn validate_udf_enums(
    runtime: &UdfRuntime,
    sandbox_type: Option<&UdfSandboxType>,
    sandbox_version: Option<&UdfSandboxVersion>,
) -> CloudResult<()> {
    if matches!(runtime, UdfRuntime::Unknown(_)) {
        return Err(CloudError::new("Unsupported UDF runtime"));
    }
    if matches!(sandbox_type, Some(UdfSandboxType::Unknown(_))) {
        return Err(CloudError::new("Unsupported UDF sandboxType"));
    }
    if matches!(sandbox_version, Some(UdfSandboxVersion::Unknown(_))) {
        return Err(CloudError::new("Unsupported UDF sandboxVersion"));
    }
    Ok(())
}

fn build_udf_create_request(mut value: Value, upload_id: &str) -> CloudResult<UdfCreateRequest> {
    match validate_udf_config(&mut value, upload_id, true)?.as_str() {
        "executable" => {
            let body: UdfCreateRequestV1 = deserialize_strict_config(value, "UDF definition")?;
            validate_udf_enums(
                &body.runtime,
                body.sandbox_type.as_ref(),
                body.sandbox_version.as_ref(),
            )?;
            Ok(UdfCreateRequest::UdfCreateRequestV1(body))
        }
        "executable_pool" => {
            let body: UdfCreateRequestV2 = deserialize_strict_config(value, "UDF definition")?;
            validate_udf_enums(
                &body.runtime,
                body.sandbox_type.as_ref(),
                body.sandbox_version.as_ref(),
            )?;
            Ok(UdfCreateRequest::UdfCreateRequestV2(body))
        }
        _ => Err(CloudError::new("Unsupported UDF type")),
    }
}

fn build_udf_version_create_request(
    mut value: Value,
    upload_id: &str,
) -> CloudResult<UdfVersionCreateRequest> {
    match validate_udf_config(&mut value, upload_id, false)?.as_str() {
        "executable" => {
            let body: UdfVersionCreateRequestV1 =
                deserialize_strict_config(value, "UDF definition")?;
            validate_udf_enums(
                &body.runtime,
                body.sandbox_type.as_ref(),
                body.sandbox_version.as_ref(),
            )?;
            Ok(UdfVersionCreateRequest::UdfVersionCreateRequestV1(body))
        }
        "executable_pool" => {
            let body: UdfVersionCreateRequestV2 =
                deserialize_strict_config(value, "UDF definition")?;
            validate_udf_enums(
                &body.runtime,
                body.sandbox_type.as_ref(),
                body.sandbox_version.as_ref(),
            )?;
            Ok(UdfVersionCreateRequest::UdfVersionCreateRequestV2(body))
        }
        _ => Err(CloudError::new("Unsupported UDF type")),
    }
}

async fn open_artifact(path: &std::path::Path) -> CloudResult<tokio::fs::File> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| CloudError::new("Cannot open UDF ZIP archive"))?;
    let metadata = file
        .metadata()
        .await
        .map_err(|_| CloudError::new("Cannot inspect UDF ZIP archive"))?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(CloudError::new("UDF ZIP archive must be a nonempty file"));
    }
    Ok(file)
}

async fn upload_artifact(
    client: &CloudClient,
    org: &str,
    file: tokio::fs::File,
) -> CloudResult<String> {
    let session = client.create_udf_upload_session(org).await?;
    let id = session
        .upload_id
        .filter(|id| !id.is_empty())
        .ok_or_else(|| {
            CloudError::new("Upload session omitted uploadId; retry to create a fresh session")
        })?;
    let url = session.upload_url.ok_or_else(|| {
        CloudError::new("Upload session omitted uploadUrl; retry to create a fresh session")
    })?;
    let url = reqwest::Url::parse(&url)
        .map_err(|_| CloudError::new("Upload session returned an invalid URL"))?;
    let local = url
        .host_str()
        .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "[::1]"));
    if (url.scheme() != "https" && !(cfg!(debug_assertions) && local && url.scheme() == "http"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(CloudError::new(
            "Upload session requires an HTTPS URL without user credentials or a fragment",
        ));
    }
    let length = file
        .metadata()
        .await
        .map_err(|_| CloudError::new("Cannot inspect UDF ZIP archive"))?
        .len();
    // Dedicated client: Cloud API authentication never reaches the artifact
    // host. Do not follow redirects, retry single-use uploads, print the URL,
    // expose transport errors containing it, or echo storage response bodies.
    let upload_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|_| CloudError::new("Cannot initialize artifact upload"))?;
    let response = upload_client
        .put(url)
        .header(reqwest::header::CONTENT_TYPE, "application/zip")
        .header(reqwest::header::CONTENT_LENGTH, length)
        .body(file)
        .send()
        .await
        .map_err(|_| CloudError::new("Artifact upload failed; retry to create a fresh session"))?;
    if !response.status().is_success() {
        return Err(CloudError::new(format!(
            "Artifact upload failed (HTTP {}); retry to create a fresh session",
            response.status().as_u16()
        )));
    }
    Ok(id)
}

impl CloudClient {
    async fn list_udfs(
        &self,
        org: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> CloudResult<UdfListResponse> {
        let response = self
            .api()
            .udf_list(org, cursor, limit)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn get_udf(&self, org: &str, name: &str) -> CloudResult<Udf> {
        let response = self
            .api()
            .udf_get(org, name)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn delete_udf(&self, org: &str, name: &str) -> CloudResult<DeleteResponse> {
        let response = self
            .api()
            .udf_delete(org, name)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }
    async fn list_udf_attachments(
        &self,
        org: &str,
        name: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> CloudResult<UdfAttachmentListResponse> {
        let response = self
            .api()
            .udf_attachment_list(org, name, cursor, limit)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn get_udf_attachment(
        &self,
        org: &str,
        name: &str,
        service: &str,
    ) -> CloudResult<UdfAttachment> {
        let response = self
            .api()
            .udf_attachment_get(org, name, service)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn attach_udf(
        &self,
        org: &str,
        name: &str,
        service: &str,
        version: Option<i64>,
    ) -> CloudResult<UdfAttachment> {
        let response = self
            .api()
            .udf_attach(org, name, service, version)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn detach_udf(
        &self,
        org: &str,
        name: &str,
        service: &str,
    ) -> CloudResult<DeleteResponse> {
        let response = self
            .api()
            .udf_detach(org, name, service)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }
    async fn list_udf_versions(
        &self,
        org: &str,
        name: &str,
        cursor: Option<&str>,
        limit: Option<i64>,
    ) -> CloudResult<UdfVersionListResponse> {
        let response = self
            .api()
            .udf_version_list(org, name, cursor, limit)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn delete_udf_version(
        &self,
        org: &str,
        name: &str,
        version: i64,
    ) -> CloudResult<DeleteResponse> {
        let response = self
            .api()
            .udf_version_delete(org, name, version)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Ok(DeleteResponse {
            status: response.status,
            request_id: response.request_id,
        })
    }
    async fn create_udf(&self, org: &str, body: &UdfCreateRequest) -> CloudResult<Udf> {
        let response = self
            .api()
            .udf_create(org, body)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn create_udf_version(
        &self,
        org: &str,
        name: &str,
        body: &UdfVersionCreateRequest,
    ) -> CloudResult<Udf> {
        let response = self
            .api()
            .udf_version_create(org, name, body)
            .await
            .map_err(|error| self.convert_error_for_organization(error, org))?;
        Self::unwrap_response(response)
    }
    async fn create_udf_upload_session(&self, org: &str) -> CloudResult<UdfUploadSession> {
        let response = self
            .api()
            .udf_upload_session_create(org)
            .await
            .map_err(|error| {
                let error = self.convert_error_for_organization(error, org);
                CloudError {
                    message: "Could not create UDF upload session; retry to create a fresh session"
                        .into(),
                    details: None,
                    ..error
                }
            })?;
        Self::unwrap_response(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use clap::Parser;
    use serde_json::json;

    fn definition(kind: &str, create: bool) -> Value {
        let mut value = json!({"type": kind, "runtime": "native", "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64"});
        if create {
            value["functionName"] = json!("my_udf");
        }
        value
    }

    #[test]
    fn udf_builders_cover_minimal_and_maximal_variants() {
        for create in [true, false] {
            for kind in ["executable", "executable_pool"] {
                for full in [false, true] {
                    let mut input = definition(kind, create);
                    if full {
                        input.as_object_mut().unwrap().extend(json!({
                            "commandReadTimeout": 5000, "commandWriteTimeout": 6000,
                            "memoryLimitMib": 128, "deterministic": false,
                            "sendChunkHeader": false, "format": "JSONEachRow", "returnName": "result",
                            "sandboxType": "netenable", "sandboxVersion": "v3",
                            "maxCommandExecutionTime": 20,
                            "poolSize": if kind == "executable_pool" { json!(4) } else { Value::Null }
                        }).as_object().unwrap().clone());
                    }
                    let output = if create {
                        let request = build_udf_create_request(input.clone(), "upload-1").unwrap();
                        match &request {
                            UdfCreateRequest::UdfCreateRequestV1(body) => {
                                assert_eq!(body.upload_id, "upload-1");
                                assert_eq!(body.deterministic, full.then_some(false));
                                assert_eq!(body.memory_limit_mib, full.then_some(128));
                            }
                            UdfCreateRequest::UdfCreateRequestV2(body) => {
                                assert_eq!(body.pool_size, full.then_some(4));
                                assert_eq!(body.memory_limit_mib, full.then_some(128));
                            }
                            _ => panic!("unexpected union variant"),
                        }
                        serde_json::to_value(request).unwrap()
                    } else {
                        let request =
                            build_udf_version_create_request(input.clone(), "upload-1").unwrap();
                        match &request {
                            UdfVersionCreateRequest::UdfVersionCreateRequestV1(body) => {
                                assert_eq!(body.upload_id, "upload-1");
                                assert_eq!(body.deterministic, full.then_some(false));
                                assert_eq!(body.memory_limit_mib, full.then_some(128));
                            }
                            UdfVersionCreateRequest::UdfVersionCreateRequestV2(body) => {
                                assert_eq!(body.pool_size, full.then_some(4));
                                assert_eq!(body.memory_limit_mib, full.then_some(128));
                            }
                            _ => panic!("unexpected union variant"),
                        }
                        serde_json::to_value(request).unwrap()
                    };
                    input["uploadId"] = json!("upload-1");
                    if kind == "executable" {
                        input.as_object_mut().unwrap().remove("poolSize");
                    }
                    assert_eq!(output, input);
                }
            }
        }
    }

    #[test]
    fn udf_builders_reject_lossy_or_incomplete_requests() {
        for (key, value) in [
            ("type", json!("future")),
            ("runtime", json!("future")),
            ("sandboxType", json!("future")),
            ("sandboxVersion", json!("future")),
            ("deterministic", Value::Null),
            ("deterministic", json!("false")),
            ("memoryLimitMib", json!(0)),
            ("memoryLimitMib", json!(1048577)),
            ("commandReadTimeout", json!(-1)),
            ("poolSize", json!(2)),
            ("typo", Value::Null),
            ("uploadId", json!("old")),
            (
                "arguments",
                json!([{"name": "x", "type": "String", "typo": null}]),
            ),
            ("functionName", json!("../oops")),
        ] {
            let mut input = definition("executable", true);
            input[key] = value;
            assert!(build_udf_create_request(input, "fresh").is_err(), "{key}");
        }
        for required in ["type", "runtime", "arguments", "returnType", "functionName"] {
            let mut input = definition("executable", true);
            input.as_object_mut().unwrap().remove(required);
            assert!(
                build_udf_create_request(input, "fresh").is_err(),
                "{required}"
            );
        }
        assert!(build_udf_version_create_request(json!({}), "fresh").is_err());
        assert!(build_udf_version_create_request(definition("executable", true), "fresh").is_err());
        let mut nullable = definition("executable", true);
        nullable["memoryLimitMib"] = Value::Null;
        assert!(build_udf_create_request(nullable, "fresh").is_ok());
    }

    #[test]
    fn udf_clap_auth_classification_and_values() {
        for (args, write) in [
            (vec!["list", "--cursor", "next", "--limit", "2"], false),
            (vec!["get", "my_udf"], false),
            (vec!["delete", "my_udf"], true),
            (
                vec!["create", "--config-file", "-", "--artifact", "code.zip"],
                true,
            ),
            (vec!["attach", "my_udf", "svc-1", "--version", "2"], true),
            (vec!["detach", "my_udf", "svc-1"], true),
            (
                vec![
                    "attachment",
                    "list",
                    "my_udf",
                    "--cursor",
                    "next",
                    "--limit",
                    "100",
                ],
                false,
            ),
            (vec!["attachment", "get", "my_udf", "svc-1"], false),
            (vec!["version", "list", "my_udf"], false),
            (
                vec![
                    "version",
                    "create",
                    "my_udf",
                    "--config-file",
                    "file.json",
                    "--artifact",
                    "code.zip",
                ],
                true,
            ),
            (vec!["version", "delete", "my_udf", "3"], true),
        ] {
            let mut all = vec!["chctl", "cloud", "udf"];
            all.extend(args);
            all.extend(["--org-id", "org-1"]);
            let cli = Cli::try_parse_from(all).unwrap();
            let Commands::Cloud(cloud) = cli.command else {
                panic!("cloud");
            };
            assert_eq!(cloud.command.is_write_command(), write);
            let CloudCommands::Udf(udf) = cloud.command else {
                panic!("udf");
            };
            assert_eq!(udf.org_id.as_deref(), Some("org-1"));
            match udf.command {
                UdfCommands::List(page) => {
                    assert_eq!(page.cursor.as_deref(), Some("next"));
                    assert_eq!(page.limit, Some(2));
                }
                UdfCommands::Create(input) => {
                    assert_eq!(input.config, "-");
                    assert_eq!(input.artifact, PathBuf::from("code.zip"));
                }
                UdfCommands::Attach { version, .. } => assert_eq!(version, Some(2)),
                UdfCommands::Version {
                    command: UdfVersionCommands::Create { input, .. },
                } => {
                    assert_eq!(input.config, "file.json");
                    assert_eq!(input.artifact, PathBuf::from("code.zip"));
                }
                _ => {}
            }
        }
        for args in [
            vec!["list", "--limit", "0"],
            vec!["list", "--limit", "101"],
            vec!["attach", "my_udf", "svc-1", "--version", "0"],
            vec!["version", "delete", "my_udf", "0"],
            vec!["get", "../oops"],
            vec!["create", "--config-file", "config.json"],
        ] {
            assert!(
                Cli::try_parse_from(["chctl", "cloud", "udf"].into_iter().chain(args)).is_err()
            );
        }
    }
}
