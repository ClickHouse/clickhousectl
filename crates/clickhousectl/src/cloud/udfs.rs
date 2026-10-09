use super::permissions::Declaration as Permission;
use clickhouse_cloud_api::meta::operations as op;

// Declare every API call made by these workflows, including optional lookups.
pub(super) const PERMISSIONS: &[Permission] = &[
    Permission::api("udf list", &[&op::UDF_LIST]),
    Permission::api("udf get", &[&op::UDF_GET]),
    Permission::api(
        "udf create",
        &[&op::UDF_UPLOAD_SESSION_CREATE, &op::UDF_CREATE],
    ),
    Permission::api("udf delete", &[&op::UDF_DELETE]),
    Permission::api("udf attach", &[&op::UDF_ATTACH]),
    Permission::api("udf detach", &[&op::UDF_DETACH]),
    Permission::api("udf attachment list", &[&op::UDF_ATTACHMENT_LIST]),
    Permission::api("udf attachment get", &[&op::UDF_ATTACHMENT_GET]),
    Permission::api("udf version list", &[&op::UDF_VERSION_LIST]),
    Permission::api(
        "udf version create",
        &[&op::UDF_UPLOAD_SESSION_CREATE, &op::UDF_VERSION_CREATE],
    ),
    Permission::api("udf version delete", &[&op::UDF_VERSION_DELETE]),
];

use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::config::{deserialize_strict_config, read_config_value};
use crate::cloud::output::{eprint_line, or_absent, print_human, print_line};
use crate::cloud::shared::{PollProgress, resolve_org_id};
use crate::cloud::types::DeleteResponse;
use crate::failure::{ApiFailure, FailureKind};
use crate::udf::{self, DEFINITION_FILE, SourceEntry, UdfDirArg, UdfRuntimeKind};
use clap::{Args, Subcommand};
use clickhouse_cloud_api::models::*;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tabled::{Table, Tabled, settings::Style};

/// Gap between polls while the CLI waits on the service (a wake).
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
        after_help = "CONTEXT FOR AGENTS:\n  NAME archives clickhouse/udfs/NAME/ (or --dir PATH); its udf.json is the definition unless --file is given.\n  Without NAME, pass --file and --artifact (a ZIP you built). Symbolic links are rejected before any upload.\n  python3.11 archives need main.py at the root; native ones ship only amd64/main and arm64/main.\n  Returns while the build runs: poll `cloud udf get <name>` until status is `ready` or `error`."
    )]
    Create(UdfCreateArgs),
    /// Delete a UDF
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Deletes every version and detaches the UDF from all services.\n  A UDF cannot be deleted while any version is still building.\n  Service removal completes asynchronously."
    )]
    Delete(UdfNameArgs),
    /// Attach a UDF to a service
    #[command(
        after_help = "CONTEXT FOR AGENTS:\n  Replaces the service's attached version; omission selects the latest ready version.\n  An idle service fails with HTTP 424 unless --wake wakes it first; a stopped service must be started.\n  Returns while it provisions: poll `cloud udf attachment get <name> <service-id>` until status is `deployed`."
    )]
    Attach {
        #[command(flatten)]
        target: UdfAttachmentArgs,
        /// UDF version number
        #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
        version: Option<i64>,
        /// Wake an idle service before attaching
        #[arg(long)]
        wake: bool,
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
        after_help = "CONTEXT FOR AGENTS:\n  Supply the complete definition; omitted options use defaults, not previous values.\n  Archives clickhouse/udfs/NAME/ (or --dir PATH), whose udf.json must name NAME; --artifact needs --file.\n  Returns while the build runs: poll `cloud udf get <name>` until status is `ready` or `error`.\n  Each retry uploads a fresh archive and consumes a new upload session."
    )]
    Create {
        #[command(flatten)]
        name: UdfNameArgs,
        #[command(flatten)]
        input: UdfVersionInputArgs,
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

/// `create NAME [--dir PATH]` archives a directory holding `udf.json`;
/// `create --file PATH --artifact PATH` uploads a ZIP as is.
#[derive(Args)]
pub struct UdfCreateArgs {
    /// Function name; archives NAME/ under --dir instead of --artifact
    #[arg(value_name = "NAME", value_parser = parse_udf_name, conflicts_with = "artifact")]
    name: Option<String>,
    #[command(flatten)]
    dir: UdfDirArg,
    /// JSON definition without uploadId, used with --artifact (file path or - for stdin)
    #[arg(
        long = "file",
        value_name = "PATH",
        aliases = ["config-file", "config"],
        required_unless_present = "name",
        conflicts_with_all = ["name", "dir"]
    )]
    config: Option<String>,
    /// Source archive path in ZIP format
    #[arg(
        long,
        value_name = "PATH",
        required_unless_present = "name",
        conflicts_with_all = ["name", "dir"]
    )]
    artifact: Option<PathBuf>,
}

/// `version create NAME [--dir PATH]` archives the directory holding `udf.json`;
/// `--file PATH --artifact PATH` uploads a ZIP as is.
#[derive(Args)]
pub struct UdfVersionInputArgs {
    #[command(flatten)]
    dir: UdfDirArg,
    /// JSON definition without uploadId, used with --artifact (file path or - for stdin)
    #[arg(
        long = "file",
        value_name = "PATH",
        aliases = ["config-file", "config"],
        requires = "artifact"
    )]
    config: Option<String>,
    /// Source archive path in ZIP format; requires --file
    #[arg(long, value_name = "PATH", requires = "config", conflicts_with = "dir")]
    artifact: Option<PathBuf>,
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
            let source = match &input.name {
                Some(name) => Some(resolve_source(&input.dir, name)?),
                None => None,
            };
            let definition = definition_source(input.config.as_deref(), source.as_deref())?;
            if let Some(name) = &input.name {
                check_definition_name(&definition, source.as_deref(), name)?;
            }
            let mut request = build_udf_create_request(definition, "pending")?;
            let artifact = resolve_artifact(
                input.artifact.as_deref(),
                source.as_deref(),
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
            output(&created, json)
        }
        UdfCommands::Version {
            command: UdfVersionCommands::Create { name, input },
        } => {
            let source = match &input.artifact {
                Some(_) => None,
                None => Some(resolve_source(&input.dir, &name.function_name)?),
            };
            let mut definition = definition_source(input.config.as_deref(), source.as_deref())?;
            if source.is_some() {
                check_definition_name(&definition, source.as_deref(), &name.function_name)?;
                udf::strip_function_name(&mut definition);
            }
            let mut request = build_udf_version_create_request(definition, "pending")?;
            let artifact = resolve_artifact(
                input.artifact.as_deref(),
                source.as_deref(),
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
            output(&created, json)
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
                    wake,
                } => output(
                    &attach_with_wake(
                        client,
                        &org,
                        &target.name.function_name,
                        &target.service_id,
                        version,
                        wake,
                        !json,
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

/// `<--dir>/<NAME>/`, a real directory (no symlink), or a usage error.
fn resolve_source(dir: &UdfDirArg, name: &str) -> CloudResult<PathBuf> {
    udf::resolve_source_dir(&dir.dir, name).map_err(|error| CloudError::usage(error.to_string()))
}

/// The definition JSON: `--file` (only with `--artifact`), else `DIR/udf.json`.
fn definition_source(config: Option<&str>, source: Option<&Path>) -> CloudResult<Value> {
    match (config, source) {
        (Some(config), _) => read_config_value(config),
        (None, Some(dir)) => {
            udf::load_definition_from_dir(dir).map_err(|error| CloudError::new(error.to_string()))
        }
        (None, None) => Err(CloudError::usage("Pass NAME or --file <PATH>")),
    }
}

/// A `udf.json` read from `NAME/` must name `NAME`.
fn check_definition_name(definition: &Value, source: Option<&Path>, name: &str) -> CloudResult<()> {
    udf::check_function_name(definition, name).map_err(|reason| {
        let path = source
            .map(|dir| dir.join(DEFINITION_FILE))
            .unwrap_or_else(|| PathBuf::from(DEFINITION_FILE));
        CloudError::usage(format!("{}: {reason}", path.display()))
    })
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

/// The archive to upload: a user-supplied ZIP, or one packaged from the UDF
/// directory that lives until the upload completes.
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
    source: Option<&Path>,
    runtime: UdfRuntimeKind,
) -> CloudResult<Artifact> {
    match (artifact, source) {
        (Some(path), _) => Ok(Artifact::File(path.to_path_buf())),
        (None, Some(dir)) => package_source_dir(dir, runtime).map(Artifact::Packaged),
        (None, None) => Err(CloudError::usage("Pass NAME or --artifact <PATH>")),
    }
}

/// Package a UDF directory into a deterministic ZIP: the shared walk fixes
/// the entry order and exclusions (for `native`, only `amd64/` and
/// `arm64/`), timestamps are constant, and Unix permission bits are kept so
/// a native entrypoint stays executable.
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

/// What one poll showed.
enum Outcome {
    Done,
    Pending(String),
    Failed(String),
}

/// Poll `fetch` every [`UDF_POLL_INTERVAL`] until `classify` says the wait
/// is over. Progress lines (state transitions only) go to stderr in human
/// mode; the caller prints the final object exactly once. `follow_up` is the
/// command a timeout points at.
async fn wait_for<T, Fut, F, C>(
    what: String,
    timeout: Duration,
    verbose: bool,
    follow_up: &str,
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
            return Err(wait_timeout_error(&what, timeout, follow_up));
        }
        tokio::time::sleep(UDF_POLL_INTERVAL.min(timeout - elapsed)).await;
    }
}

fn wait_timeout_error(what: &str, timeout: Duration, follow_up: &str) -> CloudError {
    CloudError::new(format!(
        "{what} did not finish within {}s; it may still complete, check `{follow_up}`",
        timeout.as_secs()
    ))
    .with_failure(ApiFailure::new(FailureKind::Timeout))
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
        &format!("clickhousectl cloud service get {service}"),
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

/// Attach once; with `wake`, an idle service is woken (or an awaking one
/// waited for) and the attach retried once. Returns as soon as the API
/// accepts the attachment; it does not wait for `deployed`.
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
            // service is in a state the wait would fail on anyway.
            let state = client.get_service(org, service).await?.state;
            if let Outcome::Failed(_) = classify_wake_state(state.as_ref())? {
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
    use crate::cloud::client::CloudErrorKind;
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

    fn write_python_dir(dir: &Path) {
        std::fs::create_dir_all(dir.join("lib/__pycache__")).unwrap();
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        let mut udf = definition("executable", true);
        udf["runtime"] = json!("python3.11");
        std::fs::write(dir.join("udf.json"), udf.to_string()).unwrap();
        std::fs::write(dir.join("main.py"), "import sys\n").unwrap();
        std::fs::write(dir.join("lib/helper.py"), "x = 1\n").unwrap();
        std::fs::write(dir.join("lib/__pycache__/helper.pyc"), "").unwrap();
        std::fs::write(dir.join(".env"), "SECRET=1\n").unwrap();
        std::fs::write(dir.join("bin/tool"), "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(dir.join("bin/tool"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }

    fn write_native_dir(dir: &Path) {
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("udf.json"),
            definition("executable", true).to_string(),
        )
        .unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\n").unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}\n").unwrap();
        for arch in ["amd64", "arm64"] {
            std::fs::create_dir_all(dir.join(arch)).unwrap();
            std::fs::write(dir.join(arch).join("main"), "binary\n").unwrap();
            std::fs::set_permissions(
                dir.join(arch).join("main"),
                std::fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
        std::fs::write(dir.join("amd64/model.bin"), "weights\n").unwrap();
    }

    fn archive_names(file: &tempfile::NamedTempFile) -> Vec<String> {
        let bytes = std::fs::read(file.path()).unwrap();
        assert!(bytes.starts_with(b"PK\x03\x04"));
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().to_string())
            .collect()
    }

    #[test]
    fn package_source_dir_is_deterministic_and_applies_the_shared_walk() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("my_udf");
        write_python_dir(&dir);

        let first = package_source_dir(&dir, UdfRuntimeKind::Python311).unwrap();
        let second = package_source_dir(&dir, UdfRuntimeKind::Python311).unwrap();
        assert_eq!(
            std::fs::read(first.path()).unwrap(),
            std::fs::read(second.path()).unwrap()
        );
        assert_eq!(
            archive_names(&first),
            ["bin/", "bin/tool", "lib/", "lib/helper.py", "main.py"]
        );
        let mut archive = zip::ZipArchive::new(std::fs::File::open(first.path()).unwrap()).unwrap();
        let tool = archive.by_name("bin/tool").unwrap();
        assert_eq!(tool.unix_mode().unwrap() & 0o111, 0o111);
        drop(tool);
        let mut main = archive.by_name("main.py").unwrap();
        let mut contents = String::new();
        std::io::Read::read_to_string(&mut main, &mut contents).unwrap();
        assert_eq!(contents, "import sys\n");
    }

    #[test]
    fn native_archives_hold_only_the_architecture_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("my_udf");
        write_native_dir(&dir);
        let archive = package_source_dir(&dir, UdfRuntimeKind::Native).unwrap();
        assert_eq!(
            archive_names(&archive),
            [
                "amd64/",
                "amd64/main",
                "amd64/model.bin",
                "arm64/",
                "arm64/main"
            ]
        );
        let mut zip = zip::ZipArchive::new(std::fs::File::open(archive.path()).unwrap()).unwrap();
        assert_eq!(
            zip.by_name("arm64/main").unwrap().unix_mode().unwrap() & 0o111,
            0o111
        );

        std::fs::remove_file(dir.join("arm64/main")).unwrap();
        let error = package_source_dir(&dir, UdfRuntimeKind::Native).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("is missing amd64/main or arm64/main"),
            "{error}"
        );
    }

    #[test]
    fn package_source_dir_rejects_symlinks_and_missing_entrypoints_without_an_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("my_udf");
        write_python_dir(&dir);
        std::fs::remove_file(dir.join("main.py")).unwrap();
        let error = package_source_dir(&dir, UdfRuntimeKind::Python311).unwrap_err();
        assert!(error.to_string().contains("is missing main.py"), "{error}");

        std::fs::write(dir.join("main.py"), "").unwrap();
        std::os::unix::fs::symlink(dir.join("lib/helper.py"), dir.join("link.py")).unwrap();
        let error = package_source_dir(&dir, UdfRuntimeKind::Python311).unwrap_err();
        assert!(error.to_string().contains("symbolic link"), "{error}");

        let error =
            package_source_dir(&tmp.path().join("missing"), UdfRuntimeKind::Native).unwrap_err();
        assert!(error.to_string().contains("is not a directory"), "{error}");
    }

    #[test]
    fn sources_resolve_under_dir_and_definitions_prefer_file() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path().join("udfs");
        let dir = parent.join("my_udf");
        write_python_dir(&dir);
        let dir_arg = UdfDirArg {
            dir: parent.clone(),
        };
        assert_eq!(resolve_source(&dir_arg, "my_udf").unwrap(), dir);
        let missing = resolve_source(&dir_arg, "nope").unwrap_err();
        assert_eq!(missing.kind, CloudErrorKind::Usage);
        assert!(
            missing.to_string().ends_with("is not a directory"),
            "{missing}"
        );
        std::os::unix::fs::symlink(&dir, parent.join("linked")).unwrap();
        let linked = resolve_source(&dir_arg, "linked").unwrap_err();
        assert_eq!(linked.kind, CloudErrorKind::Usage);
        assert!(
            linked.to_string().ends_with("is a symbolic link"),
            "{linked}"
        );

        let override_path = tmp.path().join("override.json");
        std::fs::write(
            &override_path,
            definition("executable_pool", true).to_string(),
        )
        .unwrap();
        let from_dir = definition_source(None, Some(&dir)).unwrap();
        assert_eq!(from_dir["runtime"], "python3.11");
        let from_file =
            definition_source(Some(override_path.to_str().unwrap()), Some(&dir)).unwrap();
        assert_eq!(from_file["type"], "executable_pool");
        assert_eq!(
            definition_source(None, None).unwrap_err().kind,
            CloudErrorKind::Usage
        );

        assert!(check_definition_name(&from_dir, Some(&dir), "my_udf").is_ok());
        let mismatch = check_definition_name(&from_dir, Some(&dir), "other").unwrap_err();
        assert_eq!(mismatch.kind, CloudErrorKind::Usage);
        assert_eq!(
            mismatch.to_string(),
            format!(
                "{}: functionName is my_udf, but the command targets other",
                dir.join("udf.json").display()
            )
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
    fn wake_states_are_classified_closed_and_timeouts_name_their_follow_up() {
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
        assert_eq!(
            outcome(classify_wake_state(Some(&ServiceState::Unknown(
                "warming".into()
            )))),
            "pending:warming"
        );
        assert!(
            outcome(classify_wake_state(Some(&ServiceState::Stopped)))
                .starts_with("failed:the service is stopped")
        );
        assert!(outcome(classify_wake_state(None)).starts_with("error:"));

        let timeout = wait_timeout_error(
            "the wake of service svc-1",
            Duration::from_secs(7),
            "clickhousectl cloud service get svc-1",
        );
        assert_eq!(
            timeout.to_string(),
            "the wake of service svc-1 did not finish within 7s; it may still complete, check `clickhousectl cloud service get svc-1`"
        );
        assert_eq!(timeout.failure.unwrap().kind, FailureKind::Timeout);
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
    fn udf_name_dir_and_wake_flags_parse_and_conflict() {
        use clap::error::ErrorKind;
        let UdfCommands::Create(input) = parse_udf(&["create", "my_udf"]).command else {
            panic!("create");
        };
        assert_eq!(input.name.as_deref(), Some("my_udf"));
        assert_eq!(input.dir.dir, PathBuf::from("clickhouse/udfs"));
        assert!(input.config.is_none());
        assert!(input.artifact.is_none());

        let UdfCommands::Create(input) =
            parse_udf(&["create", "my_udf", "--dir", "../shared/udfs"]).command
        else {
            panic!("create");
        };
        assert_eq!(input.dir.dir, PathBuf::from("../shared/udfs"));
        assert!(input.config.is_none());

        let UdfCommands::Create(input) =
            parse_udf(&["create", "--file", "def.json", "--artifact", "code.zip"]).command
        else {
            panic!("create");
        };
        assert!(input.name.is_none());
        assert_eq!(input.artifact, Some(PathBuf::from("code.zip")));

        let UdfCommands::Version {
            command: UdfVersionCommands::Create { input, name },
        } = parse_udf(&["version", "create", "my_udf", "--dir", "funcs"]).command
        else {
            panic!("version create");
        };
        assert_eq!(name.function_name, "my_udf");
        assert_eq!(input.dir.dir, PathBuf::from("funcs"));
        assert!(input.artifact.is_none());

        let UdfCommands::Attach { wake, version, .. } =
            parse_udf(&["attach", "my_udf", "svc-1", "--wake"]).command
        else {
            panic!("attach");
        };
        assert!(wake);
        assert_eq!(version, None);
        let UdfCommands::Attach { wake, .. } = parse_udf(&["attach", "my_udf", "svc-1"]).command
        else {
            panic!("attach");
        };
        assert!(!wake);

        for (args, kind) in [
            (
                vec!["create", "my_udf", "--artifact", "z.zip"],
                ErrorKind::ArgumentConflict,
            ),
            (
                vec!["create", "--dir", "d", "--file", "f", "--artifact", "z.zip"],
                ErrorKind::ArgumentConflict,
            ),
            (
                vec!["create", "--file", "f"],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                vec!["create", "--artifact", "z.zip"],
                ErrorKind::MissingRequiredArgument,
            ),
            (vec!["create"], ErrorKind::MissingRequiredArgument),
            (vec!["create", "../x"], ErrorKind::ValueValidation),
            (
                vec!["version", "create", "my_udf", "--artifact", "z.zip"],
                ErrorKind::MissingRequiredArgument,
            ),
            (
                vec![
                    "version",
                    "create",
                    "my_udf",
                    "--dir",
                    "d",
                    "--artifact",
                    "z",
                    "--file",
                    "f",
                ],
                ErrorKind::ArgumentConflict,
            ),
            (
                vec!["create", "my_udf", "--file", "f.json"],
                ErrorKind::ArgumentConflict,
            ),
            (
                vec!["version", "create", "my_udf", "--file", "f.json"],
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
            (vec!["create", "my_udf"], true),
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
            (vec!["version", "create", "my_udf"], true),
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
                UdfCommands::Create(input) if input.name.is_none() => {
                    assert_eq!(input.config.as_deref(), Some("-"));
                    assert_eq!(input.artifact, Some(PathBuf::from("code.zip")));
                }
                UdfCommands::Attach { version, wake, .. } => {
                    assert!(version == Some(2) || wake);
                }
                UdfCommands::Version {
                    command: UdfVersionCommands::Create { input, .. },
                } if input.artifact.is_some() => {
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
