use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::config::{deserialize_strict_config, read_config_value};
use crate::cloud::output::{eprint_line, or_absent, print_human, print_line};
use crate::cloud::shared::{PollProgress, resolve_org_id};
use crate::cloud::types::DeleteResponse;
use crate::failure::{ApiFailure, FailureKind};
use crate::udf::{self, SourceEntry, UdfRuntimeKind};
use clap::{Args, Subcommand};
use clickhouse_cloud_api::models::*;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tabled::{Table, Tabled, settings::Style};

/// Gap between polls while `--wait` follows a build, a wake, or a deployment.
const UDF_POLL_INTERVAL: Duration = Duration::from_secs(5);
/// Longest `--wake` waits for an idle service to reach `running`.
const SERVICE_WAKE_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Args)]
pub struct UdfArgs {
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
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  --source-dir archives a directory whose udf.json is the definition; --file overrides it.\n  python3.11 archives need main.py at the root; symbolic links are rejected before any upload.\n  Without --wait the command returns while the build runs; follow it with `udf get`."
    )]
    Create(UdfCreateArgs),
    /// Delete a UDF
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Deletes every version and detaches the UDF from all services.\n  A UDF cannot be deleted while any version is still building.\n  Service removal completes asynchronously."
    )]
    Delete(UdfNameArgs),
    /// Attach a UDF to a service
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Replaces the service's attached version; omission selects the latest ready version.\n  An idle service fails with HTTP 424 unless --wake wakes it first; a stopped service must be started.\n  --wait returns once the attachment is deployed."
    )]
    Attach {
        #[command(flatten)]
        target: UdfAttachmentArgs,
        /// UDF version number
        #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
        version: Option<i64>,
        #[command(flatten)]
        wait: UdfAttachWaitArgs,
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
        after_help = "CONTEXT FOR AGENTS:\n  Supply the complete definition; omitted options use defaults, not previous values.\n  --source-dir reads DIR/udf.json, whose functionName must match NAME and is dropped from the request.\n  Each retry uploads a fresh archive and consumes a new upload session."
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
    #[arg(
        long = "file",
        value_name = "PATH",
        aliases = ["config-file", "config"],
        required_unless_present = "source_dir"
    )]
    config: Option<String>,
    /// Source archive path in ZIP format
    #[arg(
        long,
        value_name = "PATH",
        conflicts_with = "source_dir",
        required_unless_present = "source_dir"
    )]
    artifact: Option<PathBuf>,
    /// Directory to archive; its udf.json is the definition unless --file is given
    #[arg(long, value_name = "DIR")]
    source_dir: Option<PathBuf>,
    #[command(flatten)]
    wait: UdfBuildWaitArgs,
}

#[derive(Args)]
pub struct UdfBuildWaitArgs {
    /// Wait until the build reaches ready or error
    #[arg(long)]
    wait: bool,
    /// Seconds to wait for the build; only with --wait
    #[arg(
        long,
        value_name = "SECONDS",
        default_value_t = 1800,
        requires = "wait",
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    timeout: u64,
}

#[derive(Args)]
pub struct UdfAttachWaitArgs {
    /// Wait until the attachment is deployed
    #[arg(long)]
    wait: bool,
    /// Seconds to wait for deployment; only with --wait
    #[arg(
        long,
        value_name = "SECONDS",
        default_value_t = 600,
        requires = "wait",
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    timeout: u64,
    /// Wake an idle service before attaching
    #[arg(long)]
    wake: bool,
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
    crate::udf::validate_function_name(value).map(|()| value.to_owned())
}

pub async fn run(client: &CloudClient, args: UdfArgs, json: bool) -> CloudResult<()> {
    // Parse, validate and package input before even auto-resolving the
    // organization, so nothing reaches the API for a bad definition.
    match args.command {
        UdfCommands::Create(input) => {
            let definition =
                definition_source(input.config.as_deref(), input.source_dir.as_deref())?;
            let mut request = build_udf_create_request(definition, "pending")?;
            let artifact = resolve_artifact(
                input.artifact.as_deref(),
                input.source_dir.as_deref(),
                create_request_runtime(&request)?,
            )?;
            let file = open_artifact(artifact.path()).await?;
            let org = resolve_org_id(client).await?;
            let upload_id = upload_artifact(client, &org, file).await?;
            match &mut request {
                UdfCreateRequest::UdfCreateRequestV1(body) => body.upload_id = upload_id,
                UdfCreateRequest::UdfCreateRequestV2(body) => body.upload_id = upload_id,
                UdfCreateRequest::Unknown(_) => unreachable!("builder accepts known variants only"),
            }
            let created = client.create_udf(&org, &request).await?;
            drop(artifact);
            report_build(client, &org, created, &input.wait, json).await
        }
        UdfCommands::Version {
            command: UdfVersionCommands::Create { name, input },
        } => {
            let definition = version_definition_source(
                input.config.as_deref(),
                input.source_dir.as_deref(),
                &name.function_name,
            )?;
            let mut request = build_udf_version_create_request(definition, "pending")?;
            let artifact = resolve_artifact(
                input.artifact.as_deref(),
                input.source_dir.as_deref(),
                version_request_runtime(&request)?,
            )?;
            let file = open_artifact(artifact.path()).await?;
            let org = resolve_org_id(client).await?;
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
            let created = client
                .create_udf_version(&org, &name.function_name, &request)
                .await?;
            drop(artifact);
            report_build(client, &org, created, &input.wait, json).await
        }
        command => {
            let org = resolve_org_id(client).await?;
            match command {
                UdfCommands::List(page) => {
                    let data = client
                        .list_udfs(&org, page.cursor.as_deref(), page.limit)
                        .await?;
                    if json {
                        output(&data, true)
                    } else {
                        print_udfs(data.items, data.pagination, "UDFs")
                    }
                }
                UdfCommands::Get(name) => {
                    output(&client.get_udf(&org, &name.function_name).await?, json)
                }
                UdfCommands::Delete(name) => {
                    let data = client.delete_udf(&org, &name.function_name).await?;
                    if json {
                        output(&data, true)
                    } else {
                        print_line(format!("UDF {} deleted", name.function_name));
                        Ok(())
                    }
                }
                UdfCommands::Attach {
                    target,
                    version,
                    wait,
                } => {
                    let attached = attach_with_wake(
                        client,
                        &org,
                        &target.name.function_name,
                        &target.service_id,
                        version,
                        wait.wake,
                        !json,
                    )
                    .await?;
                    if wait.wait {
                        let deployed = wait_for_attachment(
                            client,
                            &org,
                            &target.name.function_name,
                            &target.service_id,
                            Duration::from_secs(wait.timeout),
                            !json,
                        )
                        .await?;
                        output(&deployed, json)
                    } else {
                        output(&attached, json)
                    }
                }
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
                            print_udfs(data.items, data.pagination, "UDF versions")
                        }
                    }
                    UdfVersionCommands::Delete { name, version } => {
                        let data = client
                            .delete_udf_version(&org, &name.function_name, version)
                            .await?;
                        if json {
                            output(&data, true)
                        } else {
                            print_line(format!(
                                "UDF {} version {} deleted",
                                name.function_name, version
                            ));
                            Ok(())
                        }
                    }
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

fn print_udfs(
    items: Option<Vec<Udf>>,
    pagination: Option<Pagination>,
    label: &str,
) -> CloudResult<()> {
    #[derive(Tabled)]
    struct Row {
        name: String,
        version: String,
        runtime: String,
        status: String,
    }
    match items {
        Some(items) if items.is_empty() => println!("No {label} found"),
        Some(items) => {
            let rows = items.into_iter().map(|item| Row {
                name: or_absent(item.function_name),
                version: or_absent(item.version),
                runtime: or_absent(item.runtime),
                status: or_absent(item.status),
            });
            println!("{}", Table::new(rows).with(Style::markdown()));
        }
        None => println!("{label}: -"),
    }
    print_pagination(pagination);
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
    match items {
        Some(items) if items.is_empty() => println!("No UDF attachments found"),
        Some(items) => {
            let rows = items.into_iter().map(|item| Row {
                name: or_absent(item.function_name),
                service_id: or_absent(item.service_id),
                version: or_absent(item.version),
                status: or_absent(item.status),
            });
            println!("{}", Table::new(rows).with(Style::markdown()));
        }
        None => println!("UDF attachments: -"),
    }
    print_pagination(pagination);
    Ok(())
}

fn print_pagination(pagination: Option<Pagination>) {
    let Some(page) = pagination else {
        return;
    };
    let mut details = Vec::new();
    if let Some(total) = page.total_records {
        details.push(format!("{total} total records"));
    }
    if let Some(limit) = page.limit {
        details.push(format!("page limit {limit}"));
    }
    if let Some(cursor) = page.current_cursor {
        details.push(format!("current cursor: {cursor}"));
    }
    if let Some(cursor) = page.next_cursor {
        details.push(format!("next cursor: {cursor}"));
    }
    if !details.is_empty() {
        println!("Pagination: {}", details.join("; "));
    }
}

// ── definitions and archives ────────────────────────────────────────────────

/// The definition JSON: `--file` when given, else `DIR/udf.json`.
fn definition_source(config: Option<&str>, source_dir: Option<&Path>) -> CloudResult<Value> {
    match (config, source_dir) {
        (Some(config), _) => read_config_value(config),
        (None, Some(dir)) => {
            udf::load_definition_from_dir(dir).map_err(|error| CloudError::new(error.to_string()))
        }
        (None, None) => Err(CloudError::usage(
            "Pass --file <PATH> or --source-dir <DIR>",
        )),
    }
}

/// Version requests carry the name in the URL. A `udf.json` taken from
/// `--source-dir` names the function, so the name is checked and dropped; an
/// explicit `--file` is used as written.
fn version_definition_source(
    config: Option<&str>,
    source_dir: Option<&Path>,
    function_name: &str,
) -> CloudResult<Value> {
    let mut value = definition_source(config, source_dir)?;
    if config.is_none()
        && source_dir.is_some()
        && let Some(found) = udf::strip_function_name(&mut value)
        && found != function_name
    {
        return Err(CloudError::usage(format!(
            "udf.json names function {found}, but the command targets {function_name}"
        )));
    }
    Ok(value)
}

/// Shape-check the definition (shared with `local udf`), then stamp the
/// upload session ID the CLI owns into it.
fn validate_udf_config(value: &mut Value, upload_id: &str, create: bool) -> CloudResult<String> {
    let kind = crate::udf::validate_definition(value, create).map_err(CloudError::new)?;
    if let Some(object) = value.as_object_mut() {
        object.insert("uploadId".into(), Value::String(upload_id.to_owned()));
    }
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

fn runtime_kind(runtime: &UdfRuntime) -> CloudResult<UdfRuntimeKind> {
    match runtime {
        UdfRuntime::Python3_11 => Ok(UdfRuntimeKind::Python311),
        UdfRuntime::Native => Ok(UdfRuntimeKind::Native),
        UdfRuntime::Unknown(_) => Err(CloudError::new("Unsupported UDF runtime")),
    }
}

fn create_request_runtime(request: &UdfCreateRequest) -> CloudResult<UdfRuntimeKind> {
    match request {
        UdfCreateRequest::UdfCreateRequestV1(body) => runtime_kind(&body.runtime),
        UdfCreateRequest::UdfCreateRequestV2(body) => runtime_kind(&body.runtime),
        UdfCreateRequest::Unknown(_) => Err(CloudError::new("Unsupported UDF type")),
    }
}

fn version_request_runtime(request: &UdfVersionCreateRequest) -> CloudResult<UdfRuntimeKind> {
    match request {
        UdfVersionCreateRequest::UdfVersionCreateRequestV1(body) => runtime_kind(&body.runtime),
        UdfVersionCreateRequest::UdfVersionCreateRequestV2(body) => runtime_kind(&body.runtime),
        UdfVersionCreateRequest::Unknown(_) => Err(CloudError::new("Unsupported UDF type")),
    }
}

/// The archive to upload: a user-supplied ZIP, or one packaged from
/// `--source-dir` that lives until the upload completes.
enum Artifact {
    File(PathBuf),
    Packaged(tempfile::NamedTempFile),
}

impl Artifact {
    fn path(&self) -> &Path {
        match self {
            Self::File(path) => path,
            Self::Packaged(file) => file.path(),
        }
    }
}

fn resolve_artifact(
    artifact: Option<&Path>,
    source_dir: Option<&Path>,
    runtime: UdfRuntimeKind,
) -> CloudResult<Artifact> {
    match (artifact, source_dir) {
        (Some(path), _) => Ok(Artifact::File(path.to_path_buf())),
        (None, Some(dir)) => package_source_dir(dir, runtime).map(Artifact::Packaged),
        (None, None) => Err(CloudError::usage(
            "Pass --artifact <PATH> or --source-dir <DIR>",
        )),
    }
}

/// Package a source directory into a deterministic ZIP: the shared walk
/// fixes the entry order and exclusions, timestamps are constant, and Unix
/// permission bits are kept so a native entrypoint stays executable.
fn package_source_dir(dir: &Path, runtime: UdfRuntimeKind) -> CloudResult<tempfile::NamedTempFile> {
    let entries = udf::collect_source_entries(dir, runtime)
        .map_err(|error| CloudError::new(error.to_string()))?;
    let mut archive = tempfile::Builder::new()
        .prefix("chctl-udf-")
        .suffix(".zip")
        .tempfile()
        .map_err(|error| CloudError::new(format!("Cannot create a temporary archive: {error}")))?;
    write_archive(archive.as_file_mut(), &entries)
        .map_err(|error| CloudError::new(format!("Cannot archive {}: {error}", dir.display())))?;
    Ok(archive)
}

fn write_archive(file: &mut std::fs::File, entries: &[SourceEntry]) -> std::io::Result<()> {
    let mut writer = zip::ZipWriter::new(file);
    for entry in entries {
        let name = entry.relative.to_str().ok_or_else(|| {
            std::io::Error::other(format!(
                "path {} is not valid UTF-8",
                entry.relative.display()
            ))
        })?;
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .last_modified_time(zip::DateTime::default())
            .unix_permissions(entry.mode);
        if entry.is_dir {
            writer.add_directory(name, options)?;
        } else {
            writer.start_file(name, options)?;
            let mut source = std::fs::File::open(&entry.absolute)?;
            std::io::copy(&mut source, &mut writer)?;
        }
    }
    writer.finish()?;
    Ok(())
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

// ── waiting ─────────────────────────────────────────────────────────────────

/// What one poll of a build, attachment or service showed.
enum Outcome {
    Done,
    Pending(String),
    Failed(String),
}

/// Poll `fetch` every [`UDF_POLL_INTERVAL`] until `classify` says the wait
/// is over. Progress lines (state transitions only) go to stderr in human
/// mode; the caller prints the final object exactly once.
async fn wait_for<T, Fut, F, C>(
    what: String,
    timeout: Duration,
    verbose: bool,
    mut fetch: F,
    classify: C,
) -> CloudResult<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = CloudResult<T>>,
    C: Fn(&T) -> CloudResult<Outcome>,
{
    let started = Instant::now();
    let mut progress = PollProgress::default();
    if verbose {
        eprint_line(format!("Waiting for {what} (up to {}s)", timeout.as_secs()));
    }
    loop {
        let value = fetch().await?;
        match classify(&value)? {
            Outcome::Done => return Ok(value),
            Outcome::Failed(reason) => {
                return Err(CloudError::new(format!("{what} failed: {reason}")));
            }
            Outcome::Pending(state) => {
                if verbose && let Some(line) = progress.render(&state, false) {
                    eprint_line(line);
                }
            }
        }
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            return Err(wait_timeout_error(&what, timeout));
        }
        tokio::time::sleep(UDF_POLL_INTERVAL.min(timeout - elapsed)).await;
    }
}

fn wait_timeout_error(what: &str, timeout: Duration) -> CloudError {
    CloudError::new(format!(
        "{what} did not finish within {}s; it may still complete, check `cloud udf get` or \
         `cloud udf attachment get`",
        timeout.as_secs()
    ))
    .with_failure(ApiFailure::new(FailureKind::Timeout))
}

fn classify_build_status(udf: &Udf) -> CloudResult<Outcome> {
    match &udf.status {
        Some(UdfStatus::Ready) => Ok(Outcome::Done),
        Some(UdfStatus::Error) => Ok(Outcome::Failed(
            udf.error
                .clone()
                .filter(|error| !error.trim().is_empty())
                .unwrap_or_else(|| "the build reported an error without a message".into()),
        )),
        Some(status @ (UdfStatus::Building | UdfStatus::Unknown(_))) => {
            Ok(Outcome::Pending(status.to_string()))
        }
        None => Err(CloudError::new(
            "UDF response omitted status; follow the build with `cloud udf get`",
        )),
    }
}

fn classify_attachment_status(attachment: &UdfAttachment) -> CloudResult<Outcome> {
    match &attachment.status {
        Some(UdfAttachmentStatus::Deployed | UdfAttachmentStatus::Standby) => Ok(Outcome::Done),
        Some(UdfAttachmentStatus::Error) => Ok(Outcome::Failed(
            "the attachment entered the error state".into(),
        )),
        Some(UdfAttachmentStatus::Deprovisioning) => Ok(Outcome::Failed(
            "the attachment is being deprovisioned".into(),
        )),
        Some(status @ (UdfAttachmentStatus::Provisioning | UdfAttachmentStatus::Unknown(_))) => {
            Ok(Outcome::Pending(status.to_string()))
        }
        None => Err(CloudError::new(
            "attachment response omitted status; check `cloud udf attachment get`",
        )),
    }
}

fn classify_wake_state(state: Option<&ServiceState>) -> CloudResult<Outcome> {
    match state {
        Some(ServiceState::Running) => Ok(Outcome::Done),
        Some(
            state @ (ServiceState::Stopped
            | ServiceState::Stopping
            | ServiceState::Terminating
            | ServiceState::Terminated
            | ServiceState::Softdeleting
            | ServiceState::Softdeleted
            | ServiceState::Failed),
        ) => Ok(Outcome::Failed(format!(
            "the service is {state}; start it with `cloud service start <id>` before attaching"
        ))),
        Some(state) => Ok(Outcome::Pending(state.to_string())),
        None => Err(CloudError::new("service response omitted state")),
    }
}

/// Identify the version a create or version-create response describes, so
/// the wait follows exactly that build.
fn build_identity(udf: &Udf) -> CloudResult<(String, i64)> {
    match (udf.function_name.as_deref(), udf.version) {
        (Some(name), Some(version)) => Ok((name.to_owned(), version)),
        _ => Err(CloudError::new(
            "UDF response omitted functionName or version; follow the build with `cloud udf get`",
        )),
    }
}

async fn report_build(
    client: &CloudClient,
    org: &str,
    created: Udf,
    wait: &UdfBuildWaitArgs,
    json: bool,
) -> CloudResult<()> {
    if !wait.wait {
        return output(&created, json);
    }
    let (name, version) = build_identity(&created)?;
    let built = wait_for_udf_version(
        client,
        org,
        &name,
        version,
        Duration::from_secs(wait.timeout),
        !json,
    )
    .await?;
    output(&built, json)
}

/// `get` describes the latest version, which is normally the one just
/// created; if another version appeared meanwhile, scan the version list.
async fn fetch_udf_version(
    client: &CloudClient,
    org: &str,
    name: &str,
    version: i64,
) -> CloudResult<Udf> {
    let latest = client.get_udf(org, name).await?;
    if latest.version == Some(version) {
        return Ok(latest);
    }
    let mut cursor: Option<String> = None;
    loop {
        let page = client
            .list_udf_versions(org, name, cursor.as_deref(), Some(100))
            .await?;
        if let Some(found) = page
            .items
            .unwrap_or_default()
            .into_iter()
            .find(|udf| udf.version == Some(version))
        {
            return Ok(found);
        }
        cursor = page.pagination.and_then(|page| page.next_cursor);
        if cursor.is_none() {
            return Err(CloudError::new(format!(
                "UDF {name} version {version} was not found while waiting for its build"
            )));
        }
    }
}

async fn wait_for_udf_version(
    client: &CloudClient,
    org: &str,
    name: &str,
    version: i64,
    timeout: Duration,
    verbose: bool,
) -> CloudResult<Udf> {
    wait_for(
        format!("the build of UDF {name} version {version}"),
        timeout,
        verbose,
        || fetch_udf_version(client, org, name, version),
        classify_build_status,
    )
    .await
}

async fn wait_for_attachment(
    client: &CloudClient,
    org: &str,
    name: &str,
    service: &str,
    timeout: Duration,
    verbose: bool,
) -> CloudResult<UdfAttachment> {
    wait_for(
        format!("the deployment of UDF {name} to service {service}"),
        timeout,
        verbose,
        || client.get_udf_attachment(org, name, service),
        classify_attachment_status,
    )
    .await
}

async fn wait_for_service_running(
    client: &CloudClient,
    org: &str,
    service: &str,
    timeout: Duration,
    verbose: bool,
) -> CloudResult<Service> {
    wait_for(
        format!("the wake of service {service}"),
        timeout,
        verbose,
        || client.get_service(org, service),
        |service| classify_wake_state(service.state.as_ref()),
    )
    .await
}

// ── attaching and waking ────────────────────────────────────────────────────

/// What the typed HTTP 424 payload allows the CLI to do about it.
#[derive(Debug, PartialEq, Eq)]
enum WakeAction {
    /// The service is idle and may be woken.
    Wake,
    /// The service is already coming up; just wait.
    WaitOnly,
    /// Stopped, unknown, or not wakeable: report.
    GiveUp,
}

fn wake_action(response: &UdfAttachResponse424) -> WakeAction {
    match (&response.code, response.can_wake, &response.service_state) {
        (Some(UdfAttachErrorCode::ServiceIdle), Some(true), _) => WakeAction::Wake,
        (
            Some(UdfAttachErrorCode::ServiceNotRunning),
            _,
            Some(ServiceState::Awaking | ServiceState::Starting | ServiceState::Provisioning),
        ) => WakeAction::WaitOnly,
        _ => WakeAction::GiveUp,
    }
}

/// Keep the API's message and classification, add the typed reason, and
/// name the way out.
fn attach_unavailable_error(
    error: CloudError,
    response: &UdfAttachResponse424,
    service: &str,
) -> CloudError {
    let mut message = error.message.clone();
    let mut context = Vec::new();
    if let Some(code) = &response.code {
        context.push(code.to_string());
    }
    if let Some(state) = &response.service_state {
        context.push(format!("service state {state}"));
    }
    if !context.is_empty() {
        message.push_str(&format!(" ({})", context.join(", ")));
    }
    let hint = match wake_action(response) {
        WakeAction::Wake => Some(format!(
            "Rerun with --wake, or run `clickhousectl cloud service wake {service}` and retry once it is running."
        )),
        WakeAction::WaitOnly => Some(
            "The service is starting; retry shortly, or rerun with --wake to wait for it.".into(),
        ),
        WakeAction::GiveUp => matches!(response.code, Some(UdfAttachErrorCode::ServiceStopped))
            .then(|| {
                format!("Start it with `clickhousectl cloud service start {service}` and retry.")
            }),
    };
    if let Some(hint) = hint {
        message.push('\n');
        message.push_str(&hint);
    }
    CloudError { message, ..error }
}

async fn attach_with_wake(
    client: &CloudClient,
    org: &str,
    name: &str,
    service: &str,
    version: Option<i64>,
    wake: bool,
    verbose: bool,
) -> CloudResult<UdfAttachment> {
    let (response, error) = match client
        .attach_udf_checked(org, name, service, version)
        .await?
    {
        UdfAttachOutcome::Attached(attachment) => return Ok(attachment),
        UdfAttachOutcome::Unavailable { response, error } => (response, error),
    };
    let action = wake_action(&response);
    if !wake || action == WakeAction::GiveUp {
        return Err(attach_unavailable_error(error, &response, service));
    }
    if action == WakeAction::Wake {
        if verbose {
            eprint_line(format!(
                "Service {service} is idle; waking it before attaching"
            ));
        }
        if let Err(wake_error) = client
            .change_service_state(org, service, ServiceStatePatchRequestCommand::Awake)
            .await
        {
            // Another caller may have woken it first; only give up when the
            // service is not on its way up.
            let state = client.get_service(org, service).await?.state;
            if !matches!(
                state,
                Some(ServiceState::Awaking | ServiceState::Starting | ServiceState::Running)
            ) {
                return Err(wake_error);
            }
        }
    } else if verbose {
        eprint_line(format!(
            "Service {service} is starting; waiting before attaching"
        ));
    }
    wait_for_service_running(client, org, service, SERVICE_WAKE_TIMEOUT, verbose).await?;
    match client
        .attach_udf_checked(org, name, service, version)
        .await?
    {
        UdfAttachOutcome::Attached(attachment) => Ok(attachment),
        UdfAttachOutcome::Unavailable { response, error } => {
            Err(attach_unavailable_error(error, &response, service))
        }
    }
}

/// Result of an attach call that keeps the typed HTTP 424 payload.
pub(crate) enum UdfAttachOutcome {
    Attached(UdfAttachment),
    Unavailable {
        response: Box<UdfAttachResponse424>,
        /// The same failure converted the ordinary way, so classification
        /// and the API's message are preserved when it is reported.
        error: CloudError,
    },
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
    /// Attach, keeping the typed HTTP 424 payload instead of flattening it
    /// into a message, so callers can decide whether to wake the service.
    async fn attach_udf_checked(
        &self,
        org: &str,
        name: &str,
        service: &str,
        version: Option<i64>,
    ) -> CloudResult<UdfAttachOutcome> {
        match self.api().udf_attach(org, name, service, version).await {
            Ok(response) => Ok(UdfAttachOutcome::Attached(Self::unwrap_response(response)?)),
            Err(clickhouse_cloud_api::Error::UdfAttachmentUnavailable {
                status,
                message,
                response,
            }) => {
                let error = self.convert_error_for_organization(
                    clickhouse_cloud_api::Error::UdfAttachmentUnavailable {
                        status,
                        message,
                        response: response.clone(),
                    },
                    org,
                );
                Ok(UdfAttachOutcome::Unavailable { response, error })
            }
            Err(error) => Err(self.convert_error_for_organization(error, org)),
        }
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
    #[test]
    fn primary_json_file_argument_contract() {
        crate::cloud::config::assert_primary_json_input(
            &["cloud", "udf", "create", "--artifact", "source.zip"],
            "config",
            &["config-file", "config"],
        );
        crate::cloud::config::assert_primary_json_input(
            &[
                "cloud",
                "udf",
                "version",
                "create",
                "my_udf",
                "--artifact",
                "source.zip",
            ],
            "config",
            &["config-file", "config"],
        );
    }

    use super::*;
    use crate::cli::{Cli, Commands};
    use crate::cloud::cli::CloudCommands;
    use clap::Parser;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;

    fn definition(kind: &str, create: bool) -> Value {
        let mut value = json!({"type": kind, "runtime": "native", "arguments": [{"name": "x", "type": "UInt64"}], "returnType": "UInt64"});
        if create {
            value["functionName"] = json!("my_udf");
        }
        value
    }

    #[test]
    fn udf_builders_cover_minimal_and_maximal_variants() {
        let minimal = build_udf_create_request(definition("executable", true), "fresh").unwrap();
        let UdfCreateRequest::UdfCreateRequestV1(body) = minimal else {
            panic!("executable");
        };
        assert_eq!(body.upload_id, "fresh");
        assert_eq!(body.function_name, "my_udf");
        assert_eq!(body.runtime, UdfRuntime::Native);
        assert_eq!(body.pool_size, None);
        assert_eq!(body.memory_limit_mib, None);
        assert_eq!(body.deterministic, None);

        let mut maximal = definition("executable_pool", false);
        maximal.as_object_mut().unwrap().extend(
            json!({
                "returnName": "result", "format": "JSONEachRow", "commandReadTimeout": 5000,
                "commandWriteTimeout": 6000, "maxCommandExecutionTime": 20, "memoryLimitMib": 256,
                "sendChunkHeader": true, "deterministic": true, "sandboxType": "netenable",
                "sandboxVersion": "v3", "poolSize": 4
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        let maximal = build_udf_version_create_request(maximal, "fresh").unwrap();
        let UdfVersionCreateRequest::UdfVersionCreateRequestV2(body) = maximal else {
            panic!("executable_pool");
        };
        assert_eq!(body.upload_id, "fresh");
        assert_eq!(body.return_name.as_deref(), Some("result"));
        assert_eq!(body.format.as_deref(), Some("JSONEachRow"));
        assert_eq!(body.command_read_timeout, Some(5000));
        assert_eq!(body.command_write_timeout, Some(6000));
        assert_eq!(body.max_command_execution_time, Some(20));
        assert_eq!(body.memory_limit_mib, Some(256));
        assert_eq!(body.send_chunk_header, Some(true));
        assert_eq!(body.deterministic, Some(true));
        assert_eq!(body.sandbox_type, Some(UdfSandboxType::Netenable));
        assert_eq!(body.sandbox_version, Some(UdfSandboxVersion::V3));
        assert_eq!(body.pool_size, Some(4));
        assert_eq!(
            serde_json::to_value(&body).unwrap()["deterministic"],
            json!(true)
        );
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
    fn local_udf_scaffold_definition_builds_a_cloud_create_request() {
        use crate::local::cli::{UdfRuntimeArg, UdfTypeArg};
        for (runtime, kind, expected) in [
            (
                UdfRuntimeArg::Python311,
                UdfTypeArg::Executable,
                UdfRuntime::Python3_11,
            ),
            (
                UdfRuntimeArg::Native,
                UdfTypeArg::ExecutablePool,
                UdfRuntime::Native,
            ),
        ] {
            let text = crate::local::udf::definition_template("my_fn", runtime, kind);
            let value: Value = serde_json::from_str(&text).unwrap();
            match build_udf_create_request(value, "fresh").unwrap() {
                UdfCreateRequest::UdfCreateRequestV1(body) => {
                    assert_eq!(kind, UdfTypeArg::Executable);
                    assert_eq!(body.runtime, expected);
                    assert_eq!(body.function_name, "my_fn");
                }
                UdfCreateRequest::UdfCreateRequestV2(body) => {
                    assert_eq!(kind, UdfTypeArg::ExecutablePool);
                    assert_eq!(body.runtime, expected);
                    assert_eq!(body.upload_id, "fresh");
                }
                UdfCreateRequest::Unknown(_) => panic!("known variant"),
            }
        }
    }

    fn write_source_dir(dir: &Path) {
        std::fs::create_dir_all(dir.join("lib/__pycache__")).unwrap();
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(
            dir.join("udf.json"),
            definition("executable", true).to_string(),
        )
        .unwrap();
        std::fs::write(dir.join("main.py"), "import sys\n").unwrap();
        std::fs::write(dir.join("lib/helper.py"), "x = 1\n").unwrap();
        std::fs::write(dir.join("lib/__pycache__/helper.pyc"), "").unwrap();
        std::fs::write(dir.join(".env"), "SECRET=1\n").unwrap();
        std::fs::write(dir.join("bin/tool"), "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(dir.join("bin/tool"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }

    #[test]
    fn package_source_dir_is_deterministic_and_applies_the_shared_walk() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("my_udf");
        write_source_dir(&dir);

        let first = package_source_dir(&dir, UdfRuntimeKind::Python311).unwrap();
        let second = package_source_dir(&dir, UdfRuntimeKind::Python311).unwrap();
        let bytes = std::fs::read(first.path()).unwrap();
        assert_eq!(bytes, std::fs::read(second.path()).unwrap());
        assert!(bytes.starts_with(b"PK\x03\x04"));

        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().to_string())
            .collect();
        assert_eq!(
            names,
            ["bin/", "bin/tool", "lib/", "lib/helper.py", "main.py"]
        );
        let tool = archive.by_name("bin/tool").unwrap();
        assert_eq!(tool.unix_mode().unwrap() & 0o111, 0o111);
        drop(tool);
        let mut main = archive.by_name("main.py").unwrap();
        let mut contents = String::new();
        std::io::Read::read_to_string(&mut main, &mut contents).unwrap();
        assert_eq!(contents, "import sys\n");
    }

    #[test]
    fn package_source_dir_rejects_symlinks_and_missing_entrypoints_without_an_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("my_udf");
        write_source_dir(&dir);
        std::fs::remove_file(dir.join("main.py")).unwrap();
        let error = package_source_dir(&dir, UdfRuntimeKind::Python311).unwrap_err();
        assert!(error.to_string().contains("is missing main.py"), "{error}");
        assert!(package_source_dir(&dir, UdfRuntimeKind::Native).is_ok());

        std::os::unix::fs::symlink(dir.join("lib/helper.py"), dir.join("link.py")).unwrap();
        let error = package_source_dir(&dir, UdfRuntimeKind::Native).unwrap_err();
        assert!(error.to_string().contains("symbolic link"), "{error}");

        let error =
            package_source_dir(&tmp.path().join("missing"), UdfRuntimeKind::Native).unwrap_err();
        assert!(error.to_string().contains("is not a directory"), "{error}");
    }

    #[test]
    fn definition_sources_prefer_file_and_check_version_names() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("my_udf");
        write_source_dir(&dir);
        let override_path = tmp.path().join("override.json");
        std::fs::write(
            &override_path,
            definition("executable_pool", true).to_string(),
        )
        .unwrap();

        let from_dir = definition_source(None, Some(&dir)).unwrap();
        assert_eq!(from_dir["type"], "executable");
        let from_file =
            definition_source(Some(override_path.to_str().unwrap()), Some(&dir)).unwrap();
        assert_eq!(from_file["type"], "executable_pool");
        assert_eq!(
            definition_source(None, None).unwrap_err().kind,
            crate::cloud::client::CloudErrorKind::Usage
        );

        let stripped = version_definition_source(None, Some(&dir), "my_udf").unwrap();
        assert!(stripped.get("functionName").is_none());
        let kept =
            version_definition_source(Some(override_path.to_str().unwrap()), Some(&dir), "other")
                .unwrap();
        assert_eq!(kept["functionName"], "my_udf");
        let mismatch = version_definition_source(None, Some(&dir), "other").unwrap_err();
        assert_eq!(mismatch.kind, crate::cloud::client::CloudErrorKind::Usage);
        assert!(
            mismatch.to_string().contains("names function my_udf"),
            "{mismatch}"
        );
    }

    #[test]
    fn request_runtimes_map_to_the_shared_kind() {
        let python = build_udf_create_request(
            {
                let mut value = definition("executable", true);
                value["runtime"] = json!("python3.11");
                value
            },
            "fresh",
        )
        .unwrap();
        assert_eq!(
            create_request_runtime(&python).unwrap(),
            UdfRuntimeKind::Python311
        );
        let native =
            build_udf_version_create_request(definition("executable_pool", false), "fresh")
                .unwrap();
        assert_eq!(
            version_request_runtime(&native).unwrap(),
            UdfRuntimeKind::Native
        );
        assert!(runtime_kind(&UdfRuntime::Unknown("future".into())).is_err());
    }

    fn outcome(result: CloudResult<Outcome>) -> String {
        match result {
            Ok(Outcome::Done) => "done".into(),
            Ok(Outcome::Pending(state)) => format!("pending:{state}"),
            Ok(Outcome::Failed(reason)) => format!("failed:{reason}"),
            Err(error) => format!("error:{error}"),
        }
    }

    #[test]
    fn build_attachment_and_wake_states_are_classified_closed() {
        let udf = |status: Option<UdfStatus>, error: Option<&str>| Udf {
            status,
            error: error.map(str::to_owned),
            ..Default::default()
        };
        assert_eq!(
            outcome(classify_build_status(&udf(Some(UdfStatus::Ready), None))),
            "done"
        );
        assert_eq!(
            outcome(classify_build_status(&udf(Some(UdfStatus::Building), None))),
            "pending:building"
        );
        assert_eq!(
            outcome(classify_build_status(&udf(
                Some(UdfStatus::Unknown("queued".into())),
                None
            ))),
            "pending:queued"
        );
        assert_eq!(
            outcome(classify_build_status(&udf(
                Some(UdfStatus::Error),
                Some("pip failed")
            ))),
            "failed:pip failed"
        );
        assert_eq!(
            outcome(classify_build_status(&udf(
                Some(UdfStatus::Error),
                Some("  ")
            ))),
            "failed:the build reported an error without a message"
        );
        assert!(outcome(classify_build_status(&udf(None, None))).starts_with("error:"));

        let attachment = |status: Option<UdfAttachmentStatus>| UdfAttachment {
            status,
            ..Default::default()
        };
        for done in [UdfAttachmentStatus::Deployed, UdfAttachmentStatus::Standby] {
            assert_eq!(
                outcome(classify_attachment_status(&attachment(Some(done)))),
                "done"
            );
        }
        assert_eq!(
            outcome(classify_attachment_status(&attachment(Some(
                UdfAttachmentStatus::Provisioning
            )))),
            "pending:provisioning"
        );
        assert!(
            outcome(classify_attachment_status(&attachment(Some(
                UdfAttachmentStatus::Error
            ))))
            .starts_with("failed:")
        );
        assert!(
            outcome(classify_attachment_status(&attachment(Some(
                UdfAttachmentStatus::Deprovisioning
            ))))
            .starts_with("failed:")
        );
        assert!(outcome(classify_attachment_status(&attachment(None))).starts_with("error:"));

        assert_eq!(
            outcome(classify_wake_state(Some(&ServiceState::Running))),
            "done"
        );
        assert_eq!(
            outcome(classify_wake_state(Some(&ServiceState::Awaking))),
            "pending:awaking"
        );
        assert_eq!(
            outcome(classify_wake_state(Some(&ServiceState::Idle))),
            "pending:idle"
        );
        assert!(
            outcome(classify_wake_state(Some(&ServiceState::Stopped)))
                .starts_with("failed:the service is stopped")
        );
        assert!(outcome(classify_wake_state(None)).starts_with("error:"));
    }

    #[test]
    fn wake_actions_and_messages_follow_the_typed_424_payload() {
        let idle = UdfAttachResponse424 {
            code: Some(UdfAttachErrorCode::ServiceIdle),
            service_state: Some(ServiceState::Idle),
            can_wake: Some(true),
            ..Default::default()
        };
        assert_eq!(wake_action(&idle), WakeAction::Wake);
        let message = attach_unavailable_error(CloudError::new("service is idle"), &idle, "svc-1")
            .to_string();
        assert_eq!(
            message,
            "service is idle (SERVICE_IDLE, service state idle)\nRerun with --wake, or run `clickhousectl cloud service wake svc-1` and retry once it is running."
        );

        let awaking = UdfAttachResponse424 {
            code: Some(UdfAttachErrorCode::ServiceNotRunning),
            service_state: Some(ServiceState::Awaking),
            can_wake: Some(false),
            ..Default::default()
        };
        assert_eq!(wake_action(&awaking), WakeAction::WaitOnly);
        assert!(
            attach_unavailable_error(CloudError::new("not running"), &awaking, "svc-1")
                .to_string()
                .contains("retry shortly")
        );

        let stopped = UdfAttachResponse424 {
            code: Some(UdfAttachErrorCode::ServiceStopped),
            service_state: Some(ServiceState::Stopped),
            can_wake: Some(false),
            ..Default::default()
        };
        assert_eq!(wake_action(&stopped), WakeAction::GiveUp);
        assert!(
            attach_unavailable_error(CloudError::new("stopped"), &stopped, "svc-1")
                .to_string()
                .contains("cloud service start svc-1")
        );

        let untyped = UdfAttachResponse424::default();
        assert_eq!(wake_action(&untyped), WakeAction::GiveUp);
        let plain = CloudError::new("dependency unavailable")
            .with_failure(ApiFailure::new(FailureKind::Http4xx));
        let error = attach_unavailable_error(plain, &untyped, "svc-1");
        assert_eq!(error.to_string(), "dependency unavailable");
        assert_eq!(error.failure.unwrap().kind, FailureKind::Http4xx);

        assert_eq!(
            build_identity(&Udf {
                function_name: Some("my_udf".into()),
                version: Some(3),
                ..Default::default()
            })
            .unwrap(),
            ("my_udf".to_string(), 3)
        );
        assert!(build_identity(&Udf::default()).is_err());
        let timeout = wait_timeout_error("the build of UDF f version 1", Duration::from_secs(7));
        assert!(timeout.to_string().contains("within 7s"), "{timeout}");
        assert_eq!(timeout.failure.unwrap().kind, FailureKind::Timeout);
    }

    fn parse_udf(args: &[&str]) -> UdfArgs {
        let mut all = vec!["chctl", "cloud", "udf"];
        all.extend_from_slice(args);
        all.extend(["--org-id", "org-1"]);
        let cli = Cli::try_parse_from(all).unwrap();
        let Commands::Cloud(cloud) = cli.command else {
            panic!("cloud");
        };
        let CloudCommands::Udf(udf) = cloud.command else {
            panic!("udf");
        };
        udf
    }

    #[test]
    fn udf_source_dir_and_wait_flags_parse_and_conflict() {
        use clap::error::ErrorKind;
        let UdfCommands::Create(input) = parse_udf(&["create", "--source-dir", "src"]).command
        else {
            panic!("create");
        };
        assert!(input.config.is_none());
        assert!(input.artifact.is_none());
        assert_eq!(input.source_dir, Some(PathBuf::from("src")));
        assert!(!input.wait.wait);
        assert_eq!(input.wait.timeout, 1800);

        let UdfCommands::Create(input) = parse_udf(&[
            "create",
            "--source-dir",
            "src",
            "--file",
            "def.json",
            "--wait",
            "--timeout",
            "60",
        ])
        .command
        else {
            panic!("create");
        };
        assert_eq!(input.config.as_deref(), Some("def.json"));
        assert!(input.wait.wait);
        assert_eq!(input.wait.timeout, 60);

        let UdfCommands::Version {
            command: UdfVersionCommands::Create { input, name },
        } = parse_udf(&[
            "version",
            "create",
            "my_udf",
            "--source-dir",
            "src",
            "--wait",
        ])
        .command
        else {
            panic!("version create");
        };
        assert_eq!(name.function_name, "my_udf");
        assert_eq!(input.source_dir, Some(PathBuf::from("src")));
        assert!(input.wait.wait);

        let UdfCommands::Attach { wait, version, .. } = parse_udf(&[
            "attach",
            "my_udf",
            "svc-1",
            "--wait",
            "--wake",
            "--timeout",
            "30",
        ])
        .command
        else {
            panic!("attach");
        };
        assert!(wait.wait && wait.wake);
        assert_eq!(wait.timeout, 30);
        assert_eq!(version, None);
        let UdfCommands::Attach { wait, .. } = parse_udf(&["attach", "my_udf", "svc-1"]).command
        else {
            panic!("attach");
        };
        assert!(!wait.wait && !wait.wake);
        assert_eq!(wait.timeout, 600);

        for (args, kind) in [
            (
                vec!["create", "--source-dir", "src", "--artifact", "z.zip"],
                ErrorKind::ArgumentConflict,
            ),
            (
                vec!["create", "--file", "f", "--artifact", "z", "--timeout", "5"],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                vec!["create", "--source-dir", "src", "--wait", "--timeout", "0"],
                ErrorKind::ValueValidation,
            ),
            (vec!["create"], ErrorKind::MissingRequiredArgument),
            (
                vec!["create", "--file", "f"],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                vec!["attach", "my_udf", "svc-1", "--timeout", "5"],
                ErrorKind::MissingRequiredArgument,
            ),
        ] {
            let mut all = vec!["chctl", "cloud", "udf"];
            all.extend(args.iter().copied());
            assert_eq!(
                Cli::try_parse_from(all).err().unwrap().kind(),
                kind,
                "{args:?}"
            );
        }
    }

    #[test]
    fn udf_clap_auth_classification_and_values() {
        for (args, write) in [
            (vec!["list", "--cursor", "next", "--limit", "2"], false),
            (vec!["get", "my_udf"], false),
            (vec!["delete", "my_udf"], true),
            (
                vec!["create", "--file", "-", "--artifact", "code.zip"],
                true,
            ),
            (vec!["create", "--source-dir", "src", "--wait"], true),
            (vec!["attach", "my_udf", "svc-1", "--version", "2"], true),
            (vec!["attach", "my_udf", "svc-1", "--wake"], true),
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
                    "--file",
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
            assert_eq!(cloud.org_id.as_deref(), Some("org-1"));
            assert_eq!(cloud.command.is_write_command(), write);
            let CloudCommands::Udf(udf) = cloud.command else {
                panic!("udf");
            };

            match udf.command {
                UdfCommands::List(page) => {
                    assert_eq!(page.cursor.as_deref(), Some("next"));
                    assert_eq!(page.limit, Some(2));
                }
                UdfCommands::Create(input) if input.source_dir.is_none() => {
                    assert_eq!(input.config.as_deref(), Some("-"));
                    assert_eq!(input.artifact, Some(PathBuf::from("code.zip")));
                }
                UdfCommands::Attach { version, wait, .. } => {
                    assert!(version == Some(2) || wait.wake);
                }
                UdfCommands::Version {
                    command: UdfVersionCommands::Create { input, .. },
                } => {
                    assert_eq!(input.config.as_deref(), Some("file.json"));
                    assert_eq!(input.artifact, Some(PathBuf::from("code.zip")));
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
            vec!["create", "--file", "config.json"],
        ] {
            assert!(
                Cli::try_parse_from(["chctl", "cloud", "udf"].into_iter().chain(args)).is_err()
            );
        }
    }
}
